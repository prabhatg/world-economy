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
- **Stable 64-bit ids** that encode birth partition + counter, never reused.

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

- **Counter-based RNG** (Philox4x32-10): `rng(run_seed, stream_id, entity_id, tick)`
  — no RNG state to carry, independent of partitioning and thread count.
- **Order-independent reductions:** sorted or fixed-point / compensated summation so
  results are identical across partition counts and devices (target: bitwise across
  CPU partition counts; tolerance-bounded across CPU↔GPU, documented).
- Run identity = hash(code version, config, parameter set, seed, input data manifest).

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
