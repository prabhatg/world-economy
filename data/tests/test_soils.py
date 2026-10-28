"""Soils pack: topsoil properties per cell from projected soil-property rasters."""

import h3
import numpy as np
import pytest

from oikos_data import pack, raster, region, soils

BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
MANIFEST = {"dataset": "test-soils", "version": "1", "citation": "Test", "license": {"id": "CC-BY-4.0", "attribution": "soils (c) test"},
            "access": {"crs": "EPSG:4326"}}
SHA = "1" * 64
N = 60


def grid():
    lat = 36.2 - (np.arange(int(1.4 * N)) + 0.5) / N
    lng = 39.8 + (np.arange(int(1.4 * N)) + 0.5) / N
    return np.meshgrid(lat, lng, indexing="ij")


def tiles(**fns):
    """Per (property, depth): stored SoilGrids integers from fn(lat, lng, depth) or a constant."""
    base = {"clay": 300, "sand": 400, "silt": 300, "soc": 120, "phh2o": 75, "cfvo": 150}
    la, lo = grid()
    out = {}
    for prop, v in base.items():
        for depth, _ in soils.DEPTHS:
            data = fns[prop](la, lo, depth) if prop in fns else np.full(la.shape, float(v))
            out[(prop, depth)] = raster.Tile(np.asarray(data, dtype=np.float32), north=36.2, west=39.8, dlat=1 / N, dlng=1 / N)
    return out


def build(t):
    return pack.read(soils.build(BOX, MANIFEST, SHA, t))


def test_topsoil_properties_in_conventional_units():
    p = build(tiles())
    assert p.header["kind"] == "soils" and "time" not in p.header
    for name, want in (("clay", 30.0), ("sand", 40.0), ("silt", 30.0), ("organic_carbon", 12.0), ("ph", 7.5),
                       ("coarse_fragments", 15.0)):
        np.testing.assert_allclose(p.values(name), want, err_msg=name)
    assert p.header["sources"][0]["attribution"] == "soils (c) test"


def test_depths_are_weighted_by_thickness():
    # 0-5 cm: 100, 5-15 cm: 200, 15-30 cm: 400 g/kg clay -> (5*100 + 10*200 + 15*400) / 30 = 283.3 g/kg
    by_depth = {"0-5cm": 100, "5-15cm": 200, "15-30cm": 400}
    p = build(tiles(clay=lambda la, lo, d: np.full(la.shape, by_depth[d], dtype=float)))
    np.testing.assert_allclose(p.values("clay"), 28.33, atol=0.01)


def test_masked_pixels_are_skipped_and_unmapped_cells_have_no_data():
    c0 = h3.latlng_to_cell(*h3.cell_to_latlng(h3.int_to_str(int(BOX.cells()[0]))), 5)
    la, lo = grid()
    inside = np.array([[h3.latlng_to_cell(a, b, 5) == c0 for a, b in zip(ra, rb)] for ra, rb in zip(la, lo)])

    def city(la, lo, d):  # urban/water pixels are nodata in SoilGrids
        v = np.full(la.shape, 75.0)
        v[inside] = np.nan
        v[0, 0] = 90.0  # outside the region: ignored
        return v

    p = build(tiles(phh2o=city))
    assert np.isnan(p.values("ph")[0]) and np.all(p.values("ph")[1:] == 7.5)


def test_samples_are_located_through_the_raster_crs():
    lat, lng = soils.sample_latlng(raster.Tile(np.zeros((2, 3), np.float32), north=4_000_000.0, west=3_000_000.0,
                                               dlat=1000.0, dlng=1000.0), "ESRI:54052")
    assert lat.shape == (2, 3) and 30 < lat[0, 0] < 40 and 20 < lng[0, 0] < 30  # ~35.9 N, 26.2 E
    assert lat[0, 0] > lat[1, 0] and lng[0, 2] > lng[0, 0]
