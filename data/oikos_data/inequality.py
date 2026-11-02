"""House-size Gini coefficients: wealth inequality among the households of dated
archaeological sites, the record of when inequality rose (ADR-0074).

One Parquet table feeds the house-size observation operator (docs/05 §5.1): one row per
site in the region (site, period, best date, Gini coefficient of its house floor areas,
its 80% bootstrap interval where the authors computed one, the number of houses
measured, subsistence, location and cell). The source is Supplementary Table 2 of Kohler
et al. 2017, an Excel 97 spreadsheet in the author manuscript's supplement at Europe
PMC. It gives no coordinates, so the region's sites are located by those of their
radiocarbon samples in p3k14c ([`SITES`]). Its licence forbids passing it on, so the
table is built locally and never committed or packed.
"""

import hashlib
import io
import json
import zipfile

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests
from .raster import MissingInputs, cache_dir

MANIFEST = DATA / "manifests" / "kohler2017-house-ginis.toml"

# Sites of the region in the table, at the coordinates of their radiocarbon samples in
# p3k14c (Bird et al. 2022): Jerf el Ahmar; Catalhoyuk East; Sabi Abyad I, whose Burnt
# Village is measured; Tepe Gawra ("Gawra", located only roughly); Tell Brak.
SITES = {
    "Jerf el Ahmar": (36.39, 38.20),
    "Çatalhöyük": (37.666, 32.826),
    "Tell Sabi Abyad Burnt Village": (36.504, 39.093),
    "Tepe Gawra": (36.43, 43.34),
    "Tell Brak Level 16": (36.668, 41.059),
}


def require_local():
    """Refuses, always: the licence allows no pack or committed file to contain the table."""
    manifests.require_redistributable(manifests.load(MANIFEST))


def _optional(x):
    return None if x in ("", None) else float(x)


def build(region, sources, rows):
    """Parquet bytes of the sites of `SITES` in `region`, from the spreadsheet's `rows`
    (dicts by column name). Pure."""
    cells = region.cells()
    found = {r["Site"].strip(): r for r in rows if r.get("World") == "OW" and r["Site"].strip() in SITES}
    if missing := [s for s in SITES if s not in found]:
        raise ValueError(f"house-size table lacks {missing}; has the supplement changed?")
    out = {k: [] for k in ("site", "period", "year", "gini", "lower", "upper", "houses", "adaptation", "basis", "lat",
                           "lng", "cell", "h3")}
    for site, (lat, lng) in SITES.items():
        r = found[site]
        cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
        pos = int(np.searchsorted(cells, cell))
        if pos >= len(cells) or cells[pos] != cell:
            continue
        for k, v in (("site", site), ("period", r["Period"]), ("year", int(r["Date_AD"])), ("gini", float(r["Gini"])),
                     ("lower", _optional(r["Lower_B"])), ("upper", _optional(r["Upper_B"])),
                     ("houses", int(r["Households_in_sample"])), ("adaptation", r["Adaptation"]),
                     ("basis", r["Basis"]), ("lat", lat), ("lng", lng), ("cell", pos), ("h3", cell)):
            out[k].append(v)
    order = np.argsort(out["year"], kind="stable")
    out = {k: [v[i] for i in order] for k, v in out.items()}
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    types = {"year": pa.int32(), "gini": pa.float64(), "lower": pa.float64(), "upper": pa.float64(),
             "houses": pa.uint32(), "lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def read_sheet(xls, sheet):
    """Rows of `sheet` of an Excel 97 workbook, as dicts by the header row's names."""
    import xlrd

    s = xlrd.open_workbook(file_contents=xls).sheet_by_name(sheet)
    header = [str(c.value).strip() for c in s.row(0)]
    return [dict(zip(header, (c.value for c in s.row(i)))) for i in range(1, s.nrows)]


def _spreadsheet(access, offline):
    """The spreadsheet's bytes, cached once its SHA-256 matches the manifest's (Europe PMC
    builds the supplement archive on request, so only the member is checked)."""
    path = cache_dir() / "downloads" / access["member"]
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == access["sha256"]:
        return path.read_bytes()
    if offline:
        raise MissingInputs(f"{access['member']} is not downloaded")
    import ssl
    import urllib.request

    import certifi

    ctx = ssl.create_default_context(cafile=certifi.where())
    with urllib.request.urlopen(access["url"], timeout=120, context=ctx) as r:
        archive = r.read()
    xls = zipfile.ZipFile(io.BytesIO(archive)).read(access["member"])
    if (got := hashlib.sha256(xls).hexdigest()) != access["sha256"]:
        raise ValueError(f"{access['member']}: SHA-256 {got} does not match the manifest's; has the supplement changed?")
    path.parent.mkdir(parents=True, exist_ok=True)
    part = path.with_name(path.name + ".part")
    part.write_bytes(xls)
    part.replace(path)
    return xls


def inputs(offline=False):
    """(sources, spreadsheet rows) from the cache or Europe PMC."""
    m = manifests.load(MANIFEST)
    a = m["access"]
    rows = read_sheet(_spreadsheet(a, offline), a["sheet"])
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return sources, rows


def build_table(region, offline=False):
    """The site table for `region`; local only (the licence forbids passing it on)."""
    return build(region, *inputs(offline))
