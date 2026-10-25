# ManyWorlds — a simulator of human history as a policy laboratory

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
| 11 | [Volunteer computing](docs/11-volunteer-computing.md) | SETI@home-style volunteer network: work units, coordinator, validation, clients, governance |
| 12 | [Identity & credit](docs/12-identity-and-credit.md) | Proposed project name; credit for every kind of contributor |
| 13 | [Native clients](docs/13-native-clients.md) | Rust daemon + sandboxed engine workers + thin Flutter shell; mobile, signing, risks |
| 14 | [Engineering practices](docs/14-engineering-practices.md) | Mandatory TDD, local verification gate, performance method, targets |
| — | [Design system](design/DESIGN-SYSTEM.md) | Research-grounded principles, tokens, diagrams, Material 3 / Apple mapping |
| — | [Project website](docs/index.html) | Public explainer (GitHub Pages from `/docs`) |
| — | [Decision log](docs/adr/README.md) | Architecture decision records |
| — | [Open questions](docs/open-questions.md) | Unresolved questions to discuss next |
| — | [Credits](CREDITS.md) | Everyone who contributes |

## Decisions so far

- **Purpose:** policy laboratory for human welfare; history is calibration data.
- **Run modes:** free-running mechanism science *and* data-assimilated reanalysis → 2026 twin → scenario forks.
- **Start:** ~12,000 BCE (end of last glacial, eve of agriculture).
- **Resolution:** adaptive by era (super-individual agents that split/merge), same engine throughout.
- **Welfare:** multi-dimensional — health & longevity, material wellbeing, capabilities & freedom, sustainability.
- **Policy levers:** economic/fiscal, governance, health & education, technology & AI.
- **Stack:** Rust core + GPU kernels, Python (PyO3) for orchestration/calibration/emulators, TypeScript + deck.gl/CesiumJS web viewer, Zarr + Parquet on object storage.
- **Resources:** solo + AI coding agents on cloud; designed to scale to HPC later.
- **Volunteer computing:** SETI@home-style network for ensemble workloads, with our own
  cloud-native coordinator; browser, desktop, mobile and headless clients; all
  volunteer types (compute, explorers, citizen scientists, experts, developers,
  institutions).
- **Numerics:** bitwise cross-platform determinism; integer conserved quantities +
  restricted IEEE `f64` with our own math library.
- **Openness:** nonprofit-style, open-source project with open provenance.
- **Credit:** every contributor type is credited; each result ships a provenance-generated credit manifest.
- **Name:** *ManyWorlds* for the project and volunteer network, *Oikos* for the engine (pending trademark check).
- **Licenses:** Apache-2.0 for code ([LICENSE](LICENSE)); CC-BY 4.0 for docs and results.
- **GPU:** wgpu first, CUDA later.
- **Native clients:** thin Flutter shell over a Rust daemon (also the headless client); embedded web globe; Tauri v2 as fallback.
- **Engineering:** test-driven development is mandatory; verification runs on our own compute (no hosted CI yet); performance measured and budgeted.
- **Design:** one design system for web, apps and diagrams ([design/](design/DESIGN-SYSTEM.md)); every user-facing change is reviewed by the `design-reviewer` agent (`.claude/agents/`).
- **Engine:** M0 under [`engine/`](engine/README.md).
- **First milestone:** Neolithic Southwest Asia, 12,000–3,000 BCE.
