#!/usr/bin/env bash
# Branch coverage for the iterator sources.
#
# Runs `cargo +nightly llvm-cov --branch` on the iterator, stream, and
# open_decoded tests, then writes llvm-cov's own reports:
#   functions.txt       report
#   show-branches.txt   show --show-branches=count
#   branch.json         export of the measured sources
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${1:-iterator_coverage}"
mkdir -p "$OUT"

dir_target="target"
dir_cov="llvm-cov-target"
dir_src="src"
export CARGO_TARGET_DIR="$dir_target/$dir_cov"
export RUSTFLAGS="--remap-path-prefix=${ROOT}=."
unset RUSTC_WRAPPER || true

rm -rf "$CARGO_TARGET_DIR"
cargo +nightly llvm-cov clean --workspace
set +e
cargo +nightly llvm-cov --branch --features python,zstd --lib --tests --no-report -- --test-threads=1 >"$OUT/cargo-test-body.txt" 2>&1
cargo_code=$?
set -e
{
  printf 'HEAD %s\n' "$(git -C "$ROOT" rev-parse HEAD)"
  printf 'tree %s\n' "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
  printf 'command %s\n' "cargo +nightly llvm-cov --branch --features python,zstd --lib --tests --no-report"
  printf 'note %s\n' "tool text is copied unchanged; a later note line lists any absolute paths"
  cat "$OUT/cargo-test-body.txt"
  printf 'exit %s\n' "$cargo_code"
} > "$OUT/cargo-test.log"
rm -f "$OUT/cargo-test-body.txt"
if [[ "$cargo_code" -ne 0 ]]; then
  exit "$cargo_code"
fi

PY="${H2H_PYTHON:-python3}"
wheel_dir="$ROOT/$dir_target/py-wheel"
rm -rf "$wheel_dir"
mkdir -p "$wheel_dir/wheels" "$wheel_dir/site"
(
  export CARGO_TARGET_DIR="$wheel_dir/target"
  export RUSTFLAGS="-C instrument-coverage -Z coverage-options=branch --remap-path-prefix=${ROOT}=."
  export RUSTUP_TOOLCHAIN=nightly
  export LLVM_PROFILE_FILE="$wheel_dir/pytest-%p.profraw"
  "$PY" -m maturin build --features python --interpreter "$PY" -o "$wheel_dir/wheels"
  wheel="$(printf '%s\n' "$wheel_dir"/wheels/*.whl | head -n 1)"
  "$PY" -m pip install --no-deps --no-index --target "$wheel_dir/site" "$wheel"
)
rm -f "$CARGO_TARGET_DIR"/pytest-*.profraw
SO="$(find "$wheel_dir/site" -name '*.so' -type f -print -quit || true)"
head_sha="$(git -C "$ROOT" rev-parse HEAD)"
tree_sha="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
set +e
PYTHONPATH="$wheel_dir/site${PYTHONPATH:+:$PYTHONPATH}" \
  LLVM_PROFILE_FILE="$wheel_dir/pytest-%p.profraw" \
  "$PY" -m pytest tests/python -q --tb=line --disable-warnings --rootdir=. \
  > "$OUT/pytest-body.txt" 2>&1
code=$?
set -e
{
  printf 'HEAD %s\n' "$head_sha"
  printf 'tree %s\n' "$tree_sha"
  printf 'command %s\n' "python3 -m pytest tests/python -q --tb=line --disable-warnings --rootdir=."
  printf 'note %s\n' "tool text is copied unchanged; a later note line lists any absolute paths"
  cat "$OUT/pytest-body.txt"
  printf 'exit %s\n' "$code"
} > "$OUT/pytest.txt"
rm -f "$OUT/pytest-body.txt"
rm -f "$CARGO_TARGET_DIR"/pytest-*.profraw
mapfile -t PYRAW < <(find "$wheel_dir" -name 'pytest-*.profraw' -type f)

SYSROOT="$(rustc +nightly --print sysroot)"
LLVM="$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/bin"
PROFDATA="$OUT/merged.profdata"

dir_build="build"
mapfile -t RAW < <(find "$CARGO_TARGET_DIR" -name '*.profraw' -type f ! -path "*/${dir_build}/*")
if [[ ${#RAW[@]} -eq 0 ]]; then
  echo "no profraw" >&2
  exit 1
fi
"$LLVM/llvm-profdata" merge -sparse "${RAW[@]}" -o "$PROFDATA"

BIN="$(find "$CARGO_TARGET_DIR" -type f -path '*/out/readcon_core-*' ! -name '*.*' -printf '%s %p\n' | sort -n | awk '{print $2}' | tail -n 1)"
if [[ -z "$BIN" ]]; then
  echo "no lib test binary" >&2
  exit 1
fi
OBJECTS=("$BIN")

# The build remaps this repository to '.'. show then reads src/ relative to
# the working directory. A path equivalence on top of that remap yields an
# empty listing, so show is run without one.

SOURCES=(
  "$dir_src/c_iter.rs"
  "$dir_src/ffi.rs"
  "$dir_src/stream.rs"
  "$dir_src/compression.rs"
  "$dir_src/extxyz_scan.rs"
  "$dir_src/python.rs"
)

"$LLVM/llvm-cov" report --instr-profile="$PROFDATA" "${OBJECTS[@]}" "${SOURCES[@]}" \
  > "$OUT/functions.txt" || true
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" \
  --ignore-filename-regex='/rustc/|/cargo/registry/' \
  "${OBJECTS[@]}" > "$OUT/show-branches.txt"
"$LLVM/llvm-cov" export --format=text --instr-profile="$PROFDATA" "${OBJECTS[@]}" "${SOURCES[@]}" \
  > "$OUT/branch.json"

if [[ -n "${SO:-}" && ${#PYRAW[@]} -gt 0 ]]; then
  pydata="$wheel_dir/python.profdata"
  "$LLVM/llvm-profdata" merge -sparse "${PYRAW[@]}" -o "$pydata"
  "$LLVM/llvm-cov" export --format=text --instr-profile="$pydata" "$SO" \
    "$dir_src/python.rs" > "$OUT/python.json"
  "$LLVM/llvm-cov" show --show-branches=count --instr-profile="$pydata" \
    --ignore-filename-regex='/rustc/|/cargo/registry/' \
    "$SO" > "$OUT/python-branches.txt"
  rm -f "$pydata"
fi

rm -f "$PROFDATA"
echo "wrote rust coverage"
