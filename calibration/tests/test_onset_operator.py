"""Cultivation-onset operator: the share of crops among the charred plant remains of
dated site phases, against the effort a world's people there and then put into
cultivation (#29, ADR-0064)."""

from types import SimpleNamespace

import h3
import numpy as np
import pytest

from oikos_cal import onset, targets
from oikos_cal.radiocarbon import Window

WINDOW = Window(-12000, -5000)  # 70 centuries
RES = 5
SITES = {"a": (35.87, 38.40), "b": (36.40, 38.22), "c": (33.0, 36.0), "d": (37.0, 40.0), "e": (34.0, 37.0)}


def cell(lat, lng):
    return h3.str_to_int(h3.latlng_to_cell(lat, lng, RES))


CELLS = np.array(sorted({cell(*p) for p in SITES.values()}), dtype=np.uint64)


def table(*phases):
    """Phases (site, start, end, period, crop items, all items)."""
    rows = {k: [] for k in ("phase", "site", "lat", "lng", "h3", "period", "start_year", "end_year", "crop", "items")}
    for i, (site, start, end, period, crop, items) in enumerate(phases):
        lat, lng = SITES[site]
        for k, v in (("phase", f"{site}{i}"), ("site", site), ("lat", lat), ("lng", lng), ("h3", cell(lat, lng)),
                     ("period", period), ("start_year", start), ("end_year", end), ("crop", crop), ("items", items)):
            rows[k].append(v)
    return rows


# At sites of their own: a bootstrap draws sites whole, anchors with their site's phases.
ANCHORS = [("d", -5600, -5500, "PPNC/PN", 60, 100), ("e", -5800, -5400, "PPNC/PN", 40, 100)]  # s_PN = 0.5


def activity(people=None, cultivators=None):
    shape = (len(CELLS), WINDOW.centuries)
    return SimpleNamespace(window=WINDOW, people=np.zeros(shape) if people is None else people,
                           cultivators=np.zeros(shape) if cultivators is None else cultivators)


def at(site):
    return int(np.searchsorted(CELLS, cell(*SITES[site])))


def centuries(start, end):
    c = WINDOW.midpoints()
    return (c > start) & (c < end)


def test_pottery_neolithic_phases_anchor_the_share_of_a_farming_world():
    op = onset.Operator(table(*ANCHORS, ("a", -11100, -11000, "Pre-PPNA", 4, 100), ("b", -9500, -8700, "PPNA", 1, 10)),
                        CELLS, WINDOW, rings=0, min_items=50)
    assert op.farming_share == pytest.approx(0.5), "mean over phases, each phase once"
    assert len(op.k) == 1 and op.k[0] == 4 and op.n[0] == 100, "anchors are not targets; small phases drop"
    assert op.mid[0] == -11050


def test_an_anchor_phase_counts_however_few_crops_it_burnt():
    # ADR-0140: dung-fuel and steppe sites read near 0 among the targets too, so dropping
    # them from the anchor alone would select it on its outcome.
    dung = ("c", -6500, -6000, "PPNC/PN", 0, 10_000)
    op = onset.Operator(table(*ANCHORS, dung, ("a", -11100, -11000, "Pre-PPNA", 4, 100)), CELLS, WINDOW, rings=0)
    assert op.farming_share == pytest.approx(1 / 3)


