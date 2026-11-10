#!/bin/sh
# Copyright (c) readcon developers
#
# One branch report from two feature sets. The first run is the lean
# build. The second adds chemfiles and keeps the first profiles.
# cargo llvm-cov report merges every profraw file in the target directory.
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

mkdir -p "$CARGO_TARGET_DIR"
OUT="${COVERAGE_JSON:-$CARGO_TARGET_DIR/coverage.json}"
mkdir -p "$(dirname "$OUT")"

echo "==> lean features"
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-targets \
  --features parallel,zstd,grammar \
  --branch --no-fail-fast --no-report \
  || echo "lean tests exited $?" >&2

echo "==> chemfiles feature, profiles kept"
cargo llvm-cov --workspace --all-targets \
  --features parallel,zstd,grammar,chemfiles \
  --branch --no-fail-fast --no-report --no-clean \
  || echo "chemfiles tests exited $?" >&2

echo "==> report"
cargo llvm-cov report --branch --json --output-path "$OUT"
echo "wrote $OUT"
