"""The M1 calibration problem (docs/05 §5.2, ADR-0052): Neolithic Southwest Asia worlds,
12,000 to 3,000 BCE, against the radiocarbon record (p3k14c, IntCal20), the barley
rachis record (Allaby et al. 2017), survey site counts in Central Anatolia and Upper
Mesopotamia (Palmisano et al. 2021; ADR-0059), sourced obsidian (DObsiSS; ADR-0061),
crops among charred plant remains (Wallace et al. 2018; ADR-0064), the house-size
inequality of dated sites (Kohler et al. 2017; ADR-0074) and the areas of the survey
sites (Palmisano et al. 2021; #34).

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
Village sizes are each survey era's mean log site area in people, at 121 people per
hectare with a log-scale error of 0.45 that all of them share: 50 to 294 as a 95% range,
a judgment. The top is the higher of the densities Kuijt (2000) took from ethnographic
villages (90 in Iran, Kramer 1982; 294 in North Yemen, van Beek 1982); the bottom allows
for mounds not lived in all at once, as Kuijt and Marciniak (2024) find at Çatalhöyük
(600-800 people in an average year of its Middle phase, tens of people per hectare of
the mound, the figure depending on the area taken). Judged together, discrepancy 0.25
on each, which also covers density differing between eras and surveys.

Radiocarbon subregions (#35) are the years by which 10, 25 and 50% of each subregion's
dated activity had accrued from 12,000 BCE, with sites resampled whole; the region-wide
eras cannot see one subregion early and another late. Judged together, discrepancy 300
years on each (three century bins), a judgment. The
onset contrasts (#36) split the onset phases by the engine's wild-cereal suitability
(at least `RICH_STANDS` in the rich stands) and compare the cultivation effort the two
imply before and after 7,600 BCE; judged together, discrepancy 0.1 on each, as on the
onset eras. Success bias makes people copy the cultivators who do best, and they did
best where the wild stands were richest; adoption copies anyone, wherever they live.

The house-size Ginis are judged since the maintainer's decision of 2026-11-03 (#43);
round 10's first waves left them out (ADR-0077).

A round may leave out a record in `OPTIONAL` (`Problem(leave_out=...)`): its targets are
then neither judged nor written to the round's files, for a record whose values may not
be published with them.
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

from . import domestication, inequality, obsidian, onset, radiocarbon, record, sites, targets, villages

__all__ = ["PACKS", "END", "DISCREPANCY", "OPTIONAL", "packs", "Problem", "evaluate"]

PACKS = ("geometry", "climate", "coastline", "hydrology", "obsidian", "humid_period")
END = -3_000
DISCREPANCY = {"radiocarbon": 0.25, "domestication": 0.1, "sites": 0.25, "obsidian": 0.1, "onset": 0.1,
               "villages": 0.25, "subregions": 300.0, "contrasts": 0.1, "inequality": 0.05}
VILLAGE_DENSITY = 121.0  # people per hectare: geometric mean of 50 and 294
VILLAGE_DENSITY_SD = 0.45  # log scale: 50-294 as a 95% range
RADIOCARBON_ERAS = 10  # 800 years each: the peak and the fall after 5,900 BCE are resolved
# Radiocarbon subregions: boxes of block centers (lat from, to, lng from, to), a block
# going to the first that holds it; blocks of none (the Caucasus, Arabia, the Gulf,
# eastern Iran, the Mesopotamian alluvium's few dates) count only region-wide.
SUBREGIONS = {"southern Levant": (29.0, 34.5, 34.0, 39.0),
              "Anatolia": (36.5, 42.5, 26.0, 36.0),
              "northern Levant, Cyprus and Upper Mesopotamia": (34.5, 39.0, 34.0, 45.0),
              "Zagros and Iran": (30.0, 38.5, 45.0, 54.0)}
# Recorded, not judged: the subregion's dates come from the Western Desert while the
# African Humid Period watered it and from the Nile valley once it was farmed, from about
# 5,000 BCE, with almost none before 9,000 BCE (Kuper and Kropelin 2006). The forcing now
# has the Humid Period (ADR-0084) and floodplains can be farmed (ADR-0085), but farming
# does not reach the Nile and it holds as many foragers from 12,000 BCE as later, so no
# world can match the dates yet; #44.
UNJUDGED_SUBREGIONS = {"northeast Africa": (22.0, 32.0, 24.0, 34.0)}
SITE_EDGES = tuple(range(-6_500, END + 1, 500))
OBSIDIAN_EDGES = (-250.0, 0.0, 250.0)  # km nearer the Cappadocian than the eastern sources
OBSIDIAN_ERA = -6_900
OBSIDIAN_MIN_ARTEFACTS = 3
ONSET_EDGES = (-10_000, -8_800, -7_600)  # Late Epipalaeolithic, PPNA, Early-Middle PPNB, Late PPNB
ONSET_MIN_ITEMS = 50
RICH_STANDS = 0.95  # wild-cereal suitability of a phase's cells and centuries: nearly full
CONTRAST_MIN_PHASES = 5
OPTIONAL = ("inequality",)  # records a round may leave out; the diagnostics need the others
DIAGNOSTICS = ["log-likelihood radiocarbon", "log-likelihood domestication", "log-likelihood obsidian",
               "people 3000 BCE"] + [f"radiocarbon {n} {q}% by, not judged" for n in UNJUDGED_SUBREGIONS
                                     for q in (10, 25, 50)]


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


def stands(op):
    """The wild-cereal suitability of each onset phase (`onset.suitability`) at the
    registry's cereal parameters, which must be fixed: the record is split by them."""
    import oikos

    reg = oikos.registry()["params"]
    free = [k for k in onset.CEREAL if reg[k]["prior"]["dist"] != "fixed"]
    if free:
        raise ValueError(f"{free} have priors: the onset contrasts split the record by them, so they must be fixed")
    climate = pack.read((DATA / "build" / "m1-swasia.climate.oikospack").read_bytes())
    return onset.suitability(op, climate.values("temp_mean"), climate.values("precip"),
                             climate.header["time"]["calendar_year"], {k: reg[k]["default"] for k in onset.CEREAL})


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
        self.villages = villages.Operator(surveys, self.radiocarbon.settings.true, SITE_EDGES)
        self.obsidian = obsidian.Operator(contexts, self.cells, self.radiocarbon.settings.true, reach,
                                          min_artefacts=OBSIDIAN_MIN_ARTEFACTS,
                                          joined=obsidian.joined(habitable(self.cells), reach))
        phases = pq.read_table(io.BytesIO(archaeobotany.build_crop_table(r))).to_pydict()
        self.onset = onset.Operator(phases, self.cells, self.radiocarbon.settings.true, min_items=ONSET_MIN_ITEMS)
        self.stands = stands(self.onset)
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
                                       rng=np.random.default_rng(0)),
                     targets.VillageSizes(self.villages, density=VILLAGE_DENSITY, density_sd=VILLAGE_DENSITY_SD,
                                          discrepancy_sd=DISCREPANCY["villages"], rng=np.random.default_rng(0)),
                     targets.RadiocarbonSubregions(self.radiocarbon,
                                                   radiocarbon.subregions(self.radiocarbon.obs.blocks, SUBREGIONS),
                                                   discrepancy_sd=DISCREPANCY["subregions"],
                                                   rng=np.random.default_rng(0)),
                     targets.OnsetContrasts(self.onset, self.stands >= RICH_STANDS, edges=ONSET_EDGES[-1:],
                                            discrepancy_sd=DISCREPANCY["contrasts"], rng=np.random.default_rng(0),
                                            min_phases=CONTRAST_MIN_PHASES)]
        self.subregions = self.eras[6]
        self.unjudged = targets.RadiocarbonSubregions(
            self.radiocarbon, radiocarbon.subregions(self.radiocarbon.obs.blocks, UNJUDGED_SUBREGIONS),
            discrepancy_sd=DISCREPANCY["subregions"], rng=np.random.default_rng(0))
        if self.inequality is not None:
            self.eras.append(targets.HouseGinis(self.inequality, discrepancy_sd=DISCREPANCY["inequality"]))
        self.targets = [t for e in self.eras for t in e.targets]
        first = np.cumsum([0] + [len(e.targets) for e in self.eras])
        self.groups = [self.eras[0].group(0), *self.eras[2].groups(first[2]), self.eras[3].group(first[3]),
                       self.eras[4].group(first[4]), self.eras[5].group(first[5]), self.eras[6].group(first[6]),
                       self.eras[7].group(first[7])]
        self.names = [t.name for t in self.targets] + DIAGNOSTICS

    def outputs(self, activity):
        """Summaries then diagnostics of a recorded world, in `names` order."""
        summaries = np.concatenate([e.model(activity) for e in self.eras])
        return np.concatenate([summaries, [self.radiocarbon.log_likelihood(activity.visible(0)),
                                           self.domestication.log_likelihood(activity),
                                           self.obsidian.log_likelihood(activity),
                                           activity.people[:, -1].sum() / 100], self.unjudged.model(activity)])

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
