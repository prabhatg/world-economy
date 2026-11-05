"""Caprine size and age by faunal assemblage: when sheep and goats became domestic, and
how they were culled (#37).

One Parquet table feeds the herding observation operator (docs/05 §5.1): one row per
dated assemblage in the region with sheep (Ovis) or goat (Capra) measured, from the
supplementary tables of Arbuckle et al. (2014; PLoS ONE, CC BY): their mean log size
index (LSI; Meadow 1999: the log ratio of a bone's measurements to a standard animal's,
smaller in domestic stock) with its sd and count, and their juvenile share (by epiphyseal
fusion, or by teeth where so marked) with its count, beside the assemblage's site,
phase and dates (Table S1, thousands of years cal BC) and location. Sheep and goat are
kept apart: a juvenile share of the two combined is not kept.

The supplement's tables name assemblages and phases in different words, so
[`ASSEMBLAGES`] maps each measured assemblage to its site and phases in Table S1 (an
assemblage of several phases spans them); the tables give no coordinates, so [`SITES`]
locates them, mostly by their radiocarbon samples in p3k14c.
"""

import hashlib
import io
import json
import re
import unicodedata
import zipfile

import h3
import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests
from .raster import MissingInputs, cache_dir

MANIFEST = DATA / "manifests" / "arbuckle2014-fauna.toml"

# Measured assemblage (Tables S3, S4) -> its site and phases in Table S1. Left out:
# Direkli (no verifiable location) and Shanidar's Mousterian layers (about 50,000 BC).
ASSEMBLAGES = {
    "Ksar Akil": ("Ksar Akil", ["Epipal"]),
    "Uçağızlı": ("Üçağızlı", ["Epipal"]),
    "Hallan Çemi": ("Hallan Çemi", ["PPNA"]),
    "Körtik": ("Körtik Tepe", ["PPNA"]),
    "Mureybet Natufian": ("Mureybet", ["Epipal-PPNA"]),
    "Mureybet PPNA": ("Mureybet", ["Epipal-PPNA"]),
    "Göbekli": ("Göbekli", ["PPNA"]),
    "Cafer": ("Cafer", ["PPNB"]),
    "Karain": ("Karain B", ["Epipal"]),
    "Öküzini": ("Öküzini", ["Total"]),
    "Aşıklı": ("Aşıklı Höyük", ["Aceramic"]),
    "Çatalhöyük Early": ("Çatalhöyük", ["Early (KOPAL, South G)"]),
    "Çatalhöyük Middle": ("Çatalhöyük", ["Middle (South H-N)"]),
    "Çatalhöyük Late": ("Çatalhöyük", ["Late (4040, North Area, South P-T)"]),
    "Çatalhöyük TP": ("Çatalhöyük", ["TP Area"]),
    "Çatalhöyük West": ("Çatalhöyük", ["West Mound"]),
    "Köşk EC": ("Köşk Höyük", ["EC"]),
    "Suberde": ("Suberde", ["Aceramic"]),
    "Erbaba": ("Erbaba Höyük", ["III-I"]),
    "Pinarbaşı B": ("Pinarbaşı", ["B"]),
    "Bademağacı ENI": ("Bademağacı", ["ENI"]),
    "Bademağacı ENII": ("Bademağacı", ["ENII"]),
    "Bademağacı LN/EC": ("Bademağacı", ["PN/Chalc"]),
    "Höyücek": ("Höyücek", ["Lower Levels", "Upper Levels"]),
    "Ulucak VI": ("Ulucak", ["VI"]),
    "Uucak V": ("Ulucak", ["V"]),
    "Uucak IV": ("Ulucak", ["IV"]),
    "Çukuriçi": ("Çukuriçi", ["VIII"]),
    "Yumuktepe": ("Yumuktepe", ["PN"]),
    "Domuztepe I-III": ("Domuztepe", ["I-III"]),
    "Fikirtepe": ("Fikirtepe", ["PN"]),
    "Barcın": ("Barcın Höyük", ["VI"]),
    "Menteşe Middle": ("Menteşe", ["moyenne"]),
    "Menteşe Late": ("Menteşe", ["recent"]),
    "Ilipinar X": ("Ilıpinar", ["X"]),
    "Ilipinar IX": ("Ilıpinar", ["IX"]),
    "Ilipinar VIII": ("Ilıpinar", ["VIII"]),
    "Ilipinar VI-IV": ("Ilıpinar", ["VII, VI", "V", "IV"]),
    "Pendik": ("Pendik", ["PN"]),
    "Orman Fidanlığı": ("Orman Fidanlığı", ["PN"]),
    "Asiab": ("Asiab", ["PPNA"]),
    "ZC Shanidar": ("ZC Shanidar", ["Aceramic"]),
}
LEFT_OUT = {"Direkli", "Shanidar Mousterian"}

