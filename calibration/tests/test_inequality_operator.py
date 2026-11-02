"""House-size inequality operator (ADR-0074): the Gini coefficient of house floor areas
that a site's excavated houses would show in a world, against dated sites (Kohler et
al. 2017), small samples included."""

import h3
import numpy as np
import pytest

from oikos_cal import inequality, record, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-10_000, -3_000)
CELLS = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.latlng_to_cell(36.5, 39.0, 5), 3)), dtype=np.uint64)


def table(*sites):
    """Sites (name, cell index, year, gini, houses)."""
    keys = ["site", "h3", "year", "gini", "houses"]
    return {k: [s[i] if k != "h3" else int(CELLS[s[1]]) for s in sites] for i, k in enumerate(keys)}


def activity(cultivators, inequality_by_cell):
    """Settled people-years [cell, century] and their mean variance of log wealth per cell."""
    n = WINDOW.centuries
    c = np.zeros((len(CELLS), n))
    c[:] = cultivators
    v = c * np.asarray(inequality_by_cell, dtype=float)[:, None]
    return record.Activity(WINDOW, c * 2, c, np.zeros_like(c), {}, None, v)


def test_the_gini_of_many_lognormal_houses_is_erf_of_half_their_log_sd():
    s = inequality.SmallSample(houses=400, replicates=200, rng=np.random.default_rng(1))
    from math import erf
    for sd in (0.3, 0.6, 1.0):
        assert s.expected(sd) == pytest.approx(erf(sd / 2), abs=0.005)
        assert s.variance(sd) < 0.001


def test_a_few_houses_give_an_unbiased_but_noisy_gini():
    # The small-sample Gini (Dixon et al. 1987) is about unbiased; its spread is large
    # for a handful of houses and shrinks with more.
    from math import erf
    few = inequality.SmallSample(houses=4, replicates=20_000, rng=np.random.default_rng(2))
    many = inequality.SmallSample(houses=19, replicates=20_000, rng=np.random.default_rng(2))
    for s in (few, many):
        assert s.expected(0.6) == pytest.approx(erf(0.3), abs=0.03)
    assert few.variance(0.6) > 2 * many.variance(0.6) > 0
    assert few.log_sd(few.expected(0.6)) == pytest.approx(0.6, abs=1e-3), "the inverse of the expected Gini"


def test_two_houses_give_the_gini_of_their_pair():
    assert inequality.sample_gini(np.array([[1.0, 3.0]])) == pytest.approx([0.5])
    assert inequality.sample_gini(np.array([[2.0, 2.0, 2.0]])) == pytest.approx([0.0])


def test_a_site_sees_the_settled_inequality_around_it_in_its_centuries():
    t = table(("a", 0, -6000, 0.3, 10), ("b", 5, -4000, 0.4, 6))
    op = inequality.Operator(t, CELLS, WINDOW, rings=0, span=300, elasticity=0.7)
    v = np.full(len(CELLS), 0.2)
    v[0], v[5] = 0.8, 0.5
    np.testing.assert_allclose(op.settled_variance(activity(1.0, v)), [0.8, 0.5])
    # Houses grow less than wealth: their log sd is the elasticity times wealth's.
    expected = op.expected(activity(1.0, v))
    assert expected[0] == pytest.approx(op.samples[0].expected(0.7 * np.sqrt(0.8)), rel=1e-9)


def test_nobody_settled_around_a_site_leaves_its_gini_undefined():
    t = table(("a", 0, -6000, 0.3, 10))
    op = inequality.Operator(t, CELLS, WINDOW, rings=1)
    assert np.isnan(op.expected(activity(0.0, np.zeros(len(CELLS))))).all()


def test_sites_outside_the_window_are_left_out():
    t = table(("early", 0, -11_000, 0.3, 10), ("a", 0, -6000, 0.3, 10))
    assert inequality.Operator(t, CELLS, WINDOW).names == ["a"]


def test_targets_are_each_sites_gini_with_its_small_sample_variance():
    t = table(("a", 0, -6000, 0.3, 4), ("b", 5, -4000, 0.35, 19))
    op = inequality.Operator(t, CELLS, WINDOW, replicates=4000)
    ginis = targets.HouseGinis(op, discrepancy_sd=0.05)
    names = [x.name for x in ginis.targets]
    assert names == ["house Gini a 6000 BCE", "house Gini b 4000 BCE"]
    assert [x.observed for x in ginis.targets] == [0.3, 0.35]
    few, many = (x.variance for x in ginis.targets)
    assert few > 2 * many > 0, "four houses say less than nineteen"
    assert all(x.discrepancy == pytest.approx(0.0025) for x in ginis.targets)
    v = np.full(len(CELLS), 0.4)
    np.testing.assert_allclose(ginis.model(activity(1.0, v)), op.expected(activity(1.0, v)))
