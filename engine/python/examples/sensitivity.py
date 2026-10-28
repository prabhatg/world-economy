#!/usr/bin/env python3
"""Sensitivity sweep with the Oikos Python bindings.

A 4x4 grid over growth_rate and fission_probability, placed at the centers of four
equal bins of each prior (via sample_params), three seeds per grid point, 500 years
on the M0 toy world (base cell 22, H3 resolution 4). Prints the final population's
mean and spread across seeds for each grid point: the spread is the run-to-run
variation a calibration has to see through.

The M0 mechanisms are toys, so the numbers say nothing about history; the script shows
the workflow. Worlds run on a thread pool: the bindings release the GIL while running.

    source scripts/env.sh
    maturin develop --release -m crates/oikos-py/Cargo.toml
    python python/examples/sensitivity.py
"""

import os
import time
from concurrent.futures import ThreadPoolExecutor

import numpy as np

import oikos

YEARS = 500
SEEDS = (1, 2, 3)
BINS = (0.125, 0.375, 0.625, 0.875)


def final_population(params, seed):
    world = oikos.World(22, 4, seed=seed, params=params)
    return world.run_years(YEARS, summary_every=YEARS)[-1]["population"]


def main():
    grid = [
        oikos.sample_params({"growth_rate": g, "fission_probability": f}) for g in BINS for f in BINS
    ]
    jobs = [(p, s) for p in grid for s in SEEDS]
    t0 = time.perf_counter()
    with ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
        finals = list(pool.map(lambda job: final_population(*job), jobs))
    secs = time.perf_counter() - t0
    pops = np.array(finals, dtype=float).reshape(len(BINS), len(BINS), len(SEEDS))

    fissions = [grid[j]["fission_probability"] for j in range(len(BINS))]
    print(f"Final population after {YEARS} years: mean ± sd over seeds {SEEDS}")
    print(f"{'growth_rate':>12} | " + " | ".join(f"fission {f:<10.4f}" for f in fissions))
    for i in range(len(BINS)):
        growth = grid[i * len(BINS)]["growth_rate"]
        cells = (f"{pops[i, j].mean():8.0f} ± {pops[i, j].std(ddof=1):<7.0f}" for j in range(len(BINS)))
        print(f"{growth:>12.5f} | " + " | ".join(cells))
    print(f"{len(jobs)} worlds in {secs:.2f} s ({oikos.ENGINE})")


if __name__ == "__main__":
    main()
