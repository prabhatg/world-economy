"""Crops operator (#51): whether the people of a dated assemblage grew cultivated crops, for
when crops reached a land where they did not grow wild (Egypt: Linseele et al. 2014,
`oikos_data.egypt`).

A world gives the domestic share q of the cereals its people discarded around an
assemblage's site (cells within `rings`, its centuries weighted by overlap): cereals come
from fields in proportion to cultivation effort, their domestic share being the crop's
(`record.Activity.domestic`), and from wild grasses gathered in proportion to foraging
effort, at `gathered` for one sown (the rachis operators' `m1.GATHERED`). Of the `items`
cereal remains a sample holds, then, some are domestic with probability 1 - (1 - q)^items.
The record does not count them; `items` (10) is a judgment, and presence depends on it
little: fields of domestic crops show in ten remains wherever they are a tenth of the
cereals, and cultivating wild cereal shows nothing.

Eras hold their assemblages by midpoint, those whose crops the record does not state
left out; an era's summary is the share of its assemblages expected to show crops,
against the share that do, with the sampling variance of `stock.era_shares`.
"""

import numpy as np

from .radiocarbon import Window
from .stock import era_shares, place

__all__ = ["Operator", "ITEMS"]

ITEMS = 10  # cereal remains in an assemblage's sample: a judgment


class Operator:
    def __init__(self, assemblages, cells, window: Window, eras, rings=1, gathered=0.03, items=ITEMS,
                 label="Egypt"):
        """`assemblages`: dicts as `oikos_data.egypt.load` gives them (name, start_year,
        end_year, lat, lng and, where known, crops); `cells`: H3 ids of the world's cells,
        ascending; `eras`: (start, end) years, each assemblage going to the one holding its
        midpoint (none: left out)."""
        self.window, self.gathered, self.items, self.eras = window, gathered, items, list(eras)
        known = [a for a in assemblages if "crops" in a]
        kept, self.era_of, self.near, self.weight = place(known, cells, window, self.eras, rings)
        held = np.array([a["crops"] for a in kept], dtype=bool)
        self.names = [f"{label} cultivated crops {-s}-{-e} BCE share of assemblages" for s, e in self.eras]
        self.observed, self.counts, self.sampling_variance = era_shares(self.era_of, held, self.eras)

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near)

    def domestic_share(self, activity):
        """The domestic share of the cereals discarded around each assemblage (0 where
        nobody lives near it in its centuries)."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        herders = activity.herders if activity.herders is not None else np.zeros_like(activity.people)
        out = np.zeros(len(self.near))
        for i, (c, w) in enumerate(zip(self.near, self.weight)):
            sown = (activity.cultivators[c].sum(axis=0) * w).sum()
            foraging = ((activity.people[c] - activity.cultivators[c] - herders[c]).sum(axis=0) * w).sum()
            cereals = sown + self.gathered * max(foraging, 0.0)
            if cereals > 0:
                out[i] = (activity.domestic[c].sum(axis=0) * w).sum() / cereals
        return out

    def shown(self, activity):
        """Each assemblage's probability of holding cultivated crops in the world."""
        return 1.0 - (1.0 - np.clip(self.domestic_share(activity), 0.0, 1.0)) ** self.items

    def expected(self, activity):
        """Each era's expected share of assemblages holding cultivated crops."""
        p = self.shown(activity)
        return np.array([p[self.era_of == k].mean() for k in range(len(self.eras))])
