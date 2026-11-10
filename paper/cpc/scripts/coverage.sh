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
# No lint suppression. A nightly deny stops the run.
unset RUSTFLAGS || true
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/coverage}"

mkdir -p "$CARGO_TARGET_DIR"
OUT="$CARGO_TARGET_DIR/report"
mkdir -p "$OUT"

# cargo llvm-cov report merges only the profraw files that sit directly
# in its target directory. --no-report does not delete the previous run,
# so the doctest run and the external drivers add profiles beside the
# workspace run. Moving those files out of that directory drops them.
echo "==> workspace, all targets, branches"
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-targets \
  --features parallel,zstd,grammar \
  --branch --no-fail-fast --no-report
echo "==> doctests"
cargo llvm-cov --workspace --doc \
  --features parallel,zstd,grammar \
  --branch --no-fail-fast --no-report

PROFDIR="$CARGO_TARGET_DIR/llvm-cov-target"

if [ -f tests/c/ffi_coverage.c ] && [ -f include/readcon-core.h ]; then
  echo "==> C driver against the instrumented static library"
  STATIC=$(find "$CARGO_TARGET_DIR" -name 'libreadcon_core.a' -type f | head -n 1)
  if [ -n "$STATIC" ]; then
    cc -O0 -o "$OUT/ffi_coverage" tests/c/ffi_coverage.c "$STATIC" \
      -Iinclude -lm -lpthread -ldl
    LLVM_PROFILE_FILE="$PROFDIR/ffi-%p.profraw" \
      "$OUT/ffi_coverage" "$ROOT" "$OUT" \
  fi
fi

if command -v fpm >/dev/null 2>&1 && [ -d fortran/ReadCon ]; then
  echo "==> Fortran driver against the instrumented static library"
  if [ -n "${STATIC:-}" ]; then
    STATIC_DIR=$(CDPATH= cd -- "$(dirname "$STATIC")" && pwd)
    (
      cd fortran/ReadCon
      LLVM_PROFILE_FILE="$PROFDIR/fortran-%p.profraw" \
        fpm test --flag "-L$STATIC_DIR" \
        --link-flag "-L$STATIC_DIR -lreadcon_core -ldl -lpthread -lm"
    )
  fi
fi

echo "==> Python extension"
cargo llvm-cov show-env --branch --export-prefix > "$OUT/env.sh"
set -a
# shellcheck disable=SC1090
. "$OUT/env.sh"
set +a
# show-env points the profile at the coverage target root. The report
# reads profiles from the llvm-cov-target directory, so override that one
# variable after the source.
export LLVM_PROFILE_FILE="$PROFDIR/py-%p-%m.profraw"

VENV="${READCON_PY_VENV:-$ROOT/.venv-coverage-python}"
if [ ! -x "$VENV/bin/python" ]; then
  rm -rf "$VENV"
  python3 -m venv "$VENV"
fi
# shellcheck disable=SC1091
. "$VENV/bin/activate"
python -m pip install -U pip -q
python -m pip install maturin pytest numpy ase zstandard -q
maturin develop --features python,zstd
python -m pytest tests/python --tb=short

# The report keeps an object only when its file name is the package
# name plus an optional hex suffix. The extension's own name does not
# match, so copy it to one that does. That is the binary pytest called.
EXT=$(find "$VENV" -path '*/site-packages/readcon/*.so' -type f | head -n 1)
if [ -n "$EXT" ]; then
  mkdir -p "$PROFDIR/debug"
  cp -a "$EXT" "$PROFDIR/debug/libreadcon_core-abcdef0.so"
  chmod +x "$PROFDIR/debug/libreadcon_core-abcdef0.so"
fi

unset RUSTC_WRAPPER || true
unset CARGO_LLVM_COV_TARGET_DIR || true
unset CARGO_LLVM_COV_BUILD_DIR || true
echo "==> report"
cargo llvm-cov report --branch --summary-only | tee "$OUT/summary.txt"
cargo llvm-cov report --branch --lcov --output-path "$OUT/coverage.lcov"
cargo llvm-cov report --branch --json --output-path "$OUT/coverage.json"
echo "wrote $OUT/summary.txt"
