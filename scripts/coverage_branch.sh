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
OUT="$(cd "$OUT" && pwd)"

export CARGO_TARGET_DIR="$ROOT/target/llvm-cov-target"
unset RUSTC_WRAPPER || true

cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --branch --lib --no-report -- --test-threads=1 c_iter stream::tests open_decoded

SYSROOT="$(rustc +nightly --print sysroot)"
LLVM="$SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/bin"
PROFDATA="$(mktemp)"

mapfile -t RAW < <(find "$CARGO_TARGET_DIR" -name '*.profraw' -type f ! -path '*/build/*')
if [[ ${#RAW[@]} -eq 0 ]]; then
  echo "no profraw under $CARGO_TARGET_DIR" >&2
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
  echo "no lib test binary under $CARGO_TARGET_DIR" >&2
  exit 1
fi

strip_paths() {
  sed "s|$ROOT/||g; s|$HOME/||g"
}

SOURCES=("$ROOT/src/c_iter.rs" "$ROOT/src/ffi.rs" "$ROOT/src/stream.rs" "$ROOT/src/compression.rs")

"$LLVM/llvm-cov" report --show-functions --instr-profile="$PROFDATA" "$BIN" "${SOURCES[@]}" \
  | strip_paths > "$OUT/functions.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "$BIN" \
  "$ROOT/src/c_iter.rs" | strip_paths > "$OUT/c_iter-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "$BIN" \
  "$ROOT/src/ffi.rs" | strip_paths > "$OUT/ffi-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "$BIN" \
  "$ROOT/src/stream.rs" | strip_paths > "$OUT/stream-branches.txt"
"$LLVM/llvm-cov" show --show-branches=count --instr-profile="$PROFDATA" "$BIN" \
  "$ROOT/src/compression.rs" | strip_paths > "$OUT/compression-branches.txt"
"$LLVM/llvm-cov" export --format=text --instr-profile="$PROFDATA" "$BIN" "${SOURCES[@]}" \
  | strip_paths > "$OUT/branch.json"

rm -f "$PROFDATA"
echo "wrote $OUT"
