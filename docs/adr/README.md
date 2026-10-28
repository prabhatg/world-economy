# Architecture decision records

| ADR | Decision | Status |
|---|---|---|
| 0001 | Purpose: policy laboratory for human welfare; history is calibration data | Accepted |
| 0002 | Three run modes (free, assimilated reanalysis, scenario forks) on one engine | Accepted |
| 0003 | Start epoch ~12,000 BCE | Accepted |
| 0004 | Adaptive resolution via super-individual agents with split/merge | Accepted |
| 0005 | Multi-dimensional welfare: health, material, capabilities, sustainability | Accepted |
| 0006 | Core in Rust + GPU kernels; Python (PyO3) for orchestration/calibration/AI | Accepted |
| 0007 | Web viewer in TypeScript with deck.gl/CesiumJS; not Flutter/Firebase | Accepted |
| 0008 | Storage: Zarr (fields), Parquet/Arrow (agents, events) on object storage; DuckDB for queries | Accepted |
| 0009 | H3 geodesic hex grid with hierarchical refinement | Accepted |
| 0010 | Bulk-synchronous phased ticks + counter-based RNG for determinism | Accepted |
| 0011 | Cloud-first (spot ensembles), HPC later — amended by 0014 (volunteer tier) | Accepted |
| 0012 | Open source, open provenance | Accepted |
| 0013 | First milestone: Neolithic Southwest Asia vertical slice | Accepted |
| 0014 | Volunteer computing (SETI@home-style) for ensemble workloads; central cluster for coupled runs | Accepted |
| 0015 | Own cloud-native coordinator (Rust, GCP), not BOINC; backend-neutral work units | Accepted |
| 0016 | Bitwise cross-platform determinism is a hard requirement on CPU/WASM | Accepted |
| 0017 | Two-tier numerics: integer/fixed-point conserved quantities, restricted IEEE f64 + own math crate; integer GPU kernels where validated bitwise | Accepted |
| 0018 | Engine compiled to WASM for browser clients; native builds for desktop/mobile/headless | Accepted |
| 0019 | All client types: browser, desktop, mobile, headless | Accepted |
| 0020 | All volunteer types, with separate `science` / `explore` queues | Accepted |
| 0021 | Nonprofit, open-source governance (fiscal sponsor → foundation) | Accepted |
| 0022 | Native client UI shell: Flutter + flutter_rust_bridge, presentation + IPC only; Tauri v2 as desktop fallback | Accepted |
| 0023 | Per-user Rust daemon (`worldd`) = headless client; sandboxed per-work-unit engine workers with no network; local IPC to UI | Accepted |
| 0024 | Globe in native clients = embedded TS web viewer; daemon serves it on localhost | Accepted |
| 0025 | Licenses: Apache-2.0 for code; CC-BY 4.0 for docs and results; datasets per source | Accepted |
| 0026 | Names: ManyWorlds (project, volunteer network, apps); Oikos (engine, crates `oikos-*`) — pending trademark/domain check | Accepted |
| 0027 | GPU compute: wgpu first (Vulkan/Metal/DX12/WebGPU), CUDA backend later behind the same trait | Accepted |
| 0028 | Test-driven development is mandatory; golden/determinism tests are the oracle for every refactor and optimization | Accepted |
| 0029 | No hosted CI for now: verification gate runs on our own compute; cross-host determinism via compared reports | Accepted |
| 0030 | Performance is a first-class requirement: measured benchmarks, targets and an optimization backlog (doc 14) | Accepted |
| 0031 | ManyWorlds Design System (design/): research-grounded principles, shared tokens for web and Flutter, no harsh boxes, diagrams first | Accepted |
| 0032 | Every user-facing change is reviewed by the `design-reviewer` agent before it ships | Accepted |
| 0033 | Project website is a static page in `docs/index.html`, served by GitHub Pages from `/docs` (no build step) | Accepted |
| 0034 | Mechanism API: declared reads/writes/period/after; schedule rejects ambiguous order; multi-rate via gcd base period; cache-tiled, cell-local decide phase | Accepted |
| 0035 | Parameter registry: one versioned TOML per model declaring unit, range, prior, status and sources for every parameter; engine embeds it and validates all overrides; placeholders must say so | Accepted |
| 0036 | Outputs: own minimal Zarr v3 writer (uint64, gzip, deterministic) + Parquet via the `parquet` crate; written through the host sink; observers are read-only | Accepted |
| 0037 | Python bindings: `oikos-py` (PyO3, abi3 wheel via maturin, module `oikos`) as thin wrappers over `oikos-sim`; JSON for structured values, NumPy for arrays, `ValueError` for engine errors; outside default members; tested in the gate against the golden hash | Accepted |
| 0038 | Partitions are contiguous cell ranges of near-equal estimated cost (8 × agents + cells), re-split every 16 steps when > 10% out of balance; performance settings, not model parameters | Accepted |
| 0039 | Deterministic performance metric: exact wasm instruction counts (wasmtime fuel, found by bisection) of `oikos bench --no-timing`, instead of callgrind (not on every host) | Accepted |
| 0040 | M1 region: H3 res-5 cells whose centers lie in 24–42°N, 26–58°E (22,198 cells over base cells 15, 16, 22, 31, 33, 41; includes Cyprus, the paleo-Gulf basin and the Nile valley and delta), chosen offline from a manifest; a run gets its cell list from the geometry pack and uses only H3 topology | Accepted |
| 0041 | Forcing packs: one little-endian file (magic, canonical JSON header, aligned arrays), identified by its BLAKE3 hash; per-cell static and time-sliced fields with unit, dtype, scale/offset, nodata and sources; packs of a region share a cell-list hash | Accepted |
| 0042 | M1 climate: CHELSA-TraCE21k v1.0 bioclim (bio1, bio4, bio12, bio15) as per-cell means of 2×2-block-averaged 30″ pixels, land and sea alike, for century slices placed at their midpoint years and interpolated linearly; the pack follows the stricter of the dataset's two licence statements (CC BY-SA 4.0) | Accepted |

