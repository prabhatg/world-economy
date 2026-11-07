"""Infection lesions operator (ADR-0086, #42): how much more common infection was after
farming than before, from lesions on the skeletons of the southern Levant (Eshed et al.
2010).

A skeleton shows a lesion of infection only if its person lived with the infection long
enough for bone to react, and was buried, excavated and preserved: an unknown share k of
the infected (the osteological paradox, Wood et al. 1992). So the record's 2 of 200
Natufians and 9 of 205 Neolithic people cannot say how common infection was, only how
much more common: given that 11 skeletons show lesions, the share of them that are
Neolithic is binomial, with probability n_N k P_N / (n_N k P_N + n_A k P_A), in which k
cancels (n the skeletons examined in each period, P its prevalence of infection).

A world gives P for each period as its sites' prevalences (infected over people-years
within `rings` of each site's cell, in the period's centuries; `record.Activity.infected`)
weighted by how many of the period's skeletons each site gave. Its summary is the
Neolithic share of the lesioned; undefined if neither period carries the infection, as
then no skeleton could show it. The observation variance is binomial at the observed
share, over the 11 lesioned.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["Operator"]


class Operator:
    def __init__(self, periods, sites, cells, window: Window, rings=1):
        """`periods`: two, in time order (name, start_year, end_year, examined, affected);
        `sites`: name, period, individuals, lat, lng; `cells`: H3 ids of the world's cells,
        ascending."""
        if len(periods) != 2:
            raise ValueError(f"the operator compares two periods, not {len(periods)}")
        self.window, self.periods = window, periods
        cells = np.asarray(cells, dtype=np.uint64)
        res = h3.get_resolution(h3.int_to_str(int(cells[0])))
        lo = window.first + 100 * np.arange(window.centuries)
        self.centuries, self.sites = [], []
        for p in periods:
            if p["start_year"] < window.first or p["end_year"] > window.end:
                raise ValueError(f"{p['name']} ({p['start_year']}..{p['end_year']}) lies outside {window}")
            self.centuries.append((lo >= p["start_year"]) & (lo + 100 <= p["end_year"]))
            near = []
            for s in (s for s in sites if s["period"] == p["name"]):
                disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.latlng_to_cell(s["lat"], s["lng"], res),
                                                                                rings)), dtype=np.uint64)
                pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
                near.append((pos[cells[pos] == disk], s["individuals"]))
            self.sites.append(near)
        self.examined = np.array([p["examined"] for p in periods], dtype=float)
        affected = np.array([p["affected"] for p in periods], dtype=float)
        self.lesioned = affected.sum()
        self.observed = affected[1] / self.lesioned
        self.sampling_variance = self.observed * (1 - self.observed) / self.lesioned

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return [cells for near in self.sites for cells, _ in near]

    def prevalence(self, activity):
        """Each period's prevalence of infection around its sites, weighted by their
        skeletons (sites without people in the world are left out; NaN if all are)."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        out = []
        for centuries, near in zip(self.centuries, self.sites):
            total = weight = 0.0
            for cells, n in near:
                people = activity.people[np.ix_(cells, centuries)].sum()
                if people > 0:
                    total += n * activity.infected[np.ix_(cells, centuries)].sum() / people
                    weight += n
            out.append(total / weight if weight > 0 else np.nan)
        return np.array(out)

    def expected(self, activity):
        """The Neolithic share of the lesioned skeletons the world expects (NaN where
        neither period carries the infection)."""
        lesioned = self.examined * self.prevalence(activity)
        total = lesioned.sum()
        return float(lesioned[1] / total) if total > 0 else np.nan
