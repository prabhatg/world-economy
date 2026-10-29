"""Radiocarbon observations: p3k14c dates in a region, calibrated with IntCal20.

Two Parquet tables feed the radiocarbon-density observation operator (docs/05 §5.1):

- dates: one row per date in the region (lab id, 14C age and error, material, taxa,
  site, source dataset, location accuracy, cell, calibrated median year, and the share
  of its calibrated probability inside the window). Repeated lab ids (the same date
  compiled by several databases) keep their first row; dates without a positive error
  are dropped.
- density: each date's calibrated probability summed per cell and century of the window
  (calendar years as in the engine: negative is BCE, year = 1950 - cal BP), counting
  only dates located within 500 m or exactly (p3k14c LocAccuracy 2 or 3); province-level
  locations are too coarse for ~17 km cells.

Calibration is the standard one: the likelihood of each calendar year given the
measurement, with the curve's own error added in quadrature, on an annual grid
interpolated linearly from IntCal20, normalized to one. Taphonomic loss and research
intensity belong to the observation operator, not here.
"""

import hashlib
import io
import json
import zipfile
from dataclasses import dataclass

import h3
import numpy as np
import pyarrow as pa
import pyarrow.csv as pacsv
import pyarrow.parquet as pq

from . import DATA, manifest as manifests, raster, vector

INTCAL20_URL = "https://intcal.org/curves/intcal20.14c"
MASS_DECIMALS = 6  # stored probabilities are rounded so rebuilds give the same bytes
COLUMNS = ["LabID", "Age", "Error", "Material", "Taxa", "Long", "Lat", "LocAccuracy", "SiteName", "Country", "Source"]


@dataclass
class Curve:
    cal_bp: np.ndarray  # ascending
    c14: np.ndarray
    sigma: np.ndarray


def load_curve(text):
    rows = [line.split(",") for line in text.splitlines() if line.strip() and not line.startswith("#")]
    a = np.array([[float(x) for x in r[:3]] for r in rows])
    order = np.argsort(a[:, 0])
    return Curve(a[order, 0], a[order, 1], a[order, 2])


def calibrate(age, error, curve):
    """Annual cal BP grid and the normalized calibrated probability of each year."""
    years = np.arange(curve.cal_bp[0], curve.cal_bp[-1] + 1)
    mu = np.interp(years, curve.cal_bp, curve.c14)
    s2 = error * error + np.interp(years, curve.cal_bp, curve.sigma) ** 2
    like = np.exp(-0.5 * (age - mu) ** 2 / s2) / np.sqrt(s2)
    return years, like / like.sum()


def read_dates(data):
    opts = pacsv.ConvertOptions(column_types={c: pa.string() for c in COLUMNS}, include_columns=COLUMNS)
    t = pacsv.read_csv(io.BytesIO(data), convert_options=opts).to_pydict()
    return [dict(zip(COLUMNS, r)) for r in zip(*(t[c] for c in COLUMNS))]


