"""The M1 calibration problem (docs/05 §5.2, ADR-0052): Neolithic Southwest Asia worlds,
12,000 to 3,000 BCE, against the radiocarbon record (p3k14c, IntCal20), the barley
rachis record (Allaby et al. 2017), survey site counts in Central Anatolia and Upper
Mesopotamia (Palmisano et al. 2021; ADR-0059), sourced obsidian (DObsiSS; ADR-0061),
crops among charred plant remains (Wallace et al. 2018; ADR-0064) and the house-size
inequality of dated sites (Kohler et al. 2017; ADR-0074).

A run is a point u of the unit cube of the registry's free parameters (those with a
prior to sample; `oikos.sample_params` maps u onto the priors), a world built from the
M1 packs with those overrides, recorded every 10 years, and its summaries (`targets`)
plus diagnostics: each operator's log-likelihood and the population at the end. Runs
are independent, so they go to worker processes, each building the operators once.

Radiocarbon eras are 800 years and judged one by one and together (their bootstrap
covariance; ADR-0056). Discrepancy: 0.25 on each radiocarbon era's centered log share (dated activity off by
about a quarter in an era), 0.1 on each era's domestic share and 0.25 on each survey
era's centered log share - judgments, recorded with every wave so later waves can
revisit them. Survey eras are 500 years from 6,500 BCE, when pottery makes villages
visible to surface survey, and each survey is judged as a whole. Obsidian zones are
the mean Cappadocian share of contexts with at least three sourced artefacts, in four
zones of relative distance to the two source groups before and after 6,900 BCE (the end
of the PPNB), judged together; discrepancy 0.1 on each. Onset eras are the cultivation
effort the plant remains imply (crop share over the Pottery Neolithic anchor) in four
eras split at 10,000, 8,800 and 7,600 BCE, judged together; discrepancy 0.1 on each, as
on the domestic share (the anchor's uncertainty is in the bootstrap variance). House
Ginis are each site's, judged one by one with the sampling variance of its few houses;
discrepancy 0.05 on each (house size as a proxy of wealth, and its elasticity).

A round may leave out a record in `OPTIONAL` (`Problem(leave_out=...)`): its targets are
then neither judged nor written to the round's files. The house sizes of Kohler et al.
2017 come with no licence to republish them, so a round whose files are committed
leaves them out until that is settled.
"""

import io
import multiprocessing
from concurrent.futures import ProcessPoolExecutor

import numpy as np
import pyarrow.parquet as pq

from oikos_data import DATA, archaeobotany, pack, rachis, region
from oikos_data import inequality as inequality_pipeline
from oikos_data import obsidian as obsidian_pipeline
from oikos_data import radiocarbon as dates_pipeline
from oikos_data import sites as sites_pipeline

from . import domestication, inequality, obsidian, onset, radiocarbon, record, sites, targets

__all__ = ["PACKS", "END", "DISCREPANCY", "OPTIONAL", "packs", "Problem", "evaluate"]

PACKS = ("geometry", "climate", "coastline", "hydrology", "obsidian")
END = -3_000
DISCREPANCY = {"radiocarbon": 0.25, "domestication": 0.1, "sites": 0.25, "obsidian": 0.1, "onset": 0.1,
               "inequality": 0.05}
RADIOCARBON_ERAS = 10  # 800 years each: the peak and the fall after 5,900 BCE are resolved
SITE_EDGES = tuple(range(-6_500, END + 1, 500))
OBSIDIAN_EDGES = (-250.0, 0.0, 250.0)  # km nearer the Cappadocian than the eastern sources
OBSIDIAN_ERA = -6_900
OBSIDIAN_MIN_ARTEFACTS = 3
ONSET_EDGES = (-10_000, -8_800, -7_600)  # Late Epipalaeolithic, PPNA, Early-Middle PPNB, Late PPNB
ONSET_MIN_ITEMS = 50
OPTIONAL = ("inequality",)  # records a round may leave out; the diagnostics need the others
DIAGNOSTICS = ["log-likelihood radiocarbon", "log-likelihood domestication", "log-likelihood obsidian",
               "people 3000 BCE"]


def packs():
    paths = [DATA / "build" / f"m1-swasia.{k}.oikospack" for k in PACKS]
    missing = [p.name for p in paths if not p.exists()]
    if missing:
        raise FileNotFoundError(f"M1 packs not built: {missing}; run python -m oikos_data build <pack> m1-swasia")
    return [str(p) for p in paths]


def habitable(cells):
    """The `cells` with dry land in some century (land minus lake fraction, as the
    engine; ADR-0043), from the built coastline and hydrology packs."""
    read = {k: pack.read((DATA / "build" / f"m1-swasia.{k}.oikospack").read_bytes()) for k in ("coastline", "hydrology")}
    dry = read["coastline"].values("land_fraction").max(axis=0) - read["hydrology"].values("lake_fraction")
    return np.asarray(cells)[dry > 0]


