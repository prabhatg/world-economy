"""Cereal shares of charred plant remains by site phase: a second record of how far people
relied on cereals before and after domestication (#55).

One Parquet table feeds the cultivation-onset observation operator beside the Origins of
agriculture database (docs/05 §5.1, `oikos_data.archaeobotany`): one row per dated site
phase in the region, from the supplementary tables of Maeda et al. (2016; Quaternary
Science Reviews, CC BY): the count of cereal remains (grain and chaff of wheats, barley
and other cereals, wild or domestic) and of all identified charred remains (Table S5),
beside the phase's calibrated date range (Table S1: the 1-sigma range of its summed
radiocarbon dates, or the authors' estimate from its material culture), location and
primary archaeobotanical source, to be cited with it.

Sites the Origins of agriculture database also records ([`OVERLAP`]) are left out: their
assemblages are judged through it already (ADR-0064), and counting them twice would
narrow the record's tolerance by repetition rather than by evidence. The tables give
no coordinates, so [`SITES`] locates the rest, mostly by their radiocarbon samples in
p3k14c; phases with no verifiable location are left out ([`LEFT_OUT`]).
"""

import hashlib
import io
import json
import re

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests
from .fauna import _key, docx_rows
from .raster import MissingInputs, cache_dir

MANIFEST = DATA / "manifests" / "maeda2016-cereals.toml"

# Sites of Table S5 that the Origins of agriculture database (Wallace et al. 2018) also
# records, by the site name before any phase in parentheses. Tell Sabi Abyad I is not
# among them: the database's Sabi Abyad is Tell Sabi Abyad II, another mound. Nor are
# Chogha Golan, Dhuweila and Wadi el-Hammeh 27: the database has them only in samples
# below the 50 items its record judges (`calibration.oikos_cal.m1.ONSET_MIN_ITEMS`).
OVERLAP = {
    "Abu Hureyra", "Mureybet", "Iraq ed-Dubb", "Jilat 6", "Jilat 7", "Jilat 13", "Çayönü", "Jerf el-Ahmar", "Aswad",
    "Ghoraife", "Jericho", "Basta", "Beidha", "El Kowm 1", "El Kowm 2", "Cafer Höyük", "Ras Shamra", "Maghzaliyah",
    "Chogha Bonut", "East Chia Sabz", "Ganj Dareh", "Ramad", "Wadi Fidan C", "Azraq 31", "Bouqras", "Yarim Tepe I",
    "Ais Yiorkis", "Cape Andreas-Kastros", "Mylouthkia", "Can Hasan III", "Çatalhöyük East", "Hacilar I",
    "Hacilar II", "M'lefaat",
}

# No verifiable location: el-Hemmeh lies in the Wadi el-Hasa within 5 km of Khirbet
# Hammam (Makarewicz), Wadi Fidan A in the Wadi Fidan, neither placed more closely.
# Tenta (5-1) sums all of Kalavasos-Tenta's levels, which Table S1 dates from 8800 to
# 5080 BC: no era of the record holds it.
LEFT_OUT = {"el-Hemmeh (PPNA)", "el-Hemmeh (LPPNB/PPNC)", "Wadi Fidan A", "Tenta (5-1)"}

# (lat, lng) by site: the radiocarbon samples' location in p3k14c (Bird et al. 2022),
# the most precise where they differ (Qermez Dere by Tell Afar, Qarassa's exact entry),
# unless noted: Dederiyeh Cave from its published coordinates (to the arc minute); Salat Cami Yanı
# beside Yukarısalat, on the Salat 3 km above the Tigris (Miyake et al.); Domuztepe as
# `fauna.SITES`; Tell 'Abr for Abr 3. A res-5 cell is some 8 km across, so these suffice.
SITES = {
    "Dederiyeh cave": (36.4, 36.867), "Mallaha": (33.13, 35.58), "Wadi el-Hammeh 27": (32.45, 35.62),
    "Qaramel": (36.378, 37.275), "Gesher": (32.603, 35.556), "Netiv Hagdud": (31.98, 35.38),
    "Demirköy": (37.883, 41.083), "Hallan Çemi": (38.227, 41.239), "Hasankeyf Höyük": (37.707, 41.439),
    "Körtik Tepe": (37.816, 40.988), "Qermez Dere": (36.37, 42.45), "Abr 3": (36.61, 38.27),
    "Ain el-Kerkh": (35.82, 36.466), "Dja'de": (36.383, 38.183), "Göbekli Tepe": (37.223, 38.922),
    "Halula": (36.417, 38.167), "Nevalı Çori": (37.518, 38.606), "Nahal Hemar": (31.139, 35.294),
    "Qarassa": (32.836, 36.414), "Dhuweila": (32.03, 37.25), "Jarmo": (35.56, 44.92),
    "Chogha Golan": (33.377, 46.271), "Nahal Zehora II": (32.683, 35.053), "Nebi Mend": (34.553, 36.518),
    "Sha'ar Hagolan": (32.687, 35.603), "Yiftahel": (32.755, 35.228), "Kashkashok II": (36.637, 40.641),
    "Salat Cami Yanı": (37.842, 40.896), "Arjoune": (34.56, 36.55), "Domuztepe": (37.321, 37.036),
    "Tell Kurdu": (36.33, 36.445), "Aqab": (37.08, 40.82), "Tepe Marani": (35.211, 45.921),
    "Umm Qseir": (36.417, 41.0), "Choga Mami": (33.9, 45.17), "Khirokitia": (34.8, 33.34),
    "Erbaba": (37.77, 31.68), "Sabi Abyad": (36.53, 39.1),
}

