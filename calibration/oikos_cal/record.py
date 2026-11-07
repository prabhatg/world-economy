"""Activity recorded from a running world: what observation operators score.

People-years per cell and century, sampled every few years (a left Riemann sum of each
cell's population); the same weighted by each cell's cultivation effort (so an operator
can make cultivators more visible without re-running the world) and by effort times the
domestic share of the crop (so domestic / cultivators is the domestication of the crop
sown in any set of cells and centuries); and, per obsidian source group, people-years
times the availability of its obsidian (obsidian in use, what sourced assemblages
sample; empty without an obsidian pack); and settlement-years, each group counted as
far as it is settled (its cultivation effort), which is what surveys find (ADR-0072);
and settled people-years times the variance of log household wealth among the settled
(ADR-0074), so dividing by settled people-years gives their mean inequality anywhere;
and settlement-years times the log of each settlement's people, so dividing by
settlement-years gives the mean log size of the settlements anywhere (#34); and infected
people-years, so dividing by people-years gives the prevalence of the endemic infection
anywhere (#42); and herders, their domestic animals and the foragers hunting where wild
sheep and goats range, in people-years times the effort spent on each, the bones a faunal
assemblage samples (#37).
Recording only reads the world through the bindings; it never changes its history.
The engine writes the same record as a work-unit output (`activity`, #46), bit for bit,
so a calibration world can run wherever the `oikos` CLI runs; `read` loads it.
"""

from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from .radiocarbon import Window


@dataclass
class Activity:
    window: Window
    people: np.ndarray  # people-years [cell, century]
    cultivators: np.ndarray  # people-years times cultivation effort [cell, century]
    domestic: np.ndarray  # people-years times effort times domestic share of the crop [cell, century]
    obsidian: dict = field(default_factory=dict)  # source group -> people-years times availability [cell, century]
    settlements: np.ndarray = None  # groups times their cultivation effort, times years [cell, century]
    inequality: np.ndarray = None  # settled people-years times their variance of log wealth [cell, century]
    village_size: np.ndarray = None  # settlement-years times the log of their people [cell, century]
    infected: np.ndarray = None  # infected people-years [cell, century]
    herders: np.ndarray = None  # people-years times herding effort [cell, century]
    herd_domestic: np.ndarray = None  # the same times the herd's domestic share [cell, century]
    hunters: np.ndarray = None  # people-years times foraging effort times wild herds' range [cell, century]

    def visible(self, cultivator_visibility):
        """People-years with each cultivator counting 1 + `cultivator_visibility` times."""
        return self.people + cultivator_visibility * self.cultivators


def activity(world, end, every=10, parts=4) -> Activity:
    """Run `world` from its current year to calendar year `end`, both on century
    boundaries, sampling every `every` years."""
    first = world.year
    if first % 100 or end % 100 or end <= first:
        raise ValueError(f"record from {first} to {end}: both must be century boundaries, end after start")
    window = Window(first, end)
    people, cultivators, domestic, settlements, inequality, village_size, infected, herders, herd_domestic, hunters = (
        np.zeros((world.cells, window.centuries)) for _ in range(10))
    obsidian = {name: np.zeros((world.cells, window.centuries)) for name in world.obsidian()}
    while world.year < end:
        c = (world.year - first) // 100
        n = min(every, 100 - (world.year - first) % 100)
        pop = world.population().astype(float)
        people[:, c] += n * pop
        sown = n * pop * world.cultivation()
        cultivators[:, c] += sown
        domestic[:, c] += sown * world.domestication()
        inequality[:, c] += sown * world.inequality()
        for name, held in world.obsidian().items():
            obsidian[name][:, c] += n * pop * held
        settlements[:, c] += n * world.settlements()
        village_size[:, c] += n * world.village_sizes()
        infected[:, c] += n * world.infected()
        herding, cultivation = world.herding(), world.cultivation()
        herded = n * pop * herding
        herders[:, c] += herded
        herd_domestic[:, c] += herded * world.herd_domestication()
        hunters[:, c] += n * pop * np.clip(1.0 - cultivation - herding, 0.0, None) * world.caprine_range()
        world.run_years(n, parts=parts)
    return Activity(window, people, cultivators, domestic, obsidian, settlements, inequality, village_size, infected,
                    herders, herd_domestic, hunters)



FIELDS = ("people", "cultivators", "domestic", "settlements", "inequality", "village_size", "infected", "herders",
          "herd_domestic", "hunters")


def read(store, zones=None) -> Activity:
    """The `activity.zarr` record a work unit wrote (#46): arrays [century, cell] on disk,
    [cell, century] here, as `activity` gives them. A reduced record (#47), summed over
    the zones of a zone file, needs those `zones.Zones` and comes back `zones.Zoned`."""
    import zarr

    g = zarr.open_group(str(Path(store)), mode="r")
    a = g.attrs
    window = Window(int(a["first_year"]), int(a["end_year"]))
    wrap = lambda x: np.ascontiguousarray(x.T)
    if "zones" in a:
        from .zones import Zoned

        if zones is None or zones.id != a["zones"]:
            raise ValueError(f"{store} is summed over zones {a['zones']}: read it with those zones")
        if zones.window != window:
            raise ValueError(f"{store} covers {window}, its zones {zones.window}")
        wrap = lambda x: Zoned(zones, np.ascontiguousarray(x.T))
    elif zones is not None:
        raise ValueError(f"{store} is a full record, not summed over zones")
    arrays = {name: wrap(g[name][:]) for name in FIELDS}
    obsidian = {s: wrap(g[f"obsidian/{s}"][:]) for s in a["obsidian_sources"]}
    return Activity(window, obsidian=obsidian, **arrays)
