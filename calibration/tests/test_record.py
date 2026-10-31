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


class _ObsidianWorld:
    """Two cells, a source group whose availability rises each year in cell 0."""

    cells = 2

    def __init__(self):
        self.year = -12_000

    def population(self):
        return np.array([10, 0], dtype=np.uint64)

    def cultivation(self):
        return np.zeros(2)

    def domestication(self):
        return np.zeros(2)

    def obsidian(self):
        return {"west": np.array([(self.year + 12_000) / 1000, 0.0]), "east": np.array([0.5, 0.0])}

    def run_years(self, n, parts):
        self.year += n


def test_obsidian_in_use_is_people_years_times_availability():
    a = record.activity(_ObsidianWorld(), end=-11_900, every=50, parts=1)
    assert list(a.obsidian) == ["west", "east"]
    np.testing.assert_allclose(a.obsidian["west"][:, 0], [50 * 10 * 0.0 + 50 * 10 * 0.05, 0])
    np.testing.assert_allclose(a.obsidian["east"][:, 0], [100 * 10 * 0.5, 0])
    assert record.activity(oikos.World(seed=4), end=-11_900).obsidian == {}, "no obsidian pack, no obsidian"
