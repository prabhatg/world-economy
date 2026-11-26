"""Sickle blades among the flint tools of dated site phases: a record of how far people
harvested cereals, apart from the charred remains they burnt (#55, ADR-0143).

One Parquet table feeds the sickle record of cultivation effort (docs/05 §5.1,
`calibration.oikos_cal.m1`): one row per site phase of Maeda et al. (2016; Quaternary
Science Reviews, CC BY) Table S3, which gives each phase's median age (cal BC) and its
sickle blades as a share of all its flint tools (obsidian is not counted), with the
primary lithic source, to be cited with it. A range ("5.0-15.0%") reads as its midpoint;
a word ("rare", "a few", "less than 5%") gives no share and the phase is left out. A
phase spans the five centuries around its median age, as Maeda et al.'s single estimates
do in `oikos_data.cereals`.

Sickles are needed most once ears stop shattering, so their share follows domestication
as well as effort. Only phases before `JUDGED_BEFORE` are judged, where domestic cereals
were still rare, and phases from `ANCHOR_YEAR` anchor a farming world (ADR-0143); phases
between are not carried. Anchor phases are read only for their share, so they need no
place; judged phases are located by [`SITES`], mostly by their radiocarbon samples in
p3k14c, and those with no verifiable location are left out ([`LEFT_OUT`]).
"""

import io
import json
import re

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests
from .cereals import _member, site_of
from .fauna import _key, docx_rows

MANIFEST = DATA / "manifests" / "maeda2016-sickles.toml"

JUDGED_BEFORE = -8_800  # calendar year: phases older than this are judged
# The Pottery Neolithic's start (`calibration.oikos_cal.onset.ANCHOR_YEAR`): phases from
# then anchor a farming world.
ANCHOR_YEAR = -7_000

# Judged phases' sites with no verifiable location: Huzuq Musa (northwest of the lower
# Jordan Valley), 'Ain Darat (Judean Desert), Der Hall, Khallat 'Anaza (eastern Jordan's
# Black Desert), Ros Zin (central Negev) and Roudias (Cyprus, the Xeros valley), none placed
# more closely in the sources found.
LEFT_OUT = {"Huzuq Musa", "Ain Darat", "Der Hall", "Khallat 'Anaza", "Ros Zin", "Roudias"}

# (lat, lng) by site: the median of its radiocarbon samples in p3k14c (Bird et al. 2022),
# as `oikos_data.cereals.SITES` and `fauna.SITES` place those they share, unless noted:
# Dhra' by Zahrat adh-Dhra' 2, 2 km away on the same plain; Tzur Natan by its village;
# Mujahiya from its published coordinates (32°49'N 35°42'E); Cheikh Hassan by Jebel Aruda,
# across the Euphrates from it; Ayios Tychonas-Throumbovounos by Klimonas, in the same
# survey of Ayios Tychonas. A res-5 cell is some 8 km across, so these suffice.
SITES = {
    "El-Wad": (32.67, 34.97), "Beidha": (30.371, 35.448), "Wadi el-Hammeh 27": (32.45, 35.62),
    "Hayonim Terrace": (32.919, 35.225), "Hayonim Cave": (32.92, 35.22), "Jilat 6": (31.5, 36.42),
    "Zawi Chemi": (36.8, 44.22), "Qarassa 3": (32.836, 36.414), "Jericho": (31.871, 35.444),
    "Hatoula": (31.81, 34.98), "Iraq ed-Dubb": (32.391, 35.716), "Nahal Oren": (32.72, 34.97),
    "Mallaha": (33.13, 35.58), "Rosh Horesha": (30.5, 34.58), "Akrotiri-Aetokremnos": (34.573, 33.0),
    "Ain Rahub": (32.62, 35.93), "Abu Salem": (30.53, 34.52), "Mureybet": (36.043, 38.129),
    "Gesher": (32.603, 35.556), "Abu Madi I": (28.55, 33.99), "Sheikh-e Abad": (34.612, 47.27),
    "Dhra'": (31.255, 35.566), "Çayönü": (38.21, 39.72), "Hasankeyf Höyük": (37.707, 41.439),
    "Wadi Faynan 16": (30.661, 35.456), "Salibya IX": (31.98, 35.37), "Tzur Natan": (32.24, 35.012),
    "Netiv Hagdud": (31.98, 35.38), "Jerf el-Ahmar": (36.39, 38.2), "Mujahiya": (32.817, 35.7),
    "Zahrat Adh-Dhra 2": (31.255, 35.566), "Göbekli Tepe": (37.223, 38.922), "Cheikh Hassan": (36.235, 38.099),
    "Ayios Tychonas-Klimonas": (34.716, 33.13), "Ayios Tychonas-Throumbovounos": (34.716, 33.13),
    "Ayia Varvara-Asprokremnos": (35.07, 33.48),
}


