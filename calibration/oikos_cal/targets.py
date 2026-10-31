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
  of beta-binomial shares around it;
- site eras: in each survey's landscape, how the aoristic site count is shared among
  eras, as centered log shares, against the world's settled activity dated the same way
  (`sites.Operator`). Observation variance and covariance are a bootstrap over sites;
  each survey is one group;
- obsidian zones: the mean Cappadocian share of sourced assemblages (each context
  counting once) in zones of relative distance to the two source groups, before and
  after an era boundary (`obsidian.Operator`). Observation variance and covariance are a
  bootstrap over contexts; all zones are one group.

Discrepancy standard deviations are judgments, stated where the problem is set up and
recorded with each wave: how far a good world may honestly sit from these summaries.
"""

import numpy as np

from .history import Group, Target
from .radiocarbon import SUROVELL_2009

__all__ = ["RadiocarbonEras", "DomesticationEras", "SiteEras", "ObsidianZones"]


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


class SiteEras:
    def __init__(self, op, discrepancy_sd=0.25, replicates=2000, rng=None):
        """Eras of `op` (`sites.Operator`) in each survey window; each era needs sites."""
        self.op = op
        rng = rng or np.random.default_rng(0)
        self.targets, self.covariances = [], []
        edges = op.edges
        for name in op.windows:
            per_site = op.per_site[name]
            empty = per_site.sum(axis=0) <= 0
            if empty.any():
                raise ValueError(f"no sites in {name} {_era(edges[empty.argmax()], edges[empty.argmax() + 1])}")
            z = _clr(per_site.sum(axis=0))
            pick = rng.integers(0, len(per_site), size=(replicates, len(per_site)))
            cov = np.cov(_clr(per_site[pick].sum(axis=1)), rowvar=False)
            self.covariances.append(cov)
            self.targets += [_target(f"sites {name} {_era(edges[e], edges[e + 1])}", z[e], cov[e, e], discrepancy_sd**2)
                             for e in range(len(z))]

    def groups(self, first):
        """One `history.Group` per survey, their targets starting at index `first`."""
        n = len(self.op.edges) - 1
        return [Group(list(range(first + k * n, first + (k + 1) * n)), c, dof=n - 1)
                for k, c in enumerate(self.covariances)]

    def model(self, activity):
        """Summaries of a world: settled activity is `activity.cultivators` (survey sites
        are villages), or people-years [cell, century] given directly."""
        a = activity.cultivators if hasattr(activity, "cultivators") else activity
        e = self.op.expected(a)
        return np.concatenate([_clr(e[n]) for n in self.op.windows])


def _target(name, observed, variance, discrepancy):
    return Target(name, float(observed), float(variance), float(discrepancy))


class ObsidianZones:
    def __init__(self, op, edges=(-250.0, 0.0, 250.0), era=-6900, discrepancy_sd=0.1, replicates=2000, rng=None):
        """Zones of `op` (`obsidian.Operator`) between relative-distance `edges` (km,
        ascending; positive nearer the first source group), before and after calendar
        year `era` (by a context's period midpoint); each zone needs contexts."""
        self.op, self.edges, self.era = op, np.asarray(edges, dtype=float), era
        self.zone = self._zones()
        rng = rng or np.random.default_rng(0)
        share = op.k / op.n
        z = self._summary(share)
        pick = rng.integers(0, len(share), size=(replicates, len(share)))
        boot = np.array([self._summary(share[p], self.zone[p], strict=False) for p in pick])
        self.covariance = np.cov(boot[np.isfinite(boot).all(axis=1)], rowvar=False)
        labels = [self._label(lo, hi) for lo, hi in zip([-np.inf, *self.edges], [*self.edges, np.inf])]
        when = [f"before {-era} BCE", f"after {-era} BCE"]
        names = [f"obsidian {w}, {lab}" for w in when for lab in labels]
        self.targets = [_target(n, z[j], self.covariance[j, j], discrepancy_sd**2) for j, n in enumerate(names)]

    def _zones(self):
        """Zone index of each context: era-major, then by relative distance."""
        z = (self.op.mid >= self.era) * (len(self.edges) + 1) + np.searchsorted(self.edges, self.op.delta, side="right")
        empty = np.setdiff1d(np.arange(2 * (len(self.edges) + 1)), z)
        if empty.size:
            raise ValueError(f"no contexts in obsidian zone {empty[0]} (era-major, by relative distance)")
        return z

    def _summary(self, share, zone=None, strict=True):
        zone = self.zone if zone is None else zone
        n = 2 * (len(self.edges) + 1)
        total, count = np.bincount(zone, share, minlength=n), np.bincount(zone, minlength=n)
        return total / count if strict else np.divide(total, count, out=np.full(n, np.nan), where=count > 0)

    def _label(self, lo, hi):
        first, second = (self.op.label(g) for g in obsidian_groups())
        if np.isinf(lo):
            return f"{second} source nearer by over {-hi:g} km"
        if np.isinf(hi):
            return f"{first} source nearer by over {lo:g} km"
        if hi <= 0:
            return f"{second} source nearer by {abs(hi):g}-{-lo:g} km"
        if lo >= 0:
            return f"{first} source nearer by {lo:g}-{hi:g} km"
        return f"{first} source nearer by up to {hi:g} km or {second} by up to {-lo:g} km"

    def group(self, first):
        """All zones as one `history.Group`, their targets starting at index `first`."""
        return Group(list(range(first, first + len(self.targets))), self.covariance, dof=len(self.targets))

    def model(self, activity):
        """Summaries of a world: the model share at each context, averaged per zone."""
        return self._summary(self.op.shares(activity)[0])


def obsidian_groups():
    from .obsidian import GROUPS

    return GROUPS
