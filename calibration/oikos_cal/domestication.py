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

Cultivators kept gathering from wild stands, and foragers beside them did too, bolstering
their stores with wild cereal (Allaby et al. 2017): each people-year of foraging effort
(neither sowing nor herding) nearby brings `gathered` times the ears of a people-year
sown, all wild. So a camp that sows a little among wide wild stands reads wild, while a
village that sows most of its effort reads as its fields (ADR-0103).

Assemblages vary far more than binomial sampling: mixtures of gathered wild and sown
cereal, of contexts and phases, and of identification criteria. Counts are therefore
beta-binomial around the model share, with `concentration` φ (variance
p(1-p)(n+φ)/(n(1+φ)) of the share): φ = 10 lets a single assemblage sit far from the
regional trend without dominating the score, and the binomial is the limit φ → ∞.

Each crop is judged by its own record against its own share (#38): barley against the
crop's, emmer and einkorn against the wheats bands sow with it (`record.Activity`'s
`emmer_domestic`, `einkorn_domestic`), under the same effort.

Foragers gather each wheat only where its wild ancestor grows (`stands`, from the
`wild_wheats` pack, #38): wild emmer and einkorn range far less widely than wild barley,
which they gather wherever they forage. So a village beyond a wheat's range, as Aşıklı
Höyük for emmer, reads as its fields.

What it does not yet model: the dating uncertainty of each phase beyond the window.
"""

import math

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["CROPS", "beta_binomial", "crop", "Operator"]

# Each crop's domestic people-years in `record.Activity`.
CROPS = {"barley": "domestic", "emmer": "emmer_domestic", "einkorn": "einkorn_domestic"}

_lgamma = np.vectorize(math.lgamma, otypes=[float])


def _lbeta(a, b):
    return _lgamma(a) + _lgamma(b) - _lgamma(a + b)


def beta_binomial(k, n, p, concentration):
    """log P(k | n) for counts k of n with mean share p and concentration φ."""
    k, n = np.asarray(k, dtype=float), np.asarray(n, dtype=float)
    a, b = p * concentration, (1 - p) * concentration
    return _lgamma(n + 1) - _lgamma(k + 1) - _lgamma(n - k + 1) + _lbeta(k + a, n - k + b) - _lbeta(a, b)


def crop(table, name, leave_out=()):
    """The rows of a rachis table (columns as lists) for crop `name`, but those of the sites `leave_out`."""
    keep = [i for i, (c, s) in enumerate(zip(table["crop"], table["site"])) if c == name and s not in leave_out]
    return {k: [v[i] for i in keep] for k, v in table.items()}


class Operator:
    """Scores domestication histories against a rachis table of one crop (columns as
    `oikos_data.rachis`: crop, h3, median_year, k_nonshattering, n_rachis)."""

    def __init__(self, table, cells, window: Window, rings=1, half_window=1, concentration=2.0, wild=0.001,
                 gathered=0.0, stands=None):
        """`cells`: H3 ids of the world's cells (ascending, the order activity arrays use);
        `gathered`: ears gathered wild per people-year of foraging effort, against one sown;
        `stands`: whether each cell is in the crop's wild range (1) or not (0), one per cell
        (None: every cell is)."""
        crops = sorted(set(table["crop"]))
        if len(crops) > 1:
            raise ValueError(f"the table holds {crops}: judge one crop at a time (`crop`)")
        self.crop = crops[0] if crops else "barley"
        if self.crop not in CROPS:
            raise ValueError(f"no model share for {self.crop}; the model sows {sorted(CROPS)}")
        self.field = CROPS[self.crop]
        self.window, self.concentration, self.wild, self.gathered = window, concentration, wild, gathered
        cells = np.asarray(cells, dtype=np.uint64)
        if stands is not None and len(stands) != len(cells):
            raise ValueError(f"stands holds {len(stands)} shares, the world {len(cells)} cells: give one per cell")
        if stands is not None and not np.isin(stands, (0.0, 1.0)).all():
            raise ValueError("stands must be 0 or 1 per cell: the cells gathered from are a zone of the reduced record")
        self.stands = None if stands is None else np.asarray(stands, dtype=float)
        self.sites = list(table["site"])
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
        # The near cells where the crop's wild stands grow: foragers gather from these.
        self.gather = [n if self.stands is None else n[self.stands[n] > 0] for n in self.near]

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near) + [g for g in self.gather if g.size]

    def shares(self, activity):
        """Model domestic share of the crop at each assemblage."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        domestic = getattr(activity, self.field)
        out = np.empty(len(self.near))
        for i, (cells, gather, when) in enumerate(zip(self.near, self.gather, self.when)):
            sown = activity.cultivators[cells, when].sum()
            wild = self.gathered * self._foraging(activity, gather, when) if self.gathered and gather.size else 0.0
            out[i] = (domestic[cells, when].sum() + self.wild * wild) / (sown + wild) if sown + wild > 0 else self.wild
        return out

    @staticmethod
    def _foraging(activity, cells, when):
        """People-years times foraging effort: neither sown nor herded."""
        herded = activity.herders[cells, when].sum() if activity.herders is not None else 0.0
        return max(activity.people[cells, when].sum() - activity.cultivators[cells, when].sum() - herded, 0.0)

    def log_likelihood(self, activity):
        p = self.shares(activity).clip(self.wild, 1 - self.wild)
        return float(beta_binomial(self.k, self.n, p, self.concentration).sum())
