"""The M1 radiocarbon tables against the known Neolithic record (run when p3k14c and
IntCal20 are cached: `python -m oikos_data build radiocarbon_dates m1-swasia`)."""

import io

import h3
import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import radiocarbon, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        dates, density = (pq.read_table(io.BytesIO(b)) for b in radiocarbon.build(r, *radiocarbon.inputs(offline=True)))
    except raster.MissingInputs:
        pytest.skip("p3k14c or IntCal20 not cached; run: python -m oikos_data build radiocarbon_dates m1-swasia")
    return r, dates.to_pydict(), density.to_pydict()


def mass(density, cells=None, years=(-12000, -3000)):
    return sum(m for c, y, m in zip(density["h3"], density["century_start_year"], density["mass"])
               if years[0] <= y < years[1] and (cells is None or c in cells))


def near(lat, lng):
    """The cell of a site and its neighbors (site coordinates are rounded in sources)."""
    return {h3.str_to_int(c) for c in h3.grid_disk(h3.latlng_to_cell(lat, lng, 5), 1)}


def test_coverage(m1):
    _, dates, density = m1
    assert len(dates["lab_id"]) > 6000 and len(set(dates["lab_id"])) == len(dates["lab_id"])
    assert len(set(density["cell"])) > 200


def test_the_neolithic_rise(m1):
    _, _, d = m1
    epipalaeolithic, ppnb, late_neolithic = mass(d, years=(-12000, -10000)), mass(d, years=(-8500, -7000)), mass(d, years=(-7000, -6000))
    assert late_neolithic > ppnb > 3 * epipalaeolithic, "dates multiply from the Epipalaeolithic to the Late Neolithic"
    assert mass(d, years=(-6000, -5500)) < 0.8 * mass(d, years=(-6500, -6000)), "the decline around 6000 BCE"


def test_known_sites(m1):
    _, _, d = m1
    assert mass(d, near(37.67, 32.83), (-7100, -6000)) > 20, "Catalhoyuk East"
    assert mass(d, near(31.87, 35.44), (-9600, -8500)) > 3, "Jericho, PPNA"
    assert mass(d, near(37.22, 38.92), (-9600, -8000)) > 3, "Gobekli Tepe"
