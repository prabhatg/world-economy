"""Where wild emmer and wild einkorn grow (#38): the ranges of the wheats' ancestors.

Wild emmer (Triticum turgidum subsp. dicoccoides) grows in a narrow arc: the upper
Jordan valley, Galilee, the Golan, Hermon and the hills east of the Jordan, Karacadağ
and the foothills of southeastern Anatolia, and thinly along the Zagros; wild einkorn
(T. monococcum subsp. aegilopoides, T. boeoticum) far more widely, over the Anatolian
plateau's margins, southeastern Anatolia, the northern Iraqi and Iranian Zagros and
into the Caucasus, the Balkans and only patchily the southern Levant (Zohary, Hopf and
Weiss 2012; Harlan and Zohary 1966). Villages beyond those ranges that sowed the wheats,
as on the Damascus basin's dry plain and the central Anatolian plateau, sowed seed brought
from elsewhere, out of reach of wild stands that could mix back into their stock
(Willcox 2005).

A `wild_wheats` pack holds each cell's range share of each wheat: 1 within `rings` cells
(about 15 km a ring at resolution 5) of a georeferenced occurrence record, else 0. The
records are GBIF's, chiefly the crop-wild-relatives database (Castañeda-Álvarez et al.
2016) and genebank passport data: CC0 and CC BY 4.0 records only, without living
specimens (garden and genebank plants) or coordinates rounded to a whole degree. They are
where the wheats grow today, a modern range standing for the Holocene's; collecting
missions follow roads, which the rings bridge.
"""

import hashlib
import json

import h3
import numpy as np

from . import DATA, manifest as manifests, raster

CROPS = ("emmer", "einkorn")
SCALE = 1e-4
LICENSES = {"CC0_1_0": "CC0-1.0", "CC_BY_4_0": "CC-BY-4.0"}


def snapshot(records, datasets):
    """Canonical records (sorted, the fields the pack reads) and the datasets they come from."""
    keys = ("crop", "key", "lat", "lng", "basis", "license", "dataset")
    rows = sorted([r[k] for k in keys] for r in records)
    used = {r[-1] for r in rows}
    return {"records": rows, "datasets": {k: dict(sorted(v.items())) for k, v in sorted(datasets.items()) if k in used}}