def _number(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return float("nan")


def build(region, sources, data, curve, window):
    """(dates, density) Parquet bytes for the p3k14c CSV `data`; pure, for testing.
    `window` is (first, end) calendar years, centuries from `first`."""
    cells = region.cells()
    first, end = window
    seen, rows = set(), []
    for d in read_dates(data):
        age, err, lat, lng = (_number(d[k]) for k in ("Age", "Error", "Lat", "Long"))
        lab = (d["LabID"] or "").strip()
        key = "".join(ch for ch in lab.upper() if ch.isalnum())
        if not key or key in seen or not (err > 0) or np.isnan(age) or np.isnan(lat) or np.isnan(lng):
            continue
        cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
        pos = int(np.searchsorted(cells, cell))
        if pos >= len(cells) or cells[pos] != cell:
            continue
        seen.add(key)
        rows.append((lab, age, err, pos, cell, lat, lng, d))

    years_bp = np.arange(curve.cal_bp[0], curve.cal_bp[-1] + 1)
    cal_year = 1950 - years_bp
    in_window = (cal_year >= first) & (cal_year < end)
    century = ((cal_year[in_window] - first) // 100).astype(np.int64)
    centuries = (end - first + 99) // 100
    out = {k: [] for k in ("lab_id", "c14_age", "c14_error", "material", "taxa", "site_name", "country", "source",
                           "loc_accuracy", "lat", "lng", "cell", "h3", "cal_median_year", "mass_in_window")}
    density, counts = {}, {}
    for lab, age, err, pos, cell, lat, lng, d in rows:
        _, p = calibrate(age, err, curve)
        median_bp = years_bp[min(int(np.searchsorted(np.cumsum(p), 0.5)), len(p) - 1)]
        mass = np.bincount(century, weights=p[in_window], minlength=centuries)
        acc = int(_number(d["LocAccuracy"])) if not np.isnan(_number(d["LocAccuracy"])) else -1
        for k, v in (("lab_id", lab), ("c14_age", age), ("c14_error", err), ("material", d["Material"] or ""),
                     ("taxa", d["Taxa"] or ""), ("site_name", d["SiteName"] or ""), ("country", d["Country"] or ""),
                     ("source", d["Source"] or ""), ("loc_accuracy", acc), ("lat", lat), ("lng", lng), ("cell", pos),
                     ("h3", cell), ("cal_median_year", int(1950 - median_bp)),
                     ("mass_in_window", round(float(mass.sum()), MASS_DECIMALS))):
            out[k].append(v)
        if acc >= 2:
            for c in np.nonzero(mass >= 10.0**-MASS_DECIMALS)[0]:  # not the Gaussian's far tails
                density[(pos, int(c))] = density.get((pos, int(c)), 0.0) + float(mass[c])
                counts[(pos, int(c))] = counts.get((pos, int(c)), 0) + 1

    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode(),
        b"window": json.dumps(list(window)).encode()}
    types = {"c14_age": pa.float64(), "c14_error": pa.float64(), "loc_accuracy": pa.int8(), "lat": pa.float64(),
             "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64(), "cal_median_year": pa.int32(),
             "mass_in_window": pa.float64()}
    dates = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    keys = sorted(k for k, v in density.items() if round(v, MASS_DECIMALS) > 0)
    spd = pa.table({
        "cell": pa.array([k[0] for k in keys], pa.uint32()),
        "h3": pa.array([int(cells[k[0]]) for k in keys], pa.uint64()),
        "century_start_year": pa.array([first + 100 * k[1] for k in keys], pa.int32()),
        "mass": pa.array([round(density[k], MASS_DECIMALS) for k in keys], pa.float64()),
        "dates": pa.array([counts[k] for k in keys], pa.uint32()),
    }).replace_schema_metadata(meta)
    return _parquet(dates), _parquet(spd)


def _parquet(table):
    buf = io.BytesIO()
    pq.write_table(table, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def inputs(offline=False):
    """(sources, p3k14c CSV bytes, IntCal20 curve, window) from the cache or the web."""
    pm = manifests.load(DATA / "manifests" / "p3k14c.toml")
    cm = manifests.load(DATA / "manifests" / "intcal20.toml")
    for m in (pm, cm):
        manifests.require_redistributable(m)
    a = pm["access"]
    archive = vector.download(a["url"], a["file"], a["md5"], offline=offline)
    with zipfile.ZipFile(archive) as z:
        data = z.read(a["member"])
    text = raster.read_text(cm["access"]["url"], offline=offline)
    if hashlib.sha256(text.encode()).hexdigest() != cm["access"]["sha256"]:
        raise ValueError("IntCal20 curve differs from the manifest's sha256; check upstream before updating it")
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"], "citation": m["citation"],
                "manifest_sha256": m.sha256} for m in (pm, cm)]
    return sources, data, load_curve(text), tuple(pm["access"]["window"])


def build_dates(region, update=False, offline=False):
    return build(region, *inputs(offline))[0]


def build_density(region, update=False, offline=False):
    return build(region, *inputs(offline))[1]
