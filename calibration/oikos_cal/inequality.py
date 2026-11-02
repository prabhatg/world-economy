"""House-size inequality operator (ADR-0074): the Gini coefficient of house floor areas
that a site's excavated houses would show in a world, against those of dated sites
(Kohler et al. 2017).

A world gives the variance of log household wealth among the settled people around
each site (within `rings` of its cell, over the centuries within `span` years centred on
its date, weighted by settled people-years; `record.Activity.inequality`). House size
grows less than wealth: log floor area is `elasticity` times log wealth, so a village's
houses are lognormal with log sd `elasticity * sqrt(V)`. The elasticity, 0.7, is a
judgment: the median house-size Ginis of hunter-gatherer and agricultural sites (0.17
and 0.35, Kohler et al. 2017) are about 0.7 of the wealth Ginis of such societies (0.25
and 0.48, Borgerhoff Mulder et al. 2009); the discrepancy carries its uncertainty.

What a site shows is the small-sample Gini (Dixon et al. 1987) of its n measured houses,
a random variable. Its expectation in the world is the site's summary, and its sampling
variance at the site's own Gini the observation variance, both by Monte Carlo over fixed
standard normal draws, tabulated over the log sd so summaries are smooth in V.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["sample_gini", "SmallSample", "Operator", "ELASTICITY"]

ELASTICITY = 0.7
LOG_SD = np.linspace(0.0, 3.0, 601)  # house log sds tabulated: Ginis up to 0.97


def sample_gini(x):
    """Small-sample Gini (Dixon et al. 1987) of each row of `x` [..., n]: the mean absolute
    difference between distinct houses over twice their mean."""
    x = np.asarray(x, dtype=float)
    n = x.shape[-1]
    diff = np.abs(x[..., :, None] - x[..., None, :]).sum(axis=(-1, -2))
    return diff / (2 * n * (n - 1) * x.mean(axis=-1))


class SmallSample:
    """The Gini of `houses` lognormal house sizes as a function of their log sd: its mean
    and variance over `replicates` fixed draws."""

    def __init__(self, houses, replicates=4000, rng=None):
        rng = rng or np.random.default_rng(0)
        # exp(s z) keeps the order of z, so with each row sorted once the Gini is a rank-
        # weighted sum: sum_i (2i - n - 1) x_(i) / ((n - 1) sum x), as sample_gini.
        z = np.sort(rng.standard_normal((replicates, houses)), axis=1)
        rank = 2 * np.arange(1, houses + 1) - houses - 1
        g = np.empty((len(LOG_SD), replicates))
        for k, sd in enumerate(LOG_SD):
            x = np.exp(sd * (z - z[:, -1:]))  # scaled by the largest house: no overflow
            g[k] = x @ rank / ((houses - 1) * x.sum(axis=1))
        self.houses = houses
        self.mean, self.var = g.mean(axis=1), g.var(axis=1, ddof=1)

    def expected(self, log_sd):
        return np.interp(log_sd, LOG_SD, self.mean)

    def variance(self, log_sd):
        return np.interp(log_sd, LOG_SD, self.var)

    def log_sd(self, gini):
        """The log sd whose expected Gini is `gini` (the expectation rises with it)."""
        return np.interp(gini, self.mean, LOG_SD)


class Operator:
    def __init__(self, table, cells, window: Window, rings=1, span=300, elasticity=ELASTICITY, replicates=4000,
                 rng=None):
        """`table`: columns site, h3, year, gini and houses; `cells`: H3 ids of the world's
        cells, ascending."""
        self.window, self.elasticity = window, elasticity
        rng = rng or np.random.default_rng(0)
        cells = np.asarray(cells, dtype=np.uint64)
        lo = window.first + 100 * np.arange(window.centuries)
        self.names, self.years, self.observed, self.near, self.weight, self.samples = [], [], [], [], [], []
        by_houses = {}
        for i in range(len(table["site"])):
            year = float(table["year"][i])
            w = np.clip(np.minimum(year + span / 2, lo + 100) - np.maximum(year - span / 2, lo), 0, None) / 100
            if not w.any():
                continue
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(int(table["h3"][i])), rings)),
                            dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            n = int(table["houses"][i])
            if n not in by_houses:
                by_houses[n] = SmallSample(n, replicates, rng)
            self.names.append(table["site"][i])
            self.years.append(int(year))
            self.observed.append(float(table["gini"][i]))
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(w)
            self.samples.append(by_houses[n])

    def settled_variance(self, activity):
        """Mean variance of log wealth of the settled people around each site in its
        centuries (NaN where nobody farmed)."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        out = []
        for c, w in zip(self.near, self.weight):
            settled = (activity.cultivators[c].sum(axis=0) * w).sum()
            out.append((activity.inequality[c].sum(axis=0) * w).sum() / settled if settled > 0 else np.nan)
        return np.array(out)

    def expected(self, activity):
        """Expected small-sample house-size Gini at each site (NaN where nobody farmed)."""
        v = self.settled_variance(activity)
        return np.array([s.expected(self.elasticity * np.sqrt(x)) if np.isfinite(x) else np.nan
                         for s, x in zip(self.samples, v)])

    def sampling_variance(self, i):
        """Variance of site `i`'s small-sample Gini, at houses whose expected Gini is its own."""
        s = self.samples[i]
        return float(s.variance(s.log_sd(self.observed[i])))
