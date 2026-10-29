"""Radiocarbon-density observation operator (docs/05 §5.1, ADR-0050).

What the record measures is not how many people lived where, but how many datable
samples from their activity were preserved, found, dated and calibrated. The operator
forward-models that chain and compares in the space of calibrated dates:

1. activity: people-years per cell and century, from a recorded world (`record`), with
   cultivators optionally more visible (sedentary sites leave more datable material);
2. taphonomic loss: older deposits survive less, by the global rate of Surovell et al.
   (2009, doi:10.1016/j.jas.2009.03.029), validated by Bluhm & Surovell (2019,
   doi:10.1017/qua.2018.78);
3. measurement and calibration: the kernel K[k, t, s] is the calibrated probability that
   falls in century s for a sample whose true date is uniform in century t, measured
   with the typical error of class k and calibrated exactly as the dates pipeline does
   (curve wiggles and plateaus included) - the model is blurred like the data, rather
   than the data "deconvolved";
4. research intensity: unknown, and varying far more across space than within a region
   over time, so it is conditioned out. Dates are grouped in blocks (H3 cells at a
   coarse resolution) and only their distribution over time within each block is
   scored: a multinomial given each block's and error class's total. Any per-block
   factor cancels, and so does overall scale;
5. clustering: many dates from one site and phase are one observation of activity, not
   many. Dates of a site (cell and name) whose calibrated medians lie within 200 years
   are a bin and share one date's weight (the binning of Shennan et al. 2013,
   doi:10.1038/ncomms3486; Timpson et al. 2014, doi:10.1016/j.jas.2014.08.011). Bins
   are formed greedily from the oldest median, so a bin never spans more than 200 years
   (in one dimension this is what rcarbon's complete-linkage cut approximates, Crema &
   Bevan 2021, doi:10.1017/RDC.2020.95).

The score is a pseudo-log-likelihood: binned masses are fractional counts. A small
background share (1% by default) stands for contamination, misattributed contexts and
model error, so dates where the model has nobody cost a lot but stay finite.

What it does not constrain: relative date density between blocks (that needs a model of
research intensity), and anything inside a block. Those are follow-ups (#17).
"""

from dataclasses import dataclass, field

import h3
import numpy as np

from oikos_data.radiocarbon import Curve, calibrate

__all__ = ["Window", "Taphonomy", "SUROVELL_2009", "NO_TAPHONOMY", "Settings", "Observations", "kernel",
           "observe", "Operator", "synthetic_dates"]


@dataclass(frozen=True)
class Window:
    """Calendar years [first, end) in whole centuries (negative is BCE)."""

    first: int
    end: int

    def __post_init__(self):
        if self.end <= self.first or (self.end - self.first) % 100:
            raise ValueError(f"window {self.first}..{self.end} must span a positive whole number of centuries")

    @property
    def centuries(self):
        return (self.end - self.first) // 100

    def midpoints(self):
        return self.first + 50 + 100 * np.arange(self.centuries)


@dataclass(frozen=True)
class Taphonomy:
    """Relative survival of deposits of age t (cal BP): (t + offset)^-exponent, scaled to 1
    at 0 BP."""

    offset: float
    exponent: float

    def survival(self, years):
        t = 1950 - np.asarray(years, dtype=float)
        return ((t + self.offset) / self.offset) ** -self.exponent


SUROVELL_2009 = Taphonomy(offset=2176.4, exponent=1.3925309)
NO_TAPHONOMY = Taphonomy(offset=2176.4, exponent=0.0)


@dataclass(frozen=True)
class Settings:
    true: Window = Window(-12_000, -3_000)  # years activity is recorded for
    observed: Window = Window(-11_500, -3_500)  # centuries compared: margins keep edge effects out
    error_edges: tuple = (0, 50, 100, 200)  # error classes (edges[k], edges[k+1]]; larger errors are dropped
    min_loc_accuracy: int = 2  # p3k14c LocAccuracy: 2 within 500 m, 3 exact
    bin_years: int = 200
    block_resolution: int = 2  # H3 resolution of research-intensity blocks (~87,000 km2)
    background: float = 0.01
    step: float = 5.0  # 14C-age grid of the kernel (years)
    pad: int = 3000  # calendar years beyond the windows that calibration normalizes over


