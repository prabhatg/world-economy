"""Climate pack: bioclimatic fields per cell and century from CHELSA-TraCE21k.

Each variable and 100-year slice is a global 30-arcsec GeoTIFF stored in one-row
strips; the region's rows are read once, averaged over 2x2 pixel blocks (manifest
`access.block_factor`, ~3 km² samples, ~85 per res-5 cell) and cached. Samples are
assigned to the cell containing their center and averaged per cell, land and sea alike
(the coastline moved with sea level). Slices are placed at their midpoint year (timeID
t covers years 100(t-1) to 100t-1) and the engine interpolates linearly between them.
"""

from concurrent.futures import ThreadPoolExecutor

import h3
import numpy as np

from . import DATA, manifest as manifests, pack, raster

MARGIN = 0.2  # degrees: cells whose centers are in the box extend past it
I16_NODATA, U16_NODATA = -32768, 65535

# Source variable -> pack field: name, unit, description, dtype, scale (physical units
# per stored step) and the factor from source units to physical units.
FIELDS = {
    "bio01": ("temp_mean", "degC", "Annual mean near-surface air temperature", np.int16, 0.01, 1.0),
    "bio04": ("temp_seasonality", "degC", "Standard deviation of the monthly mean temperatures", np.uint16, 0.01, 0.01),
    "bio12": ("precip", "mm/year", "Annual precipitation", np.uint16, 1.0, 1.0),
    "bio15": ("precip_cv", "%", "Coefficient of variation of monthly precipitation (precipitation seasonality)",
              np.uint16, 0.01, 1.0),
}


def time_ids(manifest):
    lo, hi = manifest["access"]["time_ids"]
    return list(range(lo, hi + 1))


def midpoint_year(time_id):
    """timeID t averages calendar years 100(t-1) .. 100t-1 (CHELSA-TraCE21k Table 4)."""
    return 100 * time_id - 50


def bounds(region):
    s, n, w, e = region.bbox
    return (s - MARGIN, n + MARGIN, w - MARGIN, e + MARGIN)


def cell_index(region, tile):
    """Position in region.cells() of the cell containing each sample's center; -1 outside."""
    cells = region.cells()
    rows, cols = tile.data.shape
    lat = tile.north - (np.arange(rows) + 0.5) * tile.dlat
    lng = tile.west + (np.arange(cols) + 0.5) * tile.dlng
    ids = np.array([[h3.str_to_int(h3.latlng_to_cell(a, b, region.resolution)) for b in lng.tolist()] for a in lat.tolist()],
                   dtype=np.uint64).ravel()
    pos = np.minimum(np.searchsorted(cells, ids), len(cells) - 1)
    return np.where(cells[pos] == ids, pos, -1)


def cell_means(index, values, n):
    """Mean of the finite samples per cell (NaN where a cell has none)."""
    v = values.ravel().astype(np.float64)
    ok = (index >= 0) & np.isfinite(v)
    count = np.bincount(index[ok], minlength=n)
    total = np.bincount(index[ok], weights=v[ok], minlength=n)
    return np.where(count > 0, total / np.maximum(count, 1), np.nan)


def build(region, manifest, manifest_sha256, tiles):
    """Climate pack bytes from `tiles` ({(var, time_id): raster.Tile}); pure, for testing."""
    ids = time_ids(manifest)
    n = len(region.cells())
    grids = {(t.north, t.west, t.dlat, t.dlng, t.data.shape) for t in tiles.values()}
    if len(grids) != 1:
        raise ValueError("climate tiles must share one grid")
    index = cell_index(region, next(iter(tiles.values())))
    missing_cells = np.bincount(index[index >= 0], minlength=n) == 0
    if missing_cells.any():
        raise ValueError(f"{int(missing_cells.sum())} cells have no climate samples; widen the window")

    b = pack.Builder("climate", pack.region(region.name, region.resolution, region.cells()))
    b.time([midpoint_year(t) for t in ids], "linear")
    m = manifest
    b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
             manifest_sha256=manifest_sha256, attribution=m["license"]["attribution"])
    src = f" ({m['dataset']} {{var}}, mean of the cell's 30-arcsec pixels, 100-year slice at its midpoint year)"
    for var, (name, unit, description, dtype, scale, factor) in FIELDS.items():
        nodata = I16_NODATA if dtype == np.int16 else U16_NODATA
        info = np.iinfo(dtype)
        lo, hi = info.min + (nodata == info.min), info.max - (nodata == info.max)
        out = np.empty((len(ids), n), dtype=dtype)
        for i, t in enumerate(ids):
            stored = np.round(cell_means(index, tiles[(var, t)].data, n) * factor / scale)
            if np.any((stored < lo) | (stored > hi)):
                raise ValueError(f"{var} {t}: values outside the range of {np.dtype(dtype).name} at scale {scale}")
            out[i] = np.where(np.isnan(stored), nodata, np.nan_to_num(stored)).astype(dtype)
        b.field(name, unit, description + src.format(var=var), out, scale=scale, nodata=nodata)
    return b.build()


def fetch(region, manifest, offline=False, workers=8):
    """The region's window of every variable and slice, from the cache or the web."""
    access = manifest["access"]
    keys = [(var, t) for var in FIELDS for t in time_ids(manifest)]
    box = bounds(region)

    def read(key):
        var, t = key
        url = access["url_template"].format(var=var, time_id=t)
        return key, raster.read_window(url, box, access["block_factor"], offline)

    with ThreadPoolExecutor(max_workers=workers) as pool:
        return dict(pool.map(read, keys))


def build_pack(region, update=False, offline=False):
    path = DATA / "manifests" / "chelsa-trace21k.toml"
    m = manifests.load(path)
    manifests.require_redistributable(m)
    tiles = fetch(region, m, offline=offline)
    hashes = {f"{var}_{t}": raster.fingerprint(tile) for (var, t), tile in tiles.items()}
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json", hashes, update)
    return build(region, m, m.sha256, tiles)
