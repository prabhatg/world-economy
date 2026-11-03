# Documentation map

Start at [AGENTS.md](../AGENTS.md) for the rules of working in this repository; this
page lists every document under it, from purpose to detail. A lower level refines the
one above it. When two disagree, the more specific one wins, and the drift is fixed in
the same change.

## 1. Purpose and plan

- [01 Vision and principles](01-vision.md): why the project exists and what success means.
- [10 Roadmap](10-roadmap.md): milestones M0-M5 and the status of each. Every work item
  is a GitHub issue under its
  [milestone](https://github.com/prabhatg/world-economy/milestones).
- [15 Model fidelity plan](15-model-fidelity.md): what the model needs next to describe
  the past more faithfully, ordered, one issue per item (label `fidelity`).

## 2. Design, by layer

| Layer | Document | Operational detail |
|---|---|---|
| System | [02 Architecture](02-architecture.md) | – |
| Engine | [03 Simulation engine](03-engine.md) | [engine/README.md](../engine/README.md) |
| Mechanisms | [04 Domain models](04-domain-models.md) | [engine/params/m0-toy.toml](../engine/params/m0-toy.toml) (every parameter, with sources) |
| Calibration | [05 Calibration and assimilation](05-calibration-assimilation.md) | [calibration/README.md](../calibration/README.md) (commands, rounds, run files) |
| Data | [09 Data sources](09-data-sources.md) | [data/README.md](../data/README.md), `data/manifests/` (one per dataset, with licence) |
| Policy lab | [06 Scenarios and policy](06-scenarios-and-policy.md) | – |
| AI | [07 AI integration](07-ai-integration.md) | – |
| Compute | [08 Compute and scaling](08-compute-and-scaling.md), [11 Volunteer computing](11-volunteer-computing.md), [13 Native clients](13-native-clients.md) | – |
| Viewer and website | [design/DESIGN-SYSTEM.md](../design/DESIGN-SYSTEM.md) | [viewer/README.md](../viewer/README.md), [index.html](index.html) |
| Community | [12 Identity and credit](12-identity-and-credit.md) | [CREDITS.md](../CREDITS.md) |

## 3. Practice

- [14 Engineering practices](14-engineering-practices.md): TDD, the verification gate,
  performance method.

## 4. Decisions and questions

- [ADR log](adr/README.md): every architecture-level decision, numbered, with its
  evidence. Mechanisms and calibration rounds each have one.
- [Open questions](open-questions.md): what is unresolved, numbered; each points to the
  ADR that raised it and, when work is planned, to its issue.
