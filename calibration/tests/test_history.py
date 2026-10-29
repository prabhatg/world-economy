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


def correlated_targets():
    """Three summaries observed with strongly correlated errors (as era shares are)."""
    cov = 0.04 * np.array([[1.0, 0.9, 0.8], [0.9, 1.0, 0.9], [0.8, 0.9, 1.0]])
    ts = [history.Target(n, 0.0, cov[i, i], 0.0) for i, n in enumerate("abc")]
    return ts, history.Group([0, 1, 2], cov)


def test_a_group_rules_out_a_wrong_shape_that_every_summary_tolerates():
    ts, g = correlated_targets()
    level = np.array([[0.3, 0.3, 0.3]])  # shifted together, as correlated errors would
    shape = np.array([[0.3, 0.0, -0.3]])  # each within 1.5 sd, but against the correlation
    zero = np.zeros((1, 3))
    assert history.implausibility(shape, zero, ts).max() < 3
    assert g.implausibility(shape, zero, ts)[0] > history.chi2_cutoff(3)
    assert g.implausibility(level, zero, ts)[0] < history.chi2_cutoff(3)


def test_group_implausibility_is_a_mahalanobis_distance_with_all_variances():
    ts, g = correlated_targets()
    ts = [history.Target(t.name, t.observed, t.variance, 0.01) for t in ts]
    f, v = np.array([[0.1, 0.2, 0.0]]), np.array([[0.02, 0.0, 0.01]])
    total = g.covariance + np.diag([0.01 + 0.02, 0.01, 0.01 + 0.01])
    d = f[0]
    assert g.implausibility(f, v, ts)[0] == pytest.approx(d @ np.linalg.solve(total, d))
    assert history.chi2_cutoff(3) == pytest.approx(12.838, abs=1e-3), "99.5% of chi-square with 3 dof"


def test_a_wave_with_a_group_keeps_the_truth_and_rules_out_more():
    rng = np.random.default_rng(4)
    u = emulator.latin_hypercube(40, 2, rng)
    sim = lambda u: np.stack([u[:, 0] + u[:, 1], u[:, 0], u[:, 1]], axis=1) - 1.0
    truth = np.array([[0.5, 0.5]])
    ts, g = correlated_targets()
    ts = [history.Target(t.name, z, t.variance, t.discrepancy) for t, z in zip(ts, sim(truth)[0])]
    plain = history.Wave.fit(u, sim(u), ts, rng=rng)
    grouped = history.Wave.fit(u, sim(u), ts, groups=[g], rng=rng)
    space = rng.random((20_000, 2))
    assert grouped.plausible(truth)[0]
    assert grouped.plausible(space).mean() < plain.plausible(space).mean()
    assert not (grouped.plausible(space) & ~plain.plausible(space)).any(), "a group only adds a check"