# Site (as in Table S1) -> (lat, lng): the median of its radiocarbon samples in p3k14c
# (Bird et al. 2022), unless noted. Domuztepe and Fikirtepe from their published
# coordinates; Pendik (Pendik Höyük, on the Marmara shore 25 km east of Fikirtepe) and
# Orman Fidanlığı (on the Porsuk west of central Eskişehir; Efe 2001) to a few km.
SITES = {
    "Ksar Akil": (33.92, 35.6), "Üçağızlı": (35.993, 35.988), "Hallan Çemi": (38.227, 41.239),
    "Körtik Tepe": (37.816, 40.988), "Mureybet": (36.043, 38.129), "Göbekli": (37.223, 38.922),
    "Cafer": (38.416, 38.753), "Karain B": (37.07, 30.55), "Öküzini": (37.089, 30.576),
    "Aşıklı Höyük": (38.349, 34.23), "Çatalhöyük": (37.666, 32.826), "Köşk Höyük": (37.83, 34.6),
    "Suberde": (37.35, 31.93), "Erbaba Höyük": (37.77, 31.68), "Pinarbaşı": (37.48, 33.03),
    "Bademağacı": (37.4, 30.48), "Höyücek": (37.456, 30.59), "Ulucak": (38.467, 27.352),
    "Çukuriçi": (37.929, 27.359), "Yumuktepe": (36.801, 34.604), "Domuztepe": (37.321, 37.036),
    "Fikirtepe": (40.993, 29.052), "Barcın Höyük": (40.272, 29.576), "Menteşe": (40.26, 29.51),
    "Ilıpinar": (40.468, 29.309), "Pendik": (40.88, 29.25), "Orman Fidanlığı": (39.78, 30.48),
    "Asiab": (34.323, 47.138), "ZC Shanidar": (36.8, 44.22),
}


def _key(s):
    """Names compared without diacritics, case or spacing (the tables spell them variously)."""
    s = unicodedata.normalize("NFKD", s.replace("ı", "i")).encode("ascii", "ignore").decode()
    return re.sub(r"\s+", " ", s).strip().lower()


def docx_rows(data):
    """The rows (lists of cell text) of the tables of a Word document's bytes."""
    xml = zipfile.ZipFile(io.BytesIO(data)).read("word/document.xml").decode("utf-8")
    rows = []
    for tbl in re.findall(r"<w:tbl>(.*?)</w:tbl>", xml, re.S):
        for row in re.findall(r"<w:tr[ >].*?</w:tr>", tbl, re.S):
            rows.append(["".join(re.findall(r"<w:t[^>]*>([^<]*)</w:t>", c)).strip()
                         for c in re.findall(r"<w:tc>.*?</w:tc>", row, re.S)])
    return rows


def years(ky):
    """Calendar years [start, end) of a span in thousands of years cal BC ("7.4-7"); a
    single date ("9") spans the five centuries around it."""
    parts = [float(x) for x in ky.split("-")]
    if len(parts) == 1:
        mid = -1000 * parts[0]
        return int(mid - 250), int(mid + 250)
    hi, lo = parts
    return int(round(-1000 * hi)), int(round(-1000 * lo))


def _number(s):
    s = s.strip().lstrip("+")
    return None if s in ("", "-", "NA") else float(s)


def _juvenile(s):
    """(share, basis) of a juvenile cell: '+' by teeth, '*' sheep and goat combined (not
    kept), else by epiphyseal fusion."""
    s = s.strip()
    if s in ("", "-") or "*" in s:
        return None, None
    return float(s.lstrip("+")), "teeth" if s.startswith("+") else "fusion"


def _measured(rows):
    """Assemblage -> its row (dict by header) of a size-and-age table, footnotes dropped."""
    header = rows[0]
    out = {}
    for r in rows[1:]:
        if len(r) != len(header) or not r[1] or r[0].startswith("+"):
            continue
        out[r[0]] = dict(zip(header, r))
    return out


