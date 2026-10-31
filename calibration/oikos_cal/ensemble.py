"""Ensembles of plausible worlds for the viewer (ADR-0063).

History is one draw from many possible worlds, and the record leaves many parameter
sets consistent with it, so the viewer shows an ensemble, never one world: the
calibrated defaults and the best consistent worlds of a `best` record, each run as an
ordinary work unit by the `oikos` CLI with run outputs (`fields.zarr`), and a
`catalog.json` that lists them with the series the viewer charts. What the viewer maps
it reads from each world's own `fields.zarr`, so it shows exactly what the engine
recorded.

    python -m oikos_cal ensemble runs/m1/round6/best.json --out ../viewer/data/m1
"""

import json
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np

__all__ = ["choose", "work_unit", "series", "build"]

SHARE_ONE = 10_000  # recorded shares are parts per 10,000 (engine FIELD_ONE)

# What the viewer maps, in its order: name in fields.zarr, label, unit, what it shows.
FIELDS = [
    ("population", "People", "people", "People living in each cell"),
    ("cultivation", "Farming", "share", "How much of their food people grow (cultivation effort)"),
    ("soil", "Soil", "share", "Soil left of what the land started with"),
    ("obsidian_cappadocia", "Obsidian", "share", "Cappadocian obsidian in hand, falling off along settled land"),
]


def choose(best, k):
    """The calibrated defaults, then up to `k` consistent candidates of a `best` record,
    best first by their rerun log-likelihood."""
    worlds = [{"id": "w0", "label": "Calibrated defaults", "params": {}}]
    ok = sorted((c for c in best["candidates"] if c["consistent"]), key=lambda c: -c["rerun_log_likelihood"])
    for i, c in enumerate(ok[:k], start=1):
        worlds.append({"id": f"w{i}", "label": f"Wave {c['wave'] + 1} run {c['run']}", "params": c["params"]})
    return worlds


def work_unit(packs, params, seed, out, years, every):
    """A work unit running the world of `packs` ([{kind, id, path}]) from 12,000 BCE for
    `years` with `params` overrides, recording outputs every `every` years into `out`."""
    geometry = [p["id"] for p in packs if p["kind"] == "geometry"]
    if len(geometry) != 1:
        raise ValueError(f"a world needs exactly one geometry pack, got {len(geometry)}")
    return {
        "format": 1,
        "id": Path(out).name,
        "world": {"geometry": geometry[0]},
        "seed": seed,
        "params": params,
        "start": {"kind": "initial"},
        "years": years,
        "partitions": 1,
        "summary_every_years": 1000,
        "outputs": {"dir": out, "every_years": every, "panel_one_in": 4096, "time_chunk": 10, "cell_chunk": 8192},
        "inputs": [{"id": p["id"], "path": p["path"]} for p in packs],
    }


def series(fields):
    """People, the people-weighted share of food grown (cultivation effort) and the
    effort-weighted domestic share of the crop at each output time, from a world's
    `population`, `cultivation` and `domestication` fields [time, cell]."""
    pop = fields["population"].astype(float)
    effort = pop * fields["cultivation"] / SHARE_ONE
    crop = effort * fields["domestication"] / SHARE_ONE
    people, sown = pop.sum(axis=1), effort.sum(axis=1)
    ratio = lambda a, b: [float(x / y) if y > 0 else 0.0 for x, y in zip(a, b)]  # noqa: E731
    return {"people": [int(x) for x in people], "farming": ratio(sown, people), "domestic": ratio(crop.sum(axis=1), sown)}


def _packs():
    from oikos_data import pack

    from . import m1

    out = []
    for path in m1.packs():
        p = pack.read(Path(path).read_bytes())
        out.append({"kind": p.header["kind"], "id": p.id, "path": str(Path(path).resolve())})
    return out


def _years(store):
    import zarr

    ticks = zarr.open_group(store, mode="r")["tick"][:]
    return [int(t) // 365 - 12_000 for t in ticks]


def build(best_path, out, k=8, seed=1, years=9_000, every=100, oikos="../engine/target/release/oikos", workers=4):
    """Runs the ensemble and writes `out`/catalog.json; returns the catalog."""
    import zarr

    out = Path(out)
    best = json.loads(Path(best_path).read_text())
    worlds, packs = choose(best, k), _packs()
    out.mkdir(parents=True, exist_ok=True)

    def run(w):
        wu = work_unit(packs, w["params"], seed, str(out / w["id"]), years, every)
        path = out / f"{w['id']}.workunit.json"
        path.write_text(json.dumps(wu, indent=1) + "\n")
        subprocess.run([oikos, "run", str(path), "-o", str(out / f"{w['id']}.result.json")], check=True)

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(run, worlds))
    for w in worlds:
        group = zarr.open_group(str(out / w["id"] / "fields.zarr"), mode="r")
        w["series"] = series({n: group[n][:] for n in ("population", "cultivation", "domestication")})
        w["store"] = f"{w['id']}/fields.zarr"
        w["state_hash"] = json.loads((out / f"{w['id']}.result.json").read_text())["state_hash"]
    first = zarr.open_group(str(out / worlds[0]["id"] / "fields.zarr"), mode="r")
    present = [f for f in FIELDS if f[0] in first]
    catalog = {
        "format": 1,
        "title": "Neolithic Southwest Asia",
        "summary": "Plausible worlds from 12,000 to 3,000 BCE: each fits the radiocarbon, barley, survey and "
                   "obsidian records the calibration judged",
        "h3_resolution": int(first.attrs["h3_resolution"]),
        "years": _years(str(out / worlds[0]["id"] / "fields.zarr")),
        "fields": [{"name": n, "label": label, "unit": unit, "description": d} for n, label, unit, d in present],
        "worlds": worlds,
        "provenance": {"engine": first.attrs["engine"], "best": str(best_path), "waves": best.get("waves", []),
                       "seed": seed, "packs": [{"kind": p["kind"], "id": p["id"]} for p in packs]},
    }
    (out / "catalog.json").write_text(json.dumps(catalog, indent=1) + "\n")
    return catalog
