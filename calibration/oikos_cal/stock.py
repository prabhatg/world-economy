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

__all__ = ["Operator", "GAME"]

GAME = 0.1  # mammal bones per unit of foraging effort, for one of herding (herds.HUNTING)


class Operator:
    def __init__(self, assemblages, cells, window: Window, eras, rings=1, game=GAME, label="Egypt"):
        """`assemblages`: dicts as `oikos_data.egypt.load` gives them (name, start_year,
        end_year, mammals, domestic, lat, lng); `cells`: H3 ids of the world's cells,
        ascending; `eras`: (start, end) years, each assemblage going to the one holding its
        midpoint (none: left out)."""
        self.window, self.game, self.eras = window, game, list(eras)
        cells = np.asarray(cells, dtype=np.uint64)
        res = h3.get_resolution(h3.int_to_str(int(cells[0])))
        lo = window.first + 100 * np.arange(window.centuries)
        self.era_of, self.near, self.weight, self.bones, held = [], [], [], [], []
        for a in assemblages:
            start, end = a["start_year"], a["end_year"]
            era = next((k for k, (s, e) in enumerate(self.eras) if s <= (start + end) / 2 < e), None)
            if era is None:
                continue
            cell = h3.latlng_to_cell(a["lat"], a["lng"], res)
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(cell, rings)), dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            self.era_of.append(era)
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(np.clip(np.minimum(end, lo + 100) - np.maximum(start, lo), 0, None) / 100)
            self.bones.append(float(a["mammals"]))
            held.append(a["domestic"] > 0)
        self.era_of, self.bones, held = np.array(self.era_of), np.array(self.bones), np.array(held)
        if missing := [e for k, e in enumerate(self.eras) if not (self.era_of == k).any()]:
            raise ValueError(f"eras {missing} hold no assemblage")
        self.names = [f"{label} domestic animals {-s}-{-e} BCE share of assemblages" for s, e in self.eras]
        self.observed = np.array([held[self.era_of == k].mean() for k in range(len(self.eras))])
        self.counts = np.array([(self.era_of == k).sum() for k in range(len(self.eras))])
        a = self.observed * self.counts + 0.5
        b = self.counts - self.observed * self.counts + 0.5
        self.sampling_variance = a * b / ((a + b) ** 2 * (a + b + 1))

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
