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
taphonomic loss (Surovell et al. 2009) and measurement plus calibration (a kernel from
IntCal20 per error class), bins dates by site and 200-year phase, and compares the
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
-92.8, rapid selection -157.5. Site counts and obsidian distance-decay are next (#17).

Operators live in Python (and Rust for hot ones) and are shared by calibration and
assimilation.

## 5.1a Parameter registry (implemented)

`engine/params/*.toml` declares every parameter once: group, unit, valid range, default,
prior (`fixed`, `uniform`, `log_uniform`, `normal`), status (`placeholder`, `literature`,
`calibrated`) and sources. The engine embeds it, builds its parameters from it and
rejects out-of-range overrides in work units and checkpoints. `oikos params` exports
it as JSON for calibration tools, which sample priors from the same definitions.
`Registry::sample` maps unit-cube coordinates onto each prior (inverse CDF of uniform
and log-uniform priors, integers rounded; normal priors to follow) with the engine's
reproducible math, so a sampler's design points give the same overrides on every host.

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
