"""Decadal climate anomalies (#41): what century means smooth away, such as the 8.2 ka
event.

CHELSA-TraCE21k's slices are century means of one model run (ADR-0042), so events
shorter than a few centuries are missing, and the model's own centennial swings are not
the real ones. A `climate_anomaly` pack holds a regional index over time from a proxy
record: the record's high-pass part (each sample less the line fitted through the
samples within half a window of it), so the century-scale signal, which the climate pack carries, is
not counted twice. The engine shifts every cell's temperature and scales its rain by
the index, at parameters the records constrain (`climate_anomaly_temp`,
`climate_anomaly_precip`).
"""

import numpy as np

from . import DATA, manifest as manifests, pack, vector

SCALE = 0.001  # index units per stored step


def calendar(ages_bp, values):
    """(calendar years, values): ages in years before 1950 rounded to whole years, in
    ascending order, with the values of samples falling in the same year averaged."""
    years = 1950 - np.rint(np.asarray(ages_bp, dtype=float)).astype(np.int64)
    out, inverse = np.unique(years, return_inverse=True)
    sums = np.bincount(inverse, weights=np.asarray(values, dtype=float))
    return out, sums / np.bincount(inverse)


def high_pass(years, values, window):
    """Each value less the least-squares line through the values whose years lie within
    `window` / 2 of its own, at its year: what varies faster than the window. A line, not
    a mean, so a steep trend (the early Holocene's warming) leaves nothing at the
    record's ends, where the window is one-sided. A sample alone in its window is 0."""
    years, values = np.asarray(years, dtype=float), np.asarray(values, dtype=float)
    out = np.zeros_like(values)
    for i, y in enumerate(years):
        near = np.abs(years - y) <= window / 2
        x, v = years[near] - y, values[near]
        if near.sum() < 2:
            continue
        dx = x - x.mean()
        slope = (dx @ (v - v.mean())) / (dx @ dx)
        out[i] = values[i] - (v.mean() - slope * x.mean())
    return out


def parse_pangaea(text, column):
    """(ages in years BP, values) of `column` from a PANGAEA tab-separated export, whose
    age column is in ka BP."""
    body = text.split("*/", 1)[1].strip().splitlines()
    names = body[0].split("\t")
    if column not in names:
        raise ValueError(f"the export has no column {column!r} (has {names})")
    rows = np.array([[float(x) for x in line.split("\t")] for line in body[1:]])
    return rows[:, names.index("Age [ka BP]")] * 1000.0, rows[:, names.index(column)]


def index_from(ages_bp, values, start_bp, window):
    """(calendar years, index): the high-pass of the samples no older than `start_bp`,
    with a 0 one sample step before and after, so the index is 0 outside the record."""
    keep = np.asarray(ages_bp) <= start_bp
    years, v = calendar(np.asarray(ages_bp)[keep], np.asarray(values)[keep])
    step = int(np.median(np.diff(years)))
    index = high_pass(years, v, window)
    return np.r_[years[0] - step, years, years[-1] + step], np.r_[0.0, index, 0.0]


def build_pack(region, sources, years, index, description):
    """Climate-anomaly pack bytes: `index` at calendar `years` (ascending), linear between.
    Pure."""
    cells = region.cells()
    bld = pack.Builder("climate_anomaly", pack.region(region.name, region.resolution, cells))
    bld.time(years, "linear")
    for src in sources:
        bld.source(**{k: src[k] for k in ("dataset", "version", "license", "citation", "manifest_sha256")},
                   attribution=src.get("attribution"))
    stored = np.rint(np.asarray(index, dtype=float) / SCALE)
    if np.abs(stored).max() > 32_767:
        raise ValueError(f"index beyond +-{32_767 * SCALE}: rescale it")
    bld.field("index", "1", description, stored.astype(np.int16), scale=SCALE, series=True)
    return bld.build()


def inputs(offline=False):
    """(sources, years, index, description) from the cache or the web."""
    m = manifests.load(DATA / "manifests" / "ngrip-gicc05-20yr.toml")
    manifests.require_redistributable(m)
    acc = m["access"]
    path = vector.download(acc["url"], acc["file"], acc["md5"], offline=offline)
    ages, d18o = parse_pangaea(path.read_text(encoding="utf-8"), "δ18O H2O [‰ SMOW]")
    years, index = index_from(ages, d18o, acc["start_bp"], acc["window_years"])
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    description = (f"NGRIP 20-year mean d18O (permil) less the line through the means within {acc['window_years'] // 2} "
                   f"years of each (GICC05, years BP to calendar years), from {acc['start_bp']} BP; 0 outside")
    return sources, years, index, description


def build_anomaly_pack(region, update=False, offline=False):
    return build_pack(region, *inputs(offline))
