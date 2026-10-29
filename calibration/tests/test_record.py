"""Recording a world's activity for observation operators."""

import numpy as np
import pytest

oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")

from oikos_cal import record  # noqa: E402


def test_activity_is_people_years_per_cell_and_century():
    w, twin = oikos.World(seed=4), oikos.World(seed=4)
    a = record.activity(w, end=-11_700, every=30, parts=2)
    assert a.window == record.Window(-12_000, -11_700) and w.year == -11_700
    people = np.zeros((w.cells, 3))
    while twin.year < -11_700:
        c = (twin.year + 12_000) // 100
        n = min(30, 100 - (twin.year + 12_000) % 100)  # samples never straddle a century
        people[:, c] += n * twin.population()
        twin.run_years(n, parts=2)
    np.testing.assert_array_equal(a.people, people)
    assert twin.state_hash() == w.state_hash(), "recording never changes the world"
    assert a.people[:, 2].sum() > 0 and not a.cultivators.any() and not a.domestic.any(), "toy worlds never cultivate"
    np.testing.assert_array_equal(a.visible(3.0), a.people)


def test_cultivators_can_be_more_visible():
    a = record.Activity(record.Window(-12_000, -11_900), np.array([[100.0]]), np.array([[40.0]]), np.array([[10.0]]))
    assert a.visible(0.0)[0, 0] == 100 and a.visible(2.0)[0, 0] == 180


def test_recording_starts_and_ends_on_century_boundaries():
    w = oikos.World(seed=4)
    with pytest.raises(ValueError, match="century"):
        record.activity(w, end=-11_750)
    w.run_years(30)
    with pytest.raises(ValueError, match="century"):
        record.activity(w, end=-11_800)
