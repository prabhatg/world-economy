"""Summaries of the record that history matching compares worlds with (docs/05 §5.2,
ADR-0052).

History matching needs a few summaries, each with an observed value, an observation
variance and a model-discrepancy variance. They are taken from the observation
operators, so the model side goes through the same forward model (taphonomy,
calibration, research-intensity blocks, the sown-crop neighbourhood) that the
likelihoods use:

- radiocarbon eras: how the binned calibrated date mass of the region is shared among
  equal eras, as centered log shares (overall scale is not observed). The model side is
  the operator's expected mass, which keeps each block's observed total, so research
  intensity across blocks still cancels. Observation variance is a bootstrap over
  blocks: it covers the counting noise and how differently regions were occupied and
  studied. The bootstrap also gives the eras' covariance, so history matching can judge
  their shape together (`group`);
- domestication eras: the mean domestic share of barley assemblages in each era,
  weighted by n / (n + φ), the share of an assemblage's information that beta-binomial
  overdispersion φ leaves (ADR-0051). Observation variance is that of the weighted mean
  of beta-binomial shares around it.

Discrepancy standard deviations are judgments, stated where the problem is set up and
recorded with each wave: how far a good world may honestly sit from these summaries.
"""

import numpy as np

from .history import Group, Target
from .radiocarbon import SUROVELL_2009

__all__ = ["RadiocarbonEras", "DomesticationEras"]


def _era(first, end):
    fmt = lambda y: f"{-y} BCE" if y < 0 else f"{y} CE"
    return f"{-first}-{fmt(end)}" if first < 0 and end <= 0 else f"{fmt(first)}-{fmt(end)}"


def _clr(mass):
    lg = np.log(mass)
    return lg - lg.mean(axis=-1, keepdims=True)


class RadiocarbonEras:
    def __init__(self, op, eras=5, discrepancy_sd=0.25, taphonomy=SUROVELL_2009, visibility=0.0, replicates=2000,
                 rng=None):
        """`visibility`: how many times more visible cultivators are (`record.Activity.visible`)."""
        w = op.settings.observed
        if w.centuries % eras:
            raise ValueError(f"{eras} eras do not divide the {w.centuries} observed centuries")
        self.op, self.eras, self.taphonomy, self.visibility = op, eras, taphonomy, visibility
        per_block = op.obs.mass.sum(axis=1)  # [block, century]
        z = self._summary(per_block.sum(axis=0))
        rng = rng or np.random.default_rng(0)
        pick = rng.integers(0, len(per_block), size=(replicates, len(per_block)))
        boot = self._summary(per_block[pick].sum(axis=1))
        self.covariance = np.cov(boot, rowvar=False)
        var = np.diag(self.covariance)
        span = 100 * w.centuries // eras
        self.targets = [_target(f"radiocarbon {_era(w.first + e * span, w.first + (e + 1) * span)}", z[e], var[e],
                                discrepancy_sd**2) for e in range(eras)]

    def group(self, first):
        """These eras as a `history.Group`, their targets starting at index `first`
        (centered log shares sum to zero: one degree of freedom fewer)."""
        return Group(list(range(first, first + self.eras)), self.covariance, dof=self.eras - 1)

    def _summary(self, mass):
        return _clr(mass.reshape(mass.shape[:-1] + (self.eras, -1)).sum(axis=-1))

    def model(self, activity):
        """Summaries of a world: `activity` is an `Activity` or people-years [cell, century]."""
        a = activity.visible(self.visibility) if hasattr(activity, "visible") else activity
        return self._summary(self.op.expected(a, self.taphonomy).sum(axis=(0, 1)))


class DomesticationEras:
    def __init__(self, op, table, edges=(-9_000, -8_000, -7_000), discrepancy_sd=0.1):
        """Eras split at calendar years `edges` (ascending); each must hold an assemblage."""
        self.op = op
        years = np.asarray(table["median_year"])
        era = np.searchsorted(np.asarray(edges), years, side="right")
        self.weights = op.n / (op.n + op.concentration)
        self.members = [np.nonzero(era == e)[0] for e in range(len(edges) + 1)]
        bounds = [None, *edges, None]
        self.targets = []
        share = op.k / op.n
        for e, m in enumerate(self.members):
            lo, hi = bounds[e], bounds[e + 1]
            name = ("before " + f"{-hi} BCE" if lo is None else "after " + f"{-lo} BCE" if hi is None
                    else _era(lo, hi))
            if not len(m):
                raise ValueError(f"no assemblages in {name}; choose wider eras")
            w = self.weights[m]
            p = (w * share[m]).sum() / w.sum()
            n, phi = op.n[m], op.concentration
            v = max(p * (1 - p), 1e-6) * (n + phi) / (n * (1 + phi))
            self.targets.append(_target(f"domestic barley {name}", p, (w * w * v).sum() / w.sum() ** 2,
                                        discrepancy_sd**2))

    def model(self, activity):
        s = self.op.shares(activity)
        return np.array([(self.weights[m] * s[m]).sum() / self.weights[m].sum() for m in self.members])


def _target(name, observed, variance, discrepancy):
    return Target(name, float(observed), float(variance), float(discrepancy))
