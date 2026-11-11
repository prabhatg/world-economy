"""Domestic stock operator (#51): whether an assemblage holds bones of domestic animals,
which a world gives from the domestic share of the mammal bones its people discarded
around the site (herds in proportion to herding effort, game in proportion to foraging)
and the number of bones identified."""

import h3
import numpy as np

from oikos_cal import record, stock
from oikos_cal.radiocarbon import Window

WINDOW = Window(-7_000, -4_000)
CENTER = h3.latlng_to_cell(29.6, 30.7, 5)
CELLS = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(CENTER, 3)), dtype=np.uint64)
LAT, LNG = h3.cell_to_latlng(CENTER)


def assemblages(*rows):
    """(name, start, end, mammals, domestic) at the center."""
    return [{"name": n, "start_year": a, "end_year": b, "mammals": m, "domestic": d, "lat": LAT, "lng": LNG}
            for n, a, b, m, d in rows]


def activity(people, cultivating, herding, domestic):
    """Per cell and century, functions of the century's midpoint: people-years, and the
    people-years times cultivation effort, herding effort and the herd's domestic share."""
    n, mid = len(CELLS), WINDOW.midpoints()
    col = lambda f: np.tile(np.array([f(y) for y in mid], dtype=float), (n, 1))
    a = record.Activity(WINDOW, col(people), col(cultivating), np.zeros((n, WINDOW.centuries)))
    a.herders, a.herd_domestic = col(herding), col(domestic)
    return a


def op(game=0.1):
    t = assemblages(("A", -6_000, -5_000, 100, 20), ("B", -5_600, -5_200, 10, 1), ("C", -4_800, -4_400, 50, 45))
    return stock.Operator(t, CELLS, WINDOW, eras=((-7_000, -5_000), (-5_000, -4_000)), game=game)


def test_eras_hold_their_assemblages_by_midpoint():
    o = op()
    assert o.names == ["Egypt domestic animals 7000-5000 BCE share of assemblages",
                       "Egypt domestic animals 5000-4000 BCE share of assemblages"]
    np.testing.assert_allclose(o.observed, [1.0, 1.0])


def test_an_assemblage_shows_domestic_animals_by_how_many_bones_it_holds():
    o = op(game=0.5)
    # Half the people herd, all of their herds domestic, the rest forage: of 50 + 0.5 x 50
    # bones, 50 domestic.
    a = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 50.0, lambda y: 50.0)
    q = 50 / (50 + 0.5 * 50)
    np.testing.assert_allclose(o.shown(a), [1 - (1 - q) ** 100, 1 - (1 - q) ** 10, 1 - (1 - q) ** 50])
    np.testing.assert_allclose(o.expected(a), [np.mean([1 - (1 - q) ** 100, 1 - (1 - q) ** 10]), 1 - (1 - q) ** 50])


def test_foragers_and_wild_herds_show_none():
    o = op()
    foragers = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 0.0, lambda y: 0.0)
    wild = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 30.0, lambda y: 0.0)
    np.testing.assert_allclose(o.expected(foragers), [0.0, 0.0])
    np.testing.assert_allclose(o.expected(wild), [0.0, 0.0])


def test_farmers_forage_less_so_their_herds_show_more():
    o = op(game=0.5)
    herds = lambda y: 5.0
    forage = o.shown(activity(lambda y: 100.0, lambda y: 0.0, herds, herds))
    farm = o.shown(activity(lambda y: 100.0, lambda y: 60.0, herds, herds))
    assert (farm > forage).all()


def test_nobody_near_a_site_shows_no_domestic_animals():
    o = op()
    empty = activity(lambda y: 0.0, lambda y: 0.0, lambda y: 0.0, lambda y: 0.0)
    np.testing.assert_allclose(o.shown(empty), [0.0, 0.0, 0.0])


def test_herds_arriving_after_an_assemblage_do_not_show_in_it():
    o = op()
    late = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 50.0 if y > -5_000 else 0.0,
                    lambda y: 50.0 if y > -5_000 else 0.0)
    shown = o.shown(late)
    assert shown[0] == shown[1] == 0.0 and shown[2] > 0.99


def test_it_reads_the_cells_near_its_sites():
    o = op()
    assert len(o.cells()) == 3 and all(len(c) == 7 for c in o.cells())


def test_an_eras_share_is_uncertain_by_how_few_assemblages_it_holds():
    o = op()
    # Jeffreys' posterior of the share of such assemblages that held domestic animals,
    # Beta(k + 1/2, n - k + 1/2): every assemblage held some, yet a few cannot prove all did.
    beta = lambda a, b: a * b / ((a + b) ** 2 * (a + b + 1))
    np.testing.assert_allclose(o.sampling_variance, [beta(2.5, 0.5), beta(1.5, 0.5)])
    assert o.sampling_variance[1] > o.sampling_variance[0], "one assemblage tells less than two"


def test_the_eras_are_judged_together():
    from oikos_cal import history, targets

    o = op()
    t = targets.AssemblageShares(o, discrepancy_sd=0.1)
    assert [x.name for x in t.targets] == o.names
    assert [x.observed for x in t.targets] == [1.0, 1.0]
    np.testing.assert_allclose([x.variance for x in t.targets], o.sampling_variance)
    assert all(x.discrepancy == 0.1**2 for x in t.targets)
    g = t.group(7)
    assert g.indices == [7, 8] and g.dof == 2
    np.testing.assert_allclose(g.covariance, np.diag(o.sampling_variance))
    herds = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 50.0, lambda y: 50.0)
    np.testing.assert_allclose(t.model(herds), o.expected(herds))
    none = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 0.0, lambda y: 0.0)
    assert history.implausibility(t.model(none)[None], 0.0, t.targets).max() > history.CUTOFF, \
        "a world without herds in Egypt is ruled out"
