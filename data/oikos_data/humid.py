"""The African Humid Period (ADR-0084): the rain it added over Africa at its height.

CHELSA-TraCE21k's CCSM3 climate leaves the Eastern Sahara as dry as today throughout
the Holocene (under 5 mm a year south of 28 N), where lake, pollen and leaf-wax
records show a wetter Sahara from about 11,000 to 5,000 years ago (Tierney et al.
2017; Shanahan et al. 2015). A `humid_period` pack holds, per cell, the rain the
Humid Period adds at its height; the engine adds it to the climate pack's precipitation
along a time course and amplitude set by parameters, which the records constrain.

The anomaly comes from Bartlein et al.'s (2011) 6 ka reconstructions, which in the
Eastern Sahara hold two grid cells, both in Sudan, where lakes were still full at 6 ka
(the West Nubian Palaeolake until about 4,000 14C years BP; Hoelzmann et al. 2000,
2001): an exponential in latitude through them, extrapolated north over Egypt, where no
reconstruction exists (+28 mm a year at 24 N, +4 mm at 27 N). It is held over Africa
only: west of the Suez isthmus, the Gulf of Suez and the Red Sea. Arabia's humid phase
is not part of it (open question 39).
"""

import csv

import h3
import numpy as np

from . import DATA, manifest as manifests, pack, vector

# The African shore of the Suez isthmus, the Gulf of Suez and the Red Sea, as (lat, lng)
# from north to south: a cell is African if its center is west of it.
AFRICA_EDGE = ((32.0, 32.3), (29.95, 32.55), (27.8, 33.6), (24.0, 35.5), (22.0, 36.9), (18.0, 38.6))
# The fit's domain: North Africa from the southernmost anchor to the Mediterranean, east
# of the Atlantic (south, north, west); outside it the exponential would only extrapolate.
DOMAIN = (18.0, 37.5, -18.0)
SCALE = 0.1  # mm/year per stored step


def african(lat, lng):
    """Whether (lat, lng) lies in North Africa north of 18 N, the fit's domain: west of
    the Suez isthmus and the Red Sea."""
    south, north, west = DOMAIN
    lats, lngs = zip(*AFRICA_EDGE[::-1])
    return south <= lat <= north and west <= lng < float(np.interp(lat, lats, lngs))


def anchor(rows, box):
    """(a, b) of ln(anomaly) = a + b * latitude, fitted by least squares to the grid cells
    of `rows` (lat, lon, map_anm_mean) whose centers lie in `box` (south, north, west,
    east) with more rain than today; at least two, falling off northward."""
    s, n, w, e = box
    pts = [(float(r["lat"]), float(r["map_anm_mean"])) for r in rows
           if s <= float(r["lat"]) <= n and w <= float(r["lon"]) <= e and float(r["map_anm_mean"]) > 0]
    if len(pts) < 2:
        raise ValueError(f"the Humid Period's anomaly needs at least two wetter grid cells in {box}; found {len(pts)}")
    lat, anomaly = np.array(pts).T
    b, a = np.polyfit(lat, np.log(anomaly), 1)
    if b >= 0:
        raise ValueError(f"the anomaly in {box} does not fall off northward (slope {b:.3f} per degree)")
    return float(a), float(b)


def build_pack(region, sources, rows, box):
    """Humid-period pack bytes: per cell, the anomaly of the Eastern Saharan fit at the
    cell's center if it is African, else 0. Pure."""
    a, b = anchor(rows, box)
    cells = region.cells()
    values = np.zeros(len(cells), dtype=np.uint16)
    for i, c in enumerate(cells):
        lat, lng = h3.cell_to_latlng(h3.int_to_str(int(c)))
        if african(lat, lng):
            values[i] = min(round(float(np.exp(a + b * lat)) / SCALE), 65_535)
    bld = pack.Builder("humid_period", pack.region(region.name, region.resolution, cells))
    for src in sources:
        bld.source(**{k: src[k] for k in ("dataset", "version", "license", "citation", "manifest_sha256")},
                   attribution=src.get("attribution"))
    bld.field("precip_anomaly", "mm/year",
              "Annual precipitation the African Humid Period adds at its height (Bartlein et al. 2011's Eastern "
              f"Saharan 6 ka anomalies, exp({a:.4f} {b:+.4f} lat), over Africa; 0 elsewhere)", values, scale=SCALE)
    return bld.build()


def inputs(offline=False):
    """(sources, rows, eastern Saharan box) from the cache or the web."""
    m = manifests.load(DATA / "manifests" / "bartlein2011-map.toml")
    manifests.require_redistributable(m)
    acc = m["access"]
    path = vector.download(acc["url"], acc["file"], acc["md5"], offline=offline)
    with path.open(newline="") as f:
        rows = [{k.strip(): v.strip() for k, v in r.items()} for r in csv.DictReader(f, skipinitialspace=True)]
    sources = [{"dataset": m["dataset"], "version": m["version"], "license": m["license"]["id"],
                "citation": m["citation"], "manifest_sha256": m.sha256, "attribution": m["license"]["attribution"]}]
    return sources, rows, tuple(acc["eastern_sahara"])


def build_humid_pack(region, update=False, offline=False):
    sources, rows, box = inputs(offline)
    return build_pack(region, sources, rows, box)
