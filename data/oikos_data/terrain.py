"""Terrain pack: elevation, relief, slope and land fraction per cell from a DEM.

Pixels are read at reduced resolution (manifest `access.overview_factor`; 4 gives
~360 m for 90 m Copernicus DEM tiles, over 2,000 samples per res-5 cell), assigned to
the cell containing their center, and aggregated. Sea pixels (exactly 0 in Copernicus
DEM) are excluded; `land_fraction` is the covered land area over the cell area.
Slopes come from the sampled grid, so they underestimate slopes at 90 m resolution.
The DEM is a surface model (vegetation and buildings included).
"""

from concurrent.futures import ThreadPoolExecutor

import h3
import numpy as np

from . import DATA, manifest as manifests, pack, raster

R = 6371007.180918475  # authalic Earth radius (m), as used by H3
DEG = np.pi / 180
MARGIN = 0.2  # degrees: cells whose centers are in the box extend past it
I16_NODATA, U16_NODATA = -32768, 65535


def parse(name):
    """'N35_E040' -> (35, 40): the south-west corner of a 1-degree tile."""
    a, b = name.split("_")
    return int(a[1:]) * (1 if a[0] == "N" else -1), int(b[1:]) * (1 if b[0] == "E" else -1)


def tiles_for(region, names):
    s, n, w, e = region.bbox
    s, n, w, e = s - MARGIN, n + MARGIN, w - MARGIN, e + MARGIN
    return sorted(x for x in names if (lambda la, lo: la < n and la + 1 > s and lo < e and lo + 1 > w)(*parse(x)))


def build(region, manifest, manifest_sha256, tiles):
    """Terrain pack bytes from `tiles` ({name: raster.Tile}); pure, for testing."""
    cells = region.cells()
    n, res = len(cells), region.resolution
    count, total, squares, slope, land_area = (np.zeros(n) for _ in range(5))
    lo, hi = np.full(n, np.inf), np.full(n, -np.inf)
    for name in sorted(tiles):
        t = tiles[name]
        rows, cols = t.data.shape
        lat = t.north - (np.arange(rows) + 0.5) * t.dlat
        lng = t.west + (np.arange(cols) + 0.5) * t.dlng
        z = t.data.astype(np.float64)
        dy = t.dlat * DEG * R
        dx = (t.dlng * DEG * R * np.cos(lat * DEG))[:, None]
        gy, gx = np.gradient(z)
        steep = np.degrees(np.arctan(np.hypot(gx / dx, gy / dy)))
        area = np.broadcast_to(dy * dx, z.shape)
        ii, jj = np.nonzero(z != 0)
        ids = np.fromiter(
            (h3.str_to_int(h3.latlng_to_cell(a, b, res)) for a, b in zip(lat[ii].tolist(), lng[jj].tolist())),
            dtype=np.uint64, count=len(ii),
        )
        pos = np.minimum(np.searchsorted(cells, ids), n - 1)
        ok = cells[pos] == ids
        idx, ii, jj = pos[ok], ii[ok], jj[ok]
        v = z[ii, jj]
        np.add.at(count, idx, 1)
        np.add.at(total, idx, v)
        np.add.at(squares, idx, v * v)
        np.add.at(slope, idx, steep[ii, jj])
        np.add.at(land_area, idx, area[ii, jj])
        np.minimum.at(lo, idx, v)
        np.maximum.at(hi, idx, v)

    has = count > 0
    safe = np.where(has, count, 1)
    mean = total / safe
    sd = np.sqrt(np.maximum(squares / safe - mean * mean, 0))
    cell_area = np.array([h3.cell_area(h3.int_to_str(int(c)), "m^2") for c in cells])
    frac = np.clip(land_area / cell_area, 0, 1)

    def q(values, dtype, nodata, scale=1.0):
        return np.where(has, np.round(np.where(has, values, 0) / scale), nodata).astype(dtype)

    src = f" ({manifest['dataset']}, sampled at 1/{manifest.get('access', {}).get('overview_factor', 1)} resolution)"
    b = pack.Builder("terrain", pack.region(region.name, res, cells))
    b.source(dataset=manifest["dataset"], version=manifest["version"], license=manifest["license"]["id"],
             citation=manifest["citation"], manifest_sha256=manifest_sha256, attribution=manifest["license"]["attribution"])
    b.field("elev_mean", "m", "Mean land elevation" + src, q(mean, np.int16, I16_NODATA), nodata=I16_NODATA)
    b.field("elev_min", "m", "Lowest land sample" + src, q(lo, np.int16, I16_NODATA), nodata=I16_NODATA)
    b.field("elev_max", "m", "Highest land sample" + src, q(hi, np.int16, I16_NODATA), nodata=I16_NODATA)
    b.field("elev_sd", "m", "Standard deviation of land elevation (relief)" + src, q(sd, np.uint16, U16_NODATA), nodata=U16_NODATA)
    b.field("slope_mean", "degrees", "Mean slope of the sampled surface" + src, q(slope / safe, np.uint16, U16_NODATA, 0.01),
            scale=0.01, nodata=U16_NODATA)
    b.field("land_fraction", "1", "Fraction of the cell area covered by land samples", np.round(frac * 10000).astype(np.uint16),
            scale=1e-4)
    return b.build()


def fetch(region, manifest, offline=False, workers=8):
    """Tiles covering `region` at the manifest's overview factor, from the cache or the web."""
    access = manifest["access"]
    listed = raster.read_text(access["tile_list"], offline=offline).split()
    names = {}
    for full in listed:
        # Copernicus_DSM_COG_30_N35_00_E040_00_DEM -> N35_E040
        parts = full.split("_")
        names[f"{parts[4]}_{parts[6]}"] = full
    wanted = tiles_for(region, names)

    def read(name):
        return name, raster.read_overview(access["url_template"].format(name=names[name]), access["overview_factor"], offline)

    with ThreadPoolExecutor(max_workers=workers) as pool:
        return dict(pool.map(read, wanted))


def build_pack(region, update=False, offline=False):
    path = DATA / "manifests" / "copernicus-dem-glo90.toml"
    m = manifests.load(path)
    manifests.require_redistributable(m)
    tiles = fetch(region, m, offline=offline)
    raster.check_inputs(DATA / "locks" / f"{m['dataset']}.json", {k: raster.fingerprint(t) for k, t in tiles.items()}, update)
    return build(region, m, m.sha256, tiles)