## ADR-0007 rationale (frontend)

Simulation output is terabytes of gridded fields and agent snapshots. Firestore's
document model and per-read pricing are a poor fit; chunked arrays/columnar files on
object storage are the right substrate. The hard UI problem is GPU rendering of
millions of geospatial features on a globe, where deck.gl/CesiumJS/WebGPU are far
ahead of Flutter. Firebase may still be used for auth/hosting if convenient.

## ADR-0017 rationale (numerics)

Integers make conservation of people, goods and money exact over 14,000 simulated years
and make results trivially comparable across hosts. `f64` keeps accuracy for continuous
dynamics; restricting it to IEEE correctly-rounded operations plus our own
transcendental functions makes it bit-reproducible on x86_64, aarch64 and wasm32.
Converting probabilities to integer thresholds makes every stochastic decision exact.
GPU vendors and WGSL do not guarantee correctly rounded division/transcendentals, hence
integer kernels where bitwise validation is needed.

## ADR-0042 rationale (M1 climate)

CHELSA-TraCE21k is the only transient climate reconstruction at ~1 km covering the whole
M1 window, so terrain-driven contrasts (Zagros foothills against the alluvium) survive
aggregation to ~250 km² cells. Its four bioclimatic summaries are what the first
foraging and cultivation mechanisms need (warmth, water, and how seasonal both are);
monthly fields would cost 12× the bytes for detail those mechanisms do not yet use.
The files are stored in one-row strips, not tiles, so a region costs whole rows (~58 MB
per file, ~21 GB for M1 once); 2×2 block means keep the cache at ~2 GB while leaving
~85 samples per cell. Slices are 100-year means, so a slice is placed at its midpoint
and the engine interpolates linearly between midpoints; events shorter than a century
(the 8.2 ka event) are smoothed out, and the climate is a single model run (CCSM3), a
structural uncertainty to carry into calibration. A seeded weather generator from
per-cell statistics replaces raw fields later (doc 11 §11.2).

## Notes

- H3 cells are not exactly equal-area; mechanisms use per-cell areas, never assume
  uniform cells.
