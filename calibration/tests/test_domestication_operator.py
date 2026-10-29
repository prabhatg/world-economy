"""The domestication-timing observation operator on synthetic worlds whose domestication
history is known (docs/05 §5.1, ADR-0051)."""

import math

import h3
import numpy as np
import pytest

from oikos_cal import domestication as dom
from oikos_cal.record import Activity
from oikos_cal.radiocarbon import Window

WINDOW = Window(-11_000, -6_000)
SITE = h3.latlng_to_cell(35.87, 38.40, 5)  # Abu Hureyra
CELLS = np.array(sorted(h3.str_to_int(c) for c in h3.grid_disk(SITE, 3)), dtype=np.uint64)


def assemblages(years, k, n, cell=SITE):
    m = len(years)
    return {"crop": ["barley"] * m, "site": [f"site {i}" for i in range(m)], "h3": [h3.str_to_int(cell)] * m,
            "median_year": list(years), "k_nonshattering": list(k), "n_rachis": list(n)}


def logistic_world(selection, half_year=-7_800, cultivated=lambda t: np.ones_like(t)):
    """Every cell cultivating (effort `cultivated`) a crop whose domestic share rises
    logistically at `selection` per year, half domestic at `half_year`."""
    t = WINDOW.midpoints().astype(float)
    d = 1 / (1 + np.exp(-selection * (t - half_year)))
    people = np.full((len(CELLS), WINDOW.centuries), 1e4)
    cultivators = people * cultivated(t)
    return Activity(WINDOW, people, cultivators, cultivators * d)


def test_beta_binomial_is_a_distribution_that_tends_to_the_binomial():
    n, p = 12, 0.3
    for phi in (0.5, 10.0):
        total = sum(math.exp(dom.beta_binomial(np.array([k]), np.array([n]), np.array([p]), phi)[0]) for k in range(n + 1))
        assert total == pytest.approx(1.0)
    k = np.arange(n + 1)
    binomial = [math.log(math.comb(n, j)) + j * math.log(p) + (n - j) * math.log(1 - p) for j in k]
    np.testing.assert_allclose(dom.beta_binomial(k, np.full(n + 1, n), np.full(n + 1, p), 1e6), binomial, rtol=1e-4)
    assert dom.beta_binomial(np.array([0]), np.array([100]), np.array([0.3]), 2.0)[0] > \
        dom.beta_binomial(np.array([0]), np.array([100]), np.array([0.3]), 1e9)[0], "overdispersion forgives outliers"


def test_the_model_share_is_the_domestic_share_of_the_crop_near_the_site_around_its_date():
    a = logistic_world(0.0)  # half domestic everywhere and always
    far = [i for i, c in enumerate(CELLS) if h3.grid_distance(h3.int_to_str(int(c)), SITE) >= 2]
    a.domestic[far] = a.cultivators[far]  # fully domestic beyond the neighbourhood: ignored
    c = (-8_000 - WINDOW.first) // 100
    a.domestic[:, c + 2:] = a.cultivators[:, c + 2:]  # fully domestic from two centuries later: ignored
    op = dom.Operator(assemblages([-7_950], [0], [10]), CELLS, WINDOW)
    assert op.shares(a)[0] == pytest.approx(0.5)
    a.cultivators[:] = 0
    a.domestic[:] = 0
    assert op.shares(a)[0] == pytest.approx(op.wild), "nobody cultivates nearby: only wild stands"


def test_assemblages_outside_the_window_or_the_world_are_refused():
    with pytest.raises(ValueError, match="window"):
        dom.Operator(assemblages([-12_000], [0], [10]), CELLS, WINDOW)
    with pytest.raises(ValueError, match="not in the world"):
        dom.Operator(assemblages([-8_000], [0], [10], cell=h3.latlng_to_cell(0.0, 0.0, 5)), CELLS, WINDOW)


def test_the_true_selection_strength_of_a_synthetic_world_scores_best():
    # Assemblages drawn from a protracted domestication (s = 0.003/yr) with the
    # overdispersion of real assemblages, at random phases.
    rng = np.random.default_rng(4)
    truth = logistic_world(0.003)
    years = rng.integers(-10_500, -6_100, 40)
    n = rng.integers(5, 400, 40)
    op = dom.Operator(assemblages(years, np.zeros(40, int), n), CELLS, WINDOW)
    p = op.shares(truth)
    k = rng.binomial(n, rng.beta(p * op.concentration, (1 - p) * op.concentration))
    op = dom.Operator(assemblages(years, k, n), CELLS, WINDOW)
    best = op.log_likelihood(truth)
    for name, alt in {"rapid (Hillman and Davies)": logistic_world(0.05),
                      "three times slower": logistic_world(0.001),
                      "800 years early": logistic_world(0.003, half_year=-8_600),
                      "800 years late": logistic_world(0.003, half_year=-7_000)}.items():
        assert op.log_likelihood(alt) < best - 20, name


def test_domestic_ears_where_the_model_has_no_crop_cost_but_stay_finite():
    op = dom.Operator(assemblages([-7_000], [96], [96]), CELLS, WINDOW)
    none = logistic_world(0.003, cultivated=lambda t: np.zeros_like(t))
    assert np.isfinite(op.log_likelihood(none)) and op.log_likelihood(none) < op.log_likelihood(logistic_world(0.003)) - 5
