"""The M1 bedrock pack against known soil geography (runs when the SoilGrids250m 2017
overview is cached: `python -m oikos_data build bedrock m1-swasia` reads it once)."""

import h3
import numpy as np
import pytest

from oikos_data import bedrock, pack, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        p = pack.read(bedrock.build_pack(r, offline=True))
    except raster.MissingInputs:
        pytest.skip("SoilGrids250m 2017 overview not cached; run: python -m oikos_data build bedrock m1-swasia")
    cells = r.cells()
    depth = p.values("depth_share")
    return depth, lambda lat, lng: depth[np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))]


def test_alluvial_plains_are_deep_and_hills_shallower(m1):
    _, at = m1
    plains = [(31.3, 45.5), (37.6, 33.0), (36.5, 41.0)]  # Mesopotamian alluvium, Konya plain, Jezira
    hills = [(31.75, 35.15), (37.3, 35.0), (29.0, 33.8)]  # Judean hills, Taurus, Sinai
    assert all(at(*p) == 1.0 for p in plains)
    assert all(at(*h) < 0.97 for h in hills)


def test_most_land_is_mapped_and_the_seas_are_not(m1):
    depth, at = m1
    assert np.isnan(at(33.5, 30.0)) and np.isnan(at(39.5, 51.5))  # eastern Mediterranean, Caspian
    land = depth[~np.isnan(depth)]
    assert 0.75 < len(land) / len(depth) and land.min() > 0.1 and land.max() == 1.0
