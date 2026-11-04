"""Infection lesions operator (#42): the share of the lesioned skeletons that are
Neolithic, which the model gives from the prevalence of its endemic infection around the
sampled sites before and after farming, whatever share of the infected shows a lesion."""

import h3
import numpy as np
import pytest

from oikos_cal import lesions, record, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-11_000, -5_000)
CENTER = h3.latlng_to_cell(32.7, 35.2, 5)
CELLS = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(CENTER, 4)), dtype=np.uint64)
PERIODS = [{"name": "Natufian", "start_year": -10_500, "end_year": -8_300, "examined": 200, "affected": 2},
           {"name": "Neolithic", "start_year": -8_300, "end_year": -5_500, "examined": 205, "affected": 9}]
LAT, LNG = h3.cell_to_latlng(CENTER)
SITES = [{"name": "A", "period": "Natufian", "individuals": 200, "lat": LAT, "lng": LNG},
         {"name": "B", "period": "Neolithic", "individuals": 205, "lat": LAT, "lng": LNG}]


def activity(prevalence):
    """Ten people per cell and century; infected at `prevalence(year)` of them."""
    people = np.full((len(CELLS), WINDOW.centuries), 10.0)
    infected = people * np.array([prevalence(y) for y in WINDOW.midpoints()])[None, :]
    a = record.Activity(WINDOW, people, np.zeros_like(people), np.zeros_like(people))
    a.infected = infected
    return a


def op():
    return lesions.Operator(PERIODS, SITES, CELLS, WINDOW)


def test_the_record_is_the_neolithic_share_of_the_lesioned():
    o = op()
    assert o.observed == pytest.approx(9 / 11)
    assert o.sampling_variance == pytest.approx((9 / 11) * (2 / 11) / 11), "binomial, given 11 lesioned"


def test_the_model_share_follows_the_rise_in_prevalence_not_its_level():
    o = op()
    rise = lambda y: 0.02 if y < -8_300 else 0.08
    # 205 * 0.08 against 200 * 0.02: 16.4 / 20.4 of the lesioned are Neolithic.
    assert o.expected(activity(rise)) == pytest.approx(16.4 / 20.4)
    tenth = lambda y: rise(y) / 10
    assert o.expected(activity(tenth)) == pytest.approx(16.4 / 20.4), "whatever share shows a lesion"
    flat = lambda y: 0.05
    assert o.expected(activity(flat)) == pytest.approx(205 / 405)
    assert o.expected(activity(lambda y: 0.0 if y < -8_300 else 0.05)) == 1.0
    assert np.isnan(o.expected(activity(lambda y: 0.0))), "no infection, no lesions: undefined"


def test_sites_weigh_by_their_skeletons_and_their_own_people():
    # Each site's prevalence is its own cells' infected over people in its period; sites
    # are weighed by how many of the period's skeletons they gave.
    far = h3.cell_to_latlng(h3.grid_disk(CENTER, 4)[-1])
    sites = SITES[:1] + [{"name": "B", "period": "Neolithic", "individuals": 150, "lat": LAT, "lng": LNG},
                         {"name": "C", "period": "Neolithic", "individuals": 55, "lat": far[0], "lng": far[1]}]
    o = lesions.Operator(PERIODS, sites, CELLS, WINDOW, rings=0)
    a = activity(lambda y: 0.02)
    c = int(np.searchsorted(CELLS, h3.str_to_int(h3.latlng_to_cell(far[0], far[1], 5))))
    a.infected[c] *= 3  # 0.06 at C's cell
    neolithic = (150 * 0.02 + 55 * 0.06) / 205
    assert o.prevalence(a)[1] == pytest.approx(neolithic)
    assert o.prevalence(a)[0] == pytest.approx(0.02)


def test_one_target_judged_by_itself():
    t = targets.InfectionLesions(op(), discrepancy_sd=0.1)
    assert [x.name for x in t.targets] == ["infection lesions Neolithic share of the lesioned"]
    assert t.targets[0].discrepancy == pytest.approx(0.01)
    assert t.model(activity(lambda y: 0.02 if y < -8_300 else 0.08)) == pytest.approx([16.4 / 20.4])
