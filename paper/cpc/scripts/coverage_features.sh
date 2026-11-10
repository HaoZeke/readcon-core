#!/bin/sh
# Copyright (c) readcon developers
#
# Branch coverage for one report. The feature set is parallel, zstd,
# grammar, python, and chemfiles. chemfiles-from-sources turns the
# chemfiles feature on and compiles libchemfiles, because the prebuilt
# archive faults on the first frame under rust-lld.
#
# cargo llvm-cov --remap-path-prefix rewrites the workspace root in the
# raw JSON. This script does not edit that JSON.
#
# pytest runs against an extension built with the same instrumentation.
# Its profiles are written into the llvm-cov target directory. The report
# reads every profraw file that sits directly in that directory.
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
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/coverage-features}"
if [ -z "${LIBRARY_PATH:-}" ]; then
  for d in /usr/lib/gcc/*/*/; do
    if [ -e "${d}libstdc++.so" ]; then
      export LIBRARY_PATH="$d"
      break
    fi
  done
fi
# clang with a custom --target does not search the gcc C++ headers.
if [ -z "${CPLUS_INCLUDE_PATH:-}" ]; then
  for inc in /usr/include/c++/*; do
    if [ -e "$inc/cassert" ]; then
      multi=/usr/include/x86_64-linux-gnu/c++/$(basename "$inc")
      export CPLUS_INCLUDE_PATH="$inc:$multi"
      break
    fi
  done
fi
if [ -z "${CXX:-}" ] && [ -x /usr/bin/g++ ]; then
  export CC=/usr/bin/gcc
  export CXX=/usr/bin/g++
fi

mkdir -p "$CARGO_TARGET_DIR"
OUT="${COVERAGE_JSON:-$CARGO_TARGET_DIR/coverage.json}"
mkdir -p "$(dirname "$OUT")"
FEATURES=parallel,zstd,grammar,python,chemfiles-from-sources

echo "==> workspace, all targets, merged features"
cargo llvm-cov clean --workspace
set +e
cargo llvm-cov --workspace --all-targets \
  --features "$FEATURES" \
  --branch --no-fail-fast --no-report --remap-path-prefix
RUST_STATUS=$?
set -e
if [ "$RUST_STATUS" -ne 0 ]; then
  echo "workspace tests exited $RUST_STATUS" >&2
fi
if ! find "$CARGO_TARGET_DIR/llvm-cov-target" -maxdepth 1 -name '*.profraw' | grep -q .; then
  echo "the feature build produced no profiles" >&2
  exit 1
fi

PROFDIR="$CARGO_TARGET_DIR/llvm-cov-target"
echo "==> python extension"
cargo llvm-cov show-env --branch --export-prefix > "$CARGO_TARGET_DIR/py-env.sh"
set -a
# shellcheck disable=SC1091
. "$CARGO_TARGET_DIR/py-env.sh"
set +a
export LLVM_PROFILE_FILE="$PROFDIR/py-%p-%m.profraw"
case "$RUSTFLAGS" in
  *remap-path-prefix*) ;;
  *) export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${ROOT}=." ;;
esac
VENV="${READCON_PY_VENV:-$ROOT/.venv-coverage-python}"
if [ ! -x "$VENV/bin/python" ]; then
  python3 -m venv "$VENV"
fi
# shellcheck disable=SC1091
. "$VENV/bin/activate"
python -m pip install -q maturin pytest numpy
maturin develop --features python,zstd
set +e
python -m pytest tests/python -q --tb=line
PY_STATUS=$?
set -e
if [ "$PY_STATUS" -ne 0 ]; then
  echo "pytest exited $PY_STATUS" >&2
fi

EXT=$(find "$VENV" -path '*/site-packages/readcon/*.so' -type f | head -n 1)
if [ -n "$EXT" ]; then
  mkdir -p "$PROFDIR/debug"
  cp "$EXT" "$PROFDIR/debug/libreadcon_core-abcdef0.so"
  chmod +x "$PROFDIR/debug/libreadcon_core-abcdef0.so"
fi

unset RUSTC_WRAPPER || true
unset CARGO_LLVM_COV_TARGET_DIR || true
unset CARGO_LLVM_COV_BUILD_DIR || true
echo "==> report"
cargo llvm-cov report --branch --remap-path-prefix --json --output-path "$OUT"
echo "wrote $OUT"
