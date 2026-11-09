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


def assemblages(years, k, n, cell=SITE, crop="barley"):
    m = len(years)
    return {"crop": [crop] * m, "site": [f"site {i}" for i in range(m)], "h3": [h3.str_to_int(cell)] * m,
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


def test_wild_cereal_gathered_by_foragers_dilutes_the_sown_harvest():
    # Cultivators gather wild cereal too, as do foragers (Allaby et al. 2017): each
    # people-year of foraging effort brings `gathered` times the ears of one sown.
    a = logistic_world(0.0)  # half domestic, a quarter of the effort sown, the rest foraged
    a.cultivators *= 0.25
    a.domestic *= 0.25
    a.herders = a.people * 0.25  # herding gathers no cereal
    op = dom.Operator(assemblages([-7_950], [0], [10]), CELLS, WINDOW, gathered=0.1)
    sown, wild = 0.25, 0.1 * 0.5
    assert op.shares(a)[0] == pytest.approx((0.5 * sown + op.wild * wild) / (sown + wild))
    assert dom.Operator(assemblages([-7_950], [0], [10]), CELLS, WINDOW).shares(a)[0] == pytest.approx(0.5), \
        "gathered 0: only sown fields"
    a.cultivators[:] = 0
    a.domestic[:] = 0
    assert op.shares(a)[0] == pytest.approx(op.wild), "foragers alone: only wild stands"
    a.herders = None
    a.cultivators = a.people.copy()
    a.domestic = a.people * 0.5
    assert op.shares(a)[0] == pytest.approx(0.5), "everyone sowing: nobody gathers"


def test_foragers_gather_a_wheat_only_where_it_grows_wild():
    # Wild emmer and einkorn grow in narrower ranges than wild barley (#38): foragers
    # gather each only from the cells its wild stands reach.
    a = logistic_world(0.0)
    a.cultivators *= 0.25
    a.domestic *= 0.25
    a.emmer_domestic = a.domestic.copy()
    emmer = lambda stands: dom.Operator(assemblages([-7_950], [0], [10], crop="emmer"), CELLS, WINDOW,
                                        gathered=0.1, stands=stands).shares(a)[0]
    near = np.isin(CELLS, [h3.str_to_int(c) for c in h3.grid_disk(SITE, 1)])
    sown, foraged = 0.25, 0.1 * 0.75
    assert emmer(None) == emmer(np.ones(len(CELLS))) == pytest.approx((0.5 * sown + 1e-3 * foraged) / (sown + foraged))
    assert emmer(np.zeros(len(CELLS))) == pytest.approx(0.5), "beyond its range only the sown wheat"
    only_site = (CELLS == h3.str_to_int(SITE)).astype(float)
    wild = foraged / near.sum()
    assert emmer(only_site) == pytest.approx((0.5 * sown + 1e-3 * wild) / (sown + wild))
    with pytest.raises(ValueError, match="one per cell"):
        emmer(np.ones(3))
    with pytest.raises(ValueError, match="0 or 1"):
        emmer(np.full(len(CELLS), 0.5))


def test_a_reduced_record_reads_the_same_gathering_as_the_full_one():
    # The cells a wheat is gathered from are a zone of their own (#47): summing every
    # near cell and weighting afterwards would count the zone once per cell.
    from oikos_cal import record, zones

    a = logistic_world(0.0)
    a.cultivators *= 0.25
    a.domestic *= 0.25
    for field in record.FIELDS:
        if getattr(a, field) is None:
            setattr(a, field, np.zeros_like(a.people))
    a.emmer_domestic = a.domestic.copy()
    for stands in (np.ones(len(CELLS)), (CELLS == h3.str_to_int(SITE)).astype(float), np.zeros(len(CELLS))):
        op = dom.Operator(assemblages([-7_950], [0], [10], crop="emmer"), CELLS, WINDOW, gathered=0.1, stands=stands)
        reduced = zones.Zones(CELLS, WINDOW, op.cells()).reduce(a)
        assert op.shares(reduced) == pytest.approx(op.shares(a))


def test_each_crop_is_judged_by_its_own_share():
    # Emmer and einkorn are sown with the crop (#38): the same cultivators, their own shares.
    a = logistic_world(0.0)
    a.emmer_domestic = a.cultivators * 0.2
    a.einkorn_domestic = a.cultivators * 0.05
    emmer = dom.Operator(assemblages([-7_950], [0], [10], crop="emmer"), CELLS, WINDOW)
    einkorn = dom.Operator(assemblages([-7_950], [0], [10], crop="einkorn"), CELLS, WINDOW)
    barley = dom.Operator(assemblages([-7_950], [0], [10]), CELLS, WINDOW)
    assert [op.shares(a)[0] for op in (barley, emmer, einkorn)] == pytest.approx([0.5, 0.2, 0.05])
    mixed = assemblages([-7_950], [0], [10])
    for k, v in assemblages([-7_900], [1], [10], crop="emmer").items():
        mixed[k] += v
    with pytest.raises(ValueError, match="one crop"):
        dom.Operator(mixed, CELLS, WINDOW)
    with pytest.raises(ValueError, match="rye"):
        dom.Operator(assemblages([-7_950], [0], [10], crop="rye"), CELLS, WINDOW)
    assert dom.crop(mixed, "emmer")["median_year"] == [-7_900]
    assert dom.crop(mixed, "emmer", leave_out=("site 0",))["median_year"] == [], "sites a crop's record leaves out"
    assert dom.crop(mixed, "barley", leave_out=("site 9",))["median_year"] == [-7_950]


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
