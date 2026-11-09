"""Decadal climate anomalies (#41): a regional index from a proxy record, high-passed so
the climate pack's century means stand, as a time series pack."""

import numpy as np
import pytest

from oikos_data import anomaly, pack, region

BOX = region.Region(name="test-box", resolution=3, bbox=(30.0, 34.0, 34.0, 38.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-proxy", "version": "1", "license": "CC0-1.0", "manifest_sha256": "1" * 64,
            "citation": "Test"}]


def test_the_high_pass_removes_trends_and_keeps_short_events():
    years = np.arange(-9000, -5000, 10)
    trend = 0.001 * (years - years[0])
    event = np.where((years >= -6250) & (years < -6090), -1.0, 0.0)
    out = anomaly.high_pass(years, trend + event, window=1000)
    inside, far = (years >= -6230) & (years < -6110), (years < -7000) & (years > -8400)
    assert np.allclose(out[far], 0.0, atol=1e-9), "a linear trend is all century-scale signal"
    assert np.all(out[inside] < -0.75) and np.all(out[inside] > -1.0)


def test_the_high_pass_takes_out_the_local_line_even_at_the_ends():
    years = np.arange(0, 1000, 20)
    steep = 0.01 * years
    assert np.allclose(anomaly.high_pass(years, steep, window=400), 0.0, atol=1e-9)
    bump = steep + np.where(years == 500, 1.0, 0.0)
    out = anomaly.high_pass(years, bump, window=400)
    assert out[years == 500][0] > 0.9 and np.all(np.abs(out[years < 280]) < 1e-9)
    assert anomaly.high_pass(np.array([0, 700]), np.array([1.0, 5.0]), window=400).tolist() == [0.0, 0.0]


def test_ages_become_ascending_calendar_years_with_duplicates_averaged():
    years, values = anomaly.calendar(np.array([8200.4, 8000.0, 8199.6, 7000.0]), np.array([1.0, 2.0, 3.0, 4.0]))
    assert years.tolist() == [-6250, -6050, -5050] and values.tolist() == [2.0, 2.0, 4.0]


def test_the_pack_holds_the_index_as_a_time_series():
    years, index = np.array([-6300, -6200, -6100]), np.array([0.25, -1.5, 0.0])
    p = pack.read(anomaly.build_pack(BOX, SOURCES, years, index, "test index"))
    assert p.header["kind"] == "climate_anomaly" and p.header["time"]["calendar_year"] == years.tolist()
    assert p.header["fields"][0]["dims"] == ["time"]
    assert p.values("index") == pytest.approx(index, abs=1e-4)


TAB = """/* DATA DESCRIPTION:
License:\tCreative Commons Attribution 3.0 Unported (CC-BY-3.0)
*/
Age [ka BP]\tDepth ice/snow [m]\tAge e [±]\tδ18O H2O [‰ SMOW]
-0.03\t7.35\t0.001\t-35.16
8.21\t1000.0\t0.048\t-36.03
11.61\t1400.0\t0.1\t-36.5
"""


def test_pangaea_exports_give_ages_before_1950_and_values():
    ages, values = anomaly.parse_pangaea(TAB, "δ18O H2O [‰ SMOW]")
    assert ages.tolist() == pytest.approx([-30.0, 8210.0, 11610.0]) and values.tolist() == [-35.16, -36.03, -36.5]
    with pytest.raises(ValueError, match="no column"):
        anomaly.parse_pangaea(TAB, "d13C")


def test_the_index_starts_and_ends_at_zero_after_the_younger_dryas():
    ages = np.arange(0.0, 13000.0, 20.0)
    values = np.where((ages > 8100) & (ages < 8260), -1.0, 0.0) + np.where(ages > 11700, -5.0, 0.0)
    years, index = anomaly.index_from(ages, values, start_bp=11600, window=1000)
    assert years[0] == 1950 - 11600 - 20 and years[-1] == 1950 + 20 and index[0] == 0.0 and index[-1] == 0.0
    assert np.all(np.diff(years) > 0) and np.abs(index).max() < 1.0
    assert index[(years >= -6300) & (years <= -6170)].max() < -0.75, "the event stands out"
    assert np.abs(index[years < -9000]).max() < 0.05, "the Younger Dryas is not read"
