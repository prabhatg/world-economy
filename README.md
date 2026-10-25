# World Economy — a simulator of human history as a policy laboratory

An evolving, agent-based simulation of humanity from **~12,000 BCE to the present**,
built on the accumulated knowledge of economics, history, archaeology, climate science,
ecology, demography, psychology, neuroscience and the social sciences.

The purpose is practical: **find paths that make life better for the majority of
humans**. History is the training data. A calibrated "digital twin of humanity" in
2026 is the starting point for exploring policy scenarios — economic, governance,
health/education and technology — and for finding strategies that are *robust* and
*politically achievable*, not just optimal on paper.

> Status: **design phase**. No engine code yet. Everything here is open for discussion.

## Design documents

| # | Document | Contents |
|---|----------|----------|
| 1 | [Vision & principles](docs/01-vision.md) | Goal, the "training a model" analogy and its limits, success criteria |
| 2 | [System architecture](docs/02-architecture.md) | Three run modes, layered system, component map |
| 3 | [Simulation engine](docs/03-engine.md) | Rust core, data layout, scheduling, determinism, distribution, checkpoint/fork |
| 4 | [Domain models](docs/04-domain-models.md) | Earth, ecology, agents, cognition, institutions, knowledge, power, disease |
| 5 | [Calibration & assimilation](docs/05-calibration-assimilation.md) | Fitting to history, observation operators, reanalysis of humanity |
| 6 | [Scenarios & policy](docs/06-scenarios-and-policy.md) | Policy DSL, welfare metrics, robust decision making, policy search |
| 7 | [AI integration](docs/07-ai-integration.md) | Knowledge ingestion, agent archetypes, emulators, differentiable sim |
| 8 | [Compute & scaling](docs/08-compute-and-scaling.md) | Cost model, cloud-first plan, path to supercomputers |
| 9 | [Data sources](docs/09-data-sources.md) | Forcing, calibration and validation datasets |
| 10 | [Roadmap](docs/10-roadmap.md) | Milestones, starting with the Neolithic Southwest Asia vertical slice |
| — | [Decision log](docs/adr/README.md) | Architecture decision records |
| — | [Open questions](docs/open-questions.md) | Unresolved questions to discuss next |

## Decisions so far

- **Purpose:** policy laboratory for human welfare; history is calibration data.
- **Run modes:** free-running mechanism science *and* data-assimilated reanalysis → 2026 twin → scenario forks.
- **Start:** ~12,000 BCE (end of last glacial, eve of agriculture).
- **Resolution:** adaptive by era (super-individual agents that split/merge), same engine throughout.
- **Welfare:** multi-dimensional — health & longevity, material wellbeing, capabilities & freedom, sustainability.
- **Policy levers:** economic/fiscal, governance, health & education, technology & AI.
- **Stack:** Rust core + GPU kernels, Python (PyO3) for orchestration/calibration/emulators, TypeScript + deck.gl/CesiumJS web viewer, Zarr + Parquet on object storage.
- **Resources:** solo + AI coding agents on cloud; designed to scale to HPC later.
- **Openness:** open source, open provenance.
- **First milestone:** Neolithic Southwest Asia, 12,000–3,000 BCE.
