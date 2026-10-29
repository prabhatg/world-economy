"""Morris elementary-effects screening on functions whose influences are known
(docs/05 §5.2, ADR-0052)."""

import numpy as np
import pytest

from oikos_cal import screening


def registry():
    p = lambda dist, **kw: {"prior": {"dist": dist, **kw}}
    return {"params": {"b": p("uniform", lo=0.0, hi=1.0), "a": p("log_uniform", lo=0.1, hi=1.0),
                       "c": p("fixed"), "d": p("uniform", lo=2.0, hi=3.0)}}


def test_free_parameters_are_those_with_a_prior_to_sample():
    assert screening.free_parameters(registry()) == ["a", "b", "d"]


def test_trajectories_move_one_factor_at_a_time_on_the_grid():
    d = screening.morris(k=5, trajectories=7, levels=4, rng=np.random.default_rng(1))
    assert d.shape == (7, 6, 5)
    assert np.isin(np.round(d * 3, 9), [0, 1, 2, 3]).all(), "points lie on the 4-level grid"
    steps = np.diff(d, axis=1)
    assert ((steps != 0).sum(axis=2) == 1).all(), "each step changes one factor"
    assert ((steps != 0).sum(axis=1) == 1).all(), "each factor changes once per trajectory"
    assert np.allclose(np.abs(steps).sum(axis=2), 2 / 3), "by delta = p / (2 (p - 1))"
    assert (d >= 0).all() and (d <= 1).all()


def test_elementary_effects_rank_factors_by_influence():
    f = lambda x: 3 * x[..., 0] - x[..., 1] + 5 * x[..., 3] * x[..., 4]
    d = screening.morris(k=5, trajectories=40, levels=4, rng=np.random.default_rng(2))
    e = screening.elementary_effects(d, f(d))
    assert e.mu_star[:3] == pytest.approx([3, 1, 0])
    assert e.mu[1] == pytest.approx(-1), "mu keeps the sign"
    assert e.sigma[:3] == pytest.approx([0, 0, 0], abs=1e-12), "linear effects do not vary"
    assert (e.sigma[3:] > 1).all(), "interacting factors' effects depend on the other factor"
    assert (e.mu_star[3:] > 1).all()


def test_elementary_effects_of_several_outputs():
    d = screening.morris(k=3, trajectories=5, levels=4, rng=np.random.default_rng(3))
    y = np.stack([d[..., 0], 2 * d[..., 2]], axis=-1)
    e = screening.elementary_effects(d, y)
    assert e.mu_star.shape == (3, 2)
    assert e.mu_star[:, 0] == pytest.approx([1, 0, 0]) and e.mu_star[:, 1] == pytest.approx([0, 0, 2])
