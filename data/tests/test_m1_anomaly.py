"""The M1 climate-anomaly pack against the Holocene's known abrupt events (runs when the
NGRIP export is cached: `python -m oikos_data build climate_anomaly m1-swasia`)."""

import numpy as np
import pytest

from oikos_data import anomaly, pack, raster, region


@pytest.fixture(scope="module")
def series():
    try:
        p = pack.read(anomaly.build_anomaly_pack(region.load("m1-swasia"), offline=True))
    except raster.MissingInputs:
        pytest.skip("NGRIP export not cached; run: python -m oikos_data build climate_anomaly m1-swasia")
    return np.array(p.header["time"]["calendar_year"]), p.values("index")


def century_means(years, index, first, last):
    return {c: index[(years > c - 100) & (years <= c)].mean() for c in range(first, last + 1, 100)}


def test_the_8_2_ka_event_is_the_deepest_century_of_m1s_holocene(series):
    years, index = series
    means = century_means(years, index, -9_500, -3_000)
    coldest = min(means, key=means.get)
    assert coldest == -6_200 and -1.2 < means[coldest] < -0.8, (coldest, means[coldest])
    # Its 20-year means from 8,250 to 8,110 BP (Rasmussen et al. 2007).
    event = index[(years >= -6_300) & (years <= -6_160)]
    assert len(event) == 8 and event.mean() < -0.9


def test_the_index_is_zero_through_the_younger_dryas_and_decadal_after(series):
    years, index = series
    assert np.all(index[years < 1950 - 11_600] == 0.0)
    assert set(np.diff(years[(years > -9_000) & (years < -3_000)])) == {20}
    assert 0.3 < index[(years > -9_600) & (years < -3_000)].std() < 0.5
