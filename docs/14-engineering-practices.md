# 14. Engineering practices: test-driven, performance-first

Two non-negotiables: **test-driven development** and **extreme performance and
scalability**. Verification runs on our own compute (no hosted CI for now).

## 14.1 Test-driven development (mandatory)

1. **Red → green → refactor.** No production code without a failing test first. A
   change is done when its tests pass and the local gate passes.
2. **Every bug starts as a failing test.** Example from M0: a refactor dropped migrants
   moving between cells of the same partition. The conservation test failed first;
   a dense-world determinism test was added before the fix.
3. **Refactors and optimizations must not change results.** The golden run and the
   determinism tests are the oracle: a performance change that alters a hash is a
   model change, and must be justified and reviewed as one.
4. **Golden files change only on purpose.** Regenerating `engine/golden/*` needs a
   reason in the commit message (e.g. "RNG lane layout changed").

### Test layers

| Layer | What it proves | Examples |
|---|---|---|
| Unit + known-answer | building blocks are correct | Philox vs Random123 vectors; probability thresholds; H3 neighbor symmetry |
| Property (proptest) | invariants hold for all inputs | counting sort ≡ canonical sort; fused merge ≡ reference merge; people conserved |
| Determinism | same bits for any partition count, thread count, target | 1/2/3/4/7/8 partitions; dense world; native vs wasm32 golden |
| Golden | full-run reproducibility across hosts | `golden/m0-toy.workunit.json` → committed hash |
| Conservation | accounting is exact | Δpopulation = births − deaths every step |
| Mechanism stylized facts | each mechanism behaves as theory says | logistic saturation; distance decay of exchange (M1) |
| Scientific validation | the model fits held-out history | pattern-oriented tests vs data (M1+) |
| Performance | no regressions; scaling targets met | `oikos bench`, `scripts/bench.sh` |
| Fuzzing (planned) | parsers never panic or misbehave | checkpoint decoder, work-unit parser |
| Mutation testing (planned, periodic) | tests actually catch bugs | `cargo mutants` on core crates |

## 14.2 Local verification gate (our compute)

- `engine/scripts/check.sh` runs format, clippy (`-D warnings`), release and debug
  tests (debug has overflow checks on), the wasm32 test suite under wasmtime, both
  golden work units (native and wasm), and a dense-world partition-determinism smoke
  test. It writes `target/check-report.json`.
- `engine/scripts/check.sh --quick` for the inner loop.
- Opt-in pre-push hook: `git config core.hooksPath engine/scripts/hooks`.
- **Cross-host determinism:** run the gate on each machine class (x86_64 / aarch64,
  Linux / macOS / Windows) and compare with
  `engine/scripts/compare-reports.sh report-*.json` — every hash must match.
- Later the coordinator itself runs test work units on cluster and volunteer hosts
  (dogfooding the validation path).

## 14.3 Performance method

- **Measure first.** `oikos bench` fills a world to carrying capacity and reports
  ns per agent-step, agent-steps/s and the final state hash (which must be identical
  for every partition count and host). `scripts/bench.sh` sweeps partition counts and
  appends results to `target/bench.jsonl`.
- **Deterministic performance metric (planned):** instruction counts per agent-step
  under callgrind, so regressions are detected without noisy wall-clock timing.
- **Profile before optimizing** (perf / samply flamegraphs).

### Measured (M0, toy mechanism, 117,649 cells, ~0.5M agents, 4-core container)

| Version | 1 thread | 4 cores |
|---|---|---|
| Initial (global sort, per-step threads, copies) | 117 ns/agent-step | 49 ns |
| Persistent partitions, rayon pool, counting sort, fused merge + recount, zero steady-state allocation | 42 ns | 13 ns (~76M agent-steps/s) |

Results are bit-identical between the two versions (golden hash unchanged).

### Targets

- Single core: ≤ 10 ns per agent-step for simple mechanisms (memory-bandwidth bound:
  ~20 bytes of hot state per agent).
- Node: ≥ 80% parallel efficiency up to all cores (overdecomposition + work stealing).
- Cluster: ≥ 90% weak-scaling efficiency to 1,000+ nodes; ≥ 70% strong scaling for
  modern-era runs.
- Volunteer: a work unit makes useful progress in a browser tab in ≤ 1 minute.

### Optimization backlog (each lands test-first, hashes preserved unless stated)

1. Per-cell precomputation (growth factor, least-crowded neighbor) instead of
   per-agent division and neighbor scans.
2. One Philox block per agent per mechanism, using all four lanes (**changes golden**,
   deliberate).
3. Batched/SIMD Philox (AVX2, NEON, wasm simd128).
4. Narrower hot state: partition-local `u16`/`u32` cell indices, `u32` weights where
   safe; cold components in side tables.
5. Cost-weighted partitioning along a Hilbert curve; overdecomposition; NUMA pinning.
6. Distributed: halo of boundary-cell state only; batched migrant messages (MPI);
   compute/communication overlap.
7. GPU decide-phase kernels (wgpu, integer arithmetic for bitwise validation).
8. In-situ reductions and asynchronous checkpoint I/O.
