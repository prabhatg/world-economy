"""The M1 obsidian pack and sourced assemblages against what is known of Neolithic
obsidian (runs when the DObsiSS archive is cached: `python -m oikos_data build obsidian
m1-swasia`)."""

import io

import h3
import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import DATA, obsidian, pack, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        p = pack.read(obsidian.build_obsidian_pack(r, offline=True))
        t = pq.read_table(io.BytesIO(obsidian.build_obsidian_table(r, offline=True))).to_pydict()
    except raster.MissingInputs:
        pytest.skip("DObsiSS archive not cached; run: python -m oikos_data build obsidian m1-swasia")
    return r, p, {k: np.asarray(v) for k, v in t.items()}


def cell_of(r, lat, lng):
    return list(r.cells()).index(h3.str_to_int(h3.latlng_to_cell(lat, lng, r.resolution)))


def test_the_pack_reaches_the_known_volcanoes(m1):
    r, p, _ = m1
    assert [f["name"] for f in p.header["fields"]] == ["cappadocia", "eastern_anatolia"]
    assert p.raw("cappadocia")[cell_of(r, 38.26, 34.55)] == 1, "Göllü Dağ"
    assert p.raw("eastern_anatolia")[cell_of(r, 38.65, 42.23)] == 1, "Nemrut Dağ"
    assert p.raw("cappadocia")[cell_of(r, 31.87, 35.44)] == 0, "not Jericho"
    assert 20 <= p.raw("cappadocia").sum() <= 28 and 30 <= p.raw("eastern_anatolia").sum() <= 49


def test_assemblages_show_the_known_supply_zones(m1):
    # Renfrew, Dixon and Cann (1966): the Levant got Cappadocian obsidian, the Zagros and
    # Upper Tigris obsidian from the Lake Van area.
    _, _, t = m1
    assert 150 <= len(t["context"]) <= 200
    assert (t["start_year"] < t["end_year"]).all() and t["start_year"].min() >= -12500 and t["end_year"].max() <= -3000

    def share(where):
        c, e = t["cappadocia"][where].sum(), t["eastern_anatolia"][where].sum()
        return c / (c + e)

    levant = (t["lat"] < 34.0) & (t["lng"] < 37.0)
    assert share(levant & (t["end_year"] <= -6900)) > 0.85, "the PPN Levant"
    assert share(levant & (t["start_year"] >= -6900)) < 0.7, "eastern obsidian reaches the later Levant"
    assert share(t["lng"] > 43.0) < 0.1, "Zagros and beyond"
    assert 0.2 < share((t["lng"] > 37.5) & (t["lng"] < 40.5)) < 0.8, "Middle Euphrates and Jezireh mix both"


def test_an_m1_world_passes_obsidian_down_the_line(m1):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    r, p, _ = m1
    paths = [DATA / "build" / f"m1-swasia.{k}.oikospack" for k in ("geometry", "climate", "coastline", "hydrology", "obsidian")]
    if missing := [x.name for x in paths if not x.exists()]:
        pytest.skip(f"M1 packs not built: {missing}")
    w = oikos.World.from_packs([str(x) for x in paths], seed=1)
    w.run_years(300, parts=4)
    o = w.obsidian()
    assert list(o) == ["cappadocia", "eastern_anatolia"]
    occupied = w.population() > 0
    for name, (lat, lng) in (("cappadocia", (38.26, 34.55)), ("eastern_anatolia", (38.65, 42.23))):
        assert o[name][cell_of(r, lat, lng)] == 1.0 or not occupied[cell_of(r, lat, lng)]
        assert ((o[name] >= 0) & (o[name] <= 1)).all() and (o[name][~occupied] == 0).all()
        assert (o[name][occupied] > 0).mean() > 0.2, f"{name} obsidian has travelled"
