# 6. Scenarios & policy laboratory

## 6.1 Welfare objective (multi-dimensional, never collapsed silently)

| Dimension | Example indicators |
|---|---|
| Health & longevity | HALE, child mortality, undernutrition, disease burden |
| Material wellbeing | consumption per capita, poverty headcount (multiple lines), Gini/Palma, food & housing security |
| Capabilities & freedom | schooling and learning, agency, political freedom, safety from violence, gender gaps |
| Sustainability | emissions, resource depletion, biodiversity/land integrity, welfare of future cohorts |

- Distributional by default: "majority of humans" means we report medians, bottom-40%
  outcomes and inequality, not averages dominated by the rich.
- Users can supply weightings, but results are primarily shown as **Pareto fronts**,
  so different value systems see their own trade-offs.

## 6.2 Policy DSL

A scenario = fork point + timeline of interventions + target population + adoption
assumptions. Sketch:

```yaml
scenario: ucb-plus-anticorruption
fork_from: twin-2026@v1
horizon_years: 50
interventions:
  - at: 2027-01
    where: { region: "sub-saharan-africa" }
    lever: fiscal.unconditional_cash_transfer
    params: { amount_ppp_per_month: 30, targeting: universal_rural }
    financing: { source: [aid, progressive_tax], leakage_model: governance.default }
  - at: 2027-01
    lever: governance.audit_and_transparency
    params: { coverage: 0.6 }
adoption: endogenous      # polities decide via their political-economy mechanisms
# adoption: imposed       # counterfactual "if it were simply implemented"
metrics: [welfare.all]
ensemble: { members: 200, seeds: per_member, include_structural_variants: true }
```

Levers available in v1 scope: **economic/fiscal** (transfers, UBI, tax, land reform,
trade), **governance** (anti-corruption, state capacity, rule of law, conflict
prevention), **health & education** (vaccination, nutrition, sanitation, schooling,
teacher quality), **technology & AI** (AI/automation diffusion, energy transition,
connectivity, infrastructure).

**Two adoption modes** are essential: `imposed` answers "would it help?";
`endogenous` answers "can it happen, and how?" The gap between them is the political
economy problem the project exists to solve.

## 6.3 Decision making under deep uncertainty

- Evaluate each strategy across the full ensemble of parameter posteriors, structural
  variants and future exogenous scenarios (climate, technology, shocks).
- Robust decision making (RDM): find strategies with acceptable regret across worlds;
  scenario discovery (PRIM-style) to find *where* a strategy fails.
- Adaptive strategies (signposts and triggers) are first-class: "if X happens, do Y".

## 6.4 Policy search

- Search space: sequences of levers × parameters × target regions × timing.
- Optimizers: Bayesian optimization and evolutionary search over **emulators** trained
  on scenario ensembles; confirm finalists with the full simulator. RL over emulators
  for sequential/adaptive strategies.
- Search for **adoption pathways**: coalitions, compensation schemes, sequencing and
  institutional designs that make beneficial policies endogenously adopted and stable.

## 6.5 Hindcast validation of the policy lab

Before trusting forward scenarios, replay known interventions from earlier assimilated
states and check direction/magnitude: Green Revolution, smallpox eradication,
Progresa/Oportunidades CCTs, China's 1978 reforms, East Asian education pushes,
structural adjustment programs, Marshall Plan, M-Pesa. Failures here are the most
informative results the project can produce.

## 6.6 Ethics & communication

- Results are decision support with explicit uncertainty, not prescriptions.
- Publish assumptions, priors and failure cases alongside every result.
- Avoid targeting or profiling of real individuals; the 2026 twin uses synthetic
  populations statistically consistent with data, not real persons.
