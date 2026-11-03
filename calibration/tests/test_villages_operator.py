"""Village-size operator: the mean log area of survey sites in each era, against the mean
log size of a world's settlements dated the same way (docs/05 §5.1, #34)."""

from types import SimpleNamespace

import numpy as np
import pytest

from oikos_cal import targets, villages
from oikos_cal.radiocarbon import Window

WINDOW = Window(-8000, -3000)  # 50 centuries
EDGES = (-6000, -5000, -4000, -3000)


def table(*phases):
    """Phases (survey window, site, cell, start, end, size in ha or NaN)."""
    keys = ["window", "site", "cell", "start_year", "end_year", "size_ha"]
    return {k: [p[i] for p in phases] for i, k in enumerate(keys)}


T = table(("w", "a", 0, -6000, -5000, 1.0), ("w", "b", 0, -6000, -5000, 4.0), ("w", "c", 1, -5000, -4000, 2.0),
          ("w", "d", 1, -4000, -3000, 0.5), ("w", "e", 1, -5000, -3000, np.nan), ("w", "a", 0, -5000, -4000, 3.0),
          ("w", "f", 1, -4000, -3000, 2.0))


def test_the_record_is_the_mean_log_area_of_sized_site_phases_per_era():
    op = villages.Operator(T, WINDOW, EDGES)
    assert np.allclose(op.observed["w"], [np.log(2.0), np.log(6.0) / 2, 0.0]), \
        "phases without a size are left out; a phase counts in the eras its period covers"


def test_a_phase_spread_over_two_eras_counts_half_in_each():
    t = table(("w", "a", 0, -6000, -4000, np.e), ("w", "b", 0, -6000, -5000, 1.0), ("w", "c", 0, -4000, -3000, 1.0))
    op = villages.Operator(t, WINDOW, EDGES)
    assert op.observed["w"][0] == pytest.approx(0.5 / 1.5) and op.observed["w"][1] == pytest.approx(1.0)


def test_a_world_whose_settlements_all_hold_n_people_shows_log_n_in_every_era():
    op = villages.Operator(T, WINDOW, EDGES)
    settled = np.zeros((3, WINDOW.centuries))
    settled[:2, 20:] = 4.0  # settlement-years from 6000 BCE in the surveyed cells
    settled[2] = 100.0  # outside the surveyed landscape: never seen
    a = SimpleNamespace(settlements=settled, village_size=settled * np.log(120.0))
    a.village_size[2] = settled[2] * np.log(5.0)
    assert np.allclose(op.expected(a)["w"], np.log(120.0))


def test_an_era_without_settlements_has_no_mean_size():
    op = villages.Operator(T, WINDOW, EDGES)
    settled = np.zeros((2, WINDOW.centuries))
    settled[:, 20:30] = 1.0  # only 6000-5000 BCE
    e = op.expected(SimpleNamespace(settlements=settled, village_size=settled * np.log(50.0)))["w"]
    assert e[0] == pytest.approx(np.log(50.0)) and np.isnan(e[1:]).all()


def test_targets_convert_area_to_people_and_share_the_density_error():
    t = {k: v + T[k] if k != "window" else ["v"] * len(v) + T[k] for k, v in T.items()}
    t["site"] = [f"v{s}" for s in T["site"]] + T["site"]
    op = villages.Operator(t, WINDOW, EDGES)
    vs = targets.VillageSizes(op, density=150.0, density_sd=0.3, discrepancy_sd=0.25, rng=np.random.default_rng(1),
                              min_defined=0.5)
    names = [x.name for x in vs.targets]
    assert names[0] == "village size v 6000-5000 BCE" and names[3] == "village size w 6000-5000 BCE"
    assert vs.targets[0].observed == pytest.approx(np.log(2.0) + np.log(150.0))
    assert vs.targets[0].discrepancy == pytest.approx(0.0625)
    g = vs.group(10)
    assert g.indices == list(range(10, 16)) and g.dof == 6
    assert g.covariance[0, 3] == pytest.approx(0.09, abs=1e-12), "one density converts every era of both surveys"
    assert all(x.variance == pytest.approx(g.covariance[i, i]) for i, x in enumerate(vs.targets))
    assert all(x.variance > 0.09 for x in vs.targets), "the bootstrap over sites adds sampling error"
    settled = np.zeros((2, WINDOW.centuries))
    settled[:, 20:] = 1.0
    m = vs.model(SimpleNamespace(settlements=settled, village_size=settled * np.log(90.0)))
    assert np.allclose(m, np.log(90.0)) and len(m) == 6


