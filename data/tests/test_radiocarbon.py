"""Radiocarbon observations: dates calibrated with IntCal20 and summed per cell and century."""

import io

import h3
import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import radiocarbon, raster, region

BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
SOURCES = [{"dataset": "test-dates", "version": "1", "license": {"id": "MIT"}, "sha256": "1" * 64},
           {"dataset": "test-curve", "version": "1", "license": {"id": "CC-BY-4.0"}, "sha256": "2" * 64}]
HEADER = "LabID,Age,Error,Material,Taxa,d13C,Method,Long,Lat,LocAccuracy,Period,SiteID,SiteName,Country,Province,Continent,Source,Reference\n"


def linear_curve():
    """A curve on which 14C age equals cal BP, with no curve error, at 5-year steps."""
    bp = np.arange(0, 20001, 5, dtype=float)
    return radiocarbon.Curve(bp, bp.copy(), np.zeros_like(bp))


def csv(*rows):
    lines = [HEADER] + [",".join(str(x) for x in r) + "\n" for r in rows]
    return "".join(lines).encode()


def row(lab, age, err, lat=35.5, lng=40.5, acc=3, site="Tell Test", material="charcoal"):
    return (lab, age, err, material, "", "", "", lng, lat, acc, "", "", site, "Syria", "", "Asia", "TestDB", "")


def tables(data, curve=None):
    dates, spd = radiocarbon.build(BOX, SOURCES, data, curve or linear_curve(), window=(-10_000, -9_000))
    return pq.read_table(io.BytesIO(dates)), pq.read_table(io.BytesIO(spd))


def test_calibration_on_a_known_curve_is_the_measurement():
    years, p = radiocarbon.calibrate(11_950.0, 50.0, linear_curve())
    assert p.sum() == pytest.approx(1.0)
    assert np.sum(years * p) == pytest.approx(11_950.0, abs=0.5)
    assert np.sqrt(np.sum((years - 11_950.0) ** 2 * p)) == pytest.approx(50.0, rel=0.02)


def test_dates_are_filtered_deduplicated_and_located():
    data = csv(row("A-1", 11_950, 50), row("A 1", 11_950, 50), row("B-2", 11_500, 40, acc=1),
               row("C-3", 11_000, 0), row("D-4", 11_600, 60, lat=10.0), row("E-5", 30_000, 300))
    dates, _ = tables(data)
    assert dates.column("lab_id").to_pylist() == ["A-1", "B-2", "E-5"], "same lab id twice, zero error, outside the region"
    c = h3.latlng_to_cell(35.5, 40.5, 5)
    assert dates.column("h3").to_pylist()[0] == h3.str_to_int(c)
    assert dates.column("cal_median_year").to_pylist()[0] == pytest.approx(1950 - 11_950, abs=2)
    assert dates.column("mass_in_window").to_pylist()[0] == pytest.approx(0.5, abs=0.01), "centred on the window start"
    assert dates.column("mass_in_window").to_pylist()[2] == 0
    assert dates.schema.metadata[b"sources"]


def test_density_per_cell_and_century():
    # Two dates centred on 10,000 BCE (11,950 cal BP) and 9,500 BCE in one cell; a
    # province-located date is kept in the dates table but not counted.
    data = csv(row("A-1", 11_950, 20), row("B-2", 11_450, 20), row("C-3", 11_450, 20, acc=1))
    _, spd = tables(data)
    t = spd.to_pydict()
    assert set(t["century_start_year"]) <= set(range(-10_000, -9_000, 100))
    assert sum(t["mass"]) == pytest.approx(1.5, abs=0.02), "about half of the first date falls before the window"
    by = dict(zip(t["century_start_year"], t["mass"]))
    assert by[-10_000] == pytest.approx(0.5, abs=0.02) and by[-9_600] + by[-9_500] == pytest.approx(1.0, abs=1e-3)
    assert set(t["dates"]) == {1}


def test_outputs_are_reproducible():
    data = csv(row("A-1", 11_950, 50), row("B-2", 11_450, 80))
    assert radiocarbon.build(BOX, SOURCES, data, linear_curve(), window=(-10_000, -9_000)) == \
        radiocarbon.build(BOX, SOURCES, data, linear_curve(), window=(-10_000, -9_000))


def test_intcal20_known_answer():
    try:
        curve = radiocarbon.load_curve(raster.read_text(radiocarbon.INTCAL20_URL, offline=True))
    except raster.MissingInputs:
        pytest.skip("IntCal20 not cached; run: python -m oikos_data build radiocarbon_dates m1-swasia")
    years, p = radiocarbon.calibrate(9000.0, 30.0, curve)
    median = years[np.searchsorted(np.cumsum(p), 0.5)]
    assert 10_150 < median < 10_260, "9000 +/- 30 BP calibrates to ~10,200 cal BP"
