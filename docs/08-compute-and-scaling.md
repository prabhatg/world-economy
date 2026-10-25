# 8. Compute & scaling

## 8.1 Cost model (order of magnitude)

Let `A` = agents, `T` = ticks, `c` = FLOP per agent-tick (memory traffic usually the
real limit).

| Era / run | A | T | c | FLOP per run |
|---|---|---|---|---|
| M1: SW Asia 12,000–3,000 BCE, monthly | 1e5–1e6 (weighted households) | 1.1e5 | 1e4 | ~1e14–1e15 (minutes on one GPU / node) |
| Global Holocene to 1 CE, seasonal | 1e6–1e7 | 5.6e4 | 1e4 | ~1e15–1e16 |
| 1 CE–1800, monthly | 1e7–1e8 | 2.2e4 | 1e4 | ~1e15–1e16 |
| 1800–2026, weekly → daily | 1e8 → 8e9 | ~4e4 | 1e4–1e5 | ~1e18–1e19 |
| Calibration ensembles | × 1e3–1e5 runs | | | ~1e19–1e24 |

Comparison: a frontier AI training run is ~1e25–1e26 FLOP. Full-resolution calibrated
ensembles of the modern era are in the same league — hence emulators and adaptive
resolution are not optional.

Memory: modern full resolution ≈ 8e9 agents × ~0.5–2 KB hot+cold state ≈ 4–16 TB,
plus relations (social/trade/employment graphs, ~1e11 edges) ≈ another few TB.
Fits a few hundred GPUs/nodes; trivial for leadership-class machines.

## 8.2 Where parallelism comes from

1. **Ensembles & forks** (embarrassingly parallel) — the dominant axis for calibration,
   assimilation and scenarios. Most early value comes from here.
2. **Within-run spatial decomposition** — matters mainly for 1500 → today.
3. **Within-node data parallelism** — SoA kernels on GPU / SIMD on CPU.

Deep-past runs are strong-scaling-limited (few agents, many ticks): use coarse ticks,
super-individuals, and run thousands concurrently rather than one run fast.

## 8.3 Volunteer tier

Ensemble work (axis 1 above) is the dominant compute need and the ideal fit for
volunteer computing (doc 11). Rough capacity: 10,000 active volunteers × ~4 cores ×
~10 GFLOP/s effective (memory-bound agent code) ≈ 0.4 PFLOP/s sustained ≈ 3e19
FLOP/day, i.e. tens to hundreds of thousands of M1-scale runs per day, at near-zero
marginal cost; volunteer GPUs raise this by an order of magnitude. Folding@home briefly exceeded
1 EFLOP/s in 2020 with a compelling mission. The central cluster then only needs to
handle coupled runs, assimilation and analysis.

## 8.4 Cloud-first plan (solo + AI, modest budget)

- **Phase 1:** single workstation/VM with one GPU; CPU-parallel engine (Rayon).
  M1 runs in minutes → thousands of calibration runs per day.
- **Phase 2:** ensemble farming on cloud batch (GCP Batch / Kubernetes with spot/
  preemptible GPU and CPU VMs). Runs are checkpointable → preemption-tolerant.
  Results to GCS as Zarr/Parquet; queries via DuckDB.
- **Phase 3:** multi-node single runs (MPI over cloud HPC instances) for the modern era.
- **Phase 4:** HPC allocations (NSF ACCESS → DOE INCITE / EuroHPC) once validated
  results justify it; the engine's MPI + GPU design ports directly.

The volunteer coordinator (doc 11) treats cloud spot workers and HPC nodes as just
another client class, so capacity can shift between tiers without code changes.

## 8.5 Cost controls

- Adaptive resolution (era, region) and adaptive output.
- Emulator-first experiment design; full runs only where emulators are uncertain.
- Shared early-history checkpoints: thousands of later-era experiments fork from a
  small number of expensive prefixes.
- Spot/preemptible instances everywhere; checkpoint cadence tuned to preemption rate.
