#!/usr/bin/env bash
# Throughput and strong-scaling benchmark. Appends JSON lines to target/bench.jsonl,
# tagged with host and git revision. Usage: scripts/bench.sh [res] [years]
set -euo pipefail
cd "$(dirname "$0")/.."
res=${1:-6}; years=${2:-20}
cargo build -q --release -p oikos-cli
host=$(rustc -vV | sed -n 's/^host: //p'); rev=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)
mkdir -p target
p=1
while (( p <= cores * 4 )); do
  line=$(./target/release/oikos bench --res "$res" --years "$years" --parts "$p")
  echo "{\"host\": \"$host\", \"rev\": \"$rev\", \"cores\": $cores, \"result\": $line}" | tee -a target/bench.jsonl
  p=$(( p * 2 ))
done
