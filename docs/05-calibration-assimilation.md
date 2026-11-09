# 5. Calibration & data assimilation

This is the "training" half of the project and deserves as much engineering as the
engine.

## 5.1 Observation operators

The simulator never compares raw state to data. An **observation operator** `H` maps
state to what an instrument/record actually measures, including its biases:

| Data | Operator |
|---|---|
| Radiocarbon date densities (p3k14c) | sample datable activity from settlements × taphonomic loss × research-intensity bias |
| Archaeological site counts/sizes | settlement size distribution × survey detection probability |
| Ancient DNA ancestry | ancestry-tracer proportions in sampled burials |
| Seshat variables | polity-level complexity indices computed the same way Seshat codes them |
| Historical population (HYDE, censuses) | regional sums with documented error |
| Prices, wages (Allen, Clio-Infra) | market prices of comparable baskets |
| GDP, health, governance (Maddison, GBD, V-Dem, WDI) | national aggregates of agent states |

**Implemented (M1 data):** the radiocarbon observations are two locked Parquet tables
built by `data/oikos_data/radiocarbon.py` (ADR-0046): every p3k14c date in the region
(lab id, 14C age and error, material, taxa, site, source, location accuracy, cell,
IntCal20-calibrated median year, share of probability in 12,000–3,000 BCE), and the
calibrated probability summed per cell and century for dates located within 500 m or
exactly.

