# Development environment for the Oikos engine. Source it (bash or zsh) from anywhere:
#
#   source engine/scripts/env.sh
#
# Puts rustup's cargo on PATH, creates or refreshes engine/.venv from the pinned
# requirements.txt (only when they changed), and exports OIKOS_PYTHON so the gate's
# output check uses it. Warns about missing tools instead of failing: it runs in your
# interactive shell, so it never calls `exit` or `set -e`.

if [ -n "${ZSH_VERSION:-}" ]; then
  eval '_oikos_src=${(%):-%x}'
else
  _oikos_src=${BASH_SOURCE[0]}
fi
_oikos_engine=$(cd "$(dirname "$_oikos_src")/.." && pwd)

_oikos_setup() {
  local engine=$1 venv=$1/.venv req=$1/requirements.txt

  if ! command -v cargo >/dev/null && [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
  command -v rustup >/dev/null ||
    echo "oikos: rustup not found; install it from https://rustup.rs (the toolchain is pinned in rust-toolchain.toml)"
  command -v wasmtime >/dev/null ||
    echo "oikos: wasmtime not found; the gate will skip wasm tests (brew install wasmtime, or cargo install wasmtime-cli --locked)"

  if ! cmp -s "$req" "$venv/.oikos-requirements.txt"; then
    local py=${OIKOS_BOOTSTRAP_PYTHON:-python3}
    if ! command -v "$py" >/dev/null; then
      echo "oikos: $py not found; the gate will skip the Python output check"
      return 0
    fi
    echo "oikos: installing pinned Python packages into $venv"
    if [ ! -x "$venv/bin/python" ]; then
      "$py" -m venv "$venv" || return 1
    fi
    "$venv/bin/python" -m pip install --quiet --disable-pip-version-check --require-virtualenv -r "$req" &&
      cp "$req" "$venv/.oikos-requirements.txt" || {
      echo "oikos: pip install failed; fix the error above and source env.sh again"
      return 1
    }
  fi
  export OIKOS_PYTHON="$venv/bin/python"
}

_oikos_setup "$_oikos_engine"
unset -f _oikos_setup
unset _oikos_src _oikos_engine
