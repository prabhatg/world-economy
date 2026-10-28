"""Soils pack: topsoil properties per cell from SoilGrids 2.0 (modern, as a proxy).

SoilGrids' 1 km aggregates are cloud-optimized GeoTIFFs in the Interrupted Goode
Homolosine projection. The window covering the region is read for each property and
depth, averaged over 2x2 pixel blocks (manifest `access.block_factor`), and each block
center is projected to latitude and longitude to find its cell. Non-soil pixels
(water, built-up, glaciers; nodata in SoilGrids) are skipped. Topsoil values are
depth-weighted means over 0-30 cm, converted from SoilGrids' stored integers to
conventional units. Holocene soils differed (erosion, salinization, deposition); this
is the modern state.
"""

from concurrent.futures import ThreadPoolExecutor

import h3
import numpy as np

from . import DATA, climate, manifest as manifests, pack, raster

# (depth label, thickness in cm) of the SoilGrids layers that make up the topsoil.
DEPTHS = [("0-5cm", 5), ("5-15cm", 10), ("15-30cm", 15)]
# SoilGrids property -> pack field: name, unit, description, divisor to conventional units.
PROPERTIES = {
    "clay": ("clay", "%", "Clay content (mass) of the fine earth", 10),
    "sand": ("sand", "%", "Sand content (mass) of the fine earth", 10),
    "silt": ("silt", "%", "Silt content (mass) of the fine earth", 10),
    "soc": ("organic_carbon", "g/kg", "Soil organic carbon content of the fine earth", 10),
    "phh2o": ("ph", "1", "pH in water", 10),
    "cfvo": ("coarse_fragments", "%", "Coarse fragments (> 2 mm), volume", 10),
}
U16_NODATA = 65535


def sample_latlng(tile, crs):
    """Latitude and longitude of each sample's center, from the raster's CRS."""
    from rasterio.warp import transform

    rows, cols = tile.data.shape
    y = tile.north - (np.arange(rows) + 0.5) * tile.dlat
    x = tile.west + (np.arange(cols) + 0.5) * tile.dlng
    xx, yy = np.meshgrid(x, y)
    if crs == "EPSG:4326":
        return yy, xx
    lng, lat = transform(crs, "EPSG:4326", xx.ravel().tolist(), yy.ravel().tolist())
    return np.array(lat).reshape(rows, cols), np.array(lng).reshape(rows, cols)


def cell_index(region, tile, crs):
    cells = region.cells()
    lat, lng = sample_latlng(tile, crs)
    ids = np.array([h3.str_to_int(h3.latlng_to_cell(a, b, region.resolution)) for a, b in zip(lat.ravel().tolist(), lng.ravel().tolist())],
                   dtype=np.uint64)
    pos = np.minimum(np.searchsorted(cells, ids), len(cells) - 1)
    return np.where(cells[pos] == ids, pos, -1)


def build(region, manifest, manifest_sha256, tiles):
    """Soils pack bytes from `tiles` ({(property, depth): raster.Tile} in the manifest's
    CRS); pure, for testing."""
    n = len(region.cells())
    if len({(t.north, t.west, t.dlat, t.dlng, t.data.shape) for t in tiles.values()}) != 1:
        raise ValueError("soil tiles must share one grid")
    index = cell_index(region, next(iter(tiles.values())), manifest["access"]["crs"])
    m = manifest
    b = pack.Builder("soils", pack.region(region.name, region.resolution, region.cells()))
    b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
             manifest_sha256=manifest_sha256, attribution=m["license"]["attribution"])
    total = sum(t for _, t in DEPTHS)
    for prop, (name, unit, description, divisor) in PROPERTIES.items():
        mean = sum(climate.cell_means(index, tiles[(prop, d)].data, n) * t for d, t in DEPTHS) / total / divisor
        stored = np.where(np.isnan(mean), U16_NODATA, np.round(np.nan_to_num(mean) * 100)).astype(np.uint16)
        b.field(name, unit, f"{description}, 0-30 cm depth-weighted mean ({m['dataset']}, modern)", stored,
                scale=0.01, nodata=U16_NODATA)
    return b.build()


def bounds(region, crs):
    """The region's box with margin, as (south, north, west, east) in `crs` units."""
    from rasterio.warp import transform_bounds

    s, n, w, e = climate.bounds(region)
    x0, y0, x1, y1 = transform_bounds("EPSG:4326", crs, w, s, e, n, densify_pts=101)
    return (y0, y1, x0, x1)


def fetch(region, manifest, offline=False, workers=6):
    a = manifest["access"]
    box = bounds(region, a["crs"])
    keys = [(p, d) for p in PROPERTIES for d, _ in DEPTHS]

    def read(key):
        url = a["url_template"].format(prop=key[0], depth=key[1])
        return key, raster.read_window(url, box, a["block_factor"], offline)

    with ThreadPoolExecutor(max_workers=workers) as pool:
        return dict(pool.map(read, keys))


def build_pack(region, update=False, offline=False):
    m = manifests.load(DATA / "manifests" / "soilgrids.toml")
    manifests.require_redistributable(m)
    tiles = fetch(region, m, offline=offline)
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json",
                        {f"{p}_{d}": raster.fingerprint(t) for (p, d), t in tiles.items()}, update)
    return build(region, m, m.sha256, tiles)