**Implemented (radiocarbon density operator, ADR-0050):** `calibration/oikos_cal`
(`radiocarbon.Operator`) scores a world's recorded activity (`record.activity`:
people-years per cell and century) against the dates table. It forward-models
taphonomic loss (Surovell et al. 2009), activity out of sight (on land the sea has since
drowned, and on river-fed lowland plains before they aggraded, 7,500-5,500 BCE, under
their silt; ADR-0095),
and measurement plus calibration (a kernel from IntCal20 per error class), bins dates by site and 200-year phase, and compares the
distribution of binned calibrated mass over centuries within coarse H3 blocks (a
multinomial given each block's total), so research intensity per block cancels.
`synthetic_dates` draws dates from a known history to validate it. At the placeholder
defaults the M1 world scores -4,028 against -4,026 for a flat history and -4,007 for
0.01%/year growth: the record rises about fifteenfold from 11,500 to 6,000 BCE and
then falls, the model about threefold and on (open question 25).

**Implemented (domestication-timing operator, ADR-0051):** `domestication.Operator`
scores the domestic share of the crop sown near each dated barley assemblage of Allaby
et al. 2017 (`data/oikos_data/rachis.py`) against its rachis counts, beta-binomially.
In M1 it separates the competing domestication hypotheses: protracted selection scores
-92.8, rapid selection -157.5. One operator judges each crop (ADR-0101): barley against
the crop's share, emmer and einkorn against the shares of the wheats sown with it.
An assemblage holds wild ears gathered nearby as well as sown ones (ADR-0103): each
people-year of foraging effort adds 0.03 times the ears of one sown, all wild.

**Implemented (site-count operator, ADR-0059):** `sites.Operator` counts the survey site
phases of Palmisano et al. 2021 (`data/oikos_data/sites.py`) in each era aoristically,
and dates a world's settled activity (cultivator people-years) in each survey's
landscape the same way: each century goes to the periods surveyors used that contain
it, by their popularity, and spreads over them. On synthetic surveys drawn from a known
history it recovers the era shares to 0.01. The surveys rise into the Late Chalcolithic
where radiocarbon falls, so they rule out the region-wide bust of the round-2 defaults.
Since villages form (ADR-0070, ADR-0071), sites are compared with settlements, not
settled people: each group counted as far as it is settled (ADR-0072), and since
settlements may hold several groups (ADR-0082), each settlement counted once, as far as
its groups are settled, people-weighted, and sized by all its people.

**Implemented (village-size operator, ADR-0078, #34):** `villages.Operator` takes the
areas the same surveys record (Upper Mesopotamia per phase, Central Anatolia mostly once
per site) and summarizes each era by the mean log area of its sized site phases, each
counting aoristically. A world's settlements go through the surveys' dating: their
settlement-years times the log of their people, over their settlement-years. Areas
become people at 121 per hectare, with a log-scale error of 0.45 that every era of both
surveys shares (50 to 294 as a 95% range, a judgment between mounds not lived in all at
once, as at Çatalhöyük, Kuijt and Marciniak 2024, and a North Yemeni tell village,
van Beek 1982), so the record constrains the absolute size of villages loosely and
their change through time and between the surveys more tightly. Settlements are dated
as the sized phases are. The fourteen eras are judged together (discrepancy 0.25 on
each, which also covers density differing between eras). At the calibrated defaults
settlements hold about 50 people, against the record's 126-218 at the middle density;
round 10's wave 4 finds consistent worlds with villages of up to 180 people, where
groups seldom disperse and often join, but none of them sits nearer the whole record
than the defaults, and few grow their villages through time as Upper Mesopotamia's do.
The defaults of ADR-0081 hold about 67 people, level through time too.

**Implemented (radiocarbon subregions, ADR-0079, #35):** the radiocarbon operator's
blocks are grouped into four subregions (the southern Levant; Anatolia; the northern
Levant, Cyprus and Upper Mesopotamia; the Zagros and Iran), boxes of block centers, and
each is summarized by the years by which 10, 25 and 50% of its dated activity had
accrued from 12,000 BCE. Years rather than log shares, because early eras can be empty.
The bootstrap resamples whole sites within each subregion, so a site's many dates move
together; research intensity still cancels per block. Judged together, discrepancy 300
years on each. The region-wide eras cannot see one subregion early and another late;
these do. Northeast Africa is the fifth, judged since ADR-0095 (#44): the Western Desert
is settled during the African Humid Period (ADR-0093), and the Delta's early foragers,
whose record begins about 5,500 BCE, are out of sight, on land drowned since or under the
delta's silt. At the defaults its 10% point is about 700 years early (implausibility
about 2), its 25% and 50% points near the record.

**Implemented (onset contrasts, ADR-0080, #36):** the onset phases are split by the
engine's own wild-cereal suitability at them (the same ramps of precipitation and
temperature, climate and parameters; rich at 0.95 or more), and the record's contrast
is the effort the rich phases imply minus the poor phases', before and after 7,600 BCE,
bootstrapped over phases and anchors. Success bias makes people copy the cultivators
who do best, who did best in the richest stands; adoption copies anyone. The record's
contrast is 0.21 before and 0.62 after 7,600 BCE.

**Implemented (obsidian sourcing operator, ADR-0061):** `obsidian.Operator` scores the
share of Cappadocian obsidian in use around each sourced assemblage of DObsiSS
(`data/oikos_data/obsidian.py`) over its period, against the obsidian the world's
people held there (the down-the-line tracer, ADR-0060), beta-binomially (φ = 2). The
targets are mean shares in four zones of relative distance to the two source groups,
before and after 6,900 BCE. Cypriot assemblages are dropped: their obsidian crossed the
sea by boat, which a chain over land cannot do.

**Implemented (cultivation-onset operator, ADR-0064):** `onset.Operator` compares the
share of crops (domesticates and their wild progenitors) among the charred grains and
seeds of dated site phases (Wallace et al. 2018, `data/oikos_data/archaeobotany.py`)
with the cultivation effort a world's people put in there and then. Counts are not
diets, so the share of a farming world is anchored on the record's Pottery Neolithic
phases, and an era's mean share over that anchor is the effort the record implies; both
simplifications make the test lenient. Phases vary too much at the same level of
farming to score one by one, so only four era means are targets (split at 10,000, 8,800
and 7,600 BCE). Before 10,000 BCE the record implies an effort of 0.09.

**Implemented (juvenile-index operator, ADR-0068):** `juvenile.Operator` reads the
juvenile index of a cemetery (15p5, the share of 5-19 year olds among the dead over 5;
Bocquet-Appel 2002) from the fertility and local growth of a world's people there and
then, through a stable population with the average hunter-gatherer Siler mortality of
Gurven and Kaplan (2007), scaled to that birth rate. It sees the Neolithic demographic
transition, which the stock records cannot (ADR-0067). The Levant cemetery record is to
come (#33).

**Implemented (sheep size operator, ADR-0088, #37):** `herds.Operator` compares the mean
log size index of measured sheep bones in six eras (Arbuckle et al. 2014) with a mixture
of wild and domestic sizes by the domestic share of the caprine bones the people around
each assemblage discarded: from their herds by herding effort, and from wild sheep and
goats hunted on the range at a tenth of that per unit of foraging effort. The wild and
domestic sizes are judgments from the record's earliest and latest assemblages, their
uncertainty shared by the eras. It sees when herds became domestic, not how many there were.

**Implemented (infection-lesions operator, ADR-0086, #42):** `lesions.Operator` compares
the lesions of infection on 200 Natufian and 205 Neolithic skeletons of the southern
Levant (2 and 9 affected; Eshed et al. 2010) with the prevalence of the endemic
infection (ADR-0075) around the sampled sites in each period, weighted by their
skeletons (`World::infected`). A skeleton shows a lesion for an unknown share of the
infected (the osteological paradox), so the summary is the Neolithic share of the
lesioned, in which that share cancels: it sees how much more common infection became
with farming, not how common it was.

Operators live in Python (and Rust for hot ones) and are shared by calibration and
assimilation.

## 5.1a Parameter registry (implemented)

`engine/params/*.toml` declares every parameter once: group, unit, valid range, default,
prior (`fixed`, `uniform`, `log_uniform`, `normal`), status (`placeholder`, `literature`,
`calibrated`) and sources. The engine embeds it, builds its parameters from it and
rejects out-of-range overrides in work units and checkpoints. `oikos params` exports
it as JSON for calibration tools, which sample priors from the same definitions.
`Registry::sample` maps unit-cube coordinates onto each prior (inverse CDF of uniform
and log-uniform priors, and of normal priors truncated to the parameter's range;
integers rounded) with the engine's reproducible math (`oikos_math::normal_quantile`:
Acklam's approximation refined by a Halley step on the pinned `libm`'s `erfc`), so a
sampler's design points give the same overrides on every host.

## 5.1b Python bindings (implemented)

Calibration code drives the engine in-process through the `oikos` Python module
(`engine/crates/oikos-py`, PyO3, ADR-0037) instead of shelling out to the CLI:
`World(base_cell, resolution, seed=, params=)` with `step`, `run_years`, `summary`,
`state_hash`, `population()` and `agents()` (NumPy arrays), `checkpoint()` /
`World.from_checkpoint`, and `fork(seed, params)` for branching ensembles (the
resampling step of §5.3); plus `run_work_unit` (same result as `oikos run`),
`registry()` and `sample_params(u)` (unit cube → prior, via `Registry::sample`);
`World.from_packs(paths, seed=)` runs on a forcing-pack region such as M1.
The bindings contain no simulation logic and are tested to reproduce the golden hash,
so an ensemble run from Python is the same computation a volunteer would validate.
Long runs release the GIL, so a thread pool runs ensemble members in parallel.
`engine/python/examples/sensitivity.py` is a minimal parameter sweep.

## 5.2 Calibration (Mode A — free runs)

Goal: posterior over mechanism parameters θ (and over *which* competing modules fit).

1. **Priors** from literature (parameter registry, with citations).
2. **Screening:** global sensitivity analysis (Morris/Sobol) to find the parameters
   that matter; freeze the rest at prior means.
3. **History matching** (as in climate model tuning): waves of ensembles + Gaussian-
   process or neural emulators rule out implausible regions of θ-space using an
   implausibility measure with model-discrepancy terms (Kennedy & O'Hagan).
4. **Simulation-based inference:** neural posterior estimation (`sbi` library) on the
   non-implausible region, using summary statistics from observation operators.
5. **Model comparison:** compare competing mechanism modules by posterior predictive
   fit and held-out skill.
6. **Pattern-oriented validation:** stylized facts that were *not* used in fitting.

**Implemented (first waves, ADR-0052):** `oikos_cal.screening` (Morris),
`oikos_cal.emulator` (Gaussian processes), `oikos_cal.history` (implausibility, waves,
NROY sampling) and `oikos_cal.targets` (radiocarbon and domestication eras), run on M1
by `python -m oikos_cal screen / wave` with every run recorded in `calibration/runs/m1/`.
Screening found eight of 22 free parameters that move the summaries; two waves of 200
worlds leave 22% of the prior space, with selection the one sharply constrained
parameter (0.001-0.01 per year). Radiocarbon eras do not constrain at their present
tolerances (open question 25); late domestication is the binding summary (question 24).
Round 2 (ADR-0056) judges ten 800-year radiocarbon eras one by one and together
(multivariate implausibility with their bootstrap covariance) over 27 free parameters,
including the nested demographic, climate and land-degradation hypotheses: 4.7% of the
prior space survives two waves, the radiocarbon record binds, and the worlds that fit
both records best erode their soils at 1.1-1.6e-4 per year. The best of them is the
registry default (ADR-0057): the 12 emulated parameters are `calibrated`, the others keep
their defaults, and a test keeps the default world consistent with the record.
Round 3 (ADR-0058) refocuses a third wave on 17 factors: 3.6% of the prior space
survives, and the record leaves sedentary growth and the soil's critical share free.
Round 4 (ADR-0059) adds the site counts as two groups of seven 500-year eras: 2.3% of the
prior space survives, 91 of 300 worlds fit all three records, and the best of them
(wave 4, run 251) is the registry default, with all 17 emulated parameters
`calibrated`. It fits by having almost no bust; soils that recover separate the
consistent worlds from the rest.
Round 5 (ADR-0061) adds the obsidian zones and `obsidian_loss`: 0.8% of the prior space
survives and 70 of 300 worlds fit all four records. The best of them ties the defaults
(-4,280.2 against -4,282.9 on the same seeds) as a different world, so the defaults
stay; a profile at the defaults calibrates `obsidian_loss` (0.0028), which the tracer
lets vary without changing the world. The viewer's ensemble of these worlds (ADR-0063) shows that
all of them cultivate far too early: no record judged so far dates the onset of
cultivation (open question 30, #29).
Round 6 (ADR-0064) adds the cultivation-onset eras, which rule out the defaults
(implausibility 4.4 before 10,000 BCE): 0.23% of the prior space survives, 55 of 300
worlds fit all five records, and the best of them (wave 6, run 236) is the registry
default, with 18 parameters `calibrated`. It passes by planting about a third
throughout; no world rises from little planting to much as the record does (open
question 31, #30).
Round 7 (ADR-0065) adds social learning of cultivation: bands learn mostly from the
people around them, so planting takes off late and then fast. Three more waves (the
third with the adoption rate emulated) leave 0.13% of the prior space and 154 of 300
worlds consistent; one of them plants as the record does (0.09, 0.57, 0.75, 0.54 against
0.09, 0.52, 0.64, 0.62). The best-fitting consistent world (wave 9, run 208) learns
socially and becomes the registry default, but plants 0.40 before 10,000 BCE: the
log-likelihood diagnostics, which cover radiocarbon, rachis and obsidian, prefer early
planting (open question 32).
Round 8 (ADR-0066) ranks consistent worlds by how near their summaries of all five
records sit to the record under history matching's own error model (observation
covariance plus discrepancy), not by the item log-likelihoods, which allow for no
discrepancy. Under it the late planters lead: the records do not pull apart. A tenth
wave leaves 0.10% of the prior space, and the nearest consistent world (wave 9, run
180) plants 0.20, 0.55, 0.61 and 0.48 and becomes the registry default. At it, the
records do not identify the Neolithic demographic transition's growth rates (ADR-0067):
populations follow carrying capacity, and a profile of sedentary growth from 0.1 to 4
times the forager rate is flat.
House-size inequality (ADR-0074) is a sixth record: the Gini coefficient of house floor
areas at five dated sites of the region (Kohler et al. 2017), from Jerf el Ahmar (PPNA)
to Tell Brak (Late Chalcolithic). A world's settled people around a site give the
variance of log wealth; house area is taken to grow as wealth to the power 0.7 (a
judgment from the ratio of house-size to wealth Ginis in the two compilations), and the
summary is the small-sample Gini the site's two to nineteen measured houses would show,
with its sampling variance as the observation variance and a discrepancy of 0.05.
Round 9 (ADR-0073) starts a new history after villages and settlements changed the model
and the site summaries (ADR-0070 to ADR-0072): a new screening of all 33 free
parameters and three waves over the prior, 24 factors emulated. Worlds whose summaries
are undefined (a surveyed landscape without settlements for a whole era) are ruled out
by their own outputs and left out of the emulators. The waves leave 1.35% of the prior
and keep the calibrated defaults, and none of their best worlds sits nearer the record,
so the defaults stay. Village size, aggregation, fertility and sedentary growth are not
constrained by any of the five records (open question 33).
Round 10 (ADR-0077) starts another after inequality, disease and social learning
(ADR-0074 to ADR-0076): a screening of all 43 free parameters and three waves of 300
over the prior, 24 factors emulated, with the house-size record left out (its values
come with no licence to publish them in the round's files). The waves leave 0.70% of
the prior; no candidate sits nearer the record than the calibrated defaults, which stay.
Success bias, disease's critical group size, the adoption rate and the innovation share
spread over their priors among the consistent worlds: the records see their joint
effect, the pace and timing of cultivation, not each one. A fourth wave judges the
village sizes (ADR-0078) and the house-size Ginis too, once the maintainer decided the
round's files may carry them (#43): 0.44% of the prior is left and the defaults stay.
A fifth wave judges the radiocarbon subregions (ADR-0079; 0.22% left) and a sixth every
record, the onset contrasts too (ADR-0080; 0.16% left, 18 of 300 consistent). The
defaults fail the subregions, so `best` over wave 6 gives new ones (ADR-0081): wave 6
run 216, with fertility and the wealth parameters kept at their literature values,
consistent with every seed (largest implausibility 2.70). Success bias still spans its
prior among the consistent worlds.
Round 11 (ADR-0090) starts another after the lesions, floodplain farming, the African
Humid Period and herding (ADR-0084 to ADR-0088), judged on every record: a screening of
the 52 free parameters and six waves of 300, 24 factors emulated, leave 0.04% of the
prior and 45 consistent worlds, none nearer the record than the calibrated defaults,
which stay. The herding parameters, which only the sheep-size record sees, are then
profiled at the defaults: herds turn domestic too slowly below herd_selection 0.02, and
0.035 fits best; the effort aimed at herds is consistent only from 0.1 to 0.2 of the
land's herding potential, and with that selection 0.12, the old value, fits best.
Profiles of herd wealth's transmission and shock (ADR-0091) at those defaults are flat:
herding is too small a share of M1's effort, and its dated houses too much farmers', for
the record to see how unequal herders were, so the literature values stay.
Round 13 (ADR-0104), after dates out of sight and northeast Africa's judging (ADR-0095),
screens the 57 free parameters and runs six waves of 300 (24 factors emulated): they leave
0.03% of the prior and 5 consistent worlds, all from the last two waves, as the regional
arrival dates bind. The one that stays consistent at every seed with the other parameters
at their defaults scores the summaries worse than the calibrated defaults (-94.5 against
-87.5), which stay.
Round 14 (ADR-0105) judges the wheats, seed exchange and wild gathering: its six waves leave
under 0.005% of the prior and one consistent world, which its rerun no longer finds
consistent, so the defaults stay again. A profile of seed exchange at the defaults is
consistent everywhere and rises to a plateau from 0.3 (summaries -82.9 off, -81.1 at 0.3),
mostly through the domestication record, so seed exchange is calibrated at 0.3.

Differentiable subsets (via JAX surrogates or smooth relaxations; cf. GradABM) can
accelerate step 4 where gradients are meaningful.

## 5.3 Assimilation (Mode B — reanalysis of humanity)

Goal: one ensemble of trajectories consistent with *both* the model's dynamics and the
historical record, producing the 2026 digital twin.

- **Particle-filter backbone:** an ensemble of N runs; at each observation time, weight
  members by likelihood under `H`, resample (fork the good ones, drop the bad).
  Natural fit for an ABM because forks are cheap (doc 3.7) and states are discrete.
- **Localized updates:** observations are regional; weight/resample per region
  block (localized particle filters) to avoid ensemble collapse over a whole planet.
- **Nudging for aggregates:** gentle, mechanism-respecting corrections toward observed
  aggregates (e.g. adjusting migration, fertility or mortality hazards rather than
  teleporting people), with the nudging magnitude logged as a model-error diagnostic.
  Large persistent nudges = a mechanism is wrong; feed back into Mode A.
- **Observation density grows with time:** sparse archaeology → written records →
  censuses → national accounts → modern micro-data (surveys, satellite night-lights,
  mobile/finance data). The ensemble tightens as data densifies.
- **Final synthesis (2026):** initialize individual-level modern populations from
  census microdata (IPUMS), household surveys and synthetic-population methods,
  *consistent with* the assimilated aggregate history (institutions, wealth,
  networks, beliefs carry over).

## 5.4 Uncertainty bookkeeping

Every output carries: parameter uncertainty (posterior), structural uncertainty
(module variants), stochastic uncertainty (seeds), data uncertainty (observation error),
and assimilation correction magnitude. Reports always show spread, never a single line.
