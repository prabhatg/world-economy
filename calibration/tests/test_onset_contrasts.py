"""Onset by wild-cereal stands: how much more cultivation the plant remains imply where
wild cereals grew best than where they grew poorly, against the same contrast in a world
(#36)."""

import numpy as np
import pytest

from oikos_cal import onset, targets

from test_onset_operator import ANCHORS, CELLS, WINDOW, activity, at, table

CEREAL = {"cereal_precip_min": 200.0, "cereal_precip_full": 350.0, "cereal_precip_wet": 900.0,
          "cereal_precip_max": 1500.0, "cereal_temp_min": 4.0, "cereal_temp_full": 10.0}
EARLY = [("a", -9500, -9000, "PPNA", 30, 100)] * 3 + [("c", -9500, -9000, "PPNA", 10, 100)] * 3
LATE = [("b", -7500, -7000, "PPNB", 60, 100)] * 3 + [("c", -7500, -7000, "PPNB", 20, 100)] * 3


def contrasts(rows=EARLY + LATE, **kw):
    op = onset.Operator(table(*ANCHORS, *rows), CELLS, WINDOW, rings=0)
    rich = np.array([n[0] != "c" for n in op.names])
    return op, targets.OnsetContrasts(op, rich, rng=np.random.default_rng(0), **kw)


def test_the_record_is_the_effort_rich_stands_imply_over_poor_ones_in_each_era():
    _, c = contrasts()
    assert [t.name for t in c.targets] == ["cultivation effort rich minus poor stands before 7600 BCE",
                                           "cultivation effort rich minus poor stands after 7600 BCE"]
    assert [t.observed for t in c.targets] == pytest.approx([0.4, 0.8]), "(0.3 - 0.1) / 0.5, (0.6 - 0.2) / 0.5"
    assert all(t.variance > 0 and t.discrepancy == pytest.approx(0.01) for t in c.targets)
    g = c.group(7)
    assert g.indices == [7, 8] and g.dof == 2


def test_a_world_shows_the_contrast_of_its_efforts():
    op, c = contrasts()
    people, cultivators = np.zeros((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    for site, effort in (("a", 0.8), ("b", 0.8), ("c", 0.2)):
        people[at(site)] = 100.0
        cultivators[at(site)] = 100.0 * effort
    assert c.model(activity(people, cultivators)) == pytest.approx([0.6, 0.6])
    people[at("c")] = 0.0  # nobody at the poor sites: they take the mean of the phases reached
    assert c.model(activity(people, cultivators)) == pytest.approx([0.0, 0.0])


def test_each_era_needs_rich_and_poor_phases():
    with pytest.raises(ValueError, match="poor stands after 7600 BCE has 0 phases"):
        contrasts(EARLY + LATE[:3])
    with pytest.raises(ValueError, match="rich stands before 7600 BCE has 3 phases"):
        contrasts(min_phases=4)


def test_a_phase_has_the_mean_suitability_of_its_cells_and_centuries():
    op = onset.Operator(table(*ANCHORS, ("a", -9500, -9300, "PPNA", 30, 100), ("c", -9500, -9400, "PPNA", 10, 100)),
                        CELLS, WINDOW, rings=0)
    years = WINDOW.midpoints()
    temp = np.full((len(years), len(CELLS)), 20.0)
    precip = np.full((len(years), len(CELLS)), 275.0)  # half-way up the dry ramp
    precip[years > -9400, at("a")] = 600.0  # the second century of the first phase: full suitability
    precip[:, at("c")] = 1200.0  # half-way down the wet ramp
    s = onset.suitability(op, temp, precip, years, CEREAL)
    assert s == pytest.approx([0.75, 0.5])
    temp[:, at("c")] = 7.0
    assert onset.suitability(op, temp, precip, years, CEREAL)[1] == pytest.approx(0.25), "too cold halves it"
