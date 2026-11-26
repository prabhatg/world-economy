"""The M1 sickle table against what is known of the turn to harvesting cereals (runs when
the supplement is cached: `python -m oikos_data build sickles m1-swasia`)."""

import io

import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import raster, region, sickles


@pytest.fixture(scope="module")
def m1():
    try:
        data = sickles.build_table(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("Maeda et al. 2016's supplement not cached; run: python -m oikos_data build sickles m1-swasia")
    t = pq.read_table(io.BytesIO(data)).to_pydict()
    return {k: np.array(v, dtype=float if k in ("start_year", "end_year", "share") else object) for k, v in t.items()}


def test_every_judged_phase_with_a_stated_share_is_located_in_the_region(m1):
    mid = (m1["start_year"] + m1["end_year"]) / 2
    judged, anchors = mid < sickles.JUDGED_BEFORE, mid >= sickles.ANCHOR_YEAR
    assert judged.sum() == 42, "59 phases before 8800 BC less 11 given in words and 6 at unlocated sites"
    assert anchors.sum() == 135 and judged.sum() + anchors.sum() == len(mid)
    assert all(h != 0 for h in m1["h3"][judged]) and all(h == 0 for h in m1["h3"][anchors])


def test_sickles_are_rare_before_the_holocene_and_common_by_the_pottery_neolithic(m1):
    # Maeda et al. (2016, Fig. 3a): about 1% of flint tools in the Late Epipalaeolithic,
    # a few percent in the PPNA, 10-15% by the Pottery Neolithic.
    mid = (m1["start_year"] + m1["end_year"]) / 2
    early, ppna, late = mid < -10_000, (mid >= -10_000) & (mid < -8_800), mid >= -7_000
    assert m1["share"][early].mean() < 0.02 < m1["share"][ppna].mean() < 0.06 < 0.1 < m1["share"][late].mean()
