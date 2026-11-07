"""Sheep size operator (#37): when herds became domestic, from the body size of sheep in
dated faunal assemblages (Arbuckle et al. 2014).

Domestic sheep are smaller than wild ones, so an assemblage's mean log size index (LSI;
Meadow 1999) falls as more of its sheep bones come from domestic animals. A world gives
the domestic share q of the bones its people discarded around an assemblage's site
(cells within `rings`, its centuries weighted by overlap): bones come from managed herds
in proportion to herding effort and from wild sheep and goats hunted on their range in
proportion to foraging there, at `HUNTING` caprine bones per unit of foraging effort for
one of herding (`record.Activity.herders`, `herd_domestic`, `hunters`), so
q = domestic / (herders + HUNTING hunters), which is 0 before anyone herds and is not the
herds' domestic share where hunting goes on beside herding. The assemblage's expected
LSI is the mixture (1 - q) mu_wild + q mu_domestic.

HUNTING (0.1) is a judgment: foraging effort goes to plants and many prey, of which wild
caprines are one, while a herd yields animals for slaughter with far less effort; at 1
(bones as effort, whatever is foraged) the herders' domestic animals could never make
most bones domestic while foraging keeps half of everyone's effort.

The endpoints are judgments from the assemblages themselves: wild sheep +0.045 (the six
before management, unweighted; their spread 0.027) and domestic -0.045 (the 22 from
6,500 BCE; spread 0.017), each with an uncertainty all eras share, so the observation
covariance carries it at the observed eras' q (a linearization). Sex ratios, regional
size differences, the bones measured and hunting choices are each era's discrepancy.

Eras hold their assemblages by midpoint, weighted by bones measured; each era's sampling
variance is its assemblages' sd over their counts, with POOLED_SD where the table gives
no sd. Judged together. An era is undefined where nobody near its sites herds or hunts.
"""

import h3
import numpy as np

from .radiocarbon import Window

__all__ = ["Operator", "WILD", "DOMESTIC", "POOLED_SD", "HUNTING"]

WILD = (0.045, 0.02)  # mean LSI of wild sheep, and its uncertainty
DOMESTIC = (-0.045, 0.015)  # of domestic sheep
POOLED_SD = 0.035  # within-assemblage sd of LSI where the table gives none (its median)
HUNTING = 0.1  # caprine bones per unit of foraging effort on the range, for one of herding


class Operator:
    def __init__(self, table, cells, window: Window, edges, rings=1, wild=WILD, domestic=DOMESTIC, hunting=HUNTING):
        """`table`: the fauna table's columns (assemblage, start_year, end_year, lat, lng,
        ovis_lsi, ovis_lsi_sd, ovis_lsi_n); `cells`: H3 ids of the world's cells,
        ascending; `edges`: the years between eras inside `window`."""
        self.window, self.wild, self.domestic, self.hunting = window, wild, domestic, hunting
        cells = np.asarray(cells, dtype=np.uint64)
        res = h3.get_resolution(h3.int_to_str(int(cells[0])))
        self.eras = list(zip([window.first, *edges], [*edges, window.end]))
        lo = window.first + 100 * np.arange(window.centuries)
        self.era_of, self.near, self.weight, self.bones = [], [], [], []
        lsi, sd = [], []
        for i in range(len(table["assemblage"])):
            if table["ovis_lsi"][i] is None or not table["ovis_lsi_n"][i]:
                continue
            start, end = table["start_year"][i], table["end_year"][i]
            mid = (start + end) / 2
            era = next((k for k, (a, b) in enumerate(self.eras) if a <= mid < b), None)
            if era is None:
                continue
            w = np.clip(np.minimum(end, lo + 100) - np.maximum(start, lo), 0, None) / 100
            cell = h3.latlng_to_cell(table["lat"][i], table["lng"][i], res)
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(cell, rings)), dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            self.era_of.append(era)
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(w)
            self.bones.append(float(table["ovis_lsi_n"][i]))
            lsi.append(float(table["ovis_lsi"][i]))
            sd.append(POOLED_SD if table["ovis_lsi_sd"][i] is None else float(table["ovis_lsi_sd"][i]))
        self.era_of, self.bones, lsi, sd = np.array(self.era_of), np.array(self.bones), np.array(lsi), np.array(sd)
        if missing := [k for k in range(len(self.eras)) if not (self.era_of == k).any()]:
            raise ValueError(f"eras {[self.eras[k] for k in missing]} hold no measured sheep")
        self.names = [f"sheep size (LSI) {-a}-{-b} BCE" for a, b in self.eras]
        self.observed, sampling = np.zeros(len(self.eras)), np.zeros(len(self.eras))
        for k in range(len(self.eras)):
            m = self.era_of == k
            n = self.bones[m]
            self.observed[k] = (n * lsi[m]).sum() / n.sum()
            sampling[k] = (n * sd[m] ** 2).sum() / n.sum() ** 2
        (mw, sw), (md, sdd) = wild, domestic
        q = np.clip((mw - self.observed) / (mw - md), 0.0, 1.0)
        self.covariance = np.outer(1 - q, 1 - q) * sw**2 + np.outer(q, q) * sdd**2 + np.diag(sampling)

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near)

    def domestic_share(self, activity):
        """The domestic share of each assemblage's bones in the world (NaN where nobody
        near it herds or hunts in its centuries)."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        out = np.full(len(self.near), np.nan)
        for i, (c, w) in enumerate(zip(self.near, self.weight)):
            bones = ((activity.herders[c] + self.hunting * activity.hunters[c]).sum(axis=0) * w).sum()
            if bones > 0:
                out[i] = (activity.herd_domestic[c].sum(axis=0) * w).sum() / bones
        return out

    def expected(self, activity):
        """Each era's expected mean LSI (NaN where none of its assemblages is defined)."""
        (mw, _), (md, _) = self.wild, self.domestic
        lsi = mw + (md - mw) * self.domestic_share(activity)
        out = np.full(len(self.eras), np.nan)
        for k in range(len(self.eras)):
            m = (self.era_of == k) & np.isfinite(lsi)
            if m.any():
                out[k] = (self.bones[m] * lsi[m]).sum() / self.bones[m].sum()
        return out