class Problem:
    """Operators, targets and the simulator of M1; build once per process."""

    def __init__(self, leave_out=()):
        unknown = sorted(set(leave_out) - set(OPTIONAL))
        if unknown:
            raise ValueError(f"cannot leave out {unknown}: only {list(OPTIONAL)} can be")
        r = region.load("m1-swasia")
        self.cells = r.cells()
        sources = dates_pipeline.inputs(offline=True)
        dates = pq.read_table(io.BytesIO(dates_pipeline.build(r, *sources)[0])).to_pydict()
        barley = pq.read_table(io.BytesIO(rachis.build(r, *rachis.inputs(offline=True)))).to_pydict()
        surveys = pq.read_table(io.BytesIO(sites_pipeline.build(r, *sites_pipeline.inputs(offline=True)))).to_pydict()
        contexts = pq.read_table(io.BytesIO(obsidian_pipeline.build_obsidian_table(r, offline=True))).to_pydict()
        sources_pack = pack.read(obsidian_pipeline.build_obsidian_pack(r, offline=True))
        reach = {g: self.cells[sources_pack.raw(g) > 0] for g in obsidian.GROUPS}
        self.radiocarbon = radiocarbon.Operator(dates, sources[2], self.cells)
        if self.radiocarbon.settings.true.end != END:
            raise ValueError(f"the radiocarbon window ends at {self.radiocarbon.settings.true.end}, runs at {END}")
        self.domestication = domestication.Operator(barley, self.cells, self.radiocarbon.settings.true)
        self.sites = sites.Operator(surveys, self.radiocarbon.settings.true, SITE_EDGES)
        self.obsidian = obsidian.Operator(contexts, self.cells, self.radiocarbon.settings.true, reach,
                                          min_artefacts=OBSIDIAN_MIN_ARTEFACTS,
                                          joined=obsidian.joined(habitable(self.cells), reach))
        phases = pq.read_table(io.BytesIO(archaeobotany.build_crop_table(r))).to_pydict()
        self.onset = onset.Operator(phases, self.cells, self.radiocarbon.settings.true, min_items=ONSET_MIN_ITEMS)
        self.inequality = None
        if "inequality" not in leave_out:
            houses = pq.read_table(io.BytesIO(inequality_pipeline.build_table(r, offline=True))).to_pydict()
            self.inequality = inequality.Operator(houses, self.cells, self.radiocarbon.settings.true)
        self.eras = [targets.RadiocarbonEras(self.radiocarbon, eras=RADIOCARBON_ERAS,
                                             discrepancy_sd=DISCREPANCY["radiocarbon"], rng=np.random.default_rng(0)),
                     targets.DomesticationEras(self.domestication, barley,
                                               discrepancy_sd=DISCREPANCY["domestication"]),
                     targets.SiteEras(self.sites, discrepancy_sd=DISCREPANCY["sites"], rng=np.random.default_rng(0)),
                     targets.ObsidianZones(self.obsidian, edges=OBSIDIAN_EDGES, era=OBSIDIAN_ERA,
                                           discrepancy_sd=DISCREPANCY["obsidian"], rng=np.random.default_rng(0)),
                     targets.OnsetEras(self.onset, edges=ONSET_EDGES, discrepancy_sd=DISCREPANCY["onset"],
                                       rng=np.random.default_rng(0))]
        if self.inequality is not None:
            self.eras.append(targets.HouseGinis(self.inequality, discrepancy_sd=DISCREPANCY["inequality"]))
        self.targets = [t for e in self.eras for t in e.targets]
        first = np.cumsum([0] + [len(e.targets) for e in self.eras])
        self.groups = [self.eras[0].group(0), *self.eras[2].groups(first[2]), self.eras[3].group(first[3]),
                       self.eras[4].group(first[4])]
        self.names = [t.name for t in self.targets] + DIAGNOSTICS

    def outputs(self, activity):
        """Summaries then diagnostics of a recorded world, in `names` order."""
        summaries = np.concatenate([e.model(activity) for e in self.eras])
        return np.concatenate([summaries, [self.radiocarbon.log_likelihood(activity.visible(0)),
                                           self.domestication.log_likelihood(activity),
                                           self.obsidian.log_likelihood(activity),
                                           activity.people[:, -1].sum() / 100]])

    def simulate(self, params, seed):
        """Outputs of the world with parameter overrides `params` (values, not u)."""
        import oikos

        w = oikos.World.from_packs(packs(), seed=seed, params=params)
        return self.outputs(record.activity(w, end=END, parts=1))


_problem = None


def _start(leave_out):
    global _problem
    _problem = Problem(leave_out)


def _run(job):
    params, seed = job
    return _problem.simulate(params, seed)


def evaluate(jobs, workers, leave_out=()):
    """Outputs [job, name] of (params, seed) jobs, run in `workers` processes, judged
    without the records `leave_out`."""
    ctx = multiprocessing.get_context("spawn")  # never fork a process whose rayon pool is running
    with ProcessPoolExecutor(workers, mp_context=ctx, initializer=_start, initargs=(tuple(leave_out),)) as ex:
        return np.array(list(ex.map(_run, jobs)))
