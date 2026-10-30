"""Site-count observation operator (docs/05 §5.1, #17): how many villages surface
surveys found in each era, and what a world's settled activity would show through the
same dating.

Surveys date a site phase by its pottery to a cultural period, often a long one, so a
site counts in each era in proportion to how much of its period falls there (aoristic
sums; Crema 2012; Palmisano et al. 2021). A world is put through the same dating: a
village of a given century is assigned to one of the periods surveyors used that
contain it, each with its popularity (phases dated to it per century of its length,
assuming activity is even within a period, as aoristic sums do), and spread over that
period. Centuries no period covers are invisible, as they are to the surveys.

Only the surveyed landscape counts: the cells holding any recorded site of any period,
so where surveyors walked, not where Neolithic villages happened to be found.
"""

import numpy as np

__all__ = ["Operator"]


def _overlap(lo, hi, edges):
    """[phase, era] years of each phase [lo, hi) inside each era [edges[e], edges[e+1])."""
    a, b = np.asarray(edges[:-1]), np.asarray(edges[1:])
    return np.clip(np.minimum(hi[:, None], b) - np.maximum(lo[:, None], a), 0, None)


class Operator:
    def __init__(self, table, window, edges):
        """`table`: columns window, site, cell, start_year, end_year (`oikos_data.sites`);
        `window`: the `radiocarbon.Window` of the activity records; `edges`: era bounds,
        calendar years ascending."""
        self.edges, self.window = np.asarray(edges), window
        w = np.asarray(table["window"])
        self.windows = sorted(set(w))
        self.footprint, self.observed, self.per_site, self.transfer = {}, {}, {}, {}
        mid = window.midpoints()
        for name in self.windows:
            m = w == name
            lo, hi = (np.asarray(table[k])[m].astype(float) for k in ("start_year", "end_year"))
            self.footprint[name] = np.unique(np.asarray(table["cell"])[m])
            share = _overlap(lo, hi, self.edges) / (hi - lo)[:, None]  # [phase, era]
            ids, site = np.unique(np.asarray(table["site"])[m], return_inverse=True)
            self.per_site[name] = np.zeros((len(ids), len(self.edges) - 1))
            np.add.at(self.per_site[name], site, share)
            self.observed[name] = self.per_site[name].sum(axis=0)
            periods, count = np.unique(np.stack([lo, hi], axis=1), axis=0, return_counts=True)
            popularity = count / (periods[:, 1] - periods[:, 0])
            inside = (periods[:, :1] < mid) & (mid < periods[:, 1:])  # [period, century]
            weight = popularity[:, None] * inside
            total = weight.sum(axis=0)
            dated = np.divide(weight, total, out=np.zeros_like(weight), where=total > 0)
            spread = _overlap(periods[:, 0], periods[:, 1], self.edges) / (periods[:, 1] - periods[:, 0])[:, None]
            self.transfer[name] = spread.T @ dated  # [era, century]

    def expected(self, activity):
        """Per survey window, the aoristic era sums the surveys would find in a world with
        settled `activity` [cell, century] (up to the scale of site finding)."""
        return {n: self.transfer[n] @ activity[self.footprint[n]].sum(axis=0) for n in self.windows}
