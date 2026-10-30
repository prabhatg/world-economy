"""Survey sites: dated settlement phases from archaeological surface surveys, the record
of where and when villages stood (docs/05 §5.1, site counts).

One Parquet table feeds the site-count observation operator: one row per site phase in
the region (survey window, site id within the window, location, cell, start and end
calendar years of the phase's cultural period, size in hectares or NaN when unknown,
and the source survey). The source is the survey compilation of Palmisano et al. 2021
(Central Anatolia and Upper Mesopotamia): one point layer per window, read with GDAL
from the archive.
"""

import io
import json

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests, vector

COLUMNS = ["Id", "StartDate", "EndDate", "Longitude", "Latitude", "SizeHa", "Source"]


def _id(v):
    return str(int(v)) if isinstance(v, (int, float, np.integer, np.floating)) else str(v)


def build(region, sources, layers):
    """Parquet bytes of the site phases in `region`; `layers` maps each survey window to
    its columns ({COLUMNS name: array}). Pure."""
    cells = region.cells()
    out = {k: [] for k in ("window", "site", "lat", "lng", "cell", "h3", "start_year", "end_year", "size_ha", "source")}
    for window in sorted(layers):
        c = layers[window]
        start, end = (np.asarray(c[k], dtype=float) for k in ("StartDate", "EndDate"))
        bad = ~(np.isfinite(start) & np.isfinite(end) & (start < end))
        if bad.any():
            raise ValueError(f"{window}: {bad.sum()} site phases without dates or ending before they start")
        for i in range(len(start)):
            lat, lng = float(c["Latitude"][i]), float(c["Longitude"][i])
            cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
            pos = int(np.searchsorted(cells, cell))
            if pos >= len(cells) or cells[pos] != cell:
                continue
            size = float(c["SizeHa"][i])
            for k, v in (("window", window), ("site", _id(c["Id"][i])), ("lat", lat), ("lng", lng), ("cell", pos),
                         ("h3", cell), ("start_year", int(start[i])), ("end_year", int(end[i])),
                         ("size_ha", size if size > 0 else float("nan")), ("source", str(c["Source"][i]))):
                out[k].append(v)
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    types = {"lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64(),
             "start_year": pa.int32(), "end_year": pa.int32(), "size_ha": pa.float64()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def _read_layer(archive, member):
    import pyogrio.raw

    meta, _, _, values = pyogrio.raw.read(vector.path_in_zip(archive, member), columns=COLUMNS)
    names = list(meta["fields"])
    return {k: np.asarray(values[names.index(k)]) for k in COLUMNS}


def inputs(offline=False):
    """(sources, window -> columns) from the cached archive or the web."""
    m = manifests.load(DATA / "manifests" / "palmisano2021-sites.toml")
    manifests.require_redistributable(m)
    a = m["access"]
    archive = vector.download(a["url"], a["file"], a["md5"], offline=offline)
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256}]
    return sources, {w: _read_layer(archive, member) for w, member in a["layers"].items()}


def build_table(region, update=False, offline=False):
    return build(region, *inputs(offline))
