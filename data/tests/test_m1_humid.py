"""The M1 humid-period pack against what is known of the Eastern Sahara's Humid Period
(runs when Bartlein et al.'s grid is cached: `python -m oikos_data build humid_period
m1-swasia`)."""

import h3
import numpy as np
import pytest

from oikos_data import humid, pack, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        p = pack.read(humid.build_humid_pack(r, offline=True))
    except raster.MissingInputs:
        pytest.skip("Bartlein et al.'s grid not cached; run: python -m oikos_data build humid_period m1-swasia")
    return r, p.values("precip_anomaly")


def at(r, v, lat, lng):
    return v[list(r.cells()).index(h3.str_to_int(h3.latlng_to_cell(lat, lng, r.resolution)))]


def test_the_western_desert_gets_tens_of_millimetres_falling_off_northward(m1):
    r, v = m1
    # Southern Egypt's desert, where the region begins, against the oases and the north.
    assert 20 <= at(r, v, 24.2, 30.0) <= 30, "near 24 N"
    assert 8 <= at(r, v, 25.5, 29.0) <= 15, "Dakhla"
    assert 1 <= at(r, v, 27.0, 28.0) <= 5, "Farafra"
    assert at(r, v, 30.0, 31.2) < 1, "Cairo: the north stayed dry"


def test_asia_gets_none(m1):
    r, v = m1
    for lat, lng in ((29.5, 33.8), (31.8, 35.2), (25.0, 40.0), (33.0, 44.0)):
        assert at(r, v, lat, lng) == 0, (lat, lng)
    cells = r.cells()
    lng = np.array([h3.cell_to_latlng(h3.int_to_str(int(c)))[1] for c in cells])
    assert v[lng > 37.0].max() == 0, "nothing east of the Red Sea"
