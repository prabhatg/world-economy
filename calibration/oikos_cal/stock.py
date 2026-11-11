"""Domestic stock operator (#51): whether a dated faunal assemblage holds bones of
domestic animals, for when herds reached a land with no wild sheep or goats to breed
them from (Egypt: Linseele et al. 2014, `oikos_data.egypt`).

A world gives the domestic share q of the mammal bones its people discarded around an
assemblage's site (cells within `rings`, its centuries weighted by overlap): bones come
from managed herds in proportion to herding effort, their domestic share being the herds'
(`record.Activity.herders`, `herd_domestic`), and from game in proportion to foraging
effort, at `game` bones per unit of foraging effort for one of herding. Of the
assemblage's n identified mammal bones, then, some are domestic with probability
1 - (1 - q)^n. Fish are left out of the record, as the model's foraging does not tell
fishing from hunting; `game` (0.1, the sheep-size operator's `HUNTING`) is a judgment,
and presence depends on it far less than the share would: a herd of a tenth of the
bones shows in an assemblage of tens of bones whatever the rest are.

Presence, not the domestic share, is scored because the share depends on how much of
foraging went to fish and game, which the record shows varied from the desert's hunters
(game 33-99% of bones) to the Nile's fishers (game under 8%) and the model does not
represent; whether domestic animals were there at all does not. Where nobody lives near
a site in a world, its assemblage shows none.

Eras hold their assemblages by midpoint; an era's summary is the share of its
assemblages expected to hold domestic animals, against the share that do. Its sampling
variance is that of Jeffreys' posterior of the share of such assemblages that held some,
Beta(k + 1/2, n - k + 1/2) for k of n: where every assemblage held domestic animals the
binomial variance would be none, yet a few assemblages cannot show that all did.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["Operator", "GAME", "place", "era_shares"]

GAME = 0.1  # mammal bones per unit of foraging effort, for one of herding (herds.HUNTING)


def place(assemblages, cells, window: Window, eras, rings):
    """The assemblages an operator scores, each with its era, the cells within `rings` of
    its site (positions in `cells`, H3 ids ascending) and its centuries' weights by
    overlap: (kept, era_of, near, weight). An assemblage goes to the era holding its
    midpoint; one in none is left out."""
    cells = np.asarray(cells, dtype=np.uint64)
    res = h3.get_resolution(h3.int_to_str(int(cells[0])))
    lo = window.first + 100 * np.arange(window.centuries)
    kept, era_of, near, weight = [], [], [], []
    for a in assemblages:
        start, end = a["start_year"], a["end_year"]
        era = next((k for k, (s, e) in enumerate(eras) if s <= (start + end) / 2 < e), None)
        if era is None:
            continue
        cell = h3.latlng_to_cell(a["lat"], a["lng"], res)
        disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(cell, rings)), dtype=np.uint64)
        pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
        kept.append(a)
        era_of.append(era)
        near.append(pos[cells[pos] == disk])
        weight.append(np.clip(np.minimum(end, lo + 100) - np.maximum(start, lo), 0, None) / 100)
    return kept, np.array(era_of, dtype=int), near, weight


def era_shares(era_of, held, eras):
    """Each era's share of assemblages holding the thing, how many it has, and the
    sampling variance of Jeffreys' posterior of the share, Beta(k + 1/2, n - k + 1/2)."""
    if missing := [e for k, e in enumerate(eras) if not (era_of == k).any()]:
        raise ValueError(f"eras {missing} hold no assemblage")
    observed = np.array([held[era_of == k].mean() for k in range(len(eras))])
    counts = np.array([(era_of == k).sum() for k in range(len(eras))])
    a, b = observed * counts + 0.5, counts - observed * counts + 0.5
    return observed, counts, a * b / ((a + b) ** 2 * (a + b + 1))


class Operator:
    def __init__(self, assemblages, cells, window: Window, eras, rings=1, game=GAME, label="Egypt"):
        """`assemblages`: dicts as `oikos_data.egypt.load` gives them (name, start_year,
        end_year, mammals, domestic, lat, lng); `cells`: H3 ids of the world's cells,
        ascending; `eras`: (start, end) years, each assemblage going to the one holding its
        midpoint (none: left out)."""
        self.window, self.game, self.eras = window, game, list(eras)
        kept, self.era_of, self.near, self.weight = place(assemblages, cells, window, self.eras, rings)
        self.bones = np.array([float(a["mammals"]) for a in kept])
        held = np.array([a["domestic"] > 0 for a in kept])
        self.names = [f"{label} domestic animals {-s}-{-e} BCE share of assemblages" for s, e in self.eras]
        self.observed, self.counts, self.sampling_variance = era_shares(self.era_of, held, self.eras)

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near)

    def domestic_share(self, activity):
        """The domestic share of the mammal bones discarded around each assemblage (0 where
        nobody lives near it in its centuries)."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        out = np.zeros(len(self.near))
        for i, (c, w) in enumerate(zip(self.near, self.weight)):
            herders = (activity.herders[c].sum(axis=0) * w).sum()
            foragers = ((activity.people[c] - activity.cultivators[c] - activity.herders[c]).sum(axis=0) * w).sum()
            bones = herders + self.game * max(foragers, 0.0)
            if bones > 0:
                out[i] = (activity.herd_domestic[c].sum(axis=0) * w).sum() / bones
        return out

    def shown(self, activity):
        """Each assemblage's probability of holding domestic bones in the world."""
        return 1.0 - (1.0 - np.clip(self.domestic_share(activity), 0.0, 1.0)) ** self.bones

    def expected(self, activity):
        """Each era's expected share of assemblages holding domestic bones."""
        p = self.shown(activity)
        return np.array([p[self.era_of == k].mean() for k in range(len(self.eras))])