def fingerprint(snap):
    return hashlib.sha256(json.dumps(snap, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def save(snap, path):
    """Writes a snapshot as gzipped canonical JSON, the same bytes for the same records."""
    import gzip

    path.parent.mkdir(parents=True, exist_ok=True)
    data = json.dumps(snap, sort_keys=True, separators=(",", ":")).encode()
    path.write_bytes(gzip.compress(data, compresslevel=9, mtime=0))


def load(path):
    import gzip

    return json.loads(gzip.decompress(path.read_bytes()))


def usable(row, access):
    """Whether a snapshot row counts: an allowed license, not a living specimen, and a
    coordinate finer than a whole degree."""
    _, _, lat, lng, basis, license, _ = row
    return (license in access["licenses"] and basis not in access["excluded_basis"]
            and lat != int(lat) and lng != int(lng))


def ranges(region, rows, rings):
    """Each crop's range share per region cell (ascending cells): 1 within `rings` of a record."""
    cells = region.cells()
    index = {int(c): i for i, c in enumerate(cells)}
    out = {c: np.zeros(len(cells), dtype=np.uint16) for c in CROPS}
    for crop in CROPS:
        seen = {h3.latlng_to_cell(lat, lng, region.resolution) for c, _, lat, lng, *_ in rows if c == crop}
        for cell in seen:
            for n in h3.grid_disk(cell, rings):
                if (i := index.get(h3.str_to_int(n))) is not None:
                    out[crop][i] = round(1.0 / SCALE)
    return out


def build(region, manifest, manifest_sha256, snap):
    """Wild-wheat pack bytes from a snapshot of occurrence records. Pure."""
    from . import pack

    a, m = manifest["access"], manifest
    rows = [r for r in snap["records"] if usable(r, a)]
    b = pack.Builder("wild_wheats", pack.region(region.name, region.resolution, region.cells()))
    b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
             manifest_sha256=manifest_sha256, attribution=m["license"]["attribution"])
    for key in sorted({r[-1] for r in rows}):
        # Credited under the licenses of the records used, which may be looser than the
        # dataset's own (iNaturalist's CC0 records); CC BY if any of them is.
        d, by = snap["datasets"][key], any(r[5] == "CC_BY_4_0" for r in rows if r[-1] == key)
        cite = f"{d['title']}. https://doi.org/{d['doi']}"
        b.source(dataset=f"gbif:{key}", version=m["version"], license=LICENSES["CC_BY_4_0" if by else "CC0_1_0"],
                 citation=cite, manifest_sha256=manifest_sha256,
                 attribution=f"Contains occurrence records from {cite}" if by else None)
    names = {"emmer": "wild emmer (Triticum turgidum subsp. dicoccoides)",
             "einkorn": "wild einkorn (Triticum monococcum subsp. aegilopoides)"}
    rings = reach(a["rings"], region.resolution)
    for crop, values in ranges(region, rows, rings).items():
        b.field(crop, "1", f"Range share of {names[crop]}: 1 within {rings} cells of an occurrence record "
                f"(GBIF, modern), else 0", values, scale=SCALE)
    return b.build()


def _get(url, params):
    import ssl
    import urllib.parse
    import urllib.request

    import certifi

    ctx = ssl.create_default_context(cafile=certifi.where())
    with urllib.request.urlopen(url + ("?" + urllib.parse.urlencode(params) if params else ""), timeout=120,
                                context=ctx) as r:
        return json.load(r)


def reach(rings, resolution):
    """The manifest's `rings`, counted at resolution 5, at `resolution`: each finer level's
    cells are sqrt(7) times closer, so a range reaches as far."""
    return round(rings * 7 ** ((resolution - 5) / 2))


def snapshot_path(region, manifest):
    """One snapshot per box searched, whatever the regions' resolutions."""
    s, n, w, e = region.bbox
    return DATA / "snapshots" / f"{manifest['dataset']}.box{s:g}_{n:g}_{w:g}_{e:g}.json.gz"


def fetch(region, manifest, offline=False):
    """The snapshot of the manifest's occurrence search over the region's box widened by
    `margin_deg`. GBIF's search results change as publishers update them, so the snapshot
    is kept in the repository (data/snapshots) and the search is queried only when there
    is none: to refresh it, delete it and rebuild with --update-lock."""
    a = manifest["access"]
    f = snapshot_path(region, manifest)
    if f.exists():
        return load(f)
    if offline:
        raise raster.MissingInputs(f"no snapshot of the GBIF records for {manifest['dataset']} at {f}")
    s, n, w, e = (region.bbox[0] - a["margin_deg"], region.bbox[1] + a["margin_deg"],
                  region.bbox[2] - a["margin_deg"], region.bbox[3] + a["margin_deg"])
    records, datasets = [], {}
    for crop, keys in a["taxa"].items():
        offset = 0
        while True:
            page = _get(a["url"], [("taxonKey", k) for k in keys] + [("license", x) for x in a["licenses"]] + [
                ("hasCoordinate", "true"), ("hasGeospatialIssue", "false"),
                ("decimalLatitude", f"{s},{n}"), ("decimalLongitude", f"{w},{e}"),
                ("limit", 300), ("offset", offset)])
            for r in page["results"]:
                if r.get("basisOfRecord") in a["excluded_basis"]:
                    continue
                records.append({"crop": crop, "key": r["key"], "lat": r["decimalLatitude"],
                                "lng": r["decimalLongitude"], "basis": r["basisOfRecord"],
                                "license": next((k for k in a["licenses"] if r["license"].startswith(_URL[k])), "other"),
                                "dataset": r["datasetKey"]})
            offset += 300
            if page["endOfRecords"]:
                break
    for key in sorted({r["dataset"] for r in records}):
        d = _get(f"{a['dataset_url']}/{key}", None)
        datasets[key] = {"title": d["title"], "doi": d.get("doi", "")}
    snap = snapshot(records, datasets)
    save(snap, f)
    return snap


# License URLs GBIF gives records and datasets, by the search API's license names.
_URL = {"CC0_1_0": "http://creativecommons.org/publicdomain/zero/1.0", "CC_BY_4_0": "http://creativecommons.org/licenses/by/4.0"}


def build_pack(region, update=False, offline=False):
    m = manifests.load(DATA / "manifests" / "gbif-wild-wheats.toml")
    manifests.require_redistributable(m)
    snap = fetch(region, m, offline=offline)
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json", {"records": fingerprint(snap)}, update)
    return build(region, m, m.sha256, snap)