def _name(s):
    """Names compared loosely, either apostrophe alike."""
    return _key(" ".join(s.replace("’", "'").split()))


def share(text):
    """The sickle share a cell states ("3.6%", "5.0-15.0%" as its midpoint), or None for
    words ("rare", "a few", "less than 5%")."""
    if "less than" in text or not (numbers := re.findall(r"\d+(?:\.\d+)?", text)):
        return None
    return sum(map(float, numbers)) / len(numbers) / 100


def build(region, sources, table, sites=SITES, left_out=LEFT_OUT):
    """Parquet bytes of Table S3's (`table`, the supplement's Word document) phases with a
    stated share: judged phases located in `region`, and anchor phases. Pure."""
    phases = []
    for r in docx_rows(table):
        if len(r) == 6 and r[3].strip().isdigit():
            phases.append((" ".join(r[2].replace("’", "'").split()), -int(r[3]), share(r[4]), " ".join(r[5].split())))
    names = {_name(p) for p, *_ in phases} | {_name(site_of(p)) for p, *_ in phases}
    if unknown := sorted(n for n in left_out if _name(n) not in names):
        raise ValueError(f"left-out names that are no site or phase of Table S3: {unknown}")
    skip = {_name(s) for s in left_out}
    located = {_name(s): (s, ll) for s, ll in sites.items()}
    cells = region.cells()
    cols = ["phase", "site", "start_year", "end_year", "share", "lat", "lng", "cell", "h3", "reference"]
    out = {k: [] for k in cols}
    for phase, year, value, reference in phases:
        site = site_of(phase)
        if value is None or JUDGED_BEFORE <= year < ANCHOR_YEAR or {_name(phase), _name(site)} & skip:
            continue
        row = {"phase": phase, "site": site, "start_year": year - 250, "end_year": year + 250, "share": value,
               "lat": None, "lng": None, "cell": None, "h3": 0, "reference": reference}
        if year < JUDGED_BEFORE:
            if _name(site) not in located:
                raise ValueError(f"{phase}: no location for its site {site}")
            name, (lat, lng) = located[_name(site)]
            cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
            pos = int(np.searchsorted(cells, cell))
            if pos >= len(cells) or cells[pos] != cell:
                continue
            row.update(site=name, lat=lat, lng=lng, cell=pos, h3=cell)
        for k in cols:
            out[k].append(row[k])
    order = np.lexsort((out["phase"], out["start_year"]))
    out = {k: [v[i] for i in order] for k, v in out.items()}
    types = {"start_year": pa.int32(), "end_year": pa.int32(), "share": pa.float64(), "lat": pa.float64(),
             "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64()}
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def inputs(offline=False):
    """(sources, Table S3) from the cache or the publisher."""
    m = manifests.load(MANIFEST)
    manifests.require_redistributable(m)
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return sources, _member(m["access"], "sickles", offline)


def build_table(region, update=False, offline=False):
    return build(region, *inputs(offline))
