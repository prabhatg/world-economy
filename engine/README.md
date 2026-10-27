# Oikos engine

Rust workspace for the ManyWorlds simulation engine. Design: [docs/03-engine.md](../docs/03-engine.md).

| Crate | Purpose |
|---|---|
| `oikos-core` | Time, entity ids, Philox4x32-10 counter-based RNG, integer probabilities (`no_std`) |
| `oikos-math` | Bit-reproducible transcendental functions (pinned pure-Rust `libm`) |
| `oikos-grid` | H3 grid topology: regions, neighbor lists, partitions |
| `oikos-params` | Parameter registry: units, ranges, priors, status and sources, loaded from `params/*.toml` |
| `oikos-sim` | Super-individual agent store, phased partitioned step, checkpoints, work units |
| `oikos-cli` | `oikos` binary: runs work units (natively or under wasmtime), benchmarks, exports the parameter registry (`oikos params`) |

The M0 mechanism is a toy (logistic growth, band fission and migration over the
Southwest Asia H3 base cell). It exists to exercise the engine.

## Reproducibility contract

- Results are identical for any partition count and on x86_64, aarch64 and wasm32.
- `golden/m0-toy.json` is the committed result of `golden/m0-toy.workunit.json`; the
  local gate checks it natively and on wasm32-wasip1. Run the gate on every machine
  class and compare reports to confirm cross-host determinism.
- Changing it is a deliberate, reviewed act:
  `cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json -o golden/m0-toy.json`

## Verification (test-driven; runs on our own compute)

```sh
scripts/check.sh            # full gate: fmt, clippy, tests (release+debug, native+wasm), golden, determinism
scripts/check.sh --quick    # inner loop
scripts/compare-reports.sh a.json b.json   # cross-host determinism (target/check-report.json from each host)
scripts/bench.sh [res] [years]             # throughput + strong scaling -> target/bench.jsonl
git config core.hooksPath engine/scripts/hooks   # opt-in pre-push gate
```

See [docs/14-engineering-practices.md](../docs/14-engineering-practices.md).

## Usage

```sh
cargo run --release -p oikos-cli -- bench --res 6 --years 20 --parts 4
cargo test --release
cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json

# WebAssembly (needs wasmtime)
cargo test --release --target wasm32-wasip1
cargo build --release --target wasm32-wasip1 -p oikos-cli
wasmtime run --dir . target/wasm32-wasip1/release/oikos.wasm run golden/m0-toy.workunit.json
```

A work unit can start from a checkpoint (`"start": {"kind": "checkpoint", "path": "...", "fork": true}`);
with `fork: true` the run continues under a new seed (and optionally new params), creating a new branch.
