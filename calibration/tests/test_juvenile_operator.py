"""Juvenile-index operator: the share of 5-19 year olds among the skeletons over 5 of a
cemetery, from the fertility and growth of a world's people there and then, through a
stable population with hunter-gatherer mortality (#33, ADR-0068)."""

from types import SimpleNamespace

import h3
import numpy as np
import pytest

from oikos_cal import juvenile
from oikos_cal.radiocarbon import Window

WINDOW = Window(-10000, -5000)  # 50 centuries
SITES = {"a": (35.87, 38.40), "b": (33.0, 36.0)}


def cell(site):
    return h3.str_to_int(h3.latlng_to_cell(*SITES[site], 5))


CELLS = np.array(sorted(cell(s) for s in SITES), dtype=np.uint64)


def table(*cemeteries):
    """Cemeteries (site, start, end)."""
    rows = {k: [] for k in ("cemetery", "h3", "start_year", "end_year")}
    for i, (site, start, end) in enumerate(cemeteries):
        for k, v in (("cemetery", f"{site}{i}"), ("h3", cell(site)), ("start_year", start), ("end_year", end)):
            rows[k].append(v)
    return rows


def activity(people, cultivators):
    return SimpleNamespace(window=WINDOW, people=people, cultivators=cultivators)


def test_constant_mortality_gives_the_known_stable_population():
    # With hazard m at every age, deaths at age x in a population growing at r fall off
    # as exp(-(m + r) x), and the birth rate is m + r: 15p5 = 1 - exp(-15 b).
    flat = juvenile.Siler(0.0, 1.0, 0.01, 0.0, 1.0)
    for b, r in ((0.04, 0.0), (0.04, 0.01), (0.05, -0.005)):
        level, index = juvenile.stable(b, r, flat)
        assert level == pytest.approx((b - r) / 0.01, rel=1e-4)
        assert index == pytest.approx(1 - np.exp(-15 * b), rel=1e-4)


def test_hunter_gatherer_mortality_matches_its_published_life_table():
    hg = juvenile.HUNTER_GATHERER
    assert hg.survival(np.array([15.0]))[0] == pytest.approx(0.57, abs=0.01), "Gurven and Kaplan 2007, Table 2"
    e0 = juvenile.life_expectancy(hg)
    assert e0 == pytest.approx(30.8, abs=0.1)
    level, index = juvenile.stable(1 / e0, 0.0)
    assert level == pytest.approx(1.0, rel=1e-4), "a stationary population at its own birth rate"
    assert 0.17 < index < 0.20


def test_the_index_rises_with_fertility_whatever_the_growth():
    for r in (-0.002, 0.0, 0.002):
        idx = [juvenile.stable(b, r)[1] for b in (0.033, 0.04, 0.05, 0.06)]
        assert np.all(np.diff(idx) > 0), (r, idx)


def test_low_mortality_keeps_its_old_ages():
    # Growth near fertility needs mortality far below the hunter-gatherers', and people
    # who live past 110: the tail is integrated, not cut.
    from scipy.integrate import quad

    hg = juvenile.HUNTER_GATHERER
    level, index = juvenile.stable(0.0325, 0.03)

    def living(x):  # exp(-r x) l(x)^level
        return np.exp(level * hg.log_survival(x) - 0.03 * x)

    def deaths(x):
        return living(x) * level * hg.hazard(x)

    assert 1 / quad(living, 0, 600, limit=400)[0] == pytest.approx(0.0325, rel=1e-4)
    assert index == pytest.approx(quad(deaths, 5, 20)[0] / quad(deaths, 5, 600, limit=400)[0], rel=1e-4)
    assert juvenile.stable(0.03, 0.029)[0] > 0


def test_a_birth_rate_no_mortality_can_balance_is_refused():
    with pytest.raises(ValueError, match="birth rate"):
        juvenile.stable(0.001, 0.0)


PARAMS = {"fertility": 0.0325, "sedentary_fertility": 0.05, "growth_rate": 0.01, "sedentary_growth_rate": 0.01,
          "period_ticks": 365}


def test_a_cemetery_reads_the_fertility_of_the_people_there_and_then():
    shape = (len(CELLS), WINDOW.centuries)
    people = np.full(shape, 1000.0)
    cultivators = np.zeros(shape)
    a = int(np.searchsorted(CELLS, cell("a")))
    cultivators[a, 30:] = 1000.0  # people at a farm fully from 7,000 BCE
    op = juvenile.Operator(table(("a", -8000, -7500), ("a", -6500, -6000), ("b", -6500, -6000)), CELLS, WINDOW,
                           rings=0)
    acts = activity(people, cultivators)
    assert op.birth_rates(acts, PARAMS) == pytest.approx([0.0325, 0.05, 0.0325])
    index, reached = op.indices(acts, PARAMS)
    assert reached.all()
    assert index[1] > index[0] + 0.03 and index[2] == pytest.approx(index[0])
    same, _ = op.indices(acts, {**PARAMS, "sedentary_fertility": 0.0325})
    assert same == pytest.approx(np.full(3, index[0])), "equal fertility: no transition, no signal"


def test_rates_are_read_per_year_whatever_the_mechanism_period():
    shape = (len(CELLS), WINDOW.centuries)
    acts = activity(np.full(shape, 1000.0), np.zeros(shape))
    op = juvenile.Operator(table(("a", -8000, -7500)), CELLS, WINDOW, rings=0)
    biennial = {**PARAMS, "fertility": 0.065, "sedentary_fertility": 0.1, "period_ticks": 730}
    assert op.birth_rates(acts, biennial) == pytest.approx([0.0325])
    assert op.indices(acts, biennial)[0] == pytest.approx(op.indices(acts, PARAMS)[0])


def test_a_cemetery_counts_local_growth_up_to_what_births_and_deaths_allow():
    shape = (len(CELLS), WINDOW.centuries)
    years = WINDOW.midpoints()
    people = np.zeros(shape)
    a = int(np.searchsorted(CELLS, cell("a")))
    people[a] = 1000.0 * np.exp(0.002 * (years - years[0]))  # growing at 0.2% a year
    op = juvenile.Operator(table(("a", -8000, -7000), ("b", -8000, -7000)), CELLS, WINDOW, rings=0)
    acts = activity(people, np.zeros(shape))
    assert op.growth_rates(acts)[0] == pytest.approx(0.002, rel=1e-6)
    p = {**PARAMS, "sedentary_fertility": 0.0325}
    index, reached = op.indices(acts, p)
    assert reached.tolist() == [True, False]
    assert index[0] == pytest.approx(juvenile.stable(0.0325, 0.002)[1])
    # People arriving faster than any band grows are migrants: the dead are read at the
    # band's own growth rate at most.
    people[a] = 1000.0 * np.exp(0.04 * (years - years[0]) / 10)  # 0.4% a year, twice the intrinsic rate
    slow = {**p, "growth_rate": 0.002, "sedentary_growth_rate": 0.002}
    assert op.indices(acts, slow)[0][0] == pytest.approx(juvenile.stable(0.0325, 0.002)[1])
    people[a] = 1000.0 * np.exp(0.05 * (years - years[0]))  # far beyond fertility: no stable reading without the cap
    assert np.isfinite(op.indices(acts, p)[0][0])
