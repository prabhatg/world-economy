"""Activity recorded from a running world: what observation operators score.

People-years per cell and century, sampled every few years (a left Riemann sum of each
cell's population); the same weighted by each cell's cultivation effort (so an operator
can make cultivators more visible without re-running the world) and by effort times the
domestic share of the crop (so domestic / cultivators is the domestication of the crop
sown in any set of cells and centuries); and, per obsidian source group, people-years
times the availability of its obsidian (obsidian in use, what sourced assemblages
sample; empty without an obsidian pack). Recording only reads the world through the
bindings; it never changes its history.
"""

from dataclasses import dataclass, field

import numpy as np

from .radiocarbon import Window


@dataclass
class Activity:
    window: Window
    people: np.ndarray  # people-years [cell, century]
    cultivators: np.ndarray  # people-years times cultivation effort [cell, century]
    domestic: np.ndarray  # people-years times effort times domestic share of the crop [cell, century]
    obsidian: dict = field(default_factory=dict)  # source group -> people-years times availability [cell, century]

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
    people, cultivators, domestic = (np.zeros((world.cells, window.centuries)) for _ in range(3))
    obsidian = {name: np.zeros((world.cells, window.centuries)) for name in world.obsidian()}
    while world.year < end:
        c = (world.year - first) // 100
        n = min(every, 100 - (world.year - first) % 100)
        pop = world.population().astype(float)
        people[:, c] += n * pop
        sown = n * pop * world.cultivation()
        cultivators[:, c] += sown
        domestic[:, c] += sown * world.domestication()
        for name, held in world.obsidian().items():
            obsidian[name][:, c] += n * pop * held
        world.run_years(n, parts=parts)
    return Activity(window, people, cultivators, domestic, obsidian)
