"""Terrain pack: per-cell elevation, relief, slope and land fraction from DEM tiles."""

import json
import subprocess
from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import pack, raster, region, terrain

OIKOS = Path(__file__).resolve().parents[2] / "engine" / "target" / "release" / "oikos"
BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
MANIFEST = {"dataset": "test-dem", "version": "1", "citation": "Test", "license": {"id": "T", "attribution": "produced using test"}}
MANIFEST_SHA = "1" * 64
N = 300  # samples per degree


def plane_tile(sea_west_of=None):
    """Elevation rising 1000 m per degree of latitude northward; optional sea (0) in the west."""
    lat = 36.0 - (np.arange(N) + 0.5) / N
    lng = 40.0 + (np.arange(N) + 0.5) / N
    z = np.repeat((100 + 1000 * (lat - 35))[:, None], N, axis=1).astype(np.float32)
    if sea_west_of is not None:
        z[:, lng < sea_west_of] = 0
    return raster.Tile(z, north=36.0, west=40.0, dlat=1 / N, dlng=1 / N)


def build(tiles):
    return pack.read(terrain.build(BOX, MANIFEST, MANIFEST_SHA, tiles))


def centers():
    return np.array([h3.cell_to_latlng(h3.int_to_str(int(c))) for c in BOX.cells()])


def test_plane_gives_known_mean_and_slope():
    p = build({"N35_E040": plane_tile()})
    lat = centers()[:, 0]
    np.testing.assert_allclose(p.values("elev_mean"), 100 + 1000 * (lat - 35), atol=4)
    assert np.all(p.values("elev_min") < p.values("elev_mean")) and np.all(p.values("elev_mean") < p.values("elev_max"))
    expected = np.degrees(np.arctan(1000 / (np.pi / 180 * terrain.R)))  # 0.515 degrees
    np.testing.assert_allclose(p.values("slope_mean"), expected, rtol=0.03)
    assert np.all(p.values("land_fraction") > 0.95)
    assert p.header["kind"] == "terrain" and p.header["sources"][0]["attribution"] == "produced using test"


def test_sea_and_missing_tiles():
    p = build({"N35_E040": plane_tile(sea_west_of=40.5)})
    lng = centers()[:, 1]
    frac = p.values("land_fraction")
    assert np.all(frac[lng < 40.4] < 0.05) and np.all(frac[lng > 40.6] > 0.95)
    assert np.all(np.isnan(p.values("elev_mean")[frac == 0]))
    empty = build({})
    assert np.all(empty.values("land_fraction") == 0) and np.all(np.isnan(empty.values("elev_mean")))


def test_terrain_pack_is_reproducible_and_matches_the_geometry_region():
    a = terrain.build(BOX, MANIFEST, MANIFEST_SHA, {"N35_E040": plane_tile()})
    assert a == terrain.build(BOX, MANIFEST, MANIFEST_SHA, {"N35_E040": plane_tile()})
    assert pack.read(a).header["region"]["cells_hash"] == pack.cells_hash(BOX.cells())


def test_tiles_covering_a_region():
    names = ["N35_E040", "N35_E041", "N34_E040", "N36_E040", "S01_W001"]
    assert terrain.tiles_for(BOX, names) == ["N35_E040"]
    wide = region.Region(name="w", resolution=5, bbox=(34.5, 35.5, 40.5, 41.5), sha256="0" * 64)
    assert terrain.tiles_for(wide, names) == ["N34_E040", "N35_E040", "N35_E041"]


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_engine_reads_the_terrain_pack(tmp_path):
    f = tmp_path / "t.oikospack"
    f.write_bytes(terrain.build(BOX, MANIFEST, MANIFEST_SHA, {"N35_E040": plane_tile()}))
    s = json.loads(subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True, text=True).stdout)
    assert {x["name"] for x in s["fields"]} == {"elev_mean", "elev_min", "elev_max", "elev_sd", "slope_mean", "land_fraction"}
