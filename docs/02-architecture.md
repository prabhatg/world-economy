# 2. System architecture

## Three run modes, one engine

```
           ┌─────────────────────────────────────────────────────────────┐
 Mode A    │ FREE RUN  (mechanism science)                               │
           │ 12,000 BCE ──► ... ──► "a plausible 2026"   × N ensembles   │
           │ Question: do mechanisms reproduce statistical history?       │
           └─────────────────────────────────────────────────────────────┘
           ┌─────────────────────────────────────────────────────────────┐
 Mode B    │ REANALYSIS (data assimilation)                              │
           │ 12,000 BCE ─► obs ─► obs ─► obs ─► ... ─► 2026 DIGITAL TWIN  │
           │ Ensemble nudged/resampled toward the historical record;      │
           │ observation density rises from archaeology → census → micro. │
           └─────────────────────────────────────────────────────────────┘
           ┌─────────────────────────────────────────────────────────────┐
 Mode C    │ SCENARIOS (policy laboratory)                               │
           │ 2026 twin ─fork─► baseline                                  │
           │            ├fork─► policy set P1   × M ensembles            │
           │            └fork─► policy set P2 ...  (or hindcast forks    │
           │                                       from earlier states)  │
           └─────────────────────────────────────────────────────────────┘
```

Same binary, same mechanisms, same state format. Modes differ only in the run driver
(free / assimilating / forking) — this keeps the twin honest: the dynamics that produce
2026 are exactly the dynamics used to evaluate policy.

## Layers

```
┌────────────────────────────────────────────────────────────────────────┐
│ Experience      Web viewer (TS, deck.gl/CesiumJS), notebooks, reports  │
├────────────────────────────────────────────────────────────────────────┤
│ Experiments     Run orchestration, ensembles, forks, policy search,    │
│ (Python)        calibration (SBI, history matching), assimilation       │
├────────────────────────────────────────────────────────────────────────┤
│ AI services     Literature→mechanism/parameter extraction, archetype   │
│                 generation, neural emulators, differentiable surrogates │
├────────────────────────────────────────────────────────────────────────┤
│ Mechanisms      Earth · Ecology · Demography · Cognition · Economy ·    │
│ (Rust plugins)  Institutions · Power · Knowledge/Culture · Disease ·    │
│                 Conflict · Migration · Policy hooks                     │
├────────────────────────────────────────────────────────────────────────┤
│ Engine core     Spatial grid · SoA entity store · relations/networks ·  │
│ (Rust)          multi-rate scheduler · deterministic RNG · distribution │
│                 · checkpoint/fork · in-situ reductions · I/O            │
├────────────────────────────────────────────────────────────────────────┤
│ Data            Forcing (climate, sea level, soils), observations,      │
│                 parameter registry with priors & provenance             │
│                 Zarr (fields) · Parquet/Arrow (agents, events) · GCS    │
└────────────────────────────────────────────────────────────────────────┘
```

## Component map (initial repository layout, to be created in M0)

```
engine/                     Rust workspace
  crates/core               time, ids, units, deterministic RNG, errors
  crates/grid               geodesic hex grid (H3), neighbor/halo, regridding
  crates/store              SoA component storage, super-individual agents, relations
  crates/sched              mechanism DAG, multi-rate phases, two-phase commit
  crates/dist               partitioning, halo exchange, agent migration (MPI later)
  crates/io                 Zarr/Parquet writers, checkpoint/restore, content addressing
  crates/params             parameter registry, priors, provenance metadata
  crates/mech-*             mechanism modules (earth, ecology, demography, ...)
  crates/sim                run drivers: free, assimilate, fork
  crates/py                 PyO3 bindings
python/worldsim/            orchestration, calibration, assimilation, emulators, analysis
data/                       dataset manifests (not data), ingestion pipelines
viewer/                     TypeScript web viewer
docs/                       this design
```

## Key interfaces

- **Mechanism trait** — declares components it reads/writes, its rate (tick period),
  its parameters (with priors + citations) and its outputs. The scheduler derives a DAG
  and rejects write conflicts at build time.
- **Observation operator** — maps simulator state to what a dataset actually measures
  (e.g. radiocarbon date densities, census counts, GDP per capita). Used by both
  calibration and assimilation.
- **Policy** — a time-scheduled set of parameter/rule overrides and new institutions,
  applied at fork points (see doc 6).
- **Metric** — reduction producing welfare and diagnostic indicators, in-situ.
