"""Emulators: fast statistical stand-ins for the simulator in history matching (docs/05
§5.2, ADR-0052).

A Gaussian process per output (Kennedy and O'Hagan 2001, doi:10.1111/1467-9868.00294;
Rasmussen and Williams 2006, ch. 2): a linear trend in the inputs plus a squared-
exponential correlation with one length scale per input (a long one marks an input that
does not matter) and a nugget for run-to-run noise - the seed's variability and that of
the inputs left out of the emulator. The trend's coefficients are estimated with the
rest (universal kriging), so the predictive variance includes their uncertainty.
Hyperparameters maximize the restricted likelihood (L-BFGS-B, several starts).
"""

from dataclasses import dataclass

import numpy as np
from scipy.linalg import cho_factor, cho_solve
from scipy.optimize import minimize

__all__ = ["latin_hypercube", "GP"]

_BOUNDS = [(np.log(0.03), np.log(30.0))]  # length scales in units of the unit cube
_SIGNAL = (np.log(1e-3), np.log(1e2))  # signal variance of the standardized output
_NUGGET = (np.log(1e-8), np.log(10.0))


def latin_hypercube(n, k, rng):
    """n points in [0, 1)^k, one in each of n equal slices of every factor."""
    return (np.stack([rng.permutation(n) for _ in range(k)], axis=1) + rng.random((n, k))) / n


def _corr(a, b, scales):
    d = (a[:, None, :] - b[None, :, :]) / scales
    return np.exp(-0.5 * (d * d).sum(axis=2))


def _basis(x):
    return np.hstack([np.ones((len(x), 1)), x])


def _restricted_nll(theta, x, y, h):
    k = x.shape[1]
    scales, signal, nugget = np.exp(theta[:k]), np.exp(theta[k]), np.exp(theta[k + 1])
    c = signal * _corr(x, x, scales) + (nugget + 1e-10 * signal) * np.eye(len(x))
    try:
        f = cho_factor(c, lower=True)
    except np.linalg.LinAlgError:
        return 1e10
    ki_h, ki_y = cho_solve(f, h), cho_solve(f, y)
    a = h.T @ ki_h
    try:
        fa = cho_factor(a, lower=True)
    except np.linalg.LinAlgError:
        return 1e10
    beta = cho_solve(fa, h.T @ ki_y)
    r = y - h @ beta
    return float(np.log(np.diag(f[0])).sum() + np.log(np.diag(fa[0])).sum() + 0.5 * r @ cho_solve(f, r))


@dataclass
class GP:
    x: np.ndarray
    length_scales: np.ndarray
    signal: float  # in output units squared
    noise: float  # nugget variance, in output units squared
    beta: np.ndarray
    alpha: np.ndarray  # K^-1 (y - H beta)
    chol: tuple
    chol_a: tuple
    scale: tuple  # (mean, sd) the output was standardized with

    @classmethod
    def fit(cls, x, y, rng=None, starts=4):
        x, y = np.asarray(x, dtype=float), np.asarray(y, dtype=float)
        if y.shape != (len(x),):
            raise ValueError(f"need one output per run: {len(x)} runs, outputs of shape {y.shape}")
        if not np.isfinite(y).all() or not np.isfinite(x).all():
            raise ValueError("runs and outputs must be finite; drop failed runs before fitting")
        rng = rng or np.random.default_rng(0)
        mean, sd = y.mean(), y.std() or 1.0
        z = (y - mean) / sd
        k, h = x.shape[1], _basis(x)
        bounds = _BOUNDS * k + [_SIGNAL, _NUGGET]
        best = None
        for s in range(starts):
            theta0 = np.concatenate([np.log(rng.uniform(0.2, 2.0, k)) if s else np.zeros(k),
                                     [0.0, np.log(rng.uniform(1e-4, 0.1))]])
            res = minimize(_restricted_nll, theta0, args=(x, z, h), method="L-BFGS-B", bounds=bounds)
            if best is None or res.fun < best.fun:
                best = res
        theta = best.x
        scales, signal, nugget = np.exp(theta[:k]), np.exp(theta[k]), np.exp(theta[k + 1])
        c = signal * _corr(x, x, scales) + (nugget + 1e-10 * signal) * np.eye(len(x))
        f = cho_factor(c, lower=True)
        fa = cho_factor(h.T @ cho_solve(f, h), lower=True)
        beta = cho_solve(fa, h.T @ cho_solve(f, z))
        return cls(x, scales, signal * sd * sd, nugget * sd * sd, beta, cho_solve(f, z - h @ beta), f, fa,
                   (mean, sd))

    def predict(self, x, noise=False):
        """Mean and variance of the output at `x`; with `noise`, of a new run there."""
        x = np.asarray(x, dtype=float)
        mean, sd = self.scale
        signal = self.signal / (sd * sd)
        kx = signal * _corr(x, self.x, self.length_scales)
        hx = _basis(x)
        m = hx @ self.beta + kx @ self.alpha
        v = signal - (kx * cho_solve(self.chol, kx.T).T).sum(axis=1)
        r = hx - kx @ cho_solve(self.chol, _basis(self.x))
        v += (r * cho_solve(self.chol_a, r.T).T).sum(axis=1)
        v = np.maximum(v, 0) * sd * sd
        return mean + sd * m, v + (self.noise if noise else 0.0)
