# 3. Simulation engine

Rust core, data-oriented, deterministic, designed single-node-first but with every
abstraction chosen so that multi-node / multi-GPU is an implementation, not a redesign.

## 3.1 Space

- **Geodesic hex grid (H3).** Equal-ish area, uniform neighbors, hierarchical
  (resolution levels nest ≈7:1), good libraries. Working resolutions:
  - res 4 (~1,770 km², ~22 km edge) for global coarse runs
  - res 5 (~250 km²) default regional
  - res 6–7 (~36 / ~5 km²) for dense regions / modern cities
- Cells carry environmental state (elevation, hydrology, soils, biome, climate fields,
  sea-level mask, carrying capacity, infrastructure).
- **Multi-resolution:** a cell can be refined into children where population density or
  data density warrants it (adaptive mesh refinement on the H3 hierarchy).
- Point locations for settlements/agents are stored as (cell id, local offset) for
  sub-cell precision when needed (roads, ports, cities).

## 3.2 Time

- Integer tick counter; base unit = 1 day; epoch = 12,000 BCE.
- **Multi-rate scheduling:** each mechanism declares a period (e.g. climate monthly,
  demography seasonal, markets weekly in the modern era, institutions yearly).
- **Era-adaptive rates:** period tables are functions of era/region
  (e.g. demography yearly in 10,000 BCE, monthly after 1800).
- Event queue for rare discrete events (eruptions, plague introduction, invention,
  battles) inserted between ticks, processed deterministically by (tick, priority, id).

## 3.3 State: entities, components, relations

- **Structure-of-arrays** columnar component storage, per partition, GPU-friendly.
  Hot components (location, weight, age-structure, wealth, health) contiguous; cold
  components in side tables.
- **Super-individual agents** (Scheffer et al. 1995): each agent has a `weight` =
  number of humans it represents. One code path from bands (weight 25–50) to
  individuals (weight 1). **Split/merge** operators change resolution by era/region
  while conserving totals and distributions.
- **Agent kinds** (initial): Person/Household-group (weighted), Settlement, Group
  (band/lineage/clan), Polity, Market, Firm/Workshop, Religion/Ideology, Route.
- **Relations** (membership, kinship, trade links, alliance, tribute, employment,
  belief) as partitioned edge lists (CSR + delta buffers), owned by source entity.
- **Stable 64-bit ids** that encode the birth *cell* + a per-cell counter, never reused.
  Cells (not partitions) own the counters, so ids are independent of partitioning.

## 3.4 Scheduling & phases

Each tick is bulk-synchronous with fixed phases, which gives parallelism *and*
determinism without locks:

1. **Sense** — read-only gather (neighbors, prices, rumors, threats).
2. **Decide** — agents produce intents (migrate, trade offer, marry, raid, adopt tech).
3. **Exchange** — intents crossing partitions are batched to owners.
4. **Resolve** — owners resolve conflicts deterministically (matching, auctions,
   combat), sorted by (entity id) not arrival order.
5. **Commit** — apply state changes; births/deaths/splits/merges; migrations.
6. **Reduce** — in-situ metrics and observation-operator outputs.

Mechanisms are nodes in a DAG built from declared read/write sets; independent
mechanisms run concurrently (Rayon on CPU, kernels on GPU).

## 3.5 Determinism & reproducibility

Volunteer computing (doc 11) validates results by comparing hashes from independent
hosts, so **bitwise reproducibility across platforms is a hard requirement** for the
CPU/WASM path: x86_64, aarch64, wasm32; Windows/macOS/Linux/Android/iOS; any partition
or thread count.

- **Counter-based RNG** (Philox4x32-10): `rng(run_seed, stream_id, entity_id, tick)`
  — no RNG state to carry, independent of partitioning and thread count.
- **Order-independent reductions:** sorted or integer/fixed-point accumulation, never
  arrival-order floating-point sums.
- Run identity = hash(code version, config, parameter set, seed, input data manifest).

### Numeric model (two tiers)

Chosen for the best combination of accuracy and verifiable determinism:

1. **Conserved quantities in integers.** People (agent weights, cohort counts), food
   and goods stocks (fixed-point grams/units in `i64`), money (fixed-point micro-units
   in `i64`/`i128`), land area. Conservation is exact by construction and checked
   every tick — no drift over 14,000 simulated years.
2. **Continuous dynamics in IEEE `f64`, restricted to bit-reproducible operations.**
   - Only `+ − × ÷ √` (correctly rounded by IEEE 754 on every platform and in WASM);
     no fused multiply-add unless explicit and uniform; no fast-math; no x87.
   - Transcendentals (`exp`, `ln`, `pow`, `sin`, …) from our own pure-Rust
     deterministic math crate (fixed polynomial algorithms, or correctly-rounded
     implementations), never the platform libm.
   - NaN is a bug: checked and rejected (NaN payload bits differ across platforms).
3. **Stochastic decisions are integer comparisons.** A probability `p` computed in tier 2
   is converted once to a `u32` threshold and compared with a Philox `u32` draw, so
   decisions are exact and cheap on any device.
4. **GPU kernels.** WebGPU/WGSL and vendor float division/transcendentals are not
   guaranteed correctly rounded, so GPU kernels needing bitwise validation use integer
   / fixed-point arithmetic (64-bit emulated where WGSL lacks `i64`). Kernels that use
   native GPU floats are validated by *homogeneous redundancy* (same platform class)
   and statistical tolerance, and are documented as such.

CI runs golden runs on every target and fails on any hash mismatch.

## 3.6 Distribution & load balancing

- **Partitioning:** cells ordered along a space-filling curve (Hilbert on H3);
  partitions are contiguous curve segments, weighted by measured cost (agents × work).
  Re-balanced periodically — population is extremely non-uniform and shifts over
  millennia (and cities are hotspots).
- **Halo exchange** for local interactions; **message batches** for long-range ones
  (trade routes, empires, finance, internet), which in the modern era dominate.
- **Agent migration** between partitions at commit.
- Backends: single process → multi-process on one node → MPI (rsmpi) multi-node;
  GPU via CUDA (cudarc) or wgpu for hot mechanisms. Kernels kept uniform and
  branch-light so they also map to future accelerators.

## 3.7 Checkpoint, fork, branch

- Full-state snapshots in a content-addressed store (chunked, compressed, dedup'd
  between nearby checkpoints).
- **Fork** = restore snapshot + new (params, policy, seed stream) → a new run id with
  parent link. Forms a run tree; the viewer navigates it.
- Particle-filter assimilation (doc 5) and scenario ensembles are both "fork many from
  one", so this path must be cheap.

## 3.8 Outputs

- You cannot write billions of agents every tick. Outputs are:
  - **Fields** (per cell, per output period) → Zarr.
  - **Panels**: deterministic sampled agent cohorts with full history → Parquet.
  - **Events**: births of polities, wars, inventions, migrations → Parquet.
  - **Observation-operator outputs** for calibration → compact arrays.
- In-situ reductions only; raw state only at checkpoints.

## 3.9 Engineering practices

- Rust workspace, `cargo` + `maturin` for Python wheels; CI with determinism tests
  (same result across 1/2/4/8 partitions), property tests for conservation laws
  (people, mass, money), and golden-run regression tests.
- Units checked at the type level where cheap (`uom` or newtypes).
- Every mechanism ships with: doc, citations, parameter priors, unit tests, and a
  small "stylized fact" test (e.g. logistic growth under fixed carrying capacity).
- Profiling built in (per-mechanism timings feed the load balancer).
