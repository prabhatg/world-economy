"""Coastline pack: land fraction per cell and century from CHELSA-TraCE21k's paleo-DEM.

The paleo-DEM marks the sea of each 100-year slice as nodata, so the share of a cell's
pixels with data is its land fraction for that slice. Pixels are read in 2x2 blocks
(manifest `access.block_factor`) as the fraction of each block that is land, then
averaged per cell; slices sit at their midpoint years like the climate pack's
(`climate.midpoint_year`). Lakes in the DEM count as land, as in the terrain pack.
"""

from concurrent.futures import ThreadPoolExecutor

import numpy as np

from . import DATA, climate, manifest as manifests, pack, raster


def build(region, manifest, manifest_sha256, tiles):
    """Coastline pack bytes from `tiles` ({time_id: raster.Tile of land fractions}); pure, for testing."""
    ids = climate.time_ids(manifest)
    n = len(region.cells())
    if len({(x.north, x.west, x.dlat, x.dlng, x.data.shape) for x in (tiles[t] for t in ids)}) != 1:
        raise ValueError("coastline tiles must share one grid")
    index = climate.cell_index(region, tiles[ids[0]])
    land = np.empty((len(ids), n), dtype=np.uint16)
    for i, t in enumerate(ids):
        frac = climate.cell_means(index, tiles[t].data, n)
        if np.isnan(frac).any():
            raise ValueError(f"{int(np.isnan(frac).sum())} cells have no samples; widen the window")
        land[i] = np.round(np.clip(frac, 0, 1) * 10000).astype(np.uint16)

    b = pack.Builder("coastline", pack.region(region.name, region.resolution, region.cells()))
    b.time([climate.midpoint_year(t) for t in ids], "linear")
    m = manifest
    b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
             manifest_sha256=manifest_sha256, attribution=m["license"]["attribution"])
    b.field("land_fraction", "1", f"Fraction of the cell above the sea level of the slice ({m['dataset']} land mask, "
            "100-year slice at its midpoint year)", land, scale=1e-4)
    return b.build()


def fetch(region, manifest, offline=False, workers=8):
    access = manifest["access"]
    box = climate.bounds(region)

    def read(t):
        url = access["url_template"].format(time_id=t)
        return t, raster.read_window(url, box, access["block_factor"], offline, valid=True)

    with ThreadPoolExecutor(max_workers=workers) as pool:
        return dict(pool.map(read, climate.time_ids(manifest)))


def build_pack(region, update=False, offline=False):
    path = DATA / "manifests" / "chelsa-trace21k-dem.toml"
    m = manifests.load(path)
    manifests.require_redistributable(m)
    tiles = fetch(region, m, offline=offline)
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json", {f"dem_{t}": raster.fingerprint(x) for t, x in tiles.items()}, update)
    return build(region, m, m.sha256, tiles)
