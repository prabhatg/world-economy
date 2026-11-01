"""The juvenile-index operator on an M1 world (runs when the M1 packs and the bindings
are built): where farming arrives, the index rises if cultivators bear more children,
and not otherwise (#33, ADR-0068)."""

import h3
import numpy as np
import pytest

from oikos_cal import juvenile
from oikos_data import region

ABU_HUREYRA = (35.87, 38.40)


def test_the_index_rises_where_farming_arrives_only_with_a_fertility_transition():
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import m1, record

    try:
        packs = m1.packs()
    except FileNotFoundError as e:
        pytest.skip(str(e))
    w = oikos.World.from_packs(packs, seed=1)
    a = record.activity(w, end=-6_000)
    site = h3.str_to_int(h3.latlng_to_cell(*ABU_HUREYRA, 5))
    starts = list(range(a.window.first, a.window.end, 500))
    table = {"cemetery": [str(s) for s in starts], "h3": [site] * len(starts), "start_year": starts,
             "end_year": [s + 500 for s in starts]}
    op = juvenile.Operator(table, region.load("m1-swasia").cells(), a.window, rings=1)
    effort = op.birth_rates(a, {**w.params, "fertility": 0.0, "sedentary_fertility": 1.0})  # the effort itself
    foraging, farming = effort < 0.05, effort > 0.25
    assert foraging.any() and farming.any(), f"farming arrives at Abu Hureyra within the run: {effort.round(2)}"
    transition, reached = op.indices(a, {**w.params, "sedentary_fertility": 0.06})
    none, _ = op.indices(a, {**w.params, "sedentary_fertility": w.params["fertility"]})
    print(f"Abu Hureyra by 500 years: effort {effort.round(2)}, 15p5 {transition.round(3)}, without {none.round(3)}")
    assert reached.all()
    assert transition[farming].min() > transition[foraging].max() + 0.02
    assert np.corrcoef(effort, transition - none)[0, 1] > 0.99, "the signal is the effort's"
    assert np.ptp(none) < 0.02, "no transition, no signal beyond local growth"
