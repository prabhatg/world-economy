"""History matching: rule out the parameter values whose worlds cannot be the record
(docs/05 §5.2, ADR-0052).

As in climate-model tuning (Craig et al. 1997; Vernon, Goldstein and Bower 2010,
doi:10.1214/10-BA524; Williamson et al. 2013, doi:10.1007/s00382-013-1896-4): each wave
runs worlds over the space not yet ruled out, fits an emulator per summary, and rules
out points whose implausibility

    I(x) = |z - E[f(x)]| / sqrt(V_obs + V_discrepancy + V_emulator(x))

exceeds 3 for any summary (Pukelsheim's three-sigma rule: at least 95% of any unimodal
distribution lies within three standard deviations). Summaries observed with correlated
errors (the eras of one dated record share its sites and blocks) are also checked
together: a group's multivariate implausibility, the Mahalanobis distance with the
observation covariance plus discrepancy and emulator variances, must stay below the
99.5% quantile of chi-square (Vernon, Goldstein and Bower 2010). A shape every summary
tolerates on its own can still be ruled out. The model-discrepancy variance
says how far the best world may honestly sit from the record; without it, history
matching overfits a known-imperfect model. What survives every wave is "not ruled out
yet" (NROY), not a posterior: it is where simulation-based inference starts (§5.2 step 4).
An empty NROY space means no parameter values reconcile the model with the record: a
mechanism is missing or wrong.
"""

from dataclasses import dataclass

import numpy as np
from scipy.stats import chi2

from .emulator import GP

__all__ = ["Target", "Group", "implausibility", "chi2_cutoff", "Wave", "nroy", "CUTOFF"]

CUTOFF = 3.0


@dataclass(frozen=True)
class Target:
    """One summary of the record: its observed value, the observation variance and the
    model-discrepancy variance."""

    name: str
    observed: float
    variance: float
    discrepancy: float


def implausibility(mean, var, targets):
    """I[point, target] from emulator means and variances [point, target]."""
    z = np.array([t.observed for t in targets])
    v = np.array([t.variance + t.discrepancy for t in targets])
    return np.abs(z - mean) / np.sqrt(v + var)


def chi2_cutoff(dof, q=0.995):
    return float(chi2.ppf(q, dof))


@dataclass
class Group:
    """Targets (by index) whose observation errors are correlated, with their
    observation covariance; `dof` defaults to the group's size (one less for summaries
    constrained to sum to zero)."""

    indices: list
    covariance: np.ndarray
    dof: int = None

    def __post_init__(self):
        self.indices = list(self.indices)
        self.covariance = np.asarray(self.covariance, dtype=float)
        if self.covariance.shape != (len(self.indices),) * 2:
            raise ValueError(f"covariance must be {len(self.indices)} x {len(self.indices)}")
        if self.dof is None:
            self.dof = len(self.indices)

    def implausibility(self, mean, var, targets):
        """Squared Mahalanobis distance per point, from emulator means and variances
        [point, target] over all targets."""
        mean, var = np.atleast_2d(mean), np.broadcast_to(np.asarray(var, dtype=float), np.shape(np.atleast_2d(mean)))
        z = np.array([targets[i].observed for i in self.indices])
        disc = np.array([targets[i].discrepancy for i in self.indices])
        d = z - mean[:, self.indices]
        total = self.covariance + np.einsum("pi,ij->pij", disc + var[:, self.indices], np.eye(len(z)))
        return np.einsum("pi,pi->p", d, np.linalg.solve(total, d[..., None])[..., 0])


@dataclass
class Wave:
    targets: list
    emulators: list
    active: list  # factors the emulators see; the others are noise to them
    cutoff: float = CUTOFF
    groups: list = None

    @classmethod
    def fit(cls, u, outputs, targets, active=None, rng=None, cutoff=CUTOFF, groups=None):
        """Emulators of `outputs` [run, target] from runs at unit-cube points `u` [run, factor]."""
        u, y = np.asarray(u, dtype=float), np.asarray(outputs, dtype=float)
        if y.shape != (len(u), len(targets)):
            raise ValueError(f"outputs must be [run, target] = {(len(u), len(targets))}, not {y.shape}")
        active = list(range(u.shape[1])) if active is None else list(active)
        rng = rng or np.random.default_rng(0)
        return cls(list(targets), [GP.fit(u[:, active], y[:, j], rng=rng) for j in range(len(targets))], active,
                   cutoff, list(groups or []))

    def _predict(self, u):
        x = np.asarray(u, dtype=float)[:, self.active]
        mv = [g.predict(x, noise=True) for g in self.emulators]
        return np.stack([m for m, _ in mv], axis=1), np.stack([v for _, v in mv], axis=1)

    def implausibility(self, u):
        return implausibility(*self._predict(u), self.targets)

    def group_implausibility(self, u):
        """[point, group] multivariate implausibilities."""
        m, v = self._predict(u)
        return np.stack([g.implausibility(m, v, self.targets) for g in self.groups], axis=1) if self.groups else \
            np.zeros((len(m), 0))

    def plausible(self, u):
        m, v = self._predict(u)
        ok = implausibility(m, v, self.targets).max(axis=1) <= self.cutoff
        for g in self.groups or []:
            ok &= g.implausibility(m, v, self.targets) <= chi2_cutoff(g.dof)
        return ok


def nroy(waves, k, n, rng, draws=1_000_000, chunk=20_000):
    """Up to `n` uniform points of the unit cube that no wave rules out, and the share
    of uniform draws that survived (an estimate of the NROY volume). Gives up after
    `draws` draws."""
    kept, tried, survived = [], 0, 0
    while tried < draws and sum(len(c) for c in kept) < n:
        u = rng.random((min(chunk, draws - tried), k))
        ok = np.ones(len(u), dtype=bool)
        for w in waves:
            ok[ok] = w.plausible(u[ok])
        tried += len(u)
        survived += int(ok.sum())
        kept.append(u[ok])
    points = np.concatenate(kept)[:n] if kept else np.empty((0, k))
    return points, survived / tried
