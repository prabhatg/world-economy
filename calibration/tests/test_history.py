"""History matching on a toy simulator whose true parameters are known (docs/05 §5.2,
ADR-0052): waves rule out most of the space and never the truth."""

import numpy as np
import pytest

from oikos_cal import emulator, history

TRUTH = np.array([0.3, 0.6, 0.5])


def simulator(u):
    """Two outputs of the first two factors; the third does nothing."""
    return np.stack([np.sin(5 * u[:, 0]) + 2 * u[:, 1], u[:, 0] - u[:, 1] ** 2], axis=1)


def targets():
    z = simulator(TRUTH[None])[0]
    return [history.Target("a", z[0], 0.02**2, 0.02**2), history.Target("b", z[1], 0.02**2, 0.02**2)]


def test_implausibility_is_the_standardized_distance_to_the_record():
    t = [history.Target("x", observed=1.0, variance=0.5, discrepancy=0.25)]
    i = history.implausibility(np.array([[2.5]]), np.array([[0.25]]), t)
    assert i[0, 0] == pytest.approx(1.5)


@pytest.mark.parametrize("seed", range(5))
def test_waves_rule_out_most_of_the_space_and_keep_the_truth(seed):
    rng = np.random.default_rng(seed)
    space = rng.uniform(size=(20_000, 3))
    exact = history.implausibility(simulator(space), 0.0, targets()).max(axis=1) <= history.CUTOFF

    u = emulator.latin_hypercube(30, 3, rng)  # ten runs per factor (Loeppky, Sacks and Welch 2009)
    first = history.Wave.fit(u, simulator(u), targets(), rng=rng)
    kept = first.plausible(space)
    assert first.plausible(TRUTH[None])[0]
    assert kept.mean() < 0.05, "one wave already rules out most of the space"

    u2, accepted = history.nroy([first], k=3, n=30, rng=rng)
    assert u2.shape == (30, 3) and accepted == pytest.approx(kept.mean(), rel=0.2)
    assert first.plausible(u2).all()
    second = history.Wave.fit(u2, simulator(u2), targets(), rng=rng)
    both = kept & second.plausible(space)
    assert second.plausible(TRUTH[None])[0]
    assert both.mean() < 1.05 * exact.mean(), "the waves keep little beyond what the simulator allows"
    assert (exact & ~both).mean() < 0.02 * exact.mean(), "and rule out almost nothing it allows"


def test_a_wave_emulates_only_the_active_factors():
    rng = np.random.default_rng(2)
    u = emulator.latin_hypercube(40, 3, rng)
    w = history.Wave.fit(u, simulator(u), targets(), active=[0, 1], rng=rng)
    assert w.active == [0, 1] and all(len(g.length_scales) == 2 for g in w.emulators)
    assert w.plausible(TRUTH[None])[0]


def test_an_empty_space_is_reported_not_sampled_forever():
    rng = np.random.default_rng(3)
    u = emulator.latin_hypercube(30, 3, rng)
    far = [history.Target("a", 10.0, 1e-4, 1e-4), history.Target("b", 10.0, 1e-4, 1e-4)]
    w = history.Wave.fit(u, simulator(u), far, rng=rng)
    points, accepted = history.nroy([w], k=3, n=10, rng=rng, draws=50_000)
    assert accepted == 0 and len(points) == 0
