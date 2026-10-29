"""The M1 calibration problem (docs/05 §5.2, ADR-0052): Neolithic Southwest Asia worlds,
12,000 to 3,000 BCE, against the radiocarbon record (p3k14c, IntCal20) and the barley
rachis record (Allaby et al. 2017).

A run is a point u of the unit cube of the registry's free parameters (those with a
prior to sample; `oikos.sample_params` maps u onto the priors), a world built from the
M1 packs with those overrides, recorded every 10 years, and its summaries (`targets`)
plus diagnostics: each operator's log-likelihood and the population at the end. Runs
are independent, so they go to worker processes, each building the operators once.

Radiocarbon eras are 800 years and judged one by one and together (their bootstrap
covariance; ADR-0056). Discrepancy: 0.25 on each radiocarbon era's centered log share (dated activity off by
about a quarter in an era) and 0.1 on each era's domestic share - judgments, recorded
with every wave so later waves can revisit them.
"""

import io
import multiprocessing
from concurrent.futures import ProcessPoolExecutor

import numpy as np
import pyarrow.parquet as pq

from oikos_data import DATA, rachis, region
from oikos_data import radiocarbon as dates_pipeline

from . import domestication, radiocarbon, record, targets

__all__ = ["PACKS", "END", "DISCREPANCY", "packs", "Problem", "evaluate"]

PACKS = ("geometry", "climate", "coastline", "hydrology")
END = -3_000
DISCREPANCY = {"radiocarbon": 0.25, "domestication": 0.1}
RADIOCARBON_ERAS = 10  # 800 years each: the peak and the fall after 5,900 BCE are resolved
DIAGNOSTICS = ["log-likelihood radiocarbon", "log-likelihood domestication", "people 3000 BCE"]


def packs():
    paths = [DATA / "build" / f"m1-swasia.{k}.oikospack" for k in PACKS]
    missing = [p.name for p in paths if not p.exists()]
    if missing:
        raise FileNotFoundError(f"M1 packs not built: {missing}; run python -m oikos_data build <pack> m1-swasia")
    return [str(p) for p in paths]


class Problem:
    """Operators, targets and the simulator of M1; build once per process."""

    def __init__(self):
        r = region.load("m1-swasia")
        self.cells = r.cells()
        sources = dates_pipeline.inputs(offline=True)
        dates = pq.read_table(io.BytesIO(dates_pipeline.build(r, *sources)[0])).to_pydict()
        barley = pq.read_table(io.BytesIO(rachis.build(r, *rachis.inputs(offline=True)))).to_pydict()
        self.radiocarbon = radiocarbon.Operator(dates, sources[2], self.cells)
        if self.radiocarbon.settings.true.end != END:
            raise ValueError(f"the radiocarbon window ends at {self.radiocarbon.settings.true.end}, runs at {END}")
        self.domestication = domestication.Operator(barley, self.cells, self.radiocarbon.settings.true)
        self.eras = [targets.RadiocarbonEras(self.radiocarbon, eras=RADIOCARBON_ERAS,
                                             discrepancy_sd=DISCREPANCY["radiocarbon"], rng=np.random.default_rng(0)),
                     targets.DomesticationEras(self.domestication, barley,
                                               discrepancy_sd=DISCREPANCY["domestication"])]
        self.targets = [t for e in self.eras for t in e.targets]
        self.groups = [self.eras[0].group(0)]
        self.names = [t.name for t in self.targets] + DIAGNOSTICS

    def outputs(self, activity):
        """Summaries then diagnostics of a recorded world, in `names` order."""
        summaries = np.concatenate([e.model(activity) for e in self.eras])
        return np.concatenate([summaries, [self.radiocarbon.log_likelihood(activity.visible(0)),
                                           self.domestication.log_likelihood(activity),
                                           activity.people[:, -1].sum() / 100]])

    def simulate(self, params, seed):
        """Outputs of the world with parameter overrides `params` (values, not u)."""
        import oikos

        w = oikos.World.from_packs(packs(), seed=seed, params=params)
        return self.outputs(record.activity(w, end=END, parts=1))


_problem = None


def _start():
    global _problem
    _problem = Problem()


def _run(job):
    params, seed = job
    return _problem.simulate(params, seed)


def evaluate(jobs, workers):
    """Outputs [job, name] of (params, seed) jobs, run in `workers` processes."""
    ctx = multiprocessing.get_context("spawn")  # never fork a process whose rayon pool is running
    with ProcessPoolExecutor(workers, mp_context=ctx, initializer=_start) as ex:
        return np.array(list(ex.map(_run, jobs)))
