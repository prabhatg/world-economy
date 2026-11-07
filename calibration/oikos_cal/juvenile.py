"""Juvenile-index observation operator (docs/05 §5.1, #33; ADR-0068).

Skeletons of a cemetery are its dead, and their ages tell the population's fertility:
the share of 5-19 year olds among the dead over 5, the juvenile index 15p5, rises with
the birth rate and hardly depends on mortality (Bocquet-Appel 2002). Children under 5
are left out because their small bones are often missing from cemeteries. The index
rises a few centuries after farming arrives, the skeletal signature of the Neolithic
demographic transition, which the M1 stock records cannot see (ADR-0067).

A world's bands have fertility (crude birth rate, moving with cultivation effort from
`fertility` to `sedentary_fertility`) and grow at a local rate, but no ages. Their dead
are read through a stable population (Lotka): deaths at age x in proportion to
exp(-r x) l(x) h(x), where l and h are the survivorship and hazard of a Siler mortality
schedule at the level that makes the stable birth rate the world's. The schedule's shape
is the average hunter-gatherer one of Gurven and Kaplan (2007, Table 2); its level is a
multiplier on the hazard (proportional hazards).

At a cemetery, a world's birth rate is its people-weighted fertility over the cells
within `rings` of the site and the centuries of the cemetery's use, each weighted by
its overlap (cultivator people-years over people-years give the effort, and fertility
is linear in it). Its growth rate is the trend of log people-years over those
centuries, up to the bands' intrinsic growth rate at that effort either way: people
arriving or leaving faster than that are migrants, whose ages a stable population does
not describe. Rates are per year, from the registry's per-period ones. Cemeteries nobody
reached in a world are left out of its scores.
"""

from dataclasses import dataclass

import h3
import numpy as np
from scipy.optimize import brentq

from .radiocarbon import Window

__all__ = ["Siler", "HUNTER_GATHERER", "life_expectancy", "stable", "Operator"]

AGES = np.linspace(0.0, 300.0, 6001)  # every 0.05 years; the Gompertz term ends every level of the schedule
TICKS_PER_YEAR = 365


@dataclass(frozen=True)
class Siler:
    """Siler competing hazards: h(x) = a1 exp(-b1 x) + a2 + a3 exp(b3 x) (Siler 1979)."""

    a1: float
    b1: float
    a2: float
    a3: float
    b3: float

    def hazard(self, x):
        return self.a1 * np.exp(-self.b1 * x) + self.a2 + self.a3 * np.exp(self.b3 * x)

    def log_survival(self, x):
        return -self.a1 / self.b1 * (1 - np.exp(-self.b1 * x)) - self.a2 * x + self.a3 / self.b3 * (1 - np.exp(self.b3 * x))

    def survival(self, x):
        return np.exp(self.log_survival(x))


# Average of five hunter-gatherer populations (Gurven and Kaplan 2007, Table 2;
# doi:10.1111/j.1728-4457.2007.00171.x): e0 about 31 years, 57% surviving to 15.
HUNTER_GATHERER = Siler(0.422, 1.131, 0.013, 1.47e-4, 0.086)


def _integral(y):
    return np.trapezoid(y, AGES)


def life_expectancy(siler=HUNTER_GATHERER):
    return float(_integral(siler.survival(AGES)))


def stable(b, r, siler=HUNTER_GATHERER):
    """The hazard level and juvenile index (15p5) of the stable population with birth
    rate `b` and growth rate `r` per year whose mortality is `siler` times the level."""
    log_l, h = siler.log_survival(AGES), siler.hazard(AGES)

    def births(level):  # stable birth rate: 1 / integral of exp(-r x) l(x)^level
        return 1.0 / _integral(np.exp(level * log_l - r * AGES))

    lo, hi = 1e-6, 1e3
    if not births(lo) < b < births(hi):
        raise ValueError(f"no mortality level gives birth rate {b} at growth {r}: needs {births(lo):.4f} to "
                         f"{births(hi):.4f}")
    level = brentq(lambda k: births(k) - b, lo, hi, xtol=1e-12, rtol=1e-12)
    deaths = np.exp(level * log_l - r * AGES) * level * h
    over5, upto20 = AGES >= 5, (AGES >= 5) & (AGES <= 20)
    return level, float(np.trapezoid(deaths[upto20], AGES[upto20]) / np.trapezoid(deaths[over5], AGES[over5]))


class Operator:
    def __init__(self, table, cells, window: Window, rings=1):
        """`table`: columns cemetery, h3, start_year and end_year; `cells`: H3 ids of the
        world's cells, ascending."""
        self.window = window
        cells = np.asarray(cells, dtype=np.uint64)
        lo = window.first + 100 * np.arange(window.centuries)
        self.names, self.near, self.weight = [], [], []
        for i in range(len(table["cemetery"])):
            start, end = float(table["start_year"][i]), float(table["end_year"][i])
            w = np.clip(np.minimum(end, lo + 100) - np.maximum(start, lo), 0, None) / 100
            if not w.any():
                continue
            disk = np.array(sorted(h3.str_to_int(x) for x in h3.grid_disk(h3.int_to_str(int(table["h3"][i])), rings)),
                            dtype=np.uint64)
            pos = np.searchsorted(cells, disk).clip(max=len(cells) - 1)
            self.names.append(table["cemetery"][i])
            self.near.append(pos[cells[pos] == disk])
            self.weight.append(w)

    def cells(self):
        """The cell sets it reads (`zones`, #47)."""
        return list(self.near)

    def _check(self, activity):
        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the operator {self.window}")

    def _efforts(self, activity):
        self._check(activity)
        out = []
        for c, w in zip(self.near, self.weight):
            people = (activity.people[c].sum(axis=0) * w).sum()
            out.append((activity.cultivators[c].sum(axis=0) * w).sum() / people if people > 0 else 0.0)
        return np.array(out)

    def birth_rates(self, activity, params):
        """People-weighted crude birth rate per year at each cemetery, from the registry
        parameters `params` (forager rate where nobody lived)."""
        per_year = TICKS_PER_YEAR / params["period_ticks"]
        f, fs = params["fertility"], params["sedentary_fertility"]
        return (f + (fs - f) * self._efforts(activity)) * per_year

    def growth_rates(self, activity):
        """Trend of log people-years per year over each cemetery's centuries (0 with
        fewer than two centuries lived in)."""
        self._check(activity)
        years = self.window.midpoints()
        out = []
        for c, w in zip(self.near, self.weight):
            people = activity.people[c].sum(axis=0)
            used = (w > 0) & (people > 0)
            out.append(np.polyfit(years[used], np.log(people[used]), 1, w=np.sqrt(w[used]))[0]
                       if used.sum() >= 2 else 0.0)
        return np.array(out)

    def indices(self, activity, params, siler=HUNTER_GATHERER):
        """The juvenile index a world with registry parameters `params` puts at each
        cemetery, and which cemeteries anyone reached (the others are 0)."""
        b = self.birth_rates(activity, params)
        g, gs = params["growth_rate"], params["sedentary_growth_rate"]
        cap = (g + (gs - g) * self._efforts(activity)) * TICKS_PER_YEAR / params["period_ticks"]
        r = np.clip(self.growth_rates(activity), -cap, cap)
        reached = np.array([(activity.people[c].sum(axis=0) * w).sum() > 0 for c, w in zip(self.near, self.weight)])
        return np.array([stable(bi, ri, siler)[1] if ok else 0.0 for bi, ri, ok in zip(b, r, reached)]), reached
