"""Sheep size operator (#37): the mean log size index (LSI) of an era's measured sheep
bones, which a world gives as a mixture of wild and domestic sizes, by the domestic share
of the bones its people discarded: from herds in proportion to herding effort, and from
wild sheep and goats hunted on their range in proportion to foraging there."""

import h3
import numpy as np
import pytest

from oikos_cal import herds, record, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-10_000, -6_000)
CENTER = h3.latlng_to_cell(37.7, 32.8, 5)
CELLS = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(CENTER, 3)), dtype=np.uint64)
LAT, LNG = h3.cell_to_latlng(CENTER)
EDGES = (-8_000,)  # two eras: 10,000-8,000 and 8,000-6,000 BCE
WILD, DOMESTIC = (0.04, 0.02), (-0.04, 0.01)


def table(*rows):
    """Assemblages (name, start, end, lsi, sd, n) at the center."""
    keys = ["assemblage", "start_year", "end_year", "ovis_lsi", "ovis_lsi_sd", "ovis_lsi_n"]
    t = {k: [r[i] for r in rows] for i, k in enumerate(keys)}
    t["lat"], t["lng"] = [LAT] * len(rows), [LNG] * len(rows)
    return t


def activity(herded, domestic, hunted):
    """People-years per cell and century of herders, their domestic animals and hunters
    on the range, each a function of the century's midpoint year."""
    n, mid = len(CELLS), WINDOW.midpoints()
    col = lambda f: np.tile(np.array([f(y) for y in mid], dtype=float), (n, 1))
    a = record.Activity(WINDOW, np.full((n, WINDOW.centuries), 100.0), *(np.zeros((n, WINDOW.centuries)),) * 2)
    a.herders, a.herd_domestic, a.hunters = col(herded), col(domestic), col(hunted)
    return a


def op(t=None):
    t = t or table(("A", -9_500, -9_000, 0.05, 0.03, 10), ("B", -7_000, -6_500, -0.03, 0.04, 40),
                   ("C", -6_800, -6_200, -0.05, None, 10))
    return herds.Operator(t, CELLS, WINDOW, EDGES, wild=WILD, domestic=DOMESTIC, hunting=0.5)


def test_eras_hold_their_assemblages_by_midpoint_weighted_by_their_bones():
    o = op()
    np.testing.assert_allclose(o.observed, [0.05, (40 * -0.03 + 10 * -0.05) / 50])
    assert o.names == ["sheep size (LSI) 10000-8000 BCE", "sheep size (LSI) 8000-6000 BCE"]


def test_bones_are_wild_before_herding_and_domestic_with_domestic_herds():
    o = op()
    wild = activity(lambda y: 0.0, lambda y: 0.0, lambda y: 10.0)
    np.testing.assert_allclose(o.expected(wild), [0.04, 0.04], err_msg="only hunted wild sheep: wild size")
    herded = activity(lambda y: 30.0, lambda y: 30.0 if y > -8_000 else 0.0, lambda y: 10.0)
    # Early: herds still wild, all bones wild. Late: 30 of 30 + 0.5 x 10 bones domestic.
    np.testing.assert_allclose(o.expected(herded), [0.04, 0.04 - 30 / 35 * 0.08])


def test_an_era_is_undefined_where_nobody_herds_or_hunts_near_its_sites():
    o = op()
    empty = activity(lambda y: 0.0, lambda y: 0.0, lambda y: 0.0)
    assert np.isnan(o.expected(empty)).all()


def test_the_shared_endpoints_correlate_the_eras():
    o = op()
    cov = o.covariance
    assert cov.shape == (2, 2) and np.allclose(cov, cov.T)
    q = np.clip((0.04 - o.observed) / 0.08, 0, 1)
    endpoints = np.outer(1 - q, 1 - q) * 0.02**2 + np.outer(q, q) * 0.01**2
    sampling = np.diag([0.03**2 / 10, (40 * 0.04**2 + 10 * herds.POOLED_SD**2) / 50**2])
    np.testing.assert_allclose(cov, endpoints + sampling)


def test_eras_are_judged_together_with_a_discrepancy_each():
    t = targets.SheepSize(op(), discrepancy_sd=0.015)
    assert [x.name for x in t.targets] == op().names
    assert all(x.discrepancy == pytest.approx(0.015**2) for x in t.targets)
    g = t.group(4)
    assert g.indices == [4, 5] and g.dof == 2
