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
# The prebuilt libchemfiles archive SIGFPEs under rust-lld because its
# constructors live in .ctors. chemfiles-from-sources puts them in
# .init_array. CMake's compiler probe also needs libstdc++ on the link
# path, which a non-default --target triple does not search.
if [ -z "${LIBRARY_PATH:-}" ]; then
  for d in /usr/lib/gcc/*/*/; do
    if [ -e "${d}libstdc++.so" ]; then
      export LIBRARY_PATH="$d"
      break
    fi
  done
fi

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
# --no-report cannot be combined with --no-clean. The second run
# therefore starts clean, and the first profiles are copied back
# before the report so both feature sets stay in one merge.
SAVED=$(mktemp -d)
find "$CARGO_TARGET_DIR/llvm-cov-target" -maxdepth 1 -name '*.profraw' -exec cp {} "$SAVED/" \;
cargo llvm-cov --workspace --all-targets \
  --features parallel,zstd,grammar,chemfiles-from-sources \
  --branch --no-fail-fast --no-report \
  || echo "chemfiles tests exited $?" >&2
mkdir -p "$CARGO_TARGET_DIR/llvm-cov-target"
find "$SAVED" -name '*.profraw' -exec cp {} "$CARGO_TARGET_DIR/llvm-cov-target/" \;
rm -rf "$SAVED"

echo "==> report"
cargo llvm-cov report --branch --json --output-path "$OUT"
echo "wrote $OUT"
