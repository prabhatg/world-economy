"""Summaries of the record for history matching, on synthetic records whose histories
are known (docs/05 §5.2, ADR-0052)."""

import h3
import numpy as np
import pytest

from oikos_cal import domestication as dom
from oikos_cal import history, targets
from oikos_cal import radiocarbon as rc
from oikos_cal.record import Activity

from test_domestication_operator import CELLS, SITE, WINDOW, assemblages, logistic_world
from test_radiocarbon_operator import SETTINGS, TRUE, linear_curve


def eight_blocks():
    """Two cells in each of eight resolution-2 blocks across the region."""
    out = []
    for lat, lng in ((32.0, 35.5), (33.0, 47.0), (36.5, 38.0), (37.8, 32.5), (30.5, 35.8), (34.5, 36.8),
                     (36.2, 43.0), (35.0, 51.0)):
        c = h3.latlng_to_cell(lat, lng, 5)
        out += [h3.str_to_int(c), h3.str_to_int(next(n for n in h3.grid_ring(c, 1)
                                                       if h3.cell_to_parent(n, 2) == h3.cell_to_parent(c, 2)))]
    cells = np.array(sorted(set(out)), dtype=np.uint64)
    assert len({h3.cell_to_parent(h3.int_to_str(int(c)), 2) for c in cells}) == 8
    return cells


def growth(cells, rate):
    t = TRUE.midpoints()
    return np.ones((len(cells), len(t))) * np.exp(rate * (t - t[0]))


@pytest.fixture(scope="module")
def radiocarbon():
    cells, curve = eight_blocks(), linear_curve()
    truth = growth(cells, 4e-4)
    dates = rc.synthetic_dates(cells, truth, TRUE, curve, 3000, [40.0, 80.0], rc.NO_TAPHONOMY,
                               np.random.default_rng(1))
    op = rc.Operator(dates, curve, cells, SETTINGS)
    return cells, truth, targets.RadiocarbonEras(op, eras=3, discrepancy_sd=0.0, taphonomy=rc.NO_TAPHONOMY,
                                                 rng=np.random.default_rng(2))


def test_radiocarbon_eras_are_centered_log_shares_of_the_date_mass(radiocarbon):
    cells, truth, eras = radiocarbon
    z = np.array([t.observed for t in eras.targets])
    assert len(z) == 3 and z.sum() == pytest.approx(0, abs=1e-12)
    assert z[0] < z[1] < z[2], "a growing history leaves more dates late"
    assert [t.name for t in eras.targets] == ["radiocarbon 9500-8500 BCE", "radiocarbon 8500-7500 BCE",
                                              "radiocarbon 7500-6500 BCE"]
    assert all(0 < t.variance < 0.01 for t in eras.targets), "3000 dates in 8 blocks pin the shape down"


def test_the_true_history_is_plausible_and_stasis_is_not(radiocarbon):
    cells, truth, eras = radiocarbon
    i = lambda a: history.implausibility(eras.model(a)[None], 0.0, eras.targets)[0].max()
    assert i(truth) < 3
    assert i(growth(cells, 0.0)) > 10
    assert i(10 * truth) == pytest.approx(i(truth)), "overall scale is not observed"


def test_radiocarbon_eras_must_divide_the_window(radiocarbon):
    *_, eras = radiocarbon
    with pytest.raises(ValueError, match="divide"):
        targets.RadiocarbonEras(eras.op, eras=7)


def rachis_table():
    years = [-10_500, -9_500, -8_800, -8_200, -7_500, -6_500, -6_200]
    k = [0, 1, 10, 30, 40, 90, 45]
    n = [50, 100, 100, 100, 50, 100, 50]
    return assemblages(years, k, n)


def test_domestication_eras_are_weighted_mean_shares():
    t = rachis_table()
    op = dom.Operator(t, CELLS, WINDOW)
    eras = targets.DomesticationEras(op, t, edges=(-9_000, -8_000, -7_000), discrepancy_sd=0.1)
    names = [x.name for x in eras.targets]
    assert names == ["domestic barley before 9000 BCE", "domestic barley 9000-8000 BCE",
                     "domestic barley 8000-7000 BCE", "domestic barley after 7000 BCE"]
    w = lambda n: n / (n + op.concentration)
    first = (w(50) * 0 + w(100) * 0.01) / (w(50) + w(100))
    assert eras.targets[0].observed == pytest.approx(first)
    assert eras.targets[1].observed == pytest.approx(0.2)
    assert eras.targets[3].observed == pytest.approx(0.9)
    assert all(x.discrepancy == pytest.approx(0.01) for x in eras.targets)
    late, middle = eras.targets[3].variance, eras.targets[2].variance
    assert 0 < late < middle, "two assemblages pin a share down better than one"


def test_the_model_side_of_domestication_eras():
    t = rachis_table()
    op = dom.Operator(t, CELLS, WINDOW)
    eras = targets.DomesticationEras(op, t, edges=(-9_000, -8_000, -7_000))
    flat = logistic_world(0.0)  # every crop half domestic
    assert eras.model(flat) == pytest.approx([0.5] * 4)
    m = eras.model(logistic_world(0.003))
    assert (np.diff(m) > 0).all()


def test_an_era_without_assemblages_is_refused():
    t = rachis_table()
    op = dom.Operator(t, CELLS, WINDOW)
    with pytest.raises(ValueError, match="no assemblages"):
        targets.DomesticationEras(op, t, edges=(-9_000, -8_000, -7_600, -7_000))


def test_radiocarbon_eras_are_judged_together_by_their_bootstrap_covariance(radiocarbon):
    cells, truth, eras = radiocarbon
    g = eras.group(4)
    assert g.indices == [4, 5, 6] and g.dof == 2, "centered log shares: one degree of freedom fewer"
    assert np.allclose(np.diag(g.covariance), [t.variance for t in eras.targets])
    assert np.allclose(g.covariance.sum(axis=1), 0, atol=1e-12), "shares that sum to zero co-vary to zero"
    m = np.zeros((1, 7))
    m[0, 4:] = eras.model(truth)
    padded = [None] * 4 + eras.targets
    assert g.implausibility(m, 0.0, padded)[0] < history.chi2_cutoff(2)
    m[0, 4:] = eras.model(growth(cells, 0.0))
    assert g.implausibility(m, 0.0, padded)[0] > history.chi2_cutoff(2)
