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

Operators live in Python (and Rust for hot ones) and are shared by calibration and
assimilation.

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