def test_every_era_needs_sized_sites():
    t = table(("w", "a", 0, -6000, -5000, 1.0), ("w", "b", 0, -5000, -4000, np.nan), ("w", "c", 0, -4000, -3000, 1.0))
    op = villages.Operator(t, WINDOW, EDGES)
    with pytest.raises(ValueError, match="no sized sites in w 5000-4000 BCE"):
        targets.VillageSizes(op, density=150.0, density_sd=0.3, discrepancy_sd=0.25)


def test_settlements_are_dated_as_the_sized_phases_are():
    # A century of 6000-5000 BCE is dated to that era's period or, a third as often (half
    # the phases per year), to the long period, which spreads it over both eras: 5/6 of
    # it stays. Unsized phases do not enter the record, so they do not date it either.
    t = table(("w", "a", 0, -6000, -5000, 1.0), ("w", "b", 0, -5000, -4000, 1.0), ("w", "c", 0, -6000, -4000, 1.0),
              ("w", "u", 0, -6000, -3000, np.nan), ("w", "z", 0, -4000, -3000, 1.0))
    op = villages.Operator(t, WINDOW, EDGES)
    settled = np.zeros((1, WINDOW.centuries))
    settled[0, 20:40] = 1.0
    logs = settled * np.where(np.arange(WINDOW.centuries) < 30, np.log(10.0), np.log(100.0))
    e = op.expected(SimpleNamespace(settlements=settled, village_size=logs))["w"]
    assert e[0] == pytest.approx(5 / 6 * np.log(10.0) + 1 / 6 * np.log(100.0))
    assert e[1] == pytest.approx(1 / 6 * np.log(10.0) + 5 / 6 * np.log(100.0))
    assert np.isnan(e[2]), "no settlements in 4000-3000 BCE"


def test_a_site_spanning_two_eras_ties_their_sampling_errors():
    t = table(*[("w", f"s{i}", 0, -6000, -4000, float(i + 1)) for i in range(6)],
              *[("w", f"e{i}", 0, -6000, -5000, 2.0) for i in range(3)], *[("w", f"l{i}", 0, -4000, -3000, float(i + 1)) for i in range(4)])
    vs = targets.VillageSizes(villages.Operator(t, WINDOW, EDGES), density=150.0, density_sd=0.3)
    assert vs.covariance[0, 1] - 0.09 > 0.01, "the long-lived sites make both eras large or small together"
    assert vs.covariance[0, 2] - 0.09 == pytest.approx(0.0, abs=0.01)


def test_too_few_sites_for_a_bootstrap_are_refused():
    t = table(*[("w", f"s{e}", 0, lo, lo + 1000, 2.0) for e, lo in enumerate(EDGES[:-1])])
    with pytest.raises(ValueError, match="too few sized sites"):
        targets.VillageSizes(villages.Operator(t, WINDOW, EDGES), density=150.0, density_sd=0.3)


def test_one_era_is_enough():
    t = table(*[("w", f"s{i}", 0, -6000, -5000, float(i + 1)) for i in range(5)])
    vs = targets.VillageSizes(villages.Operator(t, WINDOW, (-6000, -5000)), density=150.0, density_sd=0.3)
    assert len(vs.targets) == 1 and vs.covariance.shape == (1, 1) and vs.targets[0].variance > 0.09
