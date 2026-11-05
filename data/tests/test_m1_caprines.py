"""The M1 caprine pack: wild sheep and goats' uplands where the zooarchaeology finds them
(runs when the terrain inputs are cached: `python -m oikos_data build caprines m1-swasia`)."""

import h3
import numpy as np
import pytest

from oikos_data import caprines, pack, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        p = pack.read(caprines.build_caprine_pack(r, offline=True))
    except raster.MissingInputs:
        pytest.skip("terrain inputs not cached; run: python -m oikos_data build terrain m1-swasia")
    return r, p.values("upland")


def at(r, v, lat, lng):
    return v[list(r.cells()).index(h3.str_to_int(h3.latlng_to_cell(lat, lng, r.resolution)))]


def test_the_early_herding_sites_lie_in_upland(m1):
    # Sites with early managed or hunted wild caprines (Zeder 2008; Arbuckle et al. 2014).
    r, v = m1
    for name, lat, lng in (("Aşıklı Höyük", 38.349, 34.23), ("Çayönü", 38.21, 39.72), ("Ganj Dareh", 34.42, 47.48),
                           ("Hallan Çemi", 38.227, 41.239), ("Öküzini", 37.089, 30.576)):
        assert at(r, v, lat, lng) > 0.5, name


def test_lowland_plains_and_africa_hold_none(m1):
    r, v = m1
    assert at(r, v, 31.0, 46.0) < 0.1, "the Mesopotamian alluvium"
    assert at(r, v, 25.5, 29.0) == 0 and at(r, v, 30.0, 31.2) == 0, "Egypt"
    assert 0 <= v.min() and v.max() <= 1
