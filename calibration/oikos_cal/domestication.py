"""Domestication-timing observation operator (docs/05 §5.1, ADR-0051).

The archaeobotanical record of domestication is the share of non-shattering
(domestic-type) cereal ears among the rachis fragments of dated assemblages (Tanno and
Willcox 2006; Fuller et al. 2014; Allaby et al. 2017). The model's counterpart is the
domestic share of the crop being sown near the site when it was occupied: the
people-and-effort-weighted domestication of the cells within `rings` of the site
(1: the site's cell and its six neighbours, about 25 km across at resolution 5) over
the century of the phase's median date and `half_window` centuries either side (phase
medians carry a few centuries of uncertainty). Where nobody cultivates nearby, the
model's ears are from wild stands, domestic only at the wild mutant frequency.

Assemblages vary far more than binomial sampling: mixtures of gathered wild and sown
cereal, of contexts and phases, and of identification criteria. Counts are therefore
beta-binomial around the model share, with `concentration` φ (variance
p(1-p)(n+φ)/(n(1+φ)) of the share): φ = 10 lets a single assemblage sit far from the
regional trend without dominating the score, and the binomial is the limit φ → ∞.

What it does not yet model: the share of the harvest gathered from wild stands by
cultivators, regional crops other than barley, and the dating uncertainty of each phase
beyond the window.
"""

import math

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["beta_binomial", "Operator"]

_lgamma = np.vectorize(math.lgamma, otypes=[float])


def _lbeta(a, b):
    return _lgamma(a) + _lgamma(b) - _lgamma(a + b)


def beta_binomial(k, n, p, concentration):
    """log P(k | n) for counts k of n with mean share p and concentration φ."""
    k, n = np.asarray(k, dtype=float), np.asarray(n, dtype=float)
    a, b = p * concentration, (1 - p) * concentration
    return _lgamma(n + 1) - _lgamma(k + 1) - _lgamma(n - k + 1) + _lbeta(k + a, n - k + b) - _lbeta(a, b)


class Operator:
    """Scores domestication histories against a rachis table (columns as
    `oikos_data.rachis`: h3, median_year, k_nonshattering, n_rachis)."""

    def __init__(self, table, cells, window: Window, rings=1, half_window=1, concentration=2.0, wild=0.001):
        """`cells`: H3 ids of the world's cells (ascending, the order activity arrays use)."""
        self.window, self.concentration, self.wild = window, concentration, wild
        cells = np.asarray(cells, dtype=np.uint64)
        self.k = np.asarray(table["k_nonshattering"], dtype=float)
        self.n = np.asarray(table["n_rachis"], dtype=float)
        self.near, self.when = [], []
        for site, h, year in zip(table["site"], table["h3"], table["median_year"]):
            c = (year - window.first) // 100
            if not 0 <= c < window.centuries:
                raise ValueError(f"{site}: median year {year} is outside the recorded window {window.first}..{window.end}")
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(int(h)), rings)), dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            if cells[pos[disk == h]].size == 0 or cells[pos[disk == h][0]] != h:
                raise ValueError(f"{site}: cell {h3.int_to_str(int(h))} is not in the world")
            self.near.append(pos[cells[pos] == disk])
            self.when.append(slice(max(c - half_window, 0), min(c + half_window + 1, window.centuries)))

    def shares(self, activity):
        """Model domestic share of the crop at each assemblage."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        out = np.empty(len(self.near))
        for i, (cells, when) in enumerate(zip(self.near, self.when)):
            sown = activity.cultivators[cells, when].sum()
            out[i] = activity.domestic[cells, when].sum() / sown if sown > 0 else self.wild
        return out

    def log_likelihood(self, activity):
        p = self.shares(activity).clip(self.wild, 1 - self.wild)
        return float(beta_binomial(self.k, self.n, p, self.concentration).sum())
