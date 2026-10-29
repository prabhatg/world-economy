# 10. Roadmap

Principle: **vertical slices** — each milestone runs the whole loop (engine → mechanisms
→ outputs → calibration → viewer) end to end, at increasing breadth.

## M0 — Foundations

**Status:** first slice implemented in [`engine/`](../engine/README.md) — workspace,
Philox RNG (Random123 known answers), integer probabilities, pinned deterministic math,
H3 grid, super-individual agents with split/merge, phased partitioned step,
checkpoint/fork, JSON work units, `oikos` CLI + benchmark, persistent partitions on a
rayon pool (13 ns/agent-step on 4 cores), local verification gate; golden run
reproduced bit-for-bit on x86_64 and wasm32; pluggable mechanisms with a validated,
multi-rate schedule and cache-tiled decide phase (no performance cost vs. the hard-coded
step, golden unchanged); parameter registry with units, ranges, priors, status and
sources, enforced on every override; Zarr/Parquet outputs, byte-identical native vs
wasm and verified with zarr-python and pyarrow. Cross-host verified: x86_64 Linux
and aarch64 macOS (Apple Silicon) give identical golden, wasm and dense hashes.
Python bindings (`oikos` module, PyO3 + maturin, abi3): create, run, fork, checkpoint
and read worlds, run work units, sample priors; tested in the gate to reproduce the
golden hash. Cost-weighted partitions, re-balanced as population moves (up to 2.8×
faster on hotspot worlds, results unchanged), and an exact instruction-count metric
(wasm fuel). Remaining: run the gate on Windows.

- Rust workspace skeleton (core, grid, store, sched, io, params, sim, py).
- H3 grid, SoA store with super-individuals and split/merge, phase scheduler,
  Philox RNG, checkpoint/fork, Zarr/Parquet output.
- Two-tier numerics (integer conserved quantities, restricted `f64`), deterministic
  math crate.
- Build targets: x86_64, aarch64, **wasm32**.
- Local verification gate on our compute: determinism across partition counts *and*
  across targets, conservation property tests, golden-run hashes.
- Work-unit format v0 (segment run: state in → state + summaries out).
- Toy mechanism: logistic population growth on a grid with diffusion. Python can run,
  fork and read results.
- **Exit:** bit-identical results across 1/2/4/8 partitions and across x86_64 /
  aarch64 / wasm32; fork/restore works; a work unit runs in a browser tab.

## M1 — Neolithic Southwest Asia (12,000–3,000 BCE)

**Status:** started. Region chosen (ADR-0040); forcing-pack format and engine inputs
(ADR-0041: worlds on a geometry-pack region, work units and checkpoints naming their
packs); `data/` pipelines with locked pack ids; geometry, terrain, climate
(CHELSA-TraCE21k, ADR-0042), coastline (ADR-0043: moving coastlines; flooded cells
are uninhabitable), hydrology (ADR-0044: rivers, floodplains, lakes) and soils
(ADR-0045) packs for M1; radiocarbon observation tables from p3k14c with IntCal20
(ADR-0046). First mechanism on real forcing: forager carrying capacity from climate,
dry land and aquatic resources (ADR-0047), and forager dispersal by budding into less
crowded land (ADR-0048), so M1 worlds fill the region as a wave of advance.
Cultivation and domestication (ADR-0049): bands carry cultivation effort and crop
domestication, driven by competing hypotheses nested as weights; M1 worlds start with
foragers across the region.
First observation operator (ADR-0050): radiocarbon date density, scoring recorded
worlds against binned, calibrated p3k14c dates with taphonomy and calibration
forward-modelled (`calibration/`). Domestication timing (ADR-0051) against barley
rachis assemblages: the record and M1 worlds both favour protracted domestication over
rapid. First calibration waves (ADR-0052): Morris screening and two history-matching
waves with Gaussian-process emulators leave 22% of the prior space, ruling out rapid
domestication; radiocarbon summaries do not yet constrain. Neolithic demographic transition
(ADR-0053) as a nested hypothesis; it does not explain the radiocarbon rise. Stable-
climate hypothesis of cultivation (ADR-0054), nested and off: untestable on
century-resolution climate. Land degradation (ADR-0055), nested and off: fields wear
soils down, turning the farming boom into a bust. Calibration round 2 (ADR-0056): finer
radiocarbon eras judged together; 4.7% of the prior space survives, and the best
worlds erode their soils.
Work items are GitHub issues under the M1 milestone.

- Region: Levant, Anatolia, Zagros, Mesopotamia (res 5 H3): res-5 cells with centers
  in 24–42°N, 26–58°E, 22,198 cells incl. Cyprus, the paleo-Gulf basin and the Nile valley (ADR-0040).
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
