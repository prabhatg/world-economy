"""The M1 terrain pack against known geography (runs when the DEM tiles are cached:
`python -m oikos_data build terrain m1-swasia` fetches them)."""

import h3
import numpy as np
import pytest

from oikos_data import pack, raster, region, terrain


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        data = terrain.build_pack(r, offline=True)
    except raster.MissingInputs:
        pytest.skip("DEM tiles not cached; run: python -m oikos_data build terrain m1-swasia")
    index = {int(c): i for i, c in enumerate(r.cells())}
    p = pack.read(data)

    def at(lat, lng, field):
        return p.values(field)[index[h3.str_to_int(h3.latlng_to_cell(lat, lng, 5))]]

    return p, at


def test_known_places(m1):
    p, at = m1
    assert -440 < at(31.5, 35.5, "elev_min") < -400, "Dead Sea shore, about -430 m"
    assert at(35.95, 52.11, "elev_max") > 5000, "Damavand, 5,610 m"
    assert at(30.96, 46.1, "elev_mean") < 20 and at(30.96, 46.1, "slope_mean") < 0.5, "flat southern Mesopotamia"
    assert 950 < at(37.67, 32.83, "elev_mean") < 1060, "Konya plain at Catalhoyuk, about 1,000 m"
    assert at(34.43, 47.48, "slope_mean") > at(30.96, 46.1, "slope_mean") * 10, "Zagros steeper than the alluvium"
    for sea in [(33.5, 30.0), (27.0, 51.0)]:
        assert at(*sea, "land_fraction") == 0 and np.isnan(at(*sea, "elev_mean"))


def test_region_is_mostly_land(m1):
    p, _ = m1
    frac = p.values("land_fraction")
    assert 0.75 < np.mean(frac > 0.5) < 0.9
