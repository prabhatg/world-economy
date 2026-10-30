"""Site-count operator: survey site phases dated to cultural periods, and the same dating
applied to a world's settled activity (docs/05 §5.1, #17)."""

import numpy as np
import pytest

from oikos_cal import sites, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-8000, -3000)  # 50 centuries
EDGES = (-6000, -5000, -4000, -3000)


def table(*phases):
    """Phases (survey window, site, cell, start, end)."""
    keys = ["window", "site", "cell", "start_year", "end_year"]
    return {k: [p[i] for p in phases] for i, k in enumerate(keys)}


def history(cells, values):
    """Activity [cell, century] with every listed cell following `values` per century."""
    a = np.zeros((cells, WINDOW.centuries))
    a[:] = values
    return a


def test_phases_of_one_era_count_once_and_activity_maps_to_its_era():
    t = table(("w", "a", 0, -6000, -5000), ("w", "b", 0, -5000, -4000), ("w", "c", 1, -5000, -4000))
    op = sites.Operator(t, WINDOW, EDGES)
    assert op.windows == ["w"] and list(op.footprint["w"]) == [0, 1]
    assert np.allclose(op.observed["w"], [1, 2, 0])
    a = history(3, np.arange(WINDOW.centuries, dtype=float))
    a[2] = 1000  # outside the surveyed landscape
    c = WINDOW.midpoints()
    expected = [2 * a[0, (c > lo) & (c < hi)].sum() for lo, hi in [(-6000, -5000), (-5000, -4000)]] + [0]
    assert np.allclose(op.expected(a)["w"], expected), "the era 4000-3000 BCE has no period to date sites to"


def test_long_periods_spread_sites_and_activity_alike():
    # Most sites are dated to one era; some only to a long period spanning two.
    t = table(*[("w", f"s{i}", 0, -6000, -5000) for i in range(3)], ("w", "long", 0, -6000, -4000),
              ("w", "late", 0, -5000, -4000))
    op = sites.Operator(t, WINDOW, EDGES)
    assert np.allclose(op.observed["w"], [3.5, 1.5, 0])
    a = history(1, 0.0)
    a[0, (WINDOW.midpoints() > -6000) & (WINDOW.midpoints() < -5000)] = 1  # activity only in 6000-5000 BCE
    # Popularity is phases per year of the period: 3 per 1,000 years for the era, 1 per
    # 2,000 for the long period, so a century of 6000-5000 BCE is dated to the era with
    # weight 3 / 3.5 and to the long period (half of it in each era) with 0.5 / 3.5.
    assert np.allclose(op.expected(a)["w"], [10 * (3 / 3.5 + 0.5 / 3.5 / 2), 10 * (0.5 / 3.5 / 2), 0])


def test_a_survey_drawn_from_a_known_history_is_recovered():
    # Villages occupied in proportion to a rising history, dated to a period scheme of
    # short and long periods; the operator applied to that history matches what was found.
    rng = np.random.default_rng(1)
    c = WINDOW.midpoints()
    truth = np.where(c > -6500, np.interp(c, [-6500, -3000], [1.0, 4.0]), 0.0)
    periods = [(-6500, -5800), (-5800, -5000), (-5000, -4300), (-4300, -3000), (-6500, -5000), (-5000, -3000)]
    weight = np.array([3, 3, 3, 3, 1, 1.0])
    phases = []
    for i, century in enumerate(rng.choice(len(c), size=20_000, p=truth / truth.sum())):
        year = c[century]
        k = [j for j, (lo, hi) in enumerate(periods) if lo < year < hi]
        j = rng.choice(k, p=weight[k] / weight[k].sum())
        phases.append(("w", f"s{i}", 0, *periods[j]))
    op = sites.Operator(table(*phases), WINDOW, EDGES)
    model = op.expected(history(1, truth))["w"]
    assert np.allclose(model / model.sum(), op.observed["w"] / op.observed["w"].sum(), atol=0.01)


def test_eras_give_centered_log_shares_with_bootstrap_variances_and_one_group_per_survey():
    rng = np.random.default_rng(2)
    phases = [("north", f"n{i}", 0, lo, lo + 1000) for i, lo in enumerate(rng.choice([-6000, -5000, -4000], 300))]
    phases += [("south", f"s{i}", 1, lo, lo + 1000) for i, lo in enumerate(rng.choice([-6000, -5000, -4000], 200))]
    op = sites.Operator(table(*phases), WINDOW, EDGES)
    eras = targets.SiteEras(op, discrepancy_sd=0.25, rng=np.random.default_rng(0))
    names = [t.name for t in eras.targets]
    assert names[:3] == ["sites north 6000-5000 BCE", "sites north 5000-4000 BCE", "sites north 4000-3000 BCE"]
    assert len(names) == 6 and names[3].startswith("sites south")
    z = np.array([t.observed for t in eras.targets])
    assert z[:3].sum() == pytest.approx(0) and z[3:].sum() == pytest.approx(0)
    assert all(0.001 < t.variance < 0.05 and t.discrepancy == 0.0625 for t in eras.targets)
    g = eras.groups(first=4)
    assert [x.indices for x in g] == [[4, 5, 6], [7, 8, 9]] and all(x.dof == 2 for x in g)
    a = history(2, 1.0)
    assert np.allclose(eras.model(a), 0), "an even history is even in every era"


def test_an_era_without_sites_is_an_error():
    with pytest.raises(ValueError, match="no sites"):
        targets.SiteEras(sites.Operator(table(("w", "a", 0, -6000, -5000)), WINDOW, EDGES))
