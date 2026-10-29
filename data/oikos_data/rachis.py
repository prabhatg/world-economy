"""Rachis observations: archaeobotanical cereal assemblages scored for shattering (wild)
and non-shattering (domestic) ears, the record of how domestication proceeded.

One Parquet table feeds the domestication-timing observation operator (docs/05 §5.1):
one row per assemblage in the region (crop, site and phase, location, cell, median
calendar year of the phase, share and count of non-shattering rachis among those
determined, the number excluded as indeterminate, and the primary source). The source
is the supplement of Allaby et al. 2017, a Word document; each crop's table is the one
that follows its caption.
"""

import io
import json
import zipfile
import xml.etree.ElementTree as ET

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests, vector

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
COLUMNS = ["Site", "Latitude", "Longitude", "median age (cal.BC)", "shattering %", "non-shattering %",
           "total n (det. Rachis)", "excluded indet. Rachis.", "primary source"]


def _text(e):
    return " ".join("".join(t.text or "" for t in e.iter(W + "t")).split())


def read_table(docx, caption):
    """Rows (lists of cell text) of the first table after the paragraph starting `caption`."""
    body = ET.fromstring(zipfile.ZipFile(io.BytesIO(docx)).read("word/document.xml")).find(W + "body")
    seen = False
    for el in body:
        if el.tag == W + "p" and _text(el).startswith(caption):
            seen = True
        elif el.tag == W + "tbl" and seen:
            return [[_text(c) for c in r.findall(W + "tc")] for r in el.findall(W + "tr")]
    raise ValueError(f"no table follows a paragraph starting {caption!r}; has the supplement changed?")


def _percent(s):
    return float(s.rstrip("%")) / 100


def _count(s):
    return 0 if s.strip() in ("-", "") else int(s)


def build(region, sources, docx, tables):
    """Parquet bytes of the assemblages in `region`; `tables` maps crop to caption. Pure."""
    cells = region.cells()
    out = {k: [] for k in ("crop", "site", "lat", "lng", "cell", "h3", "median_year", "nonshattering", "n_rachis",
                           "k_nonshattering", "excluded_indeterminate", "source")}
    for crop, caption in sorted(tables.items()):
        header, *rows = read_table(docx, caption)
        if header != COLUMNS:
            raise ValueError(f"{caption} columns are {header}, expected {COLUMNS}; has the supplement changed?")
        for site, lat, lng, age, _, non, n, excluded, source in rows:
            lat, lng = float(lat), float(lng)
            cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
            pos = int(np.searchsorted(cells, cell))
            if pos >= len(cells) or cells[pos] != cell:
                continue
            share, n = _percent(non), int(n)
            for k, v in (("crop", crop), ("site", site), ("lat", lat), ("lng", lng), ("cell", pos), ("h3", cell),
                         ("median_year", -int(age)), ("nonshattering", share), ("n_rachis", n),
                         ("k_nonshattering", int(round(share * n))), ("excluded_indeterminate", _count(excluded)),
                         ("source", source)):
                out[k].append(v)
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    types = {"lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64(), "median_year": pa.int32(),
             "nonshattering": pa.float64(), "n_rachis": pa.uint32(), "k_nonshattering": pa.uint32(),
             "excluded_indeterminate": pa.uint32()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def inputs(offline=False):
    """(sources, supplement bytes, crop -> caption) from the cache or the web."""
    m = manifests.load(DATA / "manifests" / "allaby2017-rachis.toml")
    manifests.require_redistributable(m)
    a = m["access"]
    docx = vector.download(a["url"], a["file"], a["md5"], offline=offline).read_bytes()
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256}]
    return sources, docx, dict(a["tables"])


def build_table(region, update=False, offline=False):
    return build(region, *inputs(offline))
