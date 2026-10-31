"""Cultivation-onset observation operator (docs/05 §5.1, #29; ADR-0064).

How much of their food people grew is not recorded directly, but what they burnt is:
the share of crops (domesticates and their wild progenitors) among the charred grains
and seeds of a site phase rises as people come to rely on planted crops (Wallace et al.
2019; `oikos_data.archaeobotany`). Counts are not food, though: arable weeds come with
the harvest, dung fuel brings wild seeds, and a cereal grain is many small seeds' worth
of food. So shares are not read as diets. The share of a farming world is anchored on
the Pottery Neolithic phases of the same record (the mean over phases, each once),
when people across the region lived on crops, and an era's mean share over that anchor
is the cultivation effort the record implies (`targets.OnsetEras`).

Both simplifications make the test lenient, never strict: Pottery Neolithic people
also gathered and herded, so full farming would show more crops than the anchor and
imply less effort than the record's; and foragers gathered wild crop progenitors,
which this leaves out. A world the record rules out under them is ruled out.

Shares vary enormously between phases at the same level of farming (logit standard
deviation 2-5 within a period: villages burning crop waste near 1, steppe sites
burning dung near 0), so phases are not scored one by one; only era means, over many
phases, carry information.

The model's effort at a phase is its people-weighted cultivation effort over the cells
within `rings` of the site (record: cultivator people-years over people-years) and the
centuries of the phase, each weighted by its overlap. Phases where a world has nobody
take the mean effort of the phases it reached, so every world is judged on the same
phases. Phases with fewer than `min_items` items (Wallace et al. analyse samples of 50
or more) are dropped.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["Operator", "ANCHOR_PERIOD"]

ANCHOR_PERIOD = "PPNC/PN"


class Operator:
    def __init__(self, table, cells, window: Window, rings=1, min_items=50):
        """`table`: columns as `oikos_data.archaeobotany` (h3, period, start_year,
        end_year, crop, items); `cells`: H3 ids of the world's cells, ascending."""
        self.window = window
        cells = np.asarray(cells, dtype=np.uint64)
        crop, items = (np.asarray(table[c], dtype=float) for c in ("crop", "items"))
        big = items >= min_items
        anchor = big & (np.asarray(table["period"]) == ANCHOR_PERIOD)
        if not anchor.any():
            raise ValueError(f"no {ANCHOR_PERIOD} phases with {min_items} items or more to anchor a farming world")
        self.anchors = crop[anchor] / items[anchor]
        self.farming_share = float(self.anchors.mean())
        lo = window.first + 100 * np.arange(window.centuries)
        self.k, self.n, self.mid, self.near, self.weight, self.names = [], [], [], [], [], []
        for i in np.flatnonzero(big & ~anchor):
            start, end = float(table["start_year"][i]), float(table["end_year"][i])
            w = np.clip(np.minimum(end, lo + 100) - np.maximum(start, lo), 0, None) / 100
            if not w.any():
                continue
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(int(table["h3"][i])), rings)),
                            dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            self.k.append(crop[i])
            self.n.append(items[i])
            self.mid.append((start + end) / 2)
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(w)
            self.names.append(table["phase"][i])
        self.k, self.n, self.mid = (np.array(x) for x in (self.k, self.n, self.mid))

    def effort(self, activity):
        """People-weighted cultivation effort at each phase, and whether anyone was there."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        sown = np.array([(activity.cultivators[c].sum(axis=0) * w).sum() for c, w in zip(self.near, self.weight)])
        people = np.array([(activity.people[c].sum(axis=0) * w).sum() for c, w in zip(self.near, self.weight)])
        reached = people > 0
        return np.where(reached, sown / np.where(reached, people, 1), 0.0), reached

    def efforts(self, activity):
        """The effort at each phase, phases nobody reached taking the mean of those
        reached, and which phases were reached."""
        e, reached = self.effort(activity)
        fill = e[reached].mean() if reached.any() else 0.0
        return np.where(reached, e, fill), reached