CEREALS = ("Wheat", "Barley", "other cereals")


def site_of(phase):
    """A phase's site: its name before any phase in parentheses."""
    return re.sub(r"\s*\(.*?\)\s*$", "", " ".join(phase.split()))


def span(text):
    """Calendar years [start, end) of Table S1's range ("9400-8750 BC", "estimated
    7200-6500 BC"); a single estimate ("estimated 7300 BC", once "6450 B") spans the five
    centuries around it."""
    if m := re.search(r"(\d{3,5})\s*[-–]\s*(\d{3,5})", text):
        return -int(m[1]), -int(m[2])
    if m := re.search(r"(\d{3,5})", text):
        return -int(m[1]) - 250, -int(m[1]) + 250
    raise ValueError(f"no date range in {text!r}")


def build(region, sources, chronology, shares, sites=SITES, overlap=OVERLAP, left_out=LEFT_OUT):
    """Parquet bytes of the dated, located site phases in `region`, from the supplement's
    Word tables: `chronology` (Table S1) and `shares` (Table S5). Pure."""
    dated = {}
    for r in docx_rows(chronology)[1:]:
        if len(r) >= 6 and r[5]:
            dated[_key(r[0])] = span(r[5])
    rows = docx_rows(shares)
    header = rows[0]
    phases = [dict(zip(header, r)) for r in rows[1:] if len(r) == len(header) and r[0]]
    names = {_key(p["site"]) for p in phases} | {_key(site_of(p["site"])) for p in phases}
    if unknown := sorted(n for n in set(overlap) | set(left_out) if _key(n) not in names):
        raise ValueError(f"overlapping or left-out names that are no site or phase of Table S5: {unknown}")
    skip_sites, skip_phases = {_key(s) for s in overlap}, {_key(s) for s in left_out}
    located = {_key(s): (s, ll) for s, ll in sites.items()}
    cells = region.cells()
    cols = ["phase", "site", "start_year", "end_year", "cereal", "items", "lat", "lng", "cell", "h3", "reference"]
    out = {k: [] for k in cols}
    for p in phases:
        phase = " ".join(p["site"].split())
        site = site_of(phase)
        if _key(site) in skip_sites or _key(phase) in skip_phases:
            continue
        if _key(phase) not in dated:
            raise ValueError(f"{phase}: not dated in Table S1")
        start, end = dated[_key(phase)]
        if _key(site) not in located:
            raise ValueError(f"{phase}: no location for its site {site}")
        name, (lat, lng) = located[_key(site)]
        cereal = sum(int(p[c]) for c in CEREALS)
        items = int(p["total NISP"])
        stated = float(p["cereals(%)"].rstrip("%"))
        if abs(100 * cereal / items - stated) > 0.1:
            raise ValueError(f"{phase}: cereals {cereal} of {items} is not the stated {stated}%")
        cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
        pos = int(np.searchsorted(cells, cell))
        if pos >= len(cells) or cells[pos] != cell:
            continue
        row = {"phase": phase, "site": name, "start_year": start, "end_year": end, "cereal": cereal, "items": items,
               "lat": lat, "lng": lng, "cell": pos, "h3": cell, "reference": " ".join(p["reference"].split())}
        for k in cols:
            out[k].append(row[k])
    order = np.lexsort((out["phase"], out["start_year"]))
    out = {k: [v[i] for i in order] for k, v in out.items()}
    types = {"start_year": pa.int32(), "end_year": pa.int32(), "cereal": pa.uint32(), "items": pa.uint32(),
             "lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64()}
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def _member(access, name, offline):
    """A supplement file's bytes, cached once its SHA-256 matches the manifest's."""
    file, sha = access["members"][name]["file"], access["members"][name]["sha256"]
    path = cache_dir() / "downloads" / "maeda2016" / file
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == sha:
        return path.read_bytes()
    if offline:
        raise MissingInputs(f"{file} is not downloaded")
    import ssl
    import urllib.request

    import certifi

    ctx = ssl.create_default_context(cafile=certifi.where())
    # The publisher's file server refuses requests without a browser-like user agent.
    req = urllib.request.Request(access["url"] + file, headers={"User-Agent": "Mozilla/5.0 (ManyWorlds oikos_data)"})
    with urllib.request.urlopen(req, timeout=120, context=ctx) as r:
        data = r.read()
    if (got := hashlib.sha256(data).hexdigest()) != sha:
        raise ValueError(f"{file}: SHA-256 {got} does not match the manifest's; has the supplement changed?")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return data


def inputs(offline=False):
    """(sources, chronology, shares) Word documents from the cache or the publisher."""
    m = manifests.load(MANIFEST)
    manifests.require_redistributable(m)
    a = m["access"]
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return (sources, *(_member(a, k, offline) for k in ("chronology", "shares")))


def build_table(region, update=False, offline=False):
    return build(region, *inputs(offline))
