# 10. Roadmap

Principle: **vertical slices** — each milestone runs the whole loop (engine → mechanisms
→ outputs → calibration → viewer) end to end, at increasing breadth.

## M0 — Foundations

**Status:** first slice implemented in [`engine/`](../engine/README.md) — workspace,
Philox RNG (Random123 known answers), integer probabilities, pinned deterministic math,
H3 grid, super-individual agents with split/merge, phased partitioned step,
checkpoint/fork, JSON work units, `oikos` CLI, golden run reproduced bit-for-bit on
x86_64 and wasm32 (CI adds aarch64, macOS, Windows). Remaining: PyO3 bindings, Zarr /
Parquet output, mechanism trait + DAG scheduler, parameter registry, weighted load
balancing.

- Rust workspace skeleton (core, grid, store, sched, io, params, sim, py).
- H3 grid, SoA store with super-individuals and split/merge, phase scheduler,
  Philox RNG, checkpoint/fork, Zarr/Parquet output.
- Two-tier numerics (integer conserved quantities, restricted `f64`), deterministic
  math crate.
- Build targets: x86_64, aarch64, **wasm32**.
- CI: determinism across partition counts *and* across targets, conservation property
  tests, golden-run hashes.
- Work-unit format v0 (segment run: state in → state + summaries out).
- Toy mechanism: logistic population growth on a grid with diffusion. Python can run,
  fork and read results.
- **Exit:** bit-identical results across 1/2/4/8 partitions and across x86_64 /
  aarch64 / wasm32; fork/restore works; a work unit runs in a browser tab.

## M1 — Neolithic Southwest Asia (12,000–3,000 BCE)
- Region: Levant, Anatolia, Zagros, Mesopotamia (res 5 H3).
- Forcing: CHELSA-TraCE21k climate, sea level, terrain, hydrology, soils.
- Mechanisms: foraging ecology; cultivation & domestication co-evolution; sedentism and
  storage; Neolithic demographic transition; band/village formation and fission;
  exchange networks (obsidian as tracer!); emerging inequality; simple disease;
  social learning of practices.
- Observation operators: radiocarbon density (p3k14c), site counts, obsidian sourcing
  distance-decay, domestication timing.
- Calibration: sensitivity analysis + first history-matching waves on cloud batch.
- Viewer v0: TS + deck.gl map playing back fields and settlements over time.
- **Coordinator v0** (work generator, dispatcher, result ingest, validator) with cloud
  spot workers as the first client class.
- **Browser client v0**: closed alpha, runs M1 work units and shows "your world".
- **Exit:** plausible emergence of farming and villages in the right windows across
  ensembles, with quantified fit; competing domestication hypotheses compared;
  first calibration wave computed partly by alpha volunteers.

## M2 — Calibration & assimilation machinery + public volunteer launch
- Emulators (GP/NN), SBI pipeline, localized particle-filter assimilation on M1.
- Knowledge-ingestion pipeline v1 feeding the parameter registry; citizen-science
  review tool for extracted parameters.
- Desktop clients (Win/macOS/Linux) and headless/Docker client; accounts, teams,
  credits; adaptive replication.
- Nonprofit set up (fiscal sponsor), charter, advisory board; **public launch**.
- Android client (charging + Wi-Fi); iOS explore-first client.

## M3 — Global Holocene to 1 CE
- All continents at res 4; independent agricultural origins (Fertile Crescent, China,
  Mesoamerica, Andes, New Guinea, Sahel…) as validation; chiefdoms → early states;
  bronze, writing, cities, long-distance trade; Seshat validation.

## M4 — 1 CE–1800
- Empires, world religions, trade networks (Silk Road, Indian Ocean, Atlantic),
  pandemics, Columbian exchange, early modern finance, colonial extraction.
- First hindcast experiments.

## M5 — 1800–2026 and the digital twin
- Industrialization, demographic transition, nation-states, globalization, digital
  economy, AI. Dense assimilation; synthetic modern population; twin-2026 v1.
- Multi-node scaling work (MPI, GPU kernels).

## M6 — Policy laboratory
- Policy DSL, scenario ensembles, RDM analysis, policy & adoption-pathway search,
  hindcast policy validation, public web viewer and reports.

A small **modern-era policy toy** (synthetic country) may be built alongside M2 to
exercise the policy DSL early, without waiting for M5.
