# Oikos engine

Rust workspace for the ManyWorlds simulation engine. Design: [docs/03-engine.md](../docs/03-engine.md).

| Crate | Purpose |
|---|---|
| `oikos-core` | Time, entity ids, Philox4x32-10 counter-based RNG, integer probabilities (`no_std`) |
| `oikos-math` | Bit-reproducible transcendental functions (pinned pure-Rust `libm`) |
| `oikos-grid` | H3 grid topology: regions, neighbor lists, partitions |
| `oikos-sim` | Super-individual agent store, phased partitioned step, checkpoints, work units |
| `oikos-cli` | `oikos` binary: runs work units natively or under wasmtime |

The M0 mechanism is a toy (logistic growth, band fission and migration over the
Southwest Asia H3 base cell). It exists to exercise the engine.

## Reproducibility contract

- Results are identical for any partition count and on x86_64, aarch64 and wasm32.
- `golden/m0-toy.json` is the committed result of `golden/m0-toy.workunit.json`; CI
  checks it on Linux x86_64/aarch64, macOS, Windows and wasm32-wasip1.
- Changing it is a deliberate, reviewed act:
  `cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json -o golden/m0-toy.json`

## Usage

```sh
cargo test --release
cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json

# WebAssembly (needs wasmtime)
cargo test --release --target wasm32-wasip1
cargo build --release --target wasm32-wasip1 -p oikos-cli
wasmtime run --dir . target/wasm32-wasip1/release/oikos.wasm run golden/m0-toy.workunit.json
```

A work unit can start from a checkpoint (`"start": {"kind": "checkpoint", "path": "...", "fork": true}`);
with `fork: true` the run continues under a new seed (and optionally new params), creating a new branch.
