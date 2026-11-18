"""The M1 cereal-share table against what is known of the turn to cereals (runs when the
supplement is cached: `python -m oikos_data build cereals m1-swasia`)."""

import io

import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import cereals, raster, region


@pytest.fixture(scope="module")
def m1():
    try:
        data = cereals.build_table(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("Maeda et al. 2016's supplement not cached; run: python -m oikos_data build cereals m1-swasia")
    t = pq.read_table(io.BytesIO(data)).to_pydict()
    return {k: np.array(v, dtype=float if k in ("start_year", "end_year", "cereal", "items") else object)
            for k, v in t.items()}


def test_every_phase_not_already_recorded_is_located_in_the_region(m1):
    assert len(m1["phase"]) == 46, "Table S5's 106 phases less 56 at overlapping sites and 4 left out"
    assert not {cereals.site_of(p) for p in m1["phase"]} & cereals.OVERLAP


def test_cereals_are_rare_before_the_holocene_and_common_by_the_pottery_neolithic(m1):
    # Maeda et al. (2016, Fig. 3c): cereals rise from a few percent of charred remains in
    # the Epipalaeolithic to a majority at most sites by the 7th millennium BC.
    share = m1["cereal"] / m1["items"]
    mid = (m1["start_year"] + m1["end_year"]) / 2
    early, late = mid < -10_000, mid >= -7_000
    assert early.sum() == 3 and late.sum() >= 15
    assert share[early].max() < 0.25
    assert np.median(share[late]) > 0.45
