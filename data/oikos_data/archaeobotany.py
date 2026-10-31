"""Archaeobotany: how much of the charred plant remains of each dated site phase are
crops or their wild progenitors, the record of when people began to plant (#29;
ADR-0064).

From the Origins of agriculture database (Wallace, Livarda, Charles and Jones 2018;
`data/manifests/origins-agriculture.toml`), one row per site phase in the region:
phase, site, location, cell, period and its approximate start and end calendar years,
the crop items (domesticates, crop progenitors and presumed crop taxa) and all plant
items, and the number of samples. Items are counted as Wallace et al. (2019) select
them for their analysis of deliberate collection: single samples not excluded by the
compilers, grains and seeds (reproductive parts), charred (not desiccated, impressed
or waterlogged; unrecorded preservation counts as charred). Taxa the compilers class as
"progenitor or wild" or indeterminate count as neither.

The database's terms allow research use but not passing on the data or anything
derived from it except to registered users, so its tables are downloaded by hand and
the phase table is built locally; neither is committed, packed or locked.
"""

import csv
import hashlib
import io
import json
from collections import defaultdict

import pyarrow as pa
import pyarrow.parquet as pq

from . import DATA, manifest as manifests
from .obsidian import _locate
from .raster import MissingInputs, cache_dir

AMBIGUOUS = {"Progenitor/Wild", ".Indet"}
EXCLUDED_PRESERVATION = {"d", "i", "w"}  # desiccated, impression, waterlogged


def _is_crop(group):
    """Domesticates, crop progenitors and presumed crops, in the compilers' spellings."""
    return "rop" in group or "omestic" in group


def build_table(region, sources, tables):
    """Parquet bytes of the dated site phases in `region` with their crop and plant
    items; `tables` are the database's tables as rows (dicts), by file name. Pure."""
    sites = {r["SiteName"]: r for r in tables["1_Site.txt"]}
    phases = {r["SitePhase-ID"]: r for r in tables["2_SitePhase.txt"]}
    records = {r["RecordID"]: r for r in tables["3_Records-Samples.txt"]
               if r["RecordType"].strip().lower() == "sample" and not r["Exclusion notes (if any)"].strip()}
    parts = {r["PlantPart"]: r["PlantPart_Cat"] for r in tables["6_PlantPart_Categories.txt"]}
    taxa = {r["OriginalTaxon"]: r["ProjectTaxon"] for r in tables["7_Taxa_Standardisation.txt"]}
    groups = {r["ProjectTaxon"]: r["DomProgWild"].strip() for r in tables["8_Taxa_Groups.txt"]}
    crop, items, samples = defaultdict(float), defaultdict(float), defaultdict(set)
    for e in tables["5_Taxa_Entries.txt"]:
        r = records.get(e["RecordID"])
        group = groups.get(taxa.get(e["OriginalTaxon"]), "")
        if (r is None or not parts.get(e["PlantPart"], "").lower().startswith("repro")
                or e["Preservation"].strip().lower() in EXCLUDED_PRESERVATION or not group or group in AMBIGUOUS):
            continue
        n = float(e["MNPP"] or 0)
        phase = r["SitePhase-ID"]
        items[phase] += n
        samples[phase].add(e["RecordID"])
        if _is_crop(group):
            crop[phase] += n
    out = {k: [] for k in ("phase", "site", "lat", "lng", "cell", "h3", "period", "start_year", "end_year", "crop",
                           "items", "samples")}
    for phase in sorted(items):
        p = phases.get(phase)
        site = sites.get(p["SiteName (SPh)"]) if p else None
        if site is None or not p["ApproxStartBC"].strip() or not p["ApproxEndBC"].strip():
            continue
        lat, lng = float(site["Latitude"]), float(site["Longitude"])
        where = _locate(region, lat, lng)
        if where is None:
            continue
        start, end = sorted((round(float(p["ApproxStartBC"])), round(float(p["ApproxEndBC"]))))
        for k, v in (("phase", phase), ("site", site["SiteName"]), ("lat", lat), ("lng", lng), ("cell", where[0]),
                     ("h3", where[1]), ("period", p["MainPeriod"]), ("start_year", start), ("end_year", end),
                     ("crop", round(crop[phase])), ("items", round(items[phase])), ("samples", len(samples[phase]))):
            out[k].append(v)
    meta = {b"sources": json.dumps(sources, sort_keys=True).encode(), b"region": json.dumps(
        {"name": region.name, "resolution": region.resolution, "cells": len(region.cells())}).encode()}
    types = {"lat": pa.float64(), "lng": pa.float64(), "cell": pa.uint32(), "h3": pa.uint64(), "start_year": pa.int32(),
             "end_year": pa.int32(), "crop": pa.uint64(), "items": pa.uint64(), "samples": pa.uint32()}
    t = pa.table({k: pa.array(v, type=types.get(k, pa.string())) for k, v in out.items()}).replace_schema_metadata(meta)
    buf = io.BytesIO()
    pq.write_table(t, buf, compression="zstd", compression_level=9)
    return buf.getvalue()


def read_tables(files, directory):
    """The tables `files` ({name: SHA-256}) from $OIKOS_DATA_CACHE/manual/`directory`,
    as rows; each must match its checksum."""
    root = cache_dir() / "manual" / directory
    missing = [name for name in files if not (root / name).exists()]
    if missing:
        m = manifests.load(DATA / "manifests" / "origins-agriculture.toml")
        raise MissingInputs(f"{', '.join(missing)} not in {root}: download them by hand (a browser is needed) from "
                            f"{m['access']['page']} (terms: {m['license']['url']})")
    out = {}
    for name, sha256 in files.items():
        raw = (root / name).read_bytes()
        if hashlib.sha256(raw).hexdigest() != sha256:
            raise ValueError(f"{root / name}: SHA-256 differs from the manifest's; download it again")
        out[name] = list(csv.DictReader(io.StringIO(raw.decode("utf-8"), newline="")))
    return out


def inputs():
    """(sources, tables) of the manifest's dataset, from the manual download."""
    m = manifests.load(DATA / "manifests" / "origins-agriculture.toml")
    a = m["access"]
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return sources, read_tables(a["files"], a["dir"])


def build_crop_table(region, update=False, offline=False):
    """The phase table for `region`; local only (the license forbids passing it on)."""
    sources, tables = inputs()
    return build_table(region, sources, tables)
