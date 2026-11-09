"""Bedrock pack: how deep each cell's soil runs before rock (#24), from SoilGrids250m 2017.

Fields wear soil down by a depth each year (Montgomery 2007), so a thin soil over rock
on the Levant's and the Zagros' hills reaches the rock in centuries where an alluvial
plain's would take millennia. SoilGrids250m 2017's depth to bedrock within 2 m
(BDRICM: the R horizon, else 200 cm; Shangguan et al. 2017, Hengl et al. 2017) is
read at its 1/16 overview (~3.7 km, a dozen samples per resolution-5 cell), cut to the
region's box and averaged per cell, skipping its no-data (water, unmapped land). The
`depth_share` field is that mean as a share of 2 m: 1 where rock lies at 2 m or deeper.
This is the modern soil, after the erosion this model simulates; it stands for the
Holocene's.
"""

import numpy as np

from . import DATA, climate, manifest as manifests, pack, raster, soils

SCALE = 1e-4


def crop(tile, box):
    """The part of `tile` inside `box` (south, north, west, east), snapped outward to
    whole pixels."""
    s, n, w, e = box
    r0 = max(int(np.floor((tile.north - n) / tile.dlat)), 0)
    r1 = min(int(np.ceil((tile.north - s) / tile.dlat)), tile.data.shape[0])
    c0 = max(int(np.floor((w - tile.west) / tile.dlng)), 0)
    c1 = min(int(np.ceil((e - tile.west) / tile.dlng)), tile.data.shape[1])
    return raster.Tile(tile.data[r0:r1, c0:c1], north=tile.north - r0 * tile.dlat, west=tile.west + c0 * tile.dlng,
                       dlat=tile.dlat, dlng=tile.dlng)


def share(depth_cm, cap_cm):
    """Depth to bedrock as a share of `cap_cm`, at most 1."""
    return np.minimum(depth_cm / cap_cm, 1.0)


def build(region, manifest, manifest_sha256, tile):
    """Bedrock pack bytes from `tile` (depths in cm on a latitude-longitude grid); pure,
    for testing."""
    a = manifest["access"]
    depth = tile.data.astype(np.float64)
    depth = np.where((depth < 0) | (depth == a["nodata"]), np.nan, depth)
    n = len(region.cells())
    mean = climate.cell_means(soils.cell_index(region, tile, "EPSG:4326"), depth, n)
    stored = np.where(np.isnan(mean), soils.U16_NODATA,
                      np.round(np.nan_to_num(share(mean, a["cap_cm"])) / SCALE)).astype(np.uint16)
    m = manifest
    b = pack.Builder("bedrock", pack.region(region.name, region.resolution, region.cells()))
    b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
             manifest_sha256=manifest_sha256, attribution=m["license"]["attribution"])
    b.field("depth_share", "1", f"Depth to bedrock within {a['cap_cm']} cm, as a share of it: the cell's mean "
            f"({m['dataset']}, modern; 1 at {a['cap_cm']} cm or deeper)", stored, scale=SCALE, nodata=soils.U16_NODATA)
    return b.build()


def fetch(region, manifest, offline=False):
    a = manifest["access"]
    s, n, w, e = climate.bounds(region)
    return crop(raster.read_overview(a["url"], a["overview_factor"], offline), (s, n, w, e))


def build_pack(region, update=False, offline=False):
    m = manifests.load(DATA / "manifests" / "soilgrids2017-bedrock.toml")
    manifests.require_redistributable(m)
    tile = fetch(region, m, offline=offline)
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json", {"BDRICM_M": raster.fingerprint(tile)}, update)
    return build(region, m, m.sha256, tile)
