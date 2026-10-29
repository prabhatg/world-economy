"""Gaussian-process emulators on functions whose values are known (docs/05 §5.2,
ADR-0052)."""

import numpy as np
import pytest

from oikos_cal import emulator


def smooth(x):
    return np.sin(3 * x[:, 0]) + x[:, 1] ** 2


def test_latin_hypercube_stratifies_every_factor():
    x = emulator.latin_hypercube(20, 3, np.random.default_rng(1))
    assert x.shape == (20, 3)
    for j in range(3):
        assert sorted(np.floor(x[:, j] * 20).astype(int)) == list(range(20))


def test_an_emulator_predicts_a_smooth_function_with_honest_uncertainty():
    rng = np.random.default_rng(2)
    x = emulator.latin_hypercube(60, 2, rng)
    gp = emulator.GP.fit(x, smooth(x), rng=rng)
    t = rng.uniform(size=(300, 2))
    mean, var = gp.predict(t)
    err = mean - smooth(t)
    assert np.sqrt((err**2).mean()) < 0.02
    assert (np.abs(err) / np.sqrt(var) < 3).mean() > 0.95


def test_an_inactive_factor_gets_a_long_length_scale():
    rng = np.random.default_rng(3)
    x = emulator.latin_hypercube(60, 3, rng)
    gp = emulator.GP.fit(x, smooth(x), rng=rng)
    assert gp.length_scales[2] > 5 * max(gp.length_scales[:2])


def test_the_nugget_estimates_run_to_run_noise():
    rng = np.random.default_rng(4)
    x = emulator.latin_hypercube(120, 2, rng)
    gp = emulator.GP.fit(x, smooth(x) + rng.normal(0, 0.1, len(x)), rng=rng)
    assert 0.06 < np.sqrt(gp.noise) < 0.15
    _, var = gp.predict(x[:5])
    _, var_noisy = gp.predict(x[:5], noise=True)
    assert (var_noisy == pytest.approx(var + gp.noise))


def test_uncertainty_grows_away_from_the_runs():
    rng = np.random.default_rng(5)
    x = emulator.latin_hypercube(30, 2, rng) * [0.5, 1]
    gp = emulator.GP.fit(x, smooth(x), rng=rng)
    _, var = gp.predict(np.array([[0.25, 0.5], [0.95, 0.5]]))
    assert var[1] > 100 * var[0]


def test_fitting_refuses_unusable_runs():
    x = np.zeros((4, 2))
    with pytest.raises(ValueError, match="finite"):
        emulator.GP.fit(x, np.array([0, 1, np.nan, 2.0]))
    with pytest.raises(ValueError, match="runs"):
        emulator.GP.fit(x, np.zeros(3))
