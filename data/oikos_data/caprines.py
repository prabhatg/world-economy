"""Where wild sheep and goats could live (#37): the uplands of Asia.

Wild goat (Capra aegagrus, the bezoar) keeps to rugged, rocky mountain slopes, and the
Asiatic mouflon (Ovis orientalis) to hills, foothills and upland plateaus above the
lowland plains (Uerpmann 1987); both ranged along the Taurus and Zagros, over the
Anatolian plateau, and in the Late Pleistocene and Natufian into the southern Levant and
its eastern desert margins (Yeomans et al. 2017). Neither lived in Africa, whose wild
caprines (the Barbary sheep, the Nubian ibex) were never domesticated.

A `caprines` pack holds each cell's upland share from the terrain pack: the larger of
an elevation ramp (300 to 900 m mean elevation) and a slope ramp (1 to 4 degrees mean
slope, at the terrain pack's ~360 m resolution, which underestimates rugged slopes), 0 in
Africa ([`humid.african`]). The thresholds are judgments that separate the alluvial and
coastal plains, and the flat steppe, from the hills and plateaus. Where the land is too
dry for their grazing the engine takes them out each year, from the climate pack's rain.
"""

import h3
import numpy as np

from . import humid, pack, terrain

ELEVATION = (300.0, 900.0)  # m: mean elevation from lowland to upland
SLOPE = (1.0, 4.0)  # degrees: mean slope from plain to hills
SCALE = 1e-4


def upland(elev, slope):
    """Upland share in [0, 1] of land at mean elevation `elev` (m) and mean slope `slope`."""
    ramp = lambda x, lo, hi: float(np.clip((x - lo) / (hi - lo), 0.0, 1.0))
    return max(ramp(elev, *ELEVATION), ramp(slope, *SLOPE))


def build_pack(region, terrain_bytes):
    """Caprine pack bytes from a terrain pack's bytes. Pure."""
    t = pack.read(terrain_bytes)
    elev, slope = t.values("elev_mean"), t.values("slope_mean")
    cells = region.cells()
    if len(cells) != len(elev):
        raise ValueError(f"terrain pack has {len(elev)} cells, the region {len(cells)}")
    values = np.zeros(len(cells), dtype=np.uint16)
    for i, c in enumerate(cells):
        lat, lng = h3.cell_to_latlng(h3.int_to_str(int(c)))
        if np.isfinite(elev[i]) and np.isfinite(slope[i]) and not humid.african(lat, lng):
            values[i] = round(upland(float(elev[i]), float(slope[i])) / SCALE)
    b = pack.Builder("caprines", pack.region(region.name, region.resolution, cells))
    for s in t.header["sources"]:
        b.source(**{k: s[k] for k in ("dataset", "version", "license", "citation", "manifest_sha256")},
                 attribution=s.get("attribution"))
    b.field("upland", "1", "Share of the cell's land that is upland where wild sheep and goats lived: the larger of "
            f"ramps of mean elevation ({ELEVATION[0]:.0f}-{ELEVATION[1]:.0f} m) and mean slope ({SLOPE[0]:.0f}-"
            f"{SLOPE[1]:.0f} degrees), 0 in Africa", values, scale=SCALE)
    return b.build()


def build_caprine_pack(region, update=False, offline=False):
    return build_pack(region, terrain.build_pack(region, update=update, offline=offline))
