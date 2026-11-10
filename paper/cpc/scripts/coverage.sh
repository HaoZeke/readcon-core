#!/bin/sh
# One measurement for line and branch coverage.
#
# Two earlier numbers for ffi.rs are not the same target. A library-only
# run counts the unit-test binary. A library-plus-integration run also
# links a cdylib that nothing calls, and llvm-cov merges that copy's
# zeros over the tested copy, so the same source moves. This script is
# the measurement to keep: workspace, every target, doctests, branch
# coverage, then the C driver and the Python extension.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"

case "$(rustc --version)" in
  *nightly*) ;;
  *)
    echo "branch coverage needs a nightly rustc" >&2
    exit 1
    ;;
esac

unset RUSTC_WRAPPER || true
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:--A dangerous_implicit_autorefs}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/coverage}"

mkdir -p "$CARGO_TARGET_DIR"
OUT="$CARGO_TARGET_DIR/report"
mkdir -p "$OUT"

echo "==> workspace, all targets, doctests, branches"
cargo llvm-cov --workspace --all-targets --doctests \
  --features parallel,zstd,grammar \
  --branch --no-fail-fast --no-report

if [ -f tests/c/ffi_coverage.c ] && [ -f include/readcon-core.h ]; then
  echo "==> C driver against the instrumented static library"
  STATIC=$(find "$CARGO_TARGET_DIR" -name 'libreadcon_core.a' -type f | head -n 1)
  if [ -n "$STATIC" ]; then
    cc -O0 -o "$OUT/ffi_coverage" tests/c/ffi_coverage.c "$STATIC" \
      -Iinclude -lm -lpthread -ldl
    LLVM_PROFILE_FILE="$CARGO_TARGET_DIR/ffi-%p.profraw" \
      "$OUT/ffi_coverage" "$ROOT" "$OUT" \
      || echo "C driver exited $?" >&2
  fi
fi

echo "==> Python extension"
cargo llvm-cov show-env --branch --export-prefix > "$OUT/env.sh"
set -a
# shellcheck disable=SC1090
. "$OUT/env.sh"
set +a

VENV="${READCON_PY_VENV:-$ROOT/.venv-coverage-python}"
if [ ! -d "$VENV" ]; then
  python3 -m venv "$VENV"
fi
# shellcheck disable=SC1091
. "$VENV/bin/activate"
python -m pip install -U pip -q
python -m pip install maturin pytest numpy -q
maturin develop --features python,zstd
python -m pytest tests/python -q --tb=line || echo "pytest exited $?" >&2

echo "==> report"
cargo llvm-cov report --branch --summary-only | tee "$OUT/summary.txt"
cargo llvm-cov report --branch --lcov --output-path "$OUT/coverage.lcov"
cargo llvm-cov report --branch --json --output-path "$OUT/coverage.json"
echo "wrote $OUT/summary.txt"
