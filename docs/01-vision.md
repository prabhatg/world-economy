# 1. Vision & principles

## Goal

Build a continuously evolving simulation of humanity from ~12,000 BCE to today, then use
it as a laboratory to search for interventions that make life better for the majority of
people — and for the *pathways* by which such interventions get adopted and sustained.

Wealthy individuals and governments largely already know many things that would help
(vaccines, nutrition, schooling, cash transfers). What usually fails is incentives,
elite capture, coordination, state capacity, and conflict. The simulator is therefore
only useful if **power and political economy are first-class mechanisms**, not an
afterthought. Its most valuable output is not "policy X raises welfare" but "this
sequence of coalitions, incentives and institutions gets X adopted and keeps it in
place, across many plausible worlds."

## The "training a giant model" analogy

| Large AI model | This simulator |
|---|---|
| Architecture / inductive bias | Mechanisms drawn from the research literature |
| Parameters | Mechanism parameters + behavioral policies shared across agent archetypes |
| Training data | The historical record (archaeology, demography, prices, wars, aDNA, climate…) |
| Loss | Mismatch between simulated and observed indicators via observation operators |
| Training | Calibration: simulation-based inference, history matching, differentiable ABM |
| Inference | Scenario ensembles forked from the 2026 twin |
| Generalization | Do interventions that work in silico work in reality? |

**Where the analogy breaks:** we have *one* human history with sparse, noisy
observations. A trillion free parameters would fit history perfectly and learn nothing.
Hence:

1. **Structure carries the knowledge.** Literature enters as constrained mechanisms;
   free parameters are few, have priors, and must be identifiable.
2. **Cross-sectional data is the real sample size.** Thousands of regions, polities,
   cities and households evolving semi-independently.
3. **Held-out validation is mandatory.** e.g. calibrate on 3000 BCE–1500 CE, predict
   1500–1900; calibrate on Afro-Eurasia, predict the Americas.

## Principles

1. **Ensembles over single runs.** History is chaotic; only distributions mean anything.
2. **Emergence over scripting.** Agriculture, trade, money, states, inequality and
   industrialization must arise from mechanisms. Known events enter only via data
   assimilation, never as hard-coded triggers.
3. **Pluralism of theory.** Competing theories (e.g. causes of state formation, growth
   theories) are swappable mechanism modules, judged by fit to data.
4. **Provenance everywhere.** Every parameter, prior and dataset cites its source;
   every run is reproducible bit-for-bit from (code version, config, seed).
5. **Robustness over optimality.** Recommend strategies that perform well across model
   variants and uncertainty, à la decision making under deep uncertainty.
6. **Adaptive resolution.** Spend compute where data and decisions are dense
   (modern era), stay coarse where they are sparse (deep past).
7. **Open by default.** Open code, data lineage and results — a closed policy oracle
   would not and should not be trusted.
8. **Guard against known failure modes.** Lucas critique (agents adapt to policy),
   Goodhart's law (multi-dimensional objectives), overconfidence (World3, pre-2008 DSGE).

## Success criteria (staged)

1. **Mechanism validity:** free runs reproduce *statistical* patterns — timing and
   location distribution of domestication, settlement-size distributions, state-size
   distributions, Malthusian dynamics, Seshat social-complexity trajectories.
2. **Out-of-sample skill:** held-out regions/periods predicted better than baselines.
3. **Reanalysis quality:** assimilated runs match observed population, urbanization,
   GDP, health and governance indicators through 2026 with calibrated uncertainty.
4. **Hindcast policy skill:** historical interventions (e.g. Green Revolution,
   conditional cash transfers, vaccination campaigns, trade liberalizations) are
   reproduced in direction and rough magnitude when replayed from earlier states.
5. **Useful forward exploration:** robust, adoptable strategies with explicit
   uncertainty and trade-offs across the four welfare dimensions.
