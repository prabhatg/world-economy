"""The M1 faunal table against what is known of caprine domestication (runs when the
supplement is cached: `python -m oikos_data build fauna m1-swasia`)."""

import io

import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import fauna, raster, region


@pytest.fixture(scope="module")
def m1():
    try:
        data = fauna.build_table(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("Arbuckle et al. 2014's supplement not cached; run: python -m oikos_data build fauna m1-swasia")
    t = pq.read_table(io.BytesIO(data)).to_pydict()
    return {k: np.array(v, dtype=float if k.endswith(("_lsi", "_n", "year")) else object) for k, v in t.items()}


def test_every_measured_assemblage_is_located_in_the_region(m1):
    assert len(m1["assemblage"]) == 42, "all but Direkli and Shanidar's Mousterian"
    assert set(fauna.ASSEMBLAGES) - set(m1["assemblage"]) == set()


def test_sheep_shrink_once_they_are_herded(m1):
    # Domestic sheep are smaller than wild ones (Zeder 2008; Arbuckle et al. 2014): wild
    # sizes before 7,500 BC, domestic ones in the Pottery Neolithic of Anatolia.
    lsi, end, start = m1["ovis_lsi"], m1["end_year"], m1["start_year"]
    early = np.isfinite(lsi) & (end <= -7_500)
    late = np.isfinite(lsi) & (start >= -6_500)
    assert early.sum() >= 6 and late.sum() >= 15
    assert (lsi[early] > 0).all(), m1["assemblage"][early]
    assert np.median(lsi[late]) < -0.03
    assert (lsi[late] < 0.0).all()


def test_goats_are_kept_apart_from_sheep(m1):
    assert np.isfinite(m1["capra_lsi"].astype(float)).sum() >= 30
    i = list(m1["assemblage"]).index("Domuztepe I-III")
    assert m1["capra_lsi"][i] == pytest.approx(-0.0683) and m1["ovis_lsi"][i] == pytest.approx(-0.0399)
