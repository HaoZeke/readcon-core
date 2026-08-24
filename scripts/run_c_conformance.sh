#!/usr/bin/bash
# Compile and run tests/c/test_conformance_goldens.c against a release cdylib.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export READCON_CORE_ROOT="$ROOT"
FEATURES="${READCON_C_FEATURES:-}"
for attempt in 1 2 3; do
  if [[ -n "$FEATURES" ]]; then
    if cargo build --release --features "${FEATURES}"; then
      break
    fi
  else
    if cargo build --release; then
      break
    fi
  fi
  if [[ "$attempt" -eq 3 ]]; then
    exit 1
  fi
  echo "cargo build failed (attempt $attempt), retrying..." >&2
  sleep 5
done
export LD_LIBRARY_PATH="$ROOT/target/release:${LD_LIBRARY_PATH:-}"
OUT="$ROOT/target/test_c_conformance_goldens"
cc -std=c99 -O2 "$ROOT/tests/c/test_conformance_goldens.c" \
  -I"$ROOT/include" \
  -L"$ROOT/target/release" \
  -Wl,-rpath,"$ROOT/target/release" \
  -lreadcon_core -ldl -lpthread -lm \
  -o "$OUT"
"$OUT"
