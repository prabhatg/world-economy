"""Bedrock pack: each cell's depth to bedrock within 2 m, from a depth raster."""

import h3
import numpy as np

from oikos_data import bedrock, pack, raster, region

BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
MANIFEST = {"dataset": "test-bedrock", "version": "1", "citation": "Test",
            "license": {"id": "ODbL-1.0", "attribution": "bedrock (c) test"},
            "access": {"nodata": 255, "cap_cm": 200}}
SHA = "1" * 64
N = 30


def tile(fn):
    """A raster over the box (and beyond it) with depths fn(lat, lng) in cm."""
    lat = 36.2 - (np.arange(int(1.4 * N)) + 0.5) / N
    lng = 39.8 + (np.arange(int(1.4 * N)) + 0.5) / N
    la, lo = np.meshgrid(lat, lng, indexing="ij")
    return raster.Tile(np.asarray(fn(la, lo), dtype=np.float32), north=36.2, west=39.8, dlat=1 / N, dlng=1 / N)


def build(t):
    return pack.read(bedrock.build(BOX, MANIFEST, SHA, t))


def test_depth_is_a_share_of_two_metres():
    p = build(tile(lambda la, lo: np.where(lo < 40.5, 50.0, 200.0)))
    assert p.header["kind"] == "bedrock" and "time" not in p.header
    cells = BOX.cells()
    lng = np.array([h3.cell_to_latlng(h3.int_to_str(int(c)))[1] for c in cells])
    depth = p.values("depth_share")
    assert np.all(depth[lng < 40.4] < 0.3) and np.all(depth[lng > 40.6] == 1.0)
    np.testing.assert_allclose(depth[lng < 40.4], 0.25, atol=1e-3)
    assert p.header["sources"][0]["attribution"] == "bedrock (c) test"
    assert p.header["sources"][0]["license"] == "ODbL-1.0"


def test_no_data_pixels_are_skipped_and_unmapped_cells_have_no_data():
    c0 = int(BOX.cells()[0])

    def water(la, lo):
        v = np.full(la.shape, 100.0)
        inside = np.vectorize(lambda a, b: h3.str_to_int(h3.latlng_to_cell(a, b, 5)) == c0)(la, lo)
        v[inside] = 255.0  # water and other unmapped land
        v[0, 0] = -32768.0
        return v

    depth = build(tile(water)).values("depth_share")
    assert np.isnan(depth[0]) and np.allclose(depth[1:], 0.5, atol=1e-3)


def test_depth_never_exceeds_the_cap():
    np.testing.assert_allclose(build(tile(lambda la, lo: np.full(la.shape, 200.0))).values("depth_share"), 1.0)
    assert bedrock.share(np.array([250.0]), 200)[0] == 1.0


def test_the_window_is_cut_from_a_larger_raster():
    big = raster.Tile(np.arange(100 * 200, dtype=np.float32).reshape(100, 200), north=40.0, west=30.0, dlat=0.1, dlng=0.1)
    w = bedrock.crop(big, (35.0, 37.0, 33.0, 34.0))
    assert w.data.shape == (20, 10) and w.north == 37.0 and w.west == 33.0
    assert w.data[0, 0] == big.data[30, 30]
