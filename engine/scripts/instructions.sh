#!/usr/bin/env bash
# Deterministic performance metric: WebAssembly instructions executed by `oikos bench`,
# counted exactly as wasmtime fuel (about one unit per wasm operator). Unlike wall-clock
# time it does not vary between runs or with machine load, so small regressions show.
# Counts depend on the code, the pinned toolchain and the wasmtime version (recorded),
# not on the host.
#
#   scripts/instructions.sh [res] [years] [occupied_pct] [parts]
#
# Finds the smallest fuel budget that completes the run (bisection over precompiled
# runs), for the full run and for setup alone, and appends a JSON line to
# target/instructions.jsonl. Compare before/after a change at the same arguments.
set -euo pipefail
cd "$(dirname "$0")/.."
res=${1:-4}; years=${2:-50}; occupied=${3:-100}; parts=${4:-1}
command -v wasmtime >/dev/null || { echo "needs wasmtime (brew install wasmtime)"; exit 1; }

cargo build -q --release --target wasm32-wasip1 -p oikos-cli
module=target/oikos-fuel.cwasm
wasmtime compile -W fuel=1 target/wasm32-wasip1/release/oikos.wasm -o "$module"
bench=(bench --res "$res" --occupied "$occupied" --parts "$parts" --no-timing)
run() { wasmtime run --allow-precompiled -W "fuel=$1" "$module" "${bench[@]}" --years "$2"; }

# Smallest fuel with which `bench --years $1` completes, i.e. the fuel it consumes.
fuel() {
  local lo=0 hi=1048576 mid
  until run "$hi" "$1" >/dev/null 2>&1; do
    lo=$hi; hi=$((hi * 2))
  done
  while ((hi - lo > 1)); do
    mid=$(((lo + hi) / 2))
    if run "$mid" "$1" >/dev/null 2>&1; then hi=$mid; else lo=$mid; fi
  done
  echo "$hi"
}

out=$(run 1000000000000000 "$years") || { echo "bench failed without a fuel limit"; exit 1; }
agent_steps=$(echo "$out" | sed 's/.*"agent_steps": \([0-9]*\).*/\1/')
setup=$(fuel 0)
total=$(fuel "$years")
[[ $(fuel "$years") == "$total" ]] || { echo "fuel count is not deterministic"; exit 1; }
per=$(awk -v t="$total" -v s="$setup" -v n="$agent_steps" 'BEGIN { printf "%.2f", (t - s) / n }')

rev=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
dirty=$([[ -n "$(git status --porcelain 2>/dev/null)" ]] && echo true || echo false)
mkdir -p target
line="{\"rev\": \"$rev\", \"dirty\": $dirty, \"wasmtime\": \"$(wasmtime --version | cut -d' ' -f2)\", \"res\": $res, \"years\": $years, \"occupied_pct\": $occupied, \"partitions\": $parts, \"agent_steps\": $agent_steps, \"fuel_setup\": $setup, \"fuel_run\": $((total - setup)), \"fuel_per_agent_step\": $per}"
echo "$line" | tee -a target/instructions.jsonl
