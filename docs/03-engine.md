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

**Implemented (M0):** `oikos_sim::mechanisms::Mechanism` declares a `MechanismSpec`
(name, period in ticks, components read and written, `after` dependencies) and a
`decide` method. `oikos_sim::schedule::Schedule::build` orders mechanisms
deterministically and **rejects** any pair that touches the same component (at least
one writing it) without an explicit order, as well as cycles, unknown dependencies,
duplicate names and zero periods. Multi-rate: the world advances by the gcd of all
periods and runs each mechanism on ticks that are multiples of its own period.
Decide runs **tile by tile**: each due mechanism processes a cache-sized run of agents
ending on a cell boundary before the next tile starts, so adding mechanisms does not add
passes over memory. Mechanisms must therefore be local to the cells of their tile and
read everything else through the read-only `Sense` snapshot. New agents are created only
through `PartitionView::spawn`, which assigns birth-cell ids and routes them to the
owning partition.

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

The local verification gate runs golden runs on every target and fails on any hash mismatch.

## 3.6 Distribution & load balancing

- **Partitioning:** cells ordered along a space-filling curve (Hilbert on H3);
  partitions are contiguous curve segments, weighted by measured cost (agents × work).
  Re-balanced periodically — population is extremely non-uniform and shifts over
  millennia (and cities are hotspots).
  *Implemented (M0, ADR-0038):* contiguous ranges in H3 index order (hierarchical,
  so spatially compact; Hilbert ordering may follow) of near-equal cost, with cost =
  8 × agents + 1 × cells (measured per-agent vs per-cell step work). Every 16 steps the
  world re-splits if the costliest partition is more than 10% above the mean,
  reusing all buffers. Results never depend on the split, so these are performance
  settings, not registry parameters.
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

**Implemented (M0):** a work unit's optional `outputs` block writes, through the host's
`write(path, bytes)` callback, `fields.zarr` (Zarr v3: `population[time, cell]`, `tick`,
`cell_h3`, `capacity`; group attributes record engine, seed, H3 base cell/resolution,
registry and parameters), `summary.parquet` (totals and flows per output time) and
`panel.parquet` (agents sampled by id hash). Recording only reads the world: the state
hash is unchanged and output bytes are identical across partition counts and between
native and wasm32. The gate verifies them with zarr-python and pyarrow.

## 3.8a Inputs: forcing packs

Environmental inputs (geometry, climate, terrain, soils, hydrology, sea level) reach a
run as **forcing packs** built offline by `data/` pipelines from dataset manifests
(doc 9). A pack is one file, so it can be content-addressed, cached and fetched by any
host (ADR-0041):

```
"OIKOSPK1" | header_len u32 | header (UTF-8 JSON, canonical) | zero padding to 8 bytes | arrays
```

- **Identity:** the pack id is the BLAKE3 hash of the whole file. Work units list pack
  ids; the engine refuses a pack whose bytes hash differently, and the ids are part of
  run identity (§3.5).
- **Header:** `format`, `kind`, `region` (`name`, `resolution`, `cells`, and
  `cells_hash`, the BLAKE3 of the ascending H3 indices as little-endian `u64`), `time`
  (ascending `calendar_year` list and the rule `step` or `linear`), `fields`, `sources`
  (dataset, version, license, citation, manifest hash, and the `attribution` notice the
  license requires, if any) and `builder` (tool, format).
  Keys are sorted, without insignificant whitespace, so a header has one encoding.
- **Fields:** `name`, `unit`, `description`, `dtype` (`u8`…`u64`, `i16`…`i64`, `f32`,
  `f64`), `dims` (`["cell"]` or `["time", "cell"]`, time-major), `scale_factor` and
  `add_offset` (physical = stored × scale_factor + add_offset, in `f64`, as in the CF
  conventions), optional integer `nodata`, and the array's byte `offset` and `len`
  (8-byte aligned, after the header). A `geometry` pack must contain `h3`.
- **Region:** the geometry pack carries `h3` (`u64` per cell), `lat`, `lng` and
  `area_km2`, computed offline, so runs never use `h3o` geometry; every pack of a
  region carries the same `cells_hash`, and the engine checks it against the world.
  The M1 region is res-5 cells with centers in 24–42°N, 26–58°E (ADR-0040).
- **Time:** `step` holds the last slice at or before the current year; `linear`
  interpolates between slices with `+ - * /` only; outside the covered years the
  nearest slice holds.
- Size target: ≤ ~50 MB per work unit (doc 11 §11.2); a seeded weather generator from
  per-cell statistics replaces raw climate fields later.

**Implemented:** `oikos-io::pack` (reader, writer, `oikos pack <file>`) and
`data/oikos_data` (Python writer, M1 geometry pack). A world's `WorldConfig` is either
`{"base_cell", "resolution"}` (M0 toy worlds) or `{"geometry": "<pack id>"}`, whose
grid comes from the pack's `h3` list (topology only). Work units list
`inputs: [{"id", "path"}]`; `Inputs` checks every id, one geometry pack, unique kinds
and one region. Mechanisms read fields through `Sense::inputs` and declare
`Component::Forcing`. Checkpoints of geometry-pack worlds use version 2, which records
the geometry pack and all input ids, so they restore only with those packs;
base-cell checkpoints keep version 1 byte for byte (golden unchanged). Python:
`oikos.World.from_packs(paths, seed=...)`, `World.from_checkpoint(data, packs=...)`.
M1 packs: geometry and terrain (static), climate (92 century slices × 4 fields,
~16 MB, `linear`; ADR-0042) and coastline (`land_fraction` per slice, ~4 MB). With a
`coastline` pack, `World::habitable` (also `Sense::habitable`) marks the cells with
land in the current year, rebuilt each step from the pack and the tick (ADR-0043).

## 3.9 Engineering practices

Test-driven development and performance methodology are in
[doc 14](14-engineering-practices.md). Verification runs on our own compute through a
local gate (`engine/scripts/check.sh`), not hosted CI. Every mechanism ships with docs,
citations, parameter priors, unit and property tests, and a stylized-fact test.