def kernel(curve: Curve, true: Window, observed: Window, errors, step=5.0, pad=3000):
    """K[k, t, s] for the representative 1-sigma errors `errors` (one per class)."""
    years = np.arange(min(true.first, observed.first) - pad, max(true.end, observed.end) + pad)
    bp = 1950 - years
    mu, sc = np.interp(bp, curve.cal_bp, curve.c14), np.interp(bp, curve.cal_bp, curve.sigma)
    obs = (years >= observed.first) & (years < observed.end)
    tru = np.nonzero((years >= true.first) & (years < true.end))[0]
    out = np.empty((len(errors), true.centuries, observed.centuries))
    for k, err in enumerate(errors):
        s2 = err * err + sc * sc
        mu_t, s_t = mu[tru], np.sqrt(s2[tru])
        ages = np.arange((mu_t - 6 * s_t).min(), (mu_t + 6 * s_t).max() + step, step)
        # cal[a, s]: calibrated mass in observed century s of a measured age ages[a]
        cal = np.empty((len(ages), observed.centuries))
        for i in range(0, len(ages), 256):
            a = ages[i : i + 256, None]
            like = np.exp(-0.5 * (a - mu) ** 2 / s2) / np.sqrt(s2)
            like /= like.sum(axis=1, keepdims=True)
            cal[i : i + 256] = like[:, obs].reshape(len(a), observed.centuries, 100).sum(axis=2)
        for t in range(true.centuries):
            y = tru[100 * t : 100 * (t + 1)]
            w = np.exp(-0.5 * (ages - mu[y, None]) ** 2 / s2[y, None])
            w /= w.sum(axis=1, keepdims=True)
            out[k, t] = w.mean(axis=0) @ cal
    return out


@dataclass
class Observations:
    """Binned calibrated date mass per block, error class and observed century."""

    blocks: np.ndarray  # H3 ids of the blocks with dates, ascending
    errors: np.ndarray  # representative error of each class (median of its dates)
    mass: np.ndarray  # [block, class, century]
    weights: np.ndarray  # bin weight of each date used
    lab_ids: list = field(default_factory=list)  # the dates used, in `weights` order


def _block(cell, resolution):
    return h3.str_to_int(h3.cell_to_parent(h3.int_to_str(int(cell)), resolution))


