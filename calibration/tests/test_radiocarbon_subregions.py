"""Radiocarbon by subregion: when each subregion's dated activity accrued, on synthetic
records whose histories are known (docs/05 §5.2, #35)."""

import h3
import numpy as np
import pytest

from oikos_cal import history, targets
from oikos_cal import radiocarbon as rc

from test_radiocarbon_operator import OBSERVED, SETTINGS, TRUE, linear_curve, table
from test_targets import eight_blocks

# the boxes split eight_blocks() into west and east of 40 E
WEST = (29.0, 39.0, 30.0, 40.0)  # lat from, to, lng from, to: the Levant and Anatolia
EAST = (29.0, 39.0, 40.0, 55.0)  # Upper Mesopotamia and the Zagros


def east(cells):
    """Cells whose block's center is east of 40 E, as the boxes split them."""
    return np.array([h3.cell_to_latlng(h3.cell_to_parent(h3.int_to_str(int(c)), 2))[1] >= 40.0 for c in cells])


def onset(cells, west_start, east_start):
    """Activity that switches on at a year in each half of the region and stays."""
    t = TRUE.midpoints()
    start = np.where(east(cells), east_start, west_start)
    return (t[None, :] >= start[:, None]).astype(float) + 0.02


@pytest.fixture(scope="module")
def record():
    cells = eight_blocks()
    truth = onset(cells, -9_000, -7_800)
    dates = rc.synthetic_dates(cells, truth, TRUE, linear_curve(), 1500, [40.0, 80.0], rc.NO_TAPHONOMY,
                               np.random.default_rng(3))
    op = rc.Operator(dates, linear_curve(), cells, SETTINGS)
    members = rc.subregions(op.obs.blocks, {"west": WEST, "east": EAST})
    subs = targets.RadiocarbonSubregions(op, members, discrepancy_sd=0.0, taphonomy=rc.NO_TAPHONOMY,
                                         rng=np.random.default_rng(4))
    return cells, truth, op, subs


def test_each_site_keeps_its_own_date_mass():
    a = int(eight_blocks()[0])
    bp = lambda year: 1950 - year
    rows = [(a, "Tell X", bp(y), 30) for y in (-9_000, -8_950, -8_000)] + [(a, "Tell Y", bp(-7_000), 30)]
    obs = rc.observe(table(rows), linear_curve(), SETTINGS)
    assert obs.site_mass.shape == (2, 2, OBSERVED.centuries), "Tell X has two phases, but a site is resampled whole"
    assert obs.site_mass.sum(axis=(1, 2)) == pytest.approx([2.0, 1.0], abs=1e-6), "sites in (cell, name) order"
    assert np.allclose(obs.site_mass.sum(axis=0), obs.mass[0])
    assert (obs.site_block == 0).all()


def test_sites_sum_to_their_blocks(record):
    *_, op, _ = record
    per_block = np.zeros_like(op.obs.mass)
    np.add.at(per_block, op.obs.site_block, op.obs.site_mass)
    assert np.allclose(per_block, op.obs.mass)


def test_subregions_are_boxes_of_block_centers(record):
    *_, op, subs = record
    m = rc.subregions(op.obs.blocks, {"west": WEST, "east": EAST})
    assert len(m["west"]) + len(m["east"]) == 8 and min(len(v) for v in m.values()) >= 3
    assert set(m["west"]).isdisjoint(m["east"])
    assert all(h3.cell_to_latlng(h3.int_to_str(int(b)))[1] >= 40.0 for b in op.obs.blocks[m["east"]])
    first = rc.subregions(op.obs.blocks, {"all": (0.0, 90.0, 0.0, 90.0), "east": EAST})
    assert len(first["all"]) == 8 and len(first["east"]) == 0, "a block goes to the first box holding it"


def test_quantiles_are_the_years_by_which_shares_of_the_mass_had_accrued():
    mass = np.array([[0.0, 1.0, 1.0, 2.0], [1.0, 0.0, 0.0, 0.0]])
    y = targets.accrued(mass, -9_000, (0.25, 0.5, 1.0))
    assert y[0] == pytest.approx([-8_800, -8_700, -8_600]), "a quarter by the end of the second century"
    assert y[1] == pytest.approx([-8_975, -8_950, -8_900])


def test_the_late_subregion_accrues_its_dates_later(record):
    *_, subs = record
    obs = {t.name: t.observed for t in subs.targets}
    assert list(obs)[:3] == ["radiocarbon west 10% by", "radiocarbon west 25% by", "radiocarbon west 50% by"]
    for q in (10, 25, 50):
        assert obs[f"radiocarbon east {q}% by"] - obs[f"radiocarbon west {q}% by"] > 600, \
            "the east started 1,200 years later"
    assert obs["radiocarbon west 10% by"] > -9_100 and obs["radiocarbon east 10% by"] > -7_900
    assert all(0 < t.variance < 150**2 for t in subs.targets)


def test_the_true_history_is_plausible_and_an_even_start_is_not(record):
    cells, truth, _, subs = record
    i = lambda a: history.implausibility(subs.model(a)[None], 0.0, subs.targets)[0].max()
    assert i(truth) < 3
    assert i(onset(cells, -9_000, -9_000)) > 5, "the region-wide shape could hide this; the subregions do not"
    busy = truth.copy()
    busy[east(cells)] *= 7.0
    assert i(busy) == pytest.approx(i(truth)), "research intensity per block cancels"


def test_subregions_are_resampled_apart_and_one_group(record):
    *_, subs = record
    c = subs.covariance
    assert c.shape == (6, 6)
    assert c[0, 1] > 0, "a subregion's quantiles move together"
    assert abs(c[0, 3]) < 0.2 * np.sqrt(c[0, 0] * c[3, 3]), "sites of two subregions are drawn apart"
    g = subs.group(10)
    assert g.indices == list(range(10, 16)) and g.dof == 6


def test_a_subregion_with_too_few_sites_is_refused(record):
    *_, op, _ = record
    with pytest.raises(ValueError, match="west has 0 sites"):
        targets.RadiocarbonSubregions(op, {"west": np.array([], dtype=int)})
    with pytest.raises(ValueError, match="in two subregions"):
        m = rc.subregions(op.obs.blocks, {"west": WEST})
        targets.RadiocarbonSubregions(op, {"a": m["west"], "b": m["west"]})


def test_only_sites_with_dates_in_the_window_count():
    a = int(eight_blocks()[0])
    bp = lambda year: 1950 - year
    rows = [(a, f"late {i}", bp(-5_000), 30) for i in range(20)] + [(a, "early", bp(-8_000), 30)]
    op = rc.Operator(table(rows), linear_curve(), [a], SETTINGS)
    with pytest.raises(ValueError, match="all has 1 sites with usable dates"):
        targets.RadiocarbonSubregions(op, {"all": np.array([0])})


def test_one_share_of_one_subregion_is_a_target(record):
    *_, op, _ = record
    m = rc.subregions(op.obs.blocks, {"west": WEST})
    one = targets.RadiocarbonSubregions(op, m, shares=(0.5,), rng=np.random.default_rng(4))
    assert [t.name for t in one.targets] == ["radiocarbon west 50% by"]
    assert one.covariance.shape == (1, 1) and one.targets[0].variance > 0