def test_the_model_effort_is_the_people_weighted_cultivation_over_the_phase():
    op = onset.Operator(table(*ANCHORS, ("a", -11000, -10000, "Pre-PPNA", 4, 100), ("c", -9000, -8000, "PPNA", 3, 100)),
                        CELLS, WINDOW, rings=0)
    people, cultivators = np.zeros((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    people[at("a"), centuries(-11000, -10000)] = 100.0
    cultivators[at("a"), centuries(-11000, -10500)] = 80.0  # 80% effort for half the phase, none after
    e, reached = op.efforts(activity(people, cultivators))
    assert e[0] == pytest.approx(0.4) and reached[0]
    assert not reached[1] and e[1] == pytest.approx(0.4), "nobody there: the mean of the phases people reached"


def eras(rows, **kw):
    op = onset.Operator(table(*rows), CELLS, WINDOW, rings=0)
    return op, targets.OnsetEras(op, rng=np.random.default_rng(0), **kw)


ROWS = [*ANCHORS] + [("a", -11100, -10100, "Pre-PPNA", 5, 100)] * 3 + [("b", -9500, -8900, "PPNA", 30, 100)] * 3 \
    + [("c", -8500, -7700, "PPNB", 40, 100)] * 3


def test_eras_give_the_effort_the_record_implies_with_the_anchor_in_their_variance():
    op, e = eras(ROWS, edges=(-10000, -8800), discrepancy_sd=0.2)
    assert [t.name for t in e.targets] == ["cultivation effort implied before 10000 BCE",
                                          "cultivation effort implied 10000-8800 BCE",
                                          "cultivation effort implied after 8800 BCE"]
    assert [t.observed for t in e.targets] == pytest.approx([0.1, 0.6, 0.8]), "era mean share over the anchor 0.5"
    assert all(t.discrepancy == pytest.approx(0.04) for t in e.targets)
    assert all(t.variance > 1e-4 for t in e.targets), "identical phases still vary with the anchor (0.4 and 0.6)"
    g = e.group(first=7)
    assert g.indices == [7, 8, 9] and g.dof == 3
    # A world whose people put in exactly the effort the record implies reproduces it.
    people, cultivators = np.ones((len(CELLS), WINDOW.centuries)), np.zeros((len(CELLS), WINDOW.centuries))
    for site, effort in (("a", 0.1), ("b", 0.6), ("c", 0.8)):
        cultivators[at(site)] = effort
    assert e.model(activity(people, cultivators)) == pytest.approx([0.1, 0.6, 0.8])


def test_a_world_that_plants_from_the_start_is_ruled_out():
    from oikos_cal import history

    _, e = eras(ROWS, edges=(-10000, -8800), discrepancy_sd=0.2)
    people = np.ones((len(CELLS), WINDOW.centuries))
    early = np.full_like(people, 0.8)
    late = np.zeros_like(people)
    late[at("a")], late[at("b")], late[at("c")] = 0.1, 0.6, 0.8
    i_early = history.implausibility(e.model(activity(people, early))[None], 0.0, e.targets)[0]
    i_late = history.implausibility(e.model(activity(people, late))[None], 0.0, e.targets)[0]
    assert i_early[0] > 3 and i_late.max() < 1


def test_an_era_without_phases_is_an_error():
    op = onset.Operator(table(*ANCHORS, ("a", -11100, -10100, "Pre-PPNA", 5, 100)), CELLS, WINDOW, rings=0)
    with pytest.raises(ValueError, match="no phases"):
        targets.OnsetEras(op, edges=(-10000,))


def test_a_record_without_periods_is_anchored_on_its_phases_from_7000_bc():
    # Maeda et al. (2016) give dates, not periods; the Pottery Neolithic begins about
    # 7000 BC, and the database's own anchor phases run from 7300 to 5000 BC.
    rows = table(("d", -6900, -6500, "", 30, 100), ("e", -6200, -5800, "", 20, 100), ("c", -9500, -9000, "", 5, 100))
    del rows["period"]
    op = onset.Operator(rows, CELLS, WINDOW, rings=0)
    assert op.farming_share == pytest.approx(0.25)
    assert list(op.k) == [5]


def pooled_eras(crops=(5, 10), **kw):
    first = onset.Operator(table(*ROWS), CELLS, WINDOW, rings=0)  # anchor 0.5
    rows = table(("d", -6900, -6500, "", 30, 100), ("e", -6200, -5800, "", 20, 100),  # anchor 0.25
                 *[("c", -9400, -9000, "", k, 100) for k in crops])
    del rows["period"]
    second = onset.Operator(rows, CELLS, WINDOW, rings=0)
    op = onset.Operator.pooled([first, second])
    return first, second, op, targets.OnsetEras(op, edges=(-10000, -8800), rng=np.random.default_rng(0), **kw)


def test_pooled_sources_read_each_phase_against_its_own_anchor():
    first, second, op, e = pooled_eras()
    assert len(op.k) == len(first.k) + len(second.k) and list(op.source) == [0] * len(first.k) + [1, 1]
    assert op.implied()[-2:] == pytest.approx([0.2, 0.4]), "shares 0.05 and 0.1 over the anchor 0.25"
    # 10000-8800 BCE: three phases at 0.3/0.5 = 0.6 and two at 0.2 and 0.4, each phase once.
    assert e.targets[1].observed == pytest.approx((3 * 0.6 + 0.2 + 0.4) / 5)
    assert e.targets[0].observed == pytest.approx(0.1) and e.targets[2].observed == pytest.approx(0.8)
    people = np.ones((len(CELLS), WINDOW.centuries))
    cultivators = np.zeros_like(people)
    cultivators[at("b")], cultivators[at("c")] = 0.6, 0.3
    assert e.model(activity(people, cultivators))[1] == pytest.approx((3 * 0.6 + 2 * 0.3) / 5)


def test_pooling_resamples_within_each_source_so_a_second_source_narrows_an_era():
    _, _, _, pooled = pooled_eras(crops=(15, 15))  # implying 0.6, as the first source's phases
    alone = eras(ROWS, edges=(-10000, -8800))[1]
    assert pooled.targets[1].variance < alone.targets[1].variance


def test_pooling_one_source_changes_nothing():
    op = onset.Operator(table(*ROWS), CELLS, WINDOW, rings=0)
    one = targets.OnsetEras(onset.Operator.pooled([op]), edges=(-10000, -8800), rng=np.random.default_rng(0))
    alone = targets.OnsetEras(op, edges=(-10000, -8800), rng=np.random.default_rng(0))
    assert [t.observed for t in one.targets] == pytest.approx([t.observed for t in alone.targets])
    assert one.covariance == pytest.approx(alone.covariance)


def test_phases_of_one_site_are_resampled_together_with_its_anchor_phases():
    # Phases of a site share its recovery and reporting, so a bootstrap draws sites whole.
    rows = [*ANCHORS] + [("a", -11100, -10100, "Pre-PPNA", 5, 100)] * 2 + [("c", -9500, -8900, "PPNA", 30, 100)]
    op = onset.Operator(table(*rows), CELLS, WINDOW, rings=0)
    for p, _ in op.resample(np.random.default_rng(1), 200):
        assert np.sum(np.isin(p, [0, 1])) % 2 == 0, "site a's two phases come together or not at all"


def test_a_pooled_record_is_not_pooled_again():
    op = onset.Operator(table(*ROWS), CELLS, WINDOW, rings=0)
    with pytest.raises(ValueError, match="pooled"):
        onset.Operator.pooled([onset.Operator.pooled([op, op])])


def test_a_record_of_shares_reads_like_one_of_counts():
    # Sickle blades among flint tools (ADR-0143) come as shares, without counts.
    rows = table(("d", -6900, -6500, "", 0, 0), ("e", -6200, -5800, "", 0, 0), ("a", -11100, -10900, "", 0, 0),
                 ("c", -9500, -9000, "", 0, 0))
    del rows["period"], rows["crop"], rows["items"]
    rows["share"] = [0.12, 0.08, 0.01, 0.03]
    op = onset.Operator(rows, CELLS, WINDOW, rings=0, min_items=0)
    assert op.farming_share == pytest.approx(0.1)
    assert op.implied() == pytest.approx([0.1, 0.3])
    e = targets.OnsetEras(op, edges=(-10_000,), label="cultivation effort implied by sickles",
                          rng=np.random.default_rng(0))
    assert [t.name for t in e.targets] == ["cultivation effort implied by sickles before 10000 BCE",
                                          "cultivation effort implied by sickles after 10000 BCE"]
