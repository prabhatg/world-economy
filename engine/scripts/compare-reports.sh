#!/usr/bin/env bash
# Compares check-report.json files gathered from several machines. All hashes must
# agree (wasm may be "skipped"). Usage: scripts/compare-reports.sh report1.json report2.json ...
set -euo pipefail
field() { sed -n "s/.*\"$1\": \"\([^\"]*\)\".*/\1/p" "$2"; }
ref=$(field golden_native "$1"); dense=$(field dense_hash "$1"); fail=0
for r in "$@"; do
  host=$(field host "$r"); n=$(field golden_native "$r"); w=$(field golden_wasm "$r"); d=$(field dense_hash "$r")
  ok=OK
  [[ "$n" == "$ref" && "$d" == "$dense" ]] || ok=MISMATCH
  [[ "$w" == "skipped" || "$w" == "$ref" ]] || ok=MISMATCH
  [[ $ok == OK ]] || fail=1
  printf '%-32s native=%.12s wasm=%.12s dense=%.12s %s\n' "$host" "$n" "$w" "$d" "$ok"
done
exit $fail
