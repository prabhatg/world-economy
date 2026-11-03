"""Village-size observation operator (docs/05 §5.1, #34): how large the settlements
surface surveys found were in each era, and how large a world's settlements are when
dated the same way.

Surveys record each site's area in hectares, Upper Mesopotamia per phase, Central
Anatolia mostly once per site (Palmisano et al. 2021). The record is the mean log area of
the sized site phases in each era, each phase counting in the eras its cultural period
covers in proportion to their overlap (the aoristic weights of `sites`). Phases without
a size are left out. The mean of logs suits sizes that span two orders of magnitude, and
it is what an area-to-people conversion by a constant density shifts by a constant.

A world is put through the dating of the sized phases (a `sites.Operator` of them: their
periods' popularity, over the cells holding sized sites), since phases without a size
neither enter the record nor date it: the mean log size of its settlements in an era is
its settlement-years times the log of their people (`record.Activity.village_size`),
dated, over its settlement-years, dated. An era whose landscape holds no settlements has
no mean size.
"""

import numpy as np

from . import sites
from .sites import _overlap

__all__ = ["Operator"]


class Operator:
    def __init__(self, table, window, edges):
        """`table`: columns window, site, cell, start_year, end_year, size_ha (NaN when
        unknown; `oikos_data.sites`); `window`: the `radiocarbon.Window` of the activity
        records; `edges`: era bounds, calendar years ascending."""
        size = np.asarray(table["size_ha"], dtype=float)
        sized = np.isfinite(size) & (size > 0)
        table = {k: np.asarray(v)[sized] for k, v in table.items()}
        self.dating = sites.Operator(table, window, edges)
        self.windows, self.edges = self.dating.windows, self.dating.edges
        w, size = table["window"], size[sized]
        self.log_area, self.weight, self.observed = {}, {}, {}
        for name in self.windows:
            m = w == name
            lo, hi = (np.asarray(table[k])[m].astype(float) for k in ("start_year", "end_year"))
            share = _overlap(lo, hi, self.edges) / (hi - lo)[:, None]  # [phase, era]
            ids, site = np.unique(np.asarray(table["site"])[m], return_inverse=True)
            self.weight[name] = np.zeros((len(ids), len(self.edges) - 1))
            self.log_area[name] = np.zeros_like(self.weight[name])
            np.add.at(self.weight[name], site, share)
            np.add.at(self.log_area[name], site, share * np.log(size[m])[:, None])
            self.observed[name] = self.mean(self.log_area[name], self.weight[name])

    @staticmethod
    def mean(log_area, weight):
        """Mean log area per era of per-site sums [site, era] (NaN in an era without any)."""
        total = weight.sum(axis=0)
        return np.divide(log_area.sum(axis=0), total, out=np.full(total.shape, np.nan), where=total > 0)

    def expected(self, activity):
        """Per survey window, the mean log people of the settlements the survey would date
        to each era in a world with `activity.settlements` and `activity.village_size`
        [cell, century]."""
        out = {}
        for n in self.windows:
            cells, transfer = self.dating.footprint[n], self.dating.transfer[n]
            years = transfer @ activity.settlements[cells].sum(axis=0)
            logs = transfer @ activity.village_size[cells].sum(axis=0)
            out[n] = np.divide(logs, years, out=np.full(years.shape, np.nan), where=years > 0)
        return out
