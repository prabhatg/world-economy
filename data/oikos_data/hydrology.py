"""Hydrology pack: rivers, floodplains and lakes per cell (modern, as a proxy).

Rivers come from RiverATLAS (HydroRIVERS reaches with HydroATLAS attributes): each reach
is cut into pieces of at most `STEP` degrees, and each piece counts toward the cell
containing its midpoint. Per cell: the length of reaches with flowing water (mean
natural discharge of at least `FLOWING` m³/s, so desert wadis do not count), the
largest reach's natural long-term discharge and Strahler order, and a floodplain
fraction (each reach's catchment area times its maximum inundation extent, GIEMS-D15,
spread along the reach by length, over the cell area, less the lake surface, which
GIEMS also counts). Lakes come from LakeATLAS (HydroLAKES polygons),
rasterized at 1/120 degree; reservoirs are left out because they are modern.
"""

import h3
import numpy as np
import shapely

from . import DATA, climate, manifest as manifests, pack, raster, vector

STEP = 0.005  # degrees (~0.5 km): reach pieces assigned to cells
FLOWING = 1.0  # m3/s: least mean natural discharge counted in river_length
LAKE_RES = 120  # lake raster samples per degree
KM_PER_DEG = 6371.0088 * np.pi / 180  # mean Earth radius
RIVER_COLUMNS = ["DIS_AV_CMS", "ORD_STRA", "CATCH_SKM", "inu_pc_cmx"]
LAKE_COLUMNS = ["Lake_type"]
RESERVOIR = 2


def river_fields(region, geoms, attrs):
    cells = region.cells()
    n = len(cells)
    length, flood = np.zeros(n), np.zeros(n)
    discharge, order = np.zeros(n), np.zeros(n, dtype=np.int64)
    for g, dis, ord_, catch, inu in zip(geoms, *(attrs[c] for c in RIVER_COLUMNS)):
        xy = np.concatenate([np.asarray(part.coords)[:, :2] for part in getattr(g, "geoms", [g])])
        xy = np.asarray(shapely.segmentize(shapely.LineString(xy), STEP).coords) if len(xy) > 1 else xy
        if len(xy) < 2:
            continue
        mid = (xy[1:] + xy[:-1]) / 2
        d = np.diff(xy, axis=0)
        km = KM_PER_DEG * np.hypot(d[:, 0] * np.cos(np.radians(mid[:, 1])), d[:, 1])
        ids = np.array([h3.str_to_int(h3.latlng_to_cell(la, lo, region.resolution)) for lo, la in mid.tolist()], dtype=np.uint64)
        pos = np.minimum(np.searchsorted(cells, ids), n - 1)
        inside = cells[pos] == ids
        if not inside.any() or km.sum() == 0:
            continue
        idx, km_in = pos[inside], km[inside]
        if dis >= FLOWING:
            np.add.at(length, idx, km_in)
        np.add.at(flood, idx, catch * max(inu, 0) / 100 * km_in / km.sum())
        np.maximum.at(discharge, idx, max(dis, 0))
        np.maximum.at(order, idx, int(ord_))
    return length, discharge, order, flood


def lake_fraction(region, geoms, attrs):
    from rasterio import features
    from rasterio.transform import from_origin

    s, n, w, e = climate.bounds(region)
    rows, cols = int(np.ceil((n - s) * LAKE_RES)), int(np.ceil((e - w) * LAKE_RES))
    natural = [g for g, t in zip(geoms, attrs["Lake_type"]) if int(t) != RESERVOIR]
    water = np.zeros((rows, cols), dtype=np.uint8)
    if natural:
        water = features.rasterize(((g, 1) for g in natural), out_shape=(rows, cols),
                                   transform=from_origin(w, n, 1 / LAKE_RES, 1 / LAKE_RES), fill=0, dtype="uint8")
    # Two by two blocks: fractions at 1/60 degree, assigned to cells like the climate samples.
    blocks = water[: rows // 2 * 2, : cols // 2 * 2].reshape(rows // 2, 2, cols // 2, 2).mean(axis=(1, 3))
    tile = raster.Tile(blocks.astype(np.float32), north=n, west=w, dlat=2 / LAKE_RES, dlng=2 / LAKE_RES)
    frac = climate.cell_means(climate.cell_index(region, tile), tile.data, len(region.cells()))
    return np.nan_to_num(frac)


def build(region, rivers, lakes):
    """Hydrology pack bytes from (manifest, manifest_sha256, (geoms, attrs)) for rivers
    and lakes; pure, for testing."""
    cells = region.cells()
    (rm, rsha, (rg, ra)), (lm, lsha, (lg, la)) = rivers, lakes
    length, discharge, order, flood = river_fields(region, rg, ra)
    area = np.array([h3.cell_area(h3.int_to_str(int(c)), "km^2") for c in cells])
    lake = lake_fraction(region, lg, la)

    b = pack.Builder("hydrology", pack.region(region.name, region.resolution, cells))
    for m, sha in ((rm, rsha), (lm, lsha)):
        b.source(dataset=m["dataset"], version=m["version"], license=m["license"]["id"], citation=m["citation"],
                 manifest_sha256=sha, attribution=m["license"]["attribution"])
    src = f" ({rm['dataset']}, modern)"
    b.field("river_length", "km", f"Length of river reaches with mean natural discharge >= {FLOWING} m3/s in the cell" + src,
            np.round(length * 100).astype(np.uint32), scale=0.01)
    b.field("river_discharge", "m3/s", "Natural long-term mean discharge of the largest reach in the cell (0: none)" + src,
            np.round(discharge * 1000).astype(np.uint32), scale=0.001)
    b.field("river_order", "1", "Strahler order of the largest reach in the cell (0: none)" + src, order.astype(np.uint8))
    b.field("floodplain_fraction", "1", "Share of the cell inundated at maximum extent along its reaches (GIEMS-D15)" + src,
            np.round(np.clip(flood / area - lake, 0, 1) * 10000).astype(np.uint16), scale=1e-4)
    b.field("lake_fraction", "1", f"Share of the cell covered by natural lakes ({lm['dataset']}, modern; reservoirs excluded)",
            np.round(np.clip(lake, 0, 1) * 10000).astype(np.uint16), scale=1e-4)
    return b.build()


def fetch(region, m, columns, offline=False):
    """The features of `m`'s dataset in the region's box: from the cache, else read from
    the downloaded (and verified) archive."""
    a = m["access"]
    box = climate.bounds(region)
    try:
        return vector.read(vector.Source(path="", identity=a["md5"], layer=a["layer"]), box, columns, offline=True)
    except raster.MissingInputs:
        if offline:
            raise
    archive = vector.download(a["url"], a["file"], a["md5"])
    src = vector.Source(path=vector.path_in_zip(archive, a["member"]), identity=a["md5"], layer=a["layer"])
    return vector.read(src, box, columns)


def build_pack(region, update=False, offline=False):
    parts = []
    for name, columns in (("riveratlas", RIVER_COLUMNS), ("lakeatlas", LAKE_COLUMNS)):
        m = manifests.load(DATA / "manifests" / f"{name}.toml")
        manifests.require_redistributable(m)
        parts.append((m, m.sha256, fetch(region, m, columns, offline=offline)))
    return build(region, *parts)
