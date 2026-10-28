"""The M1 soils pack against known soil geography (runs when the SoilGrids windows are
cached: `python -m oikos_data build soils m1-swasia` reads them once, ~1 min)."""

import h3
import numpy as np
import pytest

from oikos_data import pack, raster, region, soils


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        p = pack.read(soils.build_pack(r, offline=True))
    except raster.MissingInputs:
        pytest.skip("SoilGrids windows not cached; run: python -m oikos_data build soils m1-swasia")
    cells = r.cells()

    def at(lat, lng, field):
        return p.values(field)[np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))]

    return p, at


def test_texture_sums_to_the_whole(m1):
    p, _ = m1
    total = p.values("clay") + p.values("sand") + p.values("silt")
    ok = ~np.isnan(total)
    assert ok.mean() > 0.8, "most of the region is mapped soil"
    assert np.all(np.abs(total[ok] - 100) < 2)


def test_humid_and_dry_soils(m1):
    _, at = m1
    pontic, mesopotamia, jezira = (41.0, 40.5), (31.3, 45.5), (36.5, 41.0)
    assert at(*pontic, "ph") < 6.5 < 7.5 < at(*mesopotamia, "ph") and at(*jezira, "ph") > 7.5, "leached humid vs calcareous dry soils"
    assert at(*pontic, "organic_carbon") > 3 * at(*mesopotamia, "organic_carbon")
    for sea in [(33.5, 30.0), (39.5, 51.5)]:  # eastern Mediterranean, Caspian
        assert np.isnan(at(*sea, "clay"))
