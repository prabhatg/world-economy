# Oikos engine

Rust workspace for the ManyWorlds simulation engine. Design: [docs/03-engine.md](../docs/03-engine.md).

| Crate | Purpose |
|---|---|
| `oikos-core` | Time, entity ids, Philox4x32-10 counter-based RNG, integer probabilities (`no_std`) |
| `oikos-math` | Bit-reproducible transcendental functions (pinned pure-Rust `libm`) |
| `oikos-grid` | H3 grid topology: regions, neighbor lists, partitions |
| `oikos-params` | Parameter registry: units, ranges, priors, status and sources, loaded from `params/*.toml` |
| `oikos-io` | Output formats: minimal Zarr v3 writer (gzip, deterministic bytes) and Parquet tables |
| `oikos-sim` | Super-individual agent store, phased partitioned step, checkpoints, work units |
| `oikos-cli` | `oikos` binary: runs work units (natively or under wasmtime), benchmarks, exports the parameter registry (`oikos params`) |
| `oikos-py` | Python module `oikos` (PyO3, abi3 wheel built with maturin): create, run, fork and read worlds in-process. Not a default member: `cargo test` and the wasm build skip it |

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

Needs [rustup](https://rustup.rs) and `wasmtime` (`brew install wasmtime`). Then, once
per shell, `source scripts/env.sh`: it puts cargo on PATH, installs the pinned Python
Python tools (`requirements.txt`: zarr-python, pyarrow, maturin, pytest) into `.venv`
for the gate's output check and Python-binding tests, and activates it.

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
# Run with outputs (Zarr fields + Parquet tables) and verify them with zarr-python/pyarrow
cargo run --release -p oikos-cli -- run examples/outputs.workunit.json -o target/example-result.json
python scripts/verify-outputs.py target/example-result.json   # venv from source scripts/env.sh
cargo run --release -p oikos-cli -- bench --res 6 --years 20 --parts 4
cargo test --release
cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json

# WebAssembly (needs wasmtime)
cargo test --release --target wasm32-wasip1
cargo build --release --target wasm32-wasip1 -p oikos-cli
wasmtime run --dir . target/wasm32-wasip1/release/oikos.wasm run golden/m0-toy.workunit.json
```

### Python

```sh
source scripts/env.sh
maturin develop --release -m crates/oikos-py/Cargo.toml   # builds and installs `oikos` into .venv
pytest python/tests
python python/examples/sensitivity.py                      # 4x4 parameter sweep, 3 seeds each
```

```python
import oikos

w = oikos.World(22, 4, seed=1, params={"growth_rate": 0.02})   # registry-checked overrides
summaries = w.run_years(500, parts=4)          # list of dicts, as in a work-unit result
w.population(), w.agents()                     # NumPy arrays: per cell; id/cell/weight per agent
branch = w.fork(seed=2)                        # new branch of history from this state
restored = oikos.World.from_checkpoint(w.checkpoint())
oikos.run_work_unit(open("golden/m0-toy.workunit.json").read())   # same result as `oikos run`
oikos.sample_params({"growth_rate": 0.5})      # unit cube -> prior (uniform, log-uniform)
oikos.registry()                               # same as `oikos params`
```

Engine errors raise `ValueError` with the engine's message. `run_years` and
`run_work_unit` release the GIL, so a thread pool runs independent worlds in parallel.

A work unit can start from a checkpoint (`"start": {"kind": "checkpoint", "path": "...", "fork": true}`);
with `fork: true` the run continues under a new seed (and optionally new params), creating a new branch.