def observe(dates, curve: Curve, settings: Settings) -> Observations:
    """Bin and calibrate a dates table (columns as `oikos_data.radiocarbon`: h3, site_name,
    c14_age, c14_error, loc_accuracy, lab_id)."""
    edges, ow = settings.error_edges, settings.observed
    keep = [i for i, (e, acc) in enumerate(zip(dates["c14_error"], dates["loc_accuracy"]))
            if edges[0] < e <= edges[-1] and acc >= settings.min_loc_accuracy]
    if not keep:
        raise ValueError(f"no usable dates: none has an error in ({edges[0]}, {edges[-1]}] years and "
                         f"location accuracy >= {settings.min_loc_accuracy}")
    bp = np.arange(curve.cal_bp[0], curve.cal_bp[-1] + 1)
    cal = 1950 - bp
    inside = (cal >= ow.first) & (cal < ow.end)
    century = ((cal[inside] - ow.first) // 100).astype(np.int64)
    masses, medians = [], []
    for i in keep:
        _, p = calibrate(dates["c14_age"][i], dates["c14_error"][i], curve)
        masses.append(np.bincount(century, weights=p[inside], minlength=ow.centuries))
        medians.append(int(cal[min(int(np.searchsorted(np.cumsum(p), 0.5)), len(p) - 1)]))

    sites = {}
    for j, i in enumerate(keep):
        sites.setdefault((int(dates["h3"][i]), dates["site_name"][i].strip().casefold()), []).append(j)
    weights = np.zeros(len(keep))
    for members in sites.values():
        members.sort(key=lambda j: medians[j])
        start = 0
        for n, j in enumerate(members + [None]):
            if j is None or medians[j] - medians[members[start]] > settings.bin_years:
                weights[members[start:n]] = 1.0 / (n - start)
                start = n

    block = np.array([_block(dates["h3"][i], settings.block_resolution) for i in keep], dtype=np.uint64)
    blocks = np.unique(block)
    err = np.array([dates["c14_error"][i] for i in keep], dtype=float)
    cls = np.searchsorted(np.asarray(edges, dtype=float), err, side="left") - 1
    reps = np.array([np.median(err[cls == k]) if (cls == k).any() else 0.5 * (edges[k] + edges[k + 1])
                     for k in range(len(edges) - 1)])
    mass = np.zeros((len(blocks), len(reps), ow.centuries))
    for j, b in enumerate(np.searchsorted(blocks, block)):
        mass[b, cls[j]] += weights[j] * masses[j]
    return Observations(blocks, reps, mass, weights, [dates["lab_id"][i] for i in keep])


class Operator:
    """Scores activity histories against one observation table; build once, score many."""

    def __init__(self, dates, curve: Curve, cells, settings: Settings = Settings()):
        """`cells`: H3 ids of the world's cells, in the order activity arrays use."""
        self.settings = settings
        self.obs = observe(dates, curve, settings)
        self.kernel = kernel(curve, settings.true, settings.observed, self.obs.errors, settings.step, settings.pad)
        parents = np.array([_block(c, settings.block_resolution) for c in cells], dtype=np.uint64)
        pos = np.searchsorted(self.obs.blocks, parents).clip(max=len(self.obs.blocks) - 1)
        self.cell_block = np.where(self.obs.blocks[pos] == parents, pos, -1)

    def _shares(self, activity, taphonomy):
        """q[b, k, s]: the share of block b's class-k mass expected in century s."""
        activity = np.asarray(activity, dtype=float)
        if activity.shape != (len(self.cell_block), self.settings.true.centuries):
            raise ValueError(f"activity must be [cells, centuries] = "
                             f"{(len(self.cell_block), self.settings.true.centuries)}, not {activity.shape}")
        used = self.cell_block >= 0
        a = np.zeros((len(self.obs.blocks), activity.shape[1]))
        np.add.at(a, self.cell_block[used], activity[used])
        a *= taphonomy.survival(self.settings.true.midpoints())
        q = np.einsum("bt,kts->bks", a, self.kernel)
        total = q.sum(axis=2, keepdims=True)
        s = q.shape[2]
        q = np.divide(q, total, out=np.full_like(q, 1.0 / s), where=total > 0)
        return (1 - self.settings.background) * q + self.settings.background / s

    def expected(self, activity, taphonomy: Taphonomy = SUROVELL_2009):
        """Expected binned mass, with each block's and class's observed total."""
        return self._shares(activity, taphonomy) * self.obs.mass.sum(axis=2, keepdims=True)

    def log_likelihood(self, activity, taphonomy: Taphonomy = SUROVELL_2009):
        return float((self.obs.mass * np.log(self._shares(activity, taphonomy))).sum())


def synthetic_dates(cells, activity, true: Window, curve: Curve, n, errors, taphonomy: Taphonomy, rng):
    """A dates table drawn from a known history, for validating the operator: `n` dates,
    each from a cell and century in proportion to activity times survival, a true date
    uniform in the century, an error drawn from `errors`, and a measured age from the
    curve with that error and the curve's own. Every date is its own site, exactly
    located."""
    w = np.asarray(activity, dtype=float) * taphonomy.survival(true.midpoints())
    pick = rng.choice(w.size, size=n, p=(w / w.sum()).ravel())
    cell, century = np.unravel_index(pick, w.shape)
    years = true.first + 100 * century + rng.uniform(0, 100, n)
    err = rng.choice(np.asarray(errors, dtype=float), size=n)
    bp = 1950 - years
    mu, sc = np.interp(bp, curve.cal_bp, curve.c14), np.interp(bp, curve.cal_bp, curve.sigma)
    ages = rng.normal(mu, np.sqrt(err * err + sc * sc))
    return {"h3": [int(cells[c]) for c in cell], "site_name": [f"site {i}" for i in range(n)],
            "c14_age": ages.tolist(), "c14_error": err.tolist(), "loc_accuracy": [3] * n,
            "lab_id": [f"SYN-{i}" for i in range(n)]}
