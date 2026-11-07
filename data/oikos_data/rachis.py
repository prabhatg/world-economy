"""Rachis observations: archaeobotanical cereal assemblages scored for shattering (wild)
and non-shattering (domestic) ears, the record of how domestication proceeded.

One Parquet table feeds the domestication-timing observation operator (docs/05 §5.1):
one row per assemblage in the region (crop, site and phase, location, cell, median
calendar year of the phase, share and count of non-shattering rachis among those
determined, the number excluded as indeterminate, and the primary source). The source
is the supplement of Allaby et al. 2017, a Word document; each crop's table is the one
that follows its caption. Barley's is a Word table. Emmer's and einkorn's are pictures in
the Enhanced Metafile format (EMF), which keeps the text it draws, so they are read
exactly from that text by where it is drawn (`emf_rows`).
"""

import io
import json
import posixpath
import re
import struct
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


_BLIP = "{http://schemas.openxmlformats.org/drawingml/2006/main}blip"
_EMBED = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}embed"


def _emf_text(emf):
    """(x, y, text, width) of each EXTTEXTOUTW record of an EMF, and its font's height."""
    out, height, i = [], None, 0
    while i + 8 <= len(emf):
        kind, size = struct.unpack_from("<II", emf, i)
        if size < 8:
            raise ValueError("a malformed EMF record")
        if kind == 82 and height is None:  # EMR_EXTCREATEFONTINDIRECTW: ihFont, then lfHeight
            height = abs(struct.unpack_from("<i", emf, i + 12)[0])
        elif kind == 84:  # EMR_EXTTEXTOUTW: bounds, mode, scales, then EMRTEXT
            x, y, n, at, _, offdx = struct.unpack_from("<iiIII16xI", emf, i + 36)
            text = emf[i + at:i + at + 2 * n].decode("utf-16le")
            width = sum(struct.unpack_from(f"<{n}i", emf, i + offdx)) if offdx else 0
            out.append((x, y, text, width))
        i += size
    if height is None:
        raise ValueError("the EMF names no font")
    return out, height


def emf_rows(emf):
    """Rows (lists of cell text) of a table drawn in an EMF. Text runs closer than a space
    (a quarter of the font's height) join into a phrase, and baselines closer than the
    font's height into one row: the first is the header, over several lines. Its phrases
    that overlap across lines make a column, and each phrase of a row goes to the column
    whose centre is nearest its own (cells are centred or left-aligned, and a long site
    name may reach under the next column's header)."""
    runs, height = _emf_text(emf)
    lines = {}
    for x, y, text, width in sorted(runs, key=lambda r: (r[1], r[0])):
        line = lines.setdefault(y, [])
        if line and x - line[-1][1] < height / 4:
            start, _, t = line[-1]
            line[-1] = (start, x + width, t + text)
        else:
            line.append((x, x + width, text))
    rows = []
    for y in sorted(lines):
        if rows and y - rows[-1][0] <= height:
            rows[-1] = (y, rows[-1][1] + lines[y])
        else:
            rows.append((y, list(lines[y])))
    if not rows:
        raise ValueError("the EMF draws no text")
    columns = []  # [start, end] of the header's phrases, overlapping ones merged
    for start, end, _ in sorted(rows[0][1]):
        if columns and start <= columns[-1][1]:
            columns[-1][1] = max(columns[-1][1], end)
        else:
            columns.append([start, end])
    centres = [(a + b) / 2 for a, b in columns]
    table = []
    for _, row in rows:
        cells = [[] for _ in columns]
        for start, end, text in row:
            mid = (start + end) / 2
            cells[min(range(len(centres)), key=lambda c: abs(centres[c] - mid))].append(text)
        table.append([" ".join(" ".join(c).split()) for c in cells])
    return table


def read_table(docx, caption):
    """Rows (lists of cell text) of the first table, or picture of one, after the paragraph
    starting `caption`."""
    z = zipfile.ZipFile(io.BytesIO(docx))
    body = ET.fromstring(z.read("word/document.xml")).find(W + "body")
    seen = False
    for el in body:
        if el.tag == W + "p" and _text(el).startswith(caption):
            seen = True
        elif el.tag == W + "tbl" and seen:
            return [[_text(c) for c in r.findall(W + "tc")] for r in el.findall(W + "tr")]
        elif seen and (blip := el.find(".//" + _BLIP)) is not None:
            rels = z.read("word/_rels/document.xml.rels").decode()
            target = re.search(rf'<Relationship [^>]*?Id="{blip.get(_EMBED)}"[^>]*?/>', rels)
            path = target and re.search(r'Target="([^"]+)"', target.group(0)).group(1)
            if not path or not path.endswith(".emf"):
                raise ValueError(f"the picture after {caption!r} is not an EMF; has the supplement changed?")
            return emf_rows(z.read(posixpath.join("word", path)))
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
        if ["".join(h.split()) for h in header] != ["".join(c.split()) for c in COLUMNS]:
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
