#!/usr/bin/env bash
# Branch coverage for the iterator sources.
#
# Runs `cargo +nightly llvm-cov --branch` on the iterator, stream, and
# open_decoded tests, then writes llvm-cov's own reports:
#   functions.txt            report --show-functions
#   c_iter-branches.txt      show --show-branches=count src/c_iter.rs
#   ffi-branches.txt         show --show-branches=count src/ffi.rs
#   stream-branches.txt      show --show-branches=count src/stream.rs
#   compression-branches.txt show --show-branches=count src/compression.rs
#   branch.json              export of those files
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${1:-iterator_coverage}"
mkdir -p "$OUT"

dir_target="target"
dir_cov="llvm-cov-target"
dir_src="src"
export CARGO_TARGET_DIR="$ROOT/$dir_target/$dir_cov"
unset RUSTC_WRAPPER || true

cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --branch --features python --lib --tests --no-report -- --test-threads=1

PY="${H2H_PYTHON:-python3}"
wheel_dir="$ROOT/$dir_target/py-wheel"
rm -rf "$wheel_dir"
mkdir -p "$wheel_dir/wheels" "$wheel_dir/site"
(
  export CARGO_TARGET_DIR="$wheel_dir/target"
  export RUSTFLAGS="-C instrument-coverage"
  export LLVM_PROFILE_FILE="$wheel_dir/pytest-%p.profraw"
  "$PY" -m maturin build --features python --interpreter "$PY" -o "$wheel_dir/wheels"
  wheel="$(printf '%s\n' "$wheel_dir"/wheels/*.whl | head -n 1)"
  "$PY" -m pip install --no-deps --no-index --target "$wheel_dir/site" "$wheel"
)
mapfile -t PYRAW < <(find "$wheel_dir" -name 'pytest-*.profraw' -type f)
if [[ ${#PYRAW[@]} -gt 0 ]]; then
  cp -f "${PYRAW[@]}" "$CARGO_TARGET_DIR/"
fi
SO="$(find "$wheel_dir/site" -name '*.so' -type f -print -quit || true)"
head_sha="$(git -C "$ROOT" rev-parse HEAD)"
tree_sha="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
{
  printf 'HEAD %s\n' "$head_sha"
  printf 'tree %s\n' "$tree_sha"
  printf 'command %s\n' "python3 -m pytest tests/python -q --tb=line"
  set +e
  PYTHONPATH="$wheel_dir/site${PYTHONPATH:+:$PYTHONPATH}" \
    LLVM_PROFILE_FILE="$wheel_dir/pytest-%p.profraw" \
    "$PY" -m pytest tests/python -q --tb=line
  code=$?
  set -e
  printf 'exit %s\n' "$code"
} > "$OUT/pytest.txt"
mapfile -t PYRAW < <(find "$wheel_dir" -name 'pytest-*.profraw' -type f)
if [[ ${#PYRAW[@]} -gt 0 ]]; then
  cp -f "${PYRAW[@]}" "$CARGO_TARGET_DIR/"
fi

SYSROOT="$(rustc +nightly --print sysroot)"
LLVM="$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/bin"
PROFDATA="$(mktemp)"

dir_build="build"
mapfile -t RAW < <(find "$CARGO_TARGET_DIR" -name '*.profraw' -type f ! -path "*/${dir_build}/*")
if [[ ${#RAW[@]} -eq 0 ]]; then
  echo "no profraw" >&2
  exit 1
fi
"$LLVM/llvm-profdata" merge -sparse "${RAW[@]}" -o "$PROFDATA"

BIN=""
while IFS= read -r candidate; do
  if [[ -n "$candidate" && -x "$candidate" && -f "$candidate" ]]; then
    BIN="$candidate"
  fi
done < <(find "$CARGO_TARGET_DIR" -type f -name 'readcon_core-*' ! -name '*.*' -printf '%T@ %p\n' | sort -n | awk '{print $2}')
if [[ -z "$BIN" ]]; then
  echo "no lib test binary" >&2
  exit 1
fi
OBJECTS=("$BIN")
if [[ -n "${SO:-}" ]]; then
  OBJECTS+=("$SO")
fi

strip_paths() {
  sed "s|$ROOT/||g; s|$HOME/||g; s|$SYSROOT/||g"
}

SOURCES=(
  "$ROOT/$dir_src/c_iter.rs"
  "$ROOT/$dir_src/ffi.rs"
  "$ROOT/$dir_src/stream.rs"
  "$ROOT/$dir_src/compression.rs"
  "$ROOT/$dir_src/extxyz_scan.rs"
  "$ROOT/$dir_src/python.rs"
)

"$LLVM/llvm-cov" report --show-functions --instr-profile="$PROFDATA" "${OBJECTS[@]}" "${SOURCES[@]}" \
  | strip_paths > "$OUT/functions.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/c_iter.rs" | strip_paths > "$OUT/c_iter-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/ffi.rs" | strip_paths > "$OUT/ffi-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/stream.rs" | strip_paths > "$OUT/stream-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/compression.rs" | strip_paths > "$OUT/compression-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/extxyz_scan.rs" | strip_paths > "$OUT/extxyz_scan-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "${OBJECTS[@]}" \
  "$ROOT/$dir_src/python.rs" | strip_paths > "$OUT/python-branches.txt"
"$LLVM/llvm-cov" export --format=text --instr-profile="$PROFDATA" "${OBJECTS[@]}" "${SOURCES[@]}" \
  | strip_paths > "$OUT/branch.json"

rm -f "$PROFDATA"
echo "wrote rust coverage"
