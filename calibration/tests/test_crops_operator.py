"""Crops operator (#51): whether an assemblage holds cultivated crops, which a world gives
from the domestic share of the cereals its people discarded around the site (sown in
proportion to cultivation effort and its domestic share, gathered wild in proportion to
foraging) and the cereal remains a sample holds."""

import numpy as np

from oikos_cal import crops

from test_stock_operator import CELLS, LAT, LNG, WINDOW, activity

ERAS = ((-7_000, -5_000), (-5_000, -4_000))


def assemblages(*rows):
    """(name, start, end, crops) at the center; crops None: unknown."""
    out = [{"name": n, "start_year": a, "end_year": b, "mammals": 10, "domestic": 1, "lat": LAT, "lng": LNG}
           for n, a, b, _ in rows]
    for a, (*_, c) in zip(out, rows):
        if c is not None:
            a["crops"] = c
    return out


def op(gathered=0.5, items=10):
    t = assemblages(("A", -6_000, -5_000, False), ("B", -5_600, -5_200, None), ("C", -4_800, -4_400, True),
                    ("D", -4_600, -4_200, False))
    return crops.Operator(t, CELLS, WINDOW, ERAS, gathered=gathered, items=items)


def sown(effort, domestic):
    return activity(lambda y: 100.0, lambda y: effort, lambda y: 0.0, lambda y: 0.0), domestic


def test_eras_hold_the_assemblages_whose_crops_are_known():
    o = op()
    assert o.names == ["Egypt cultivated crops 7000-5000 BCE share of assemblages",
                       "Egypt cultivated crops 5000-4000 BCE share of assemblages"]
    np.testing.assert_allclose(o.observed, [0.0, 0.5])
    assert list(o.counts) == [1, 2], "B's crops are unknown"
    beta = lambda a, b: a * b / ((a + b) ** 2 * (a + b + 1))
    np.testing.assert_allclose(o.sampling_variance, [beta(0.5, 1.5), beta(1.5, 1.5)])


def test_an_assemblage_shows_crops_by_the_domestic_share_of_its_cereals():
    o = op(gathered=0.5, items=10)
    a = activity(lambda y: 100.0, lambda y: 40.0, lambda y: 0.0, lambda y: 0.0)
    a.domestic = a.cultivators * 0.75
    # 40 sown, three quarters domestic; 60 foraging gather 0.5 x 60 wild.
    q = 0.75 * 40 / (40 + 0.5 * 60)
    np.testing.assert_allclose(o.shown(a), [1 - (1 - q) ** 10] * 3)
    np.testing.assert_allclose(o.expected(a), [1 - (1 - q) ** 10] * 2)


def test_foragers_and_wild_crops_show_none():
    o = op()
    foragers = activity(lambda y: 100.0, lambda y: 0.0, lambda y: 0.0, lambda y: 0.0)
    foragers.domestic = foragers.cultivators * 0.0
    np.testing.assert_allclose(o.expected(foragers), [0.0, 0.0])
    wild = activity(lambda y: 100.0, lambda y: 50.0, lambda y: 0.0, lambda y: 0.0)
    wild.domestic = wild.cultivators * 0.0
    np.testing.assert_allclose(o.expected(wild), [0.0, 0.0], err_msg="cultivating wild cereal is no crop")


def test_crops_sown_after_an_assemblage_do_not_show_in_it():
    o = op()
    late = activity(lambda y: 100.0, lambda y: 50.0 if y > -5_000 else 0.0, lambda y: 0.0, lambda y: 0.0)
    late.domestic = late.cultivators * 1.0
    shown = o.shown(late)
    assert shown[0] == 0.0 and (shown[1:] > 0.99).all()


def test_the_eras_are_judged_together():
    from oikos_cal import targets

    o = op()
    t = targets.AssemblageShares(o, discrepancy_sd=0.1)
    assert [x.name for x in t.targets] == o.names and [x.observed for x in t.targets] == [0.0, 0.5]
    assert t.group(3).indices == [3, 4]
