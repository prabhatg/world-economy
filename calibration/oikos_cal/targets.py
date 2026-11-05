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
- radiocarbon subregions: in each subregion (a group of research-intensity blocks), the
  calendar years by which a tenth, a quarter and half of its binned calibrated date mass
  had accrued: where activity rose first, which the region-wide eras cannot see. Years,
  not log shares, because a subregion may have no dates at all in its early eras. The
  model side is the operator's expected mass summed over the subregion's blocks, so
  research intensity still cancels per block. Observation variance and covariance are a
  bootstrap over each subregion's sites (all its dates resampled together), apart in
  each subregion; all subregions are one group;
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
  bootstrap over contexts; all zones are one group;
- onset contrasts: before and after an era boundary, the cultivation effort the plant
  remains imply at phases in rich wild-cereal stands minus that at phases in poor ones
  (`onset.suitability`), against the same contrast of a world's efforts. Observation
  variance and covariance are a bootstrap over the phases and the anchors; both eras
  are one group;
- village sizes: in each survey's landscape and era, the mean log area of the sized
  site phases converted to people by one density, against the mean log size of the
  world's settlements dated the same way (`villages.Operator`). Observation variance is
  a bootstrap over sites plus the density's uncertainty, which every era of every
  survey shares; all are one group.

Discrepancy standard deviations are judgments, stated where the problem is set up and
recorded with each wave: how far a good world may honestly sit from these summaries.
"""

import numpy as np

from .history import Group, Target
from .radiocarbon import SUROVELL_2009

__all__ = ["RadiocarbonEras", "RadiocarbonSubregions", "accrued", "DomesticationEras", "SiteEras", "VillageSizes", "ObsidianZones", "OnsetEras",
           "OnsetContrasts", "HouseGinis"]


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


def accrued(mass, first, shares):
    """Calendar years by which each of `shares` of `mass` [..., century] (centuries from
    year `first`) had accrued, linear within a century: [..., share]."""
    mass = np.asarray(mass, dtype=float)
    flat = mass.reshape(-1, mass.shape[-1])
    out = np.empty((len(flat), len(shares)))
    for r, m in enumerate(flat):
        cum = np.cumsum(m)
        target = np.asarray(shares) * cum[-1]
        c = np.searchsorted(cum, target, side="left").clip(max=len(m) - 1)
        before = np.where(c > 0, cum[c - 1], 0.0)
        out[r] = first + 100 * (c + (target - before) / m[c])
    return out.reshape(mass.shape[:-1] + (len(shares),))


class RadiocarbonSubregions:
    def __init__(self, op, subregions, shares=(0.1, 0.25, 0.5), discrepancy_sd=300.0, taphonomy=SUROVELL_2009,
                 visibility=0.0, replicates=2000, rng=None, min_sites=20):
        """`subregions`: name -> indices of `op`'s blocks (`radiocarbon.subregions`);
        blocks of none are left out. `discrepancy_sd` in years."""
        seen = np.concatenate([np.asarray(v, dtype=int) for v in subregions.values()] or [np.array([], int)])
        if len(np.unique(seen)) < len(seen):
            raise ValueError("a block is in two subregions")
        self.op, self.shares, self.taphonomy, self.visibility = op, tuple(shares), taphonomy, visibility
        self.names, self.members = list(subregions), [np.asarray(v, dtype=int) for v in subregions.values()]
        first = op.settings.observed.first
        site_mass = op.obs.site_mass.sum(axis=1)  # [site, century]
        rng = rng or np.random.default_rng(0)
        z, boot = [], []
        for name, blocks in zip(self.names, self.members):
            sites = np.nonzero(np.isin(op.obs.site_block, blocks) & (site_mass.sum(axis=1) > 0))[0]
            if len(sites) < min_sites:
                raise ValueError(f"{name} has {len(sites)} sites with usable dates; a bootstrap needs {min_sites}")
            m = site_mass[sites]
            z.append(accrued(m.sum(axis=0), first, self.shares))
            draws = rng.multinomial(len(sites), np.full(len(sites), 1.0 / len(sites)), size=replicates)
            boot.append(accrued(draws @ m, first, self.shares))
        z, boot = np.concatenate(z), np.concatenate(boot, axis=1)
        self.covariance = np.atleast_2d(np.cov(boot, rowvar=False))
        var = np.diag(self.covariance)
        labels = [f"radiocarbon {n} {round(100 * q)}% by" for n in self.names for q in self.shares]
        self.targets = [_target(lab, z[i], var[i], discrepancy_sd**2) for i, lab in enumerate(labels)]

    def group(self, first):
        """All subregions as one `history.Group`, their targets starting at index `first`."""
        n = len(self.targets)
        return Group(list(range(first, first + n)), self.covariance, dof=n)

    def model(self, activity):
        a = activity.visible(self.visibility) if hasattr(activity, "visible") else activity
        per_block = self.op.expected(a, self.taphonomy).sum(axis=1)  # [block, century]
        mass = np.stack([per_block[b].sum(axis=0) for b in self.members])
        return accrued(mass, self.op.settings.observed.first, self.shares).ravel()


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
        """Summaries of a world: its settlements (`activity.settlements`, ADR-0072: survey
        sites are settlements, not people), else settled people-years
        (`activity.cultivators`), or an array [cell, century] given directly. A survey
        whose landscape the world leaves empty for an era has no log shares (NaN): the
        surveys found sites in every era, so such a world is ruled out by its own outputs."""
        if getattr(activity, "settlements", None) is not None:
            a = activity.settlements
        else:
            a = activity.cultivators if hasattr(activity, "cultivators") else activity
        e = self.op.expected(a)
        return np.concatenate([_clr(e[n]) if (e[n] > 0).all() else np.full(len(e[n]), np.nan)
                               for n in self.op.windows])


class VillageSizes:
    def __init__(self, op, density, density_sd, discrepancy_sd=0.25, replicates=2000, rng=None, min_defined=0.9):
        """Eras of `op` (`villages.Operator`) in each survey window, in log people: log area
        plus the log of `density` (people per hectare), whose error `density_sd` (on the
        log scale) all targets share; how density differs between eras and surveys is
        left to the discrepancy, which is each target's own. Each era needs sized sites,
        enough that at least `min_defined` of the bootstrap replicates define every era."""
        self.op = op
        rng = rng or np.random.default_rng(0)
        edges, blocks, z, names = op.edges, [], [], []
        for name in op.windows:
            obs = op.observed[name]
            if np.isnan(obs).any():
                e = int(np.isnan(obs).argmax())
                raise ValueError(f"no sized sites in {name} {_era(edges[e], edges[e + 1])}")
            a, w = op.log_area[name], op.weight[name]
            pick = rng.integers(0, len(a), size=(replicates, len(a)))
            boot = np.array([op.mean(a[p], w[p]) for p in pick])
            defined = np.isfinite(boot).all(axis=1)
            if defined.mean() < min_defined:
                raise ValueError(f"too few sized sites in {name}: only {defined.mean():.0%} of bootstrap samples "
                                 f"hold sites in every era; choose wider eras")
            blocks.append(np.atleast_2d(np.cov(boot[defined], rowvar=False)))
            z.append(obs + np.log(density))
            names += [f"village size {name} {_era(edges[e], edges[e + 1])}" for e in range(len(obs))]
        n = sum(len(b) for b in blocks)
        self.covariance = np.full((n, n), density_sd**2)
        k = 0
        for b in blocks:
            self.covariance[k : k + len(b), k : k + len(b)] += b
            k += len(b)
        z = np.concatenate(z)
        self.targets = [_target(nm, z[j], self.covariance[j, j], discrepancy_sd**2) for j, nm in enumerate(names)]

    def group(self, first):
        """All eras of all surveys as one `history.Group`, their targets from index `first`."""
        return Group(list(range(first, first + len(self.targets))), self.covariance)

    def model(self, activity):
        """Mean log people of a world's settlements per survey and era (NaN in an era whose
        landscape it leaves without settlements: ruled out by its own outputs)."""
        e = self.op.expected(activity)
        return np.concatenate([e[n] for n in self.op.windows])


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


class OnsetEras:
    def __init__(self, op, edges=(-10_000, -8_800, -7_600), discrepancy_sd=0.1, replicates=2000, rng=None):
        """Eras of `op` (`onset.Operator`) between calendar-year `edges` (ascending), by a
        phase's midpoint: the cultivation effort the record implies, the era's mean crop
        share (each phase once) over the anchor's, with a bootstrap covariance that
        resamples the era phases and the anchor phases; every era needs phases."""
        self.op, self.edges = op, np.asarray(edges, dtype=float)
        self.era = np.searchsorted(self.edges, op.mid, side="right")
        n = len(self.edges) + 1
        if empty := sorted(set(range(n)) - set(self.era.tolist())):
            raise ValueError(f"no phases in onset era {empty[0]} (eras split at {list(edges)})")
        rng = rng or np.random.default_rng(0)
        share = op.k / op.n
        pick = rng.integers(0, len(share), size=(replicates, len(share)))
        anchor = rng.integers(0, len(op.anchors), size=(replicates, len(op.anchors)))
        boot = np.array([self._summary(share[p], self.era[p], strict=False) / op.anchors[a].mean()
                         for p, a in zip(pick, anchor)])
        self.covariance = np.cov(boot[np.isfinite(boot).all(axis=1)], rowvar=False)
        bounds = [f"before {-edges[0]:g} BCE", *(f"{-a:g}-{-b:g} BCE" for a, b in zip(edges, edges[1:])),
                  f"after {-edges[-1]:g} BCE"]
        z = self._summary(share) / op.farming_share
        self.targets = [_target(f"cultivation effort implied {b}", z[j], self.covariance[j, j], discrepancy_sd**2)
                        for j, b in enumerate(bounds)]

    def _summary(self, values, era=None, strict=True):
        era = self.era if era is None else era
        n = len(self.edges) + 1
        total, count = np.bincount(era, values, minlength=n), np.bincount(era, minlength=n)
        return total / count if strict else np.divide(total, count, out=np.full(n, np.nan), where=count > 0)

    def group(self, first):
        """All eras as one `history.Group`, their targets starting at index `first`."""
        return Group(list(range(first, first + len(self.targets))), self.covariance, dof=len(self.targets))

    def model(self, activity):
        """Summaries of a world: its effort at each phase, averaged per era."""
        return self._summary(self.op.efforts(activity)[0])


class OnsetContrasts:
    def __init__(self, op, rich, edges=(-7_600,), discrepancy_sd=0.1, replicates=2000, rng=None, min_phases=3):
        """`rich`: whether each of `op`'s phases is in a rich wild-cereal stand; eras split
        at calendar-year `edges` by a phase's midpoint, each needing `min_phases` rich
        and poor phases."""
        self.op, self.rich = op, np.asarray(rich, dtype=bool)
        self.era = np.searchsorted(np.asarray(edges, dtype=float), op.mid, side="right")
        self.n = len(edges) + 1
        bounds = [f"before {-edges[0]:g} BCE", *(f"{-a:g}-{-b:g} BCE" for a, b in zip(edges, edges[1:])),
                  f"after {-edges[-1]:g} BCE"]
        for e, b in enumerate(bounds):
            for stand, m in (("rich", self.rich), ("poor", ~self.rich)):
                if (k := int(np.sum(m & (self.era == e)))) < min_phases:
                    raise ValueError(f"{stand} stands {b} has {k} phases; a contrast needs {min_phases}")
        rng = rng or np.random.default_rng(0)
        share = op.k / op.n
        pick = rng.integers(0, len(share), size=(replicates, len(share)))
        anchor = rng.integers(0, len(op.anchors), size=(replicates, len(op.anchors)))
        boot = np.array([self._summary(share[p], self.era[p], self.rich[p]) / op.anchors[a].mean()
                         for p, a in zip(pick, anchor)])
        self.covariance = np.atleast_2d(np.cov(boot[np.isfinite(boot).all(axis=1)], rowvar=False))
        z = self._summary(share) / op.farming_share
        self.targets = [_target(f"cultivation effort rich minus poor stands {b}", z[e], self.covariance[e, e],
                                discrepancy_sd**2) for e, b in enumerate(bounds)]

    def _summary(self, values, era=None, rich=None):
        """Per era, the mean of `values` at rich phases minus that at poor ones (NaN when
        a resample has none of either)."""
        era = self.era if era is None else era
        rich = self.rich if rich is None else rich
        out = np.empty(self.n)
        for e in range(self.n):
            r, p = values[(era == e) & rich], values[(era == e) & ~rich]
            out[e] = r.mean() - p.mean() if len(r) and len(p) else np.nan
        return out

    def group(self, first):
        """Both eras as one `history.Group`, their targets starting at index `first`."""
        return Group(list(range(first, first + len(self.targets))), self.covariance, dof=len(self.targets))

    def model(self, activity):
        return self._summary(self.op.efforts(activity)[0])


def obsidian_groups():
    from .obsidian import GROUPS

    return GROUPS


class InfectionLesions:
    def __init__(self, op, discrepancy_sd=0.1):
        """One target from `op` (`lesions.Operator`): the Neolithic share of the skeletons
        with lesions of infection, with its binomial variance over the lesioned; judged by
        itself."""
        self.op = op
        self.targets = [_target("infection lesions Neolithic share of the lesioned", op.observed,
                                op.sampling_variance, discrepancy_sd**2)]

    def model(self, activity):
        return np.array([self.op.expected(activity)])


class SheepSize:
    def __init__(self, op, discrepancy_sd=0.015):
        """Eras of `op` (`herds.Operator`): each era's mean sheep LSI, with the sampling
        variance of its assemblages and the wild and domestic sizes' shared uncertainty;
        judged together."""
        self.op = op
        self.targets = [_target(n, z, op.covariance[k, k], discrepancy_sd**2)
                        for k, (n, z) in enumerate(zip(op.names, op.observed))]

    def group(self, first):
        n = len(self.targets)
        return Group(list(range(first, first + n)), self.op.covariance, dof=n)

    def model(self, activity):
        return self.op.expected(activity)


class HouseGinis:
    def __init__(self, op, discrepancy_sd=0.05):
        """One target per site of `op` (`inequality.Operator`): its house-size Gini, with
        the sampling variance of a Gini of its few houses; judged one by one (the sites
        are far apart in time and place)."""
        self.op = op
        year = lambda y: f"{-y} BCE" if y < 0 else f"{y} CE"
        self.targets = [_target(f"house Gini {n} {year(y)}", g, op.sampling_variance(i), discrepancy_sd**2)
                        for i, (n, y, g) in enumerate(zip(op.names, op.years, op.observed))]

    def model(self, activity):
        """Expected house-size Gini at each site (NaN where nobody farmed around it)."""
        return self.op.expected(activity)

