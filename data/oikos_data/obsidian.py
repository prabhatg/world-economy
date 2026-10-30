"""Obsidian: where the volcanoes of each traced source group are, and which sources the
sourced artefacts of dated site contexts came from (docs/05 §5.1, obsidian
distance-decay; ADR-0060).

Two outputs from DObsiSS (Batist 2016):

- an `obsidian` forcing pack with one field per source group (manifest order): 1 in
  the cells within one step of a volcano of the group (a day's walk at res 5, the
  quarrying reach), 0 elsewhere; the engine passes obsidian down the line from them;
- a Parquet table, one row per site context in the region with an ASPRO period that
  has dates: context id, site, location, cell, period and its start and end calendar
  years ("a or b" spans both), the artefacts assigned to each group, to other sources
  ("other"), and the characterization study.
"""

import csv
import io
import json
import zipfile

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests, pack, vector


def _locate(region, lat, lng):
    """Position of the cell holding (lat, lng) among the region's cells, or None."""
    cells = region.cells()
    cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
    pos = int(np.searchsorted(cells, cell))
    return (pos, cell) if pos < len(cells) and cells[pos] == cell else None


def build_pack(region, sources, volcanoes, groups):
    """Obsidian pack bytes: `volcanoes` are rows of the sources table (Latitude,
    Longitude, Region); `groups` the manifest's source groups. Pure."""
    cells = region.cells()
    b = pack.Builder("obsidian", pack.region(region.name, region.resolution, cells))
    for s in sources:
        b.source(**{k: s[k] for k in ("dataset", "version", "license", "citation", "manifest_sha256")},
                 attribution=s.get("attribution"))
    for g in groups:
        mine = [v for v in volcanoes if v["Region"] == g["region"]]
        if not mine:
            raise ValueError(f"source group {g['name']}: no volcanoes of region {g['region']!r} in the sources table")
        reach = {h3.str_to_int(c) for v in mine
                 for c in h3.grid_disk(h3.latlng_to_cell(float(v["Latitude"]), float(v["Longitude"]), region.resolution), 1)}
        values = np.array([int(c) in reach for c in cells], dtype=np.uint8)
        b.field(g["name"], "1", f"Cells within one step of a {g['name']} obsidian source "
                f"({', '.join(v['Name'] for v in mine)})", values)
    return b.build()


def _years(period, chronology):
    """(start, end) calendar years of an ASPRO period, "a or b" / "a and b" spanning both;
    None if a bound is undated (period 0 has no start) or the period is "x"."""
    if period.strip() == "x":
        return None
    parts = period.replace(" and ", " or ").split(" or ")
    if any(p.strip() not in chronology for p in parts):
        raise ValueError(f"unknown ASPRO period {period!r}")
    begin, end = chronology[parts[0].strip()][0], chronology[parts[-1].strip()][1]
    return None if begin is None or end is None else (-int(begin), -int(end))


def build_table(region, sources, rows, chronology, groups):
    """Parquet bytes of the sourced contexts in `region`: `rows` are the database's rows,
    `chronology` maps ASPRO period to (begin, end) cal BC (None if open). Pure."""
    grouped = {c for g in groups for c in g["columns"]}
    out = {k: [] for k in ("context", "site", "lat", "lng", "cell", "h3", "period", "start_year", "end_year",
                           *(g["name"] for g in groups), "other", "source")}
    for r in rows:
        years = _years(r["ASPRO"], chronology)
        where = _locate(region, float(r["Latitude"]), float(r["Longitude"]))
        if years is None or where is None:
            continue
        counts = {k: int(float(v or 0)) for k, v in r.items() if k in grouped or k not in _FIXED}
        for k, v in (("context", r["Site ID"]), ("site", r["Site"]), ("lat", float(r["Latitude"])),
                     ("lng", float(r["Longitude"])), ("cell", where[0]), ("h3", where[1]), ("period", r["ASPRO"]),
                     ("start_year", years[0]), ("end_year", years[1]),
                     *((g["name"], sum(counts[c] for c in g["columns"] if c in counts)) for g in groups),
                     ("other", sum(v for k, v in counts.items() if k not in grouped)),
                     ("source", r["Chemical Characterization"])):
            out[k].append(v)
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(region.cells())}).encode()}
    types = {"lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64(), "start_year": pa.int32(),
             "end_year": pa.int32(), "other": pa.uint32(), **{g["name"]: pa.uint32() for g in groups}}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


# Database columns that are not artefact counts.
_FIXED = {"Site ID", "Site", "ASPRO", "Country", "Region", "Latitude", "Longitude", "Context", "Notes", "Citations",
          "Chemical Characterization", "Laboratory", "Analytical Methods", "Obsidian analyzed"}


def _csv(archive, member, encoding="utf-8"):
    with zipfile.ZipFile(archive) as z:
        return list(csv.DictReader(io.StringIO(z.read(member).decode(encoding), newline="")))


def inputs(offline=False):
    """(sources, manifest access, archive path) from the cache or the web."""
    m = manifests.load(DATA / "manifests" / "dobsiss.toml")
    manifests.require_redistributable(m)
    a = m["access"]
    archive = vector.download(a["url"], a["file"], a["md5"], offline=offline)
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return sources, a, archive


def build_obsidian_pack(region, update=False, offline=False):
    sources, a, archive = inputs(offline)
    return build_pack(region, sources, _csv(archive, a["sources"], a["sources_encoding"]), a["groups"])


def build_obsidian_table(region, update=False, offline=False):
    sources, a, archive = inputs(offline)
    chronology = {r["Period"]: (r["Begin (cal. BC)"] or None, r["End (cal. BC)"] or None)
                  for r in _csv(archive, a["chronology"])}
    return build_table(region, sources, _csv(archive, a["table"], a["table_encoding"]), chronology, a["groups"])
