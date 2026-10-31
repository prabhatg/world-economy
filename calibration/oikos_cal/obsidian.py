"""Obsidian sourcing observation operator (docs/05 §5.1, #17; ADR-0061).

Chemical characterization assigns each obsidian artefact to its volcano, so a sourced
assemblage says what share of a site's obsidian came from the Cappadocian volcanoes
rather than the Eastern Anatolian ones (DObsiSS; `oikos_data.obsidian`). Renfrew, Dixon
and Cann (1966) found each source supplying its own side of the Fertile Crescent, with
the two mixing in between; where the boundary falls, and how sharp it is, depends on
how far obsidian travels down the line and over which settled land.

The model's counterpart is the share of Cappadocian obsidian among the obsidian people
held at the site (the cells within `rings` of it; 1: the site's cell and its six
neighbours) over the site's period, each century weighted by its overlap with the
period and by people (the record holds obsidian in use: people-years times
availability, `record.activity`). Where the model's people held none of either, the
share carries no information and is 0.5.

Assemblages vary more than binomial sampling (a few large contexts, mixtures of
phases, selective sampling for analysis), so counts are beta-binomial around the model
share with `concentration` φ, as for rachis (`domestication.beta_binomial`).

Obsidian passes only between neighbouring cells, so a context no chain of the world's
cells joins to a volcano (Cyprus, supplied by boat across the sea: Shillourokambos,
Khirokitia) can never be reached; it is dropped (`isolated` counts them), not scored at
0.5, and the targets judge the mainland only (`joined`).

Each context's relative distance `delta` is how much nearer (km, great circle) it is to
the nearest cell reaching a Cappadocian volcano than to one reaching an Eastern
Anatolian one: positive on the Cappadocian side.
"""

import h3
import numpy as np

from .domestication import beta_binomial
from .radiocarbon import Window

__all__ = ["Operator", "GROUPS", "joined"]

# The traced source groups: the share is of the first, against the second.
GROUPS = ("cappadocia", "eastern_anatolia")
LABELS = {"cappadocia": "Cappadocian", "eastern_anatolia": "eastern"}


def _distance(lat, lng, cells):
    """Great-circle km from (lat, lng) to the nearest of `cells` (H3 ids)."""
    return min(h3.great_circle_distance((lat, lng), h3.cell_to_latlng(h3.int_to_str(int(c))), "km") for c in cells)


def joined(cells, reach):
    """The `cells` (H3 ids) a chain of neighbouring cells among them joins to a cell of
    any group's `reach`, ascending."""
    world = {int(c) for c in cells}
    seen = {int(c) for g in reach.values() for c in g} & world
    frontier = list(seen)
    while frontier:
        nxt = []
        for c in frontier:
            for x in h3.grid_ring(h3.int_to_str(c), 1):
                n = h3.str_to_int(x)
                if n in world and n not in seen:
                    seen.add(n)
                    nxt.append(n)
        frontier = nxt
    return np.array(sorted(seen), dtype=np.uint64)


class Operator:
    def __init__(self, table, cells, window: Window, reach, rings=1, concentration=2.0, min_artefacts=1,
                 joined=None):
        """`table`: columns as `oikos_data.obsidian` (h3, lat, lng, start_year, end_year and
        a count per group); `cells`: H3 ids of the world's cells, ascending; `reach`: the
        H3 ids of the cells reaching each group's volcanoes (the obsidian pack). Contexts
        with fewer than `min_artefacts` of either group, or whose period misses the
        window, are dropped (`dropped` counts the latter), and so are those whose cell is not
        in `joined` (if given; `isolated` counts them)."""
        self.window, self.concentration = window, concentration
        cells = np.asarray(cells, dtype=np.uint64)
        k_all = np.asarray(table[GROUPS[0]], dtype=float)
        n_all = k_all + np.asarray(table[GROUPS[1]], dtype=float)
        starts, ends = (np.asarray(table[c], dtype=float) for c in ("start_year", "end_year"))
        lo = window.first + 100 * np.arange(window.centuries)
        self.k, self.n, self.delta, self.mid, self.near, self.weight = [], [], [], [], [], []
        self.dropped = self.isolated = 0
        linked = None if joined is None else {int(c) for c in joined}
        for i, h in enumerate(table["h3"]):
            if n_all[i] < min_artefacts:
                continue
            w = np.clip(np.minimum(ends[i], lo + 100) - np.maximum(starts[i], lo), 0, None) / 100
            if not w.any():
                self.dropped += 1
                continue
            if linked is not None and int(h) not in linked:
                self.isolated += 1
                continue
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(int(h)), rings)), dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            lat, lng = float(table["lat"][i]), float(table["lng"][i])
            self.k.append(k_all[i])
            self.n.append(n_all[i])
            self.delta.append(_distance(lat, lng, reach[GROUPS[1]]) - _distance(lat, lng, reach[GROUPS[0]]))
            self.mid.append((starts[i] + ends[i]) / 2)
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(w)
        self.k, self.n, self.delta, self.mid = (np.array(x) for x in (self.k, self.n, self.delta, self.mid))
        self.weight = np.array(self.weight)

    def shares(self, activity):
        """Model share of the first group's obsidian at each context, and whether the
        model's people there held any obsidian at all."""
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")
        a, b = (activity.obsidian[g] for g in GROUPS)
        first = np.array([(a[c].sum(axis=0) * w).sum() for c, w in zip(self.near, self.weight)])
        both = first + np.array([(b[c].sum(axis=0) * w).sum() for c, w in zip(self.near, self.weight)])
        reached = both > 0
        return np.where(reached, first / np.where(reached, both, 1), 0.5), reached

    def log_likelihood(self, activity):
        p, _ = self.shares(activity)
        p = p.clip(1e-6, 1 - 1e-6)
        return float(beta_binomial(self.k, self.n, p, self.concentration).sum())

    def label(self, group):
        return LABELS.get(group, group)
