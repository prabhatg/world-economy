"""Obsidian sourcing operator: the share of each sourced assemblage that came from the
Cappadocian volcanoes rather than the Eastern Anatolian ones, against the obsidian a
world's people held there and then (docs/05 §5.1, #17)."""

from types import SimpleNamespace

import h3
import numpy as np
import pytest

from oikos_cal import obsidian, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-9000, -6000)  # 30 centuries
RES = 5
WEST, EAST = (38.26, 34.55), (38.65, 42.23)  # Göllü Dağ, Nemrut Dağ


def cell(lat, lng):
    return h3.str_to_int(h3.latlng_to_cell(lat, lng, RES))


SITES = {"near west": (38.3, 35.0), "middle": (37.5, 38.5), "near east": (38.4, 41.8)}
CELLS = np.array(sorted({cell(*p) for p in [*SITES.values(), WEST, EAST]}), dtype=np.uint64)
REACH = {"cappadocia": [cell(*WEST)], "eastern_anatolia": [cell(*EAST)]}


def table(*contexts):
    """Contexts (site, start, end, cappadocian, eastern artefacts)."""
    rows = {k: [] for k in ("context", "site", "lat", "lng", "h3", "start_year", "end_year", "cappadocia",
                            "eastern_anatolia")}
    for i, (site, start, end, c, e) in enumerate(contexts):
        lat, lng = SITES[site]
        for k, v in (("context", str(i)), ("site", site), ("lat", lat), ("lng", lng), ("h3", cell(lat, lng)),
                     ("start_year", start), ("end_year", end), ("cappadocia", c), ("eastern_anatolia", e)):
            rows[k].append(v)
    return rows


def activity(west=None, east=None):
    """Obsidian in use [cell, century] per group; by default nothing anywhere."""
    shape = (len(CELLS), WINDOW.centuries)
    return SimpleNamespace(window=WINDOW, obsidian={"cappadocia": np.zeros(shape) if west is None else west,
                                                    "eastern_anatolia": np.zeros(shape) if east is None else east})


def at(site):
    return int(np.searchsorted(CELLS, cell(*SITES[site])))


def centuries(start, end):
    c = WINDOW.midpoints()
    return (c > start) & (c < end)


def test_relative_distance_says_which_source_is_nearer():
    op = obsidian.Operator(table(("near west", -8000, -7000, 9, 1), ("near east", -8000, -7000, 0, 5)), CELLS, WINDOW,
                           REACH)
    assert op.delta[0] > 300 and op.delta[1] < -300, "km nearer the Cappadocian source than the eastern one"
    assert np.allclose(op.k, [9, 0]) and np.allclose(op.n, [10, 5])


