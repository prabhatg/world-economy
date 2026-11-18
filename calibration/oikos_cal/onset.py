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

Records of several sources pool (`Operator.pooled`, ADR-0139): each phase's share is read
against its own source's anchor, since sources count remains differently (Maeda et al.
2016 count cereal grain and chaff among all charred remains), and a bootstrap draws a
source's sites whole, each with its phases and anchor phases, which share its recovery
and reporting.

Every anchor phase counts, however few crops it burnt (ADR-0140): Çatalhöyük burnt dung
and El Kowm lay in the steppe, and target phases mix the same kinds of sites, so an anchor
of crop-waste villages alone would be selected on its outcome and read every era low.

Phases can be split by how well wild cereals grew around them (`suitability`, the
engine's wild-cereal suitability of their cells and centuries; #36): learning that
copies what visibly pays should start cultivation in the richest stands, and
`targets.OnsetContrasts` compares the effort implied there with that in poor stands.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["Operator", "ANCHOR_PERIOD", "ANCHOR_YEAR", "CEREAL", "cereal_suitability", "suitability"]

CEREAL = ("cereal_precip_min", "cereal_precip_full", "cereal_precip_wet", "cereal_precip_max", "cereal_temp_min",
          "cereal_temp_full")

ANCHOR_PERIOD = "PPNC/PN"
# A record without periods (`oikos_data.cereals`) is anchored on its phases from the
# Pottery Neolithic's start; the database's own anchor phases run from 7300 to 5000 BC.
ANCHOR_YEAR = -7000


class Operator:
    def __init__(self, table, cells, window: Window, rings=1, min_items=50):
        """`table`: columns as `oikos_data.archaeobotany` (h3, period, start_year,
        end_year, crop, items; without period, phases from `ANCHOR_YEAR` anchor it);
        `cells`: H3 ids of the world's cells, ascending."""
        self.window = window
        cells = np.asarray(cells, dtype=np.uint64)
        crop, items = (np.asarray(table[c], dtype=float) for c in ("crop", "items"))
        big = items >= min_items
        if "period" in table:
            anchor = big & (np.asarray(table["period"]) == ANCHOR_PERIOD)
        else:
            mid = (np.asarray(table["start_year"], dtype=float) + np.asarray(table["end_year"], dtype=float)) / 2
            anchor = big & (mid >= ANCHOR_YEAR)
        if not anchor.any():
            raise ValueError(f"no {ANCHOR_PERIOD} phases with {min_items} items or more to anchor a farming world")
        self.anchors = crop[anchor] / items[anchor]
        self.farming_share = float(self.anchors.mean())
        self.anchor_sets, self.farming = [self.anchors], np.array([self.farming_share])
        self.anchor_sites = [np.asarray(table["site"], dtype=object)[anchor]]
        lo = window.first + 100 * np.arange(window.centuries)
        self.k, self.n, self.mid, self.near, self.weight, self.names, self.sites = [], [], [], [], [], [], []
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
            self.sites.append(table["site"][i])
        self.k, self.n, self.mid = (np.array(x) for x in (self.k, self.n, self.mid))
        self.sites = np.array(self.sites, dtype=object)
        self.source = np.zeros(len(self.k), dtype=int)

    @classmethod
    def pooled(cls, ops):
        """One record of several sources' phases (#55): each phase's share is read against
        its own source's anchor (`implied`), since sources count remains differently, and
        resampled within its source (`resample`)."""
        if len({op.window for op in ops}) != 1:
            raise ValueError("pooled sources must share a window")
        if any(len(op.anchor_sets) > 1 for op in ops):
            raise ValueError("a pooled record is not pooled again: pool its sources once")
        out = cls.__new__(cls)
        out.window = ops[0].window
        out.k, out.n, out.mid = (np.concatenate([getattr(op, a) for op in ops]) for a in ("k", "n", "mid"))
        out.near, out.weight, out.names = ([x for op in ops for x in getattr(op, a)] for a in ("near", "weight", "names"))
        out.sites = np.concatenate([op.sites for op in ops])
        out.anchor_sites = [a for op in ops for a in op.anchor_sites]
        out.source = np.concatenate([np.full(len(op.k), i) for i, op in enumerate(ops)]).astype(int)
        out.anchor_sets = [a for op in ops for a in op.anchor_sets]
        out.farming = np.concatenate([op.farming for op in ops])
        return out

    def implied(self):
        """The effort each phase implies: its share over its source's farming share."""
        return self.k / self.n / self.farming[self.source]

    def resample(self, rng, replicates):
        """Bootstrap replicates of the record: (phase indices, the effort they imply).
        Sites are drawn with replacement within each source, each bringing all its
        phases and its anchor phases, since a site's phases share its recovery and
        reporting; a replicate whose source draws no anchor implies NaN."""
        share = self.k / self.n
        sources = []
        for s, (values, where) in enumerate(zip(self.anchor_sets, self.anchor_sites)):
            mine = np.flatnonzero(self.source == s)
            names = sorted(set(self.sites[mine]) | set(where))
            index = {x: i for i, x in enumerate(names)}
            sources.append((mine, np.array([index[x] for x in self.sites[mine]], dtype=int), values,
                            np.array([index[x] for x in where], dtype=int),
                            rng.integers(0, len(names), size=(replicates, len(names)))))
        for r in range(replicates):
            picks, scale = [], np.empty(len(sources))
            for s, (mine, at, values, anchor_at, draws) in enumerate(sources):
                times = np.bincount(draws[r], minlength=draws.shape[1])
                picks.append(np.repeat(mine, times[at]))
                drawn = np.repeat(values, times[anchor_at])
                scale[s] = drawn.mean() if len(drawn) else np.nan
            p = np.concatenate(picks)
            yield p, share[p] / scale[self.source[p]]

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near)

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


def cereal_suitability(temp, precip, params):
    """The engine's wild-cereal suitability (`environment::cereal_suitability`): ramps up
    from too dry and too cold, down toward wet forest, with the `CEREAL` `params`; 0
    without climate data."""
    ramp = lambda x, lo, hi: np.clip((x - lo) / (hi - lo), 0.0, 1.0)
    p = params
    s = (ramp(precip, p["cereal_precip_min"], p["cereal_precip_full"])
         * (1.0 - ramp(precip, p["cereal_precip_wet"], p["cereal_precip_max"]))
         * ramp(temp, p["cereal_temp_min"], p["cereal_temp_full"]))
    return np.nan_to_num(s)


def suitability(op, temp, precip, years, params):
    """Mean wild-cereal suitability at each of `op`'s phases, over its cells and centuries,
    from climate slices [time, cell] at calendar `years` (each century of the window
    takes the slice nearest its midpoint) and the `CEREAL` `params`."""
    s = cereal_suitability(temp, precip, params)
    years = np.asarray(years, dtype=float)
    nearest = np.abs(years[None, :] - op.window.midpoints()[:, None]).argmin(axis=1)
    by_century = s[nearest]  # [century, cell]
    return np.array([(by_century[:, c].mean(axis=1) * w).sum() / w.sum() for c, w in zip(op.near, op.weight)])
