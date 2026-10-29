"""Screening: which parameters change what the record sees (docs/05 §5.2, ADR-0052).

Morris's elementary effects (Morris 1991, doi:10.1080/00401706.1991.10484804) with the
mean absolute effect μ* of Campolongo, Cariboni and Saltelli (2007,
doi:10.1016/j.envsoft.2006.10.004). Each trajectory starts at a random point of a
`levels`-level grid in the unit cube of the free parameters and moves one factor at a
time, in random order and direction, by Δ = p / (2 (p - 1)). Effects are per unit of the
prior's unit interval (`oikos.sample_params` maps it onto each prior), so factors are
comparable. μ* ranks influence; σ large next to μ* means the effect depends on where the
other factors are (nonlinearity or interaction). r (k + 1) runs for k factors.
"""

from dataclasses import dataclass

import numpy as np

__all__ = ["free_parameters", "morris", "Effects", "elementary_effects"]


def free_parameters(registry):
    """Keys of the registry's parameters with a prior to sample (not "fixed"), sorted."""
    return sorted(k for k, p in registry["params"].items() if p["prior"]["dist"] != "fixed")


def morris(k, trajectories, levels=4, rng=None):
    """Points [trajectory, step, factor] in the unit cube: k + 1 steps per trajectory."""
    if levels % 2:
        raise ValueError(f"levels must be even (got {levels}) so that every step stays on the grid")
    rng = rng or np.random.default_rng()
    delta = levels / (2 * (levels - 1))
    out = np.empty((trajectories, k + 1, k))
    for r in range(trajectories):
        base = rng.integers(0, levels // 2, size=k) / (levels - 1)
        up = rng.random(k) < 0.5
        x = np.where(up, base, base + delta)
        out[r, 0] = x
        for s, j in enumerate(rng.permutation(k)):
            x = x.copy()
            x[j] += delta if up[j] else -delta
            out[r, s + 1] = x
    return out


@dataclass
class Effects:
    mu: np.ndarray  # mean effect [factor] or [factor, output]
    mu_star: np.ndarray  # mean absolute effect
    sigma: np.ndarray  # standard deviation of the effects
    effects: np.ndarray  # [trajectory, factor] or [trajectory, factor, output]


def elementary_effects(design, outputs):
    """Effects of a `morris` design from its outputs [trajectory, step] or
    [trajectory, step, output]."""
    design, y = np.asarray(design, dtype=float), np.asarray(outputs, dtype=float)
    r, steps, k = design.shape
    if y.shape[:2] != (r, steps):
        raise ValueError(f"outputs must start with [trajectory, step] = {(r, steps)}, not {y.shape}")
    dx = np.diff(design, axis=1)  # [r, k, factor]: one nonzero per step
    factor = np.abs(dx).argmax(axis=2)
    size = np.take_along_axis(dx, factor[..., None], axis=2)[..., 0]
    dy = np.diff(y, axis=1)
    extra = (1,) * (y.ndim - 2)
    ee = np.empty((r, k) + y.shape[2:])
    rows = np.arange(r)[:, None]
    ee[rows, factor] = dy / size.reshape(size.shape + extra)
    return Effects(ee.mean(axis=0), np.abs(ee).mean(axis=0), ee.std(axis=0, ddof=1) if r > 1 else
                   np.zeros(ee.shape[1:]), ee)