def test_model_shares_weigh_the_obsidian_people_held_over_the_period():
    op = obsidian.Operator(table(("middle", -8000, -7000, 3, 3), ("middle", -8500, -6500, 3, 3),
                                 ("near east", -8000, -7000, 0, 5)), CELLS, WINDOW, REACH, rings=0)
    w, e = np.zeros((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    w[at("middle"), centuries(-8000, -7000)] = 3.0
    e[at("middle"), centuries(-8000, -7000)] = 1.0
    e[at("middle"), centuries(-7000, -6500)] = 4.0
    p, reached = op.shares(activity(w, e))
    assert p[0] == pytest.approx(0.75)
    assert p[1] == pytest.approx(30 / (30 + 10 + 20)), "the whole period counts, by century"
    assert p[2] == 0.5 and not reached[2] and reached[:2].all(), "nothing reached the site: no information"


def test_periods_are_clipped_to_the_window_and_contexts_outside_it_dropped():
    op = obsidian.Operator(table(("middle", -9500, -8500, 1, 0), ("middle", -12000, -9500, 1, 0)), CELLS, WINDOW, REACH)
    assert len(op.k) == 1 and op.dropped == 1


def test_the_record_scores_a_world_that_matches_it_above_one_that_does_not():
    t = table(("near west", -8000, -7000, 40, 2), ("near east", -8000, -7000, 1, 30))
    op = obsidian.Operator(t, CELLS, WINDOW, REACH)
    w, e = np.zeros((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    w[at("near west")], e[at("near west")] = 9.0, 1.0
    w[at("near east")], e[at("near east")] = 1.0, 9.0
    gap = op.log_likelihood(activity(w, e)) - op.log_likelihood(activity(e, w))
    assert gap > 5, "overdispersed (φ = 2): no single assemblage decides"
    binomial = obsidian.Operator(t, CELLS, WINDOW, REACH, concentration=1e6)
    assert binomial.log_likelihood(activity(w, e)) - binomial.log_likelihood(activity(e, w)) > 10 * gap


def test_zones_give_mean_shares_with_bootstrap_variances_and_one_group():
    rows = [("near west", -8000, -7000, 9, 1)] * 4 + [("middle", -8000, -7000, 1, 1)] * 4 + \
           [("near east", -8000, -7000, 0, 6)] * 4 + [("near west", -7000, -6000, 5, 5)] * 4 + \
           [("middle", -7000, -6000, 2, 1)] * 4 + [("near east", -7000, -6000, 1, 5)] * 4
    op = obsidian.Operator(table(*rows), CELLS, WINDOW, REACH)
    z = targets.ObsidianZones(op, edges=(-300.0, 300.0), era=-7000, discrepancy_sd=0.1, rng=np.random.default_rng(0))
    names = [t.name for t in z.targets]
    assert names[0] == "obsidian before 7000 BCE, eastern source nearer by over 300 km"
    assert names[2] == "obsidian before 7000 BCE, Cappadocian source nearer by over 300 km" and len(names) == 6
    assert [t.observed for t in z.targets] == pytest.approx([0, 0.5, 0.9, 1 / 6, 2 / 3, 0.5])
    assert all(t.variance >= 0 and t.discrepancy == pytest.approx(0.01) for t in z.targets)
    g = z.group(first=10)
    assert g.indices == list(range(10, 16)) and g.dof == 6
    # A world holding exactly each context's observed share reproduces the summaries.
    w, e = np.zeros((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    for site, share in (("near west", 0.9), ("middle", 0.5), ("near east", 0.0)):
        w[at(site), centuries(-8000, -7000)], e[at(site), centuries(-8000, -7000)] = share, 1 - share
    for site, share in (("near west", 0.5), ("middle", 2 / 3), ("near east", 1 / 6)):
        w[at(site), centuries(-7000, -6000)], e[at(site), centuries(-7000, -6000)] = share, 1 - share
    op0 = obsidian.Operator(table(*rows), CELLS, WINDOW, REACH, rings=0)
    z0 = targets.ObsidianZones(op0, edges=(-300.0, 300.0), era=-7000, rng=np.random.default_rng(0))
    assert z0.model(activity(w, e)) == pytest.approx([t.observed for t in z0.targets])


def test_a_zone_without_contexts_is_an_error():
    op = obsidian.Operator(table(("near west", -8000, -7000, 9, 1)), CELLS, WINDOW, REACH)
    with pytest.raises(ValueError, match="no contexts"):
        targets.ObsidianZones(op, edges=(-300.0, 300.0), era=-7000)


def test_joined_cells_are_those_a_chain_of_neighbours_links_to_a_source():
    source = cell(*WEST)
    land = {h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(source), 2)}
    island = cell(35.0, 33.0)  # Cyprus: no chain of land cells to the mainland
    cells = np.array(sorted(land | {island}), dtype=np.uint64)
    joined = obsidian.joined(cells, {"cappadocia": [source], "eastern_anatolia": []})
    assert set(joined) == land and island not in joined


def test_contexts_no_chain_can_reach_are_dropped_not_scored():
    joined = [c for c in CELLS if c != cell(*SITES["middle"])]
    op = obsidian.Operator(table(("near west", -8000, -7000, 9, 1), ("middle", -8000, -7000, 3, 3)), CELLS, WINDOW,
                           REACH, joined=joined)
    assert len(op.k) == 1 and op.isolated == 1 and op.dropped == 0
