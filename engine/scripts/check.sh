#!/usr/bin/env bash
# Local verification gate for the Oikos engine. Run before every commit and push, on
# every machine class we support (x86_64, aarch64; Linux, macOS, Windows/WSL).
#
#   scripts/check.sh            full gate
#   scripts/check.sh --quick    fmt + clippy + native tests only
#
# Writes target/check-report.json (host, git revision, golden hashes) so reports from
# many machines can be compared with scripts/compare-reports.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

quick=0
[[ "${1:-}" == "--quick" ]] && quick=1

step() { printf '\n== %s\n' "$*"; }

step "format"
cargo fmt --all --check

step "clippy"
cargo clippy --all-targets -- -D warnings

step "tests (native, release)"
cargo test --release

if [[ $quick == 1 ]]; then
  echo "quick gate passed"
  exit 0
fi

step "tests (native, debug: overflow checks on)"
cargo test

step "golden work unit (native CLI)"
cargo build --release -p oikos-cli
native=$(./target/release/oikos run golden/m0-toy.workunit.json)
diff <(grep -v '"engine"' golden/m0-toy.json) <(echo "$native" | grep -v '"engine"')
native_hash=$(echo "$native" | sed -n 's/.*"state_hash": "\([0-9a-f]*\)".*/\1/p')

wasm_hash="skipped"
if command -v wasmtime >/dev/null; then
  step "tests (wasm32-wasip1 under wasmtime)"
  cargo test --release --target wasm32-wasip1
  step "golden work unit (wasm CLI)"
  cargo build --release --target wasm32-wasip1 -p oikos-cli
  wasm=$(wasmtime run --dir . target/wasm32-wasip1/release/oikos.wasm run golden/m0-toy.workunit.json)
  diff <(grep -v '"engine"' golden/m0-toy.json) <(echo "$wasm" | grep -v '"engine"')
  wasm_hash=$(echo "$wasm" | sed -n 's/.*"state_hash": "\([0-9a-f]*\)".*/\1/p')
else
  echo "SKIP wasm: wasmtime not installed (cargo install wasmtime-cli --locked)"
fi

step "outputs: native vs wasm byte-identical, readable by zarr-python and pyarrow"
rm -rf target/example-outputs target/example-outputs-native
./target/release/oikos run examples/outputs.workunit.json -o target/example-result.json
if [[ "$wasm_hash" != "skipped" ]]; then
  mv target/example-outputs target/example-outputs-native
  wasmtime run --dir . target/wasm32-wasip1/release/oikos.wasm run examples/outputs.workunit.json -o target/example-result-wasm.json
  diff -r target/example-outputs-native target/example-outputs
fi
py=${OIKOS_PYTHON:-python3}
[[ -z "${OIKOS_PYTHON:-}" && -x .venv/bin/python ]] && py=.venv/bin/python
if "$py" -c "import zarr, pyarrow, numpy" 2>/dev/null; then
  "$py" scripts/verify-outputs.py target/example-result.json
else
  echo "SKIP python verification: needs zarr, pyarrow, numpy (source scripts/env.sh)"
fi

step "python bindings: build with maturin, run pytest"
if "$py" -c "import maturin, pytest, numpy" 2>/dev/null; then
  PYO3_PYTHON="$py" cargo clippy -p oikos-py --all-targets -- -D warnings
  VIRTUAL_ENV=$("$py" -c "import sys; print(sys.prefix)") \
    "$py" -m maturin develop --release --quiet -m crates/oikos-py/Cargo.toml
  "$py" -m pytest -q python/tests
  "$py" python/examples/sensitivity.py >/dev/null
else
  echo "SKIP python bindings: needs maturin, pytest, numpy (source scripts/env.sh)"
fi

step "data pipelines: pytest, locked packs rebuild identically"
if "$py" -c "import h3, blake3, numpy, pytest" 2>/dev/null; then
  "$py" -m pytest -q ../data/tests
  (cd ../data && "$py" -m oikos_data check)
else
  echo "SKIP data pipelines: needs h3, blake3, numpy, pytest (source scripts/env.sh)"
fi

step "calibration: operators, emulators, history matching (pytest)"
if "$py" -c "import h3, numpy, pyarrow, pytest" 2>/dev/null; then
  "$py" -m pytest -q ../calibration/tests
else
  echo "SKIP calibration: needs h3, numpy, pyarrow, pytest (source scripts/env.sh)"
fi

step "dense determinism smoke (bench hashes across partition counts)"
h1=$(./target/release/oikos bench --res 5 --years 10 --parts 1 | sed 's/.*"state_hash": "\([0-9a-f]*\)".*/\1/')
h7=$(./target/release/oikos bench --res 5 --years 10 --parts 7 | sed 's/.*"state_hash": "\([0-9a-f]*\)".*/\1/')
[[ "$h1" == "$h7" ]] || { echo "partition-dependent result: $h1 vs $h7"; exit 1; }

host=$(rustc -vV | sed -n 's/^host: //p')
rev=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
dirty=$([[ -n "$(git status --porcelain 2>/dev/null)" ]] && echo true || echo false)
mkdir -p target
cat > target/check-report.json <<JSON
{"host": "$host", "rev": "$rev", "dirty": $dirty, "golden_native": "$native_hash", "golden_wasm": "$wasm_hash", "dense_hash": "$h1"}
JSON
step "passed"
cat target/check-report.json