def build(region, sources, phases, ovis, capra, assemblages=ASSEMBLAGES, sites=SITES, left_out=LEFT_OUT):
    """Parquet bytes of the measured assemblages in `region`, from the supplement's Word
    tables: `phases` (Table S1), `ovis` (S3) and `capra` (S4). Pure."""
    dated = {}
    for r in docx_rows(phases)[1:]:
        if len(r) >= 4 and r[3]:
            dated[(_key(r[0]), _key(r[1]))] = years(r[3])
    taxa = {"ovis": _measured(docx_rows(ovis)), "capra": _measured(docx_rows(capra))}
    names = sorted({a for t in taxa.values() for a in t} - set(left_out))
    known = {_key(a): a for a in assemblages}
    if missing := [a for a in names if _key(a) not in known]:
        raise ValueError(f"assemblages without a site and phase: {missing}")
    cells = region.cells()
    cols = ["assemblage", "site", "phase", "start_year", "end_year", "lat", "lng", "cell", "h3"]
    for t in taxa:
        cols += [f"{t}_lsi", f"{t}_lsi_sd", f"{t}_lsi_n", f"{t}_juvenile", f"{t}_juvenile_n", f"{t}_juvenile_basis"]
    out = {k: [] for k in cols}
    for a in names:
        site, phase_names = assemblages[known[_key(a)]]
        if site not in sites:
            raise ValueError(f"{a}: no location for its site {site}")
        spans = [dated.get((_key(site), _key(p))) for p in phase_names]
        if None in spans:
            raise ValueError(f"{a}: phases {phase_names} of {site} are not all dated in Table S1")
        lat, lng = sites[site]
        cell = h3.str_to_int(h3.latlng_to_cell(lat, lng, region.resolution))
        pos = int(np.searchsorted(cells, cell))
        if pos >= len(cells) or cells[pos] != cell:
            continue
        row = {"assemblage": a, "site": site, "phase": "; ".join(phase_names),
               "start_year": min(s for s, _ in spans), "end_year": max(e for _, e in spans),
               "lat": lat, "lng": lng, "cell": pos, "h3": cell}
        for t, measured in taxa.items():
            m = measured.get(a, {})
            juvenile, basis = _juvenile(m.get("%Juvenile", ""))
            n = _number(m.get("N (%Juv)", "")) if juvenile is not None else None
            row.update({f"{t}_lsi": _number(m.get("LSI mean", "")), f"{t}_lsi_sd": _number(m.get("sd", "")),
                        f"{t}_lsi_n": _number(m.get("N (LSI)", "")), f"{t}_juvenile": juvenile,
                        f"{t}_juvenile_n": n, f"{t}_juvenile_basis": basis})
        for k in cols:
            out[k].append(row[k])
    order = np.lexsort((out["assemblage"], out["start_year"]))
    out = {k: [v[i] for i in order] for k, v in out.items()}
    ints = {"start_year": pa.int32(), "end_year": pa.int32(), "cell": pa.uint32(), "h3": pa.uint64()}
    types = {k: pa.float64() for k in cols if k.endswith(("_lsi", "_sd", "_juvenile", "lat", "lng"))}
    types.update({k: pa.uint32() for k in cols if k.endswith("_n")}, **ints)
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(cells)}).encode()}
    t = pa.table({k: pa.array([None if x is None else (int(x) if types.get(k) == pa.uint32() else x) for x in v],
                              type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def _member(access, name, offline):
    """A supplement member's bytes, cached once its SHA-256 matches the manifest's
    (Europe PMC builds the archive on request, so only the members are checked)."""
    file, sha = access["members"][name]["file"], access["members"][name]["sha256"]
    path = cache_dir() / "downloads" / "arbuckle2014" / file
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == sha:
        return path.read_bytes()
    if offline:
        raise MissingInputs(f"{file} is not downloaded")
    import ssl
    import urllib.request

    import certifi

    ctx = ssl.create_default_context(cafile=certifi.where())
    with urllib.request.urlopen(access["url"], timeout=120, context=ctx) as r:
        archive = zipfile.ZipFile(io.BytesIO(r.read()))
    path.parent.mkdir(parents=True, exist_ok=True)
    for m in access["members"].values():
        data = archive.read(m["file"])
        if (got := hashlib.sha256(data).hexdigest()) != m["sha256"]:
            raise ValueError(f"{m['file']}: SHA-256 {got} does not match the manifest's; has the supplement changed?")
        (path.parent / m["file"]).write_bytes(data)
    return path.read_bytes()


def inputs(offline=False):
    """(sources, phases, ovis, capra) Word documents from the cache or Europe PMC."""
    m = manifests.load(MANIFEST)
    manifests.require_redistributable(m)
    a = m["access"]
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return (sources, *(_member(a, k, offline) for k in ("phases", "ovis", "capra")))


def build_table(region, update=False, offline=False):
    return build(region, *inputs(offline))
