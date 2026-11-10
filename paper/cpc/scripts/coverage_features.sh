#!/bin/sh
# Copyright (c) readcon developers
#
# Branch coverage for one report. The feature set is parallel, zstd,
# grammar, python, and chemfiles. chemfiles-from-sources turns the
# chemfiles feature on and compiles libchemfiles, because the prebuilt
# archive faults on the first frame under rust-lld.
#
# The python feature builds an extension module, so the test binary does
# not link libpython on its own. This script adds that link, then runs
# pytest against an extension built with the same instrumentation. The
# extension is copied into the llvm-cov target directory. cargo llvm-cov
# report passes every matching object, including that extension, to
# llvm-cov export. This script does not edit the JSON.
#
# cargo llvm-cov --remap-path-prefix rewrites the workspace root.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
export PATH="/usr/local/bin:${PATH}"

case "$(rustc --version)" in
  *nightly*) ;;
  *)
    echo "branch coverage needs a nightly rustc" >&2
    exit 1
    ;;
esac

unset RUSTC_WRAPPER || true
export CARGO_INCREMENTAL=0
# Link libpython. Do not pass a lint allow. Remap is added below.
export RUSTFLAGS="-C link-arg=-lpython3.12"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/coverage-features}"
if [ -z "${LIBRARY_PATH:-}" ]; then
  LIBRARY_PATH=""
fi
for d in /usr/lib/gcc/*/*/; do
  if [ -e "${d}libstdc++.so" ]; then
    LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$d"
    break
  fi
done
if [ -e /usr/lib/x86_64-linux-gnu/libpython3.12.so ]; then
  LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}/usr/lib/x86_64-linux-gnu"
fi
export LIBRARY_PATH
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
# An old cmake_minimum_required resets the install prefix to /usr/local.
# These two files put the build script's prefix back after project().
export CMAKE_TOOLCHAIN_FILE="$ROOT/paper/cpc/scripts/cmake/save-prefix.cmake"
export CMAKE_PROJECT_TOP_LEVEL_INCLUDES="$ROOT/paper/cpc/scripts/cmake/restore-prefix.cmake"
unset CMAKE_INSTALL_PREFIX || true

mkdir -p "$CARGO_TARGET_DIR"
OUT="${COVERAGE_JSON:-$CARGO_TARGET_DIR/coverage.json}"
mkdir -p "$(dirname "$OUT")"
# capi is an empty switch. rpc, metatensor and mpi build on this host.
# mpi uses the system OpenMPI (mpicc, libmpi, libclang).
# cuda is not measurable. cargo test --features cuda --lib
# allocate_cuda_succeeds panics in cudarc 0.13.9 because it cannot load
# libcuda.so or libnvcuda.so. cargo check --features cuda exits 0.
# Not measurable, ruling (e), every cfg(feature = "cuda") arm:
#   src/lib.rs:4
#   src/ffi.rs:2523
#   src/ffi.rs:2609
#   src/ffi.rs:2640
#   src/ffi.rs:4894
#   src/ffi.rs:6383
#   src/types.rs:3620
#   src/array.rs:179
#   src/array.rs:408
#   src/storage_dtype.rs:536
#   src/storage_dtype.rs:779
# src/cuda_array.rs is compiled only from the arm at src/lib.rs:4.
FEATURES=parallel,zstd,grammar,python,chemfiles-from-sources,capi,rpc,metatensor,mpi

echo "==> workspace, all targets, merged features"
cargo llvm-cov clean --workspace
# One thread. A trailing --test-threads flag is rejected by the
# criterion benches, so the harness reads the variable instead.
export RUST_TEST_THREADS=1
cargo llvm-cov --workspace --all-targets \
  --features "$FEATURES" \
  --branch --no-fail-fast --no-report --remap-path-prefix
PROFDIR="$CARGO_TARGET_DIR/llvm-cov-target"
if ! find "$PROFDIR" -maxdepth 1 -name '*.profraw' | grep -q .; then
  echo "the feature build produced no profiles" >&2
  exit 1
fi

echo "==> python extension"
cargo llvm-cov show-env --branch --export-prefix > "$CARGO_TARGET_DIR/py-env.sh"
set -a
# shellcheck disable=SC1091
. "$CARGO_TARGET_DIR/py-env.sh"
set +a
export LLVM_PROFILE_FILE="$PROFDIR/py-%p-%m.profraw"
# Keep the instrumented test binaries. The extension builds elsewhere.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR}/py-ext"
case "$RUSTFLAGS" in
  *remap-path-prefix*) ;;
  *) export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${ROOT}=." ;;
esac
case "$RUSTFLAGS" in
  *link-arg=-lpython3.12*) ;;
  *) export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-lpython3.12" ;;
esac
VENV="${READCON_PY_VENV:-$ROOT/.venv-coverage-python}"
if [ ! -x "$VENV/bin/python" ]; then
  python3 -m venv "$VENV"
fi
# shellcheck disable=SC1091
. "$VENV/bin/activate"
python -m pip install -q maturin pytest numpy h5py threadpoolctl chemparseplot xyzrender rgpycrumbs polars MDAnalysis 'jax[cpu]>=0.4'
link_host_tool() {
  tool=$1
  nightly="${RUSTUP_HOME:-$HOME/.rustup}/toolchains/nightly-x86_64-unknown-linux-gnu/bin"
  src=$(PATH="/usr/local/bin:${nightly}:/usr/bin:/bin" command -v "$tool" || true)
  if [ -n "$src" ]; then
    ln -sfn "$(readlink -f "$src")" "$VENV/bin/$tool"
  fi
}
link_host_tool pdftotext
link_host_tool pandoc
link_host_tool cargo
link_host_tool rustc
echo "==> release library, newer than the sources the probes compare"
cargo build --release --lib --features parallel,zstd,mpi --target-dir "$ROOT/target"
maturin develop --features python,zstd,chemfiles-from-sources
python -m pytest \
  tests/python/test_v143_arms.py \
  tests/python/test_v143_behaviour.py \
  tests/python/test_extension_edges.py \
  tests/python/test_edge_inputs.py \
  tests/python/test_readcon.py \
  tests/python/test_numpy_absent.py \
  tests/python/test_error_results.py \
  tests/python/test_ase.py \
  tests/python/test_ase_physical_units.py \
  tests/python/test_chemfiles_import.py \
  tests/python/test_chemfiles_selection.py \
  tests/python/test_conformance_goldens.py \
  tests/python/test_convert_migrate.py \
  tests/python/test_empty_header_lines.py \
  tests/python/test_index_proj.py \
  tests/python/test_position_slab.py \
  tests/python/test_trajectory.py \
  tests/python/test_tutorial_core.py \
  tests/python/test_tutorial_chemfiles.py \
  tests/python/test_api_coverage.py \
  tests/python/test_line_coverage.py \
  tests/python/test_physical_coverage.py \
  tests/python/test_helpers.py \
  tests/python/test_v144_public.py \
  tests/python/test_paper_cover_a.py \
  tests/python/test_paper_cover_b.py \
  tests/python/test_paper_cover_c.py \
  tests/python/test_paper_cover_d.py \
  tests/python/test_paper_cover_e.py \
  tests/python/test_paper_cover_f.py \
  tests/python/test_paper_cover_g.py \
  tests/python/test_paper_cover_h.py \
  tests/python/test_paper_branches.py \
  tests/python/test_paper_every.py \
  tests/python/test_paper_mains.py \
  tests/python/test_paper_paths.py \
  tests/python/test_indep_reader_paths.py \
  tests/python/test_cpc_helpers.py \
  tests/python/test_bench_host.py \
  tests/python/test_version_gate.py \
  tests/python/test_version_sot.py \
  tests/python/test_check_release_artifacts.py \
  --tb=short

EXT=$(find "$VENV" -path '*/site-packages/readcon/*.so' -type f | head -n 1)
if [ -z "$EXT" ]; then
  echo "the python extension was not installed" >&2
  exit 1
fi
mkdir -p "$PROFDIR/debug"
cp "$EXT" "$PROFDIR/debug/libreadcon_core-abcdef0.so"
chmod +x "$PROFDIR/debug/libreadcon_core-abcdef0.so"

unset RUSTC_WRAPPER || true
unset CARGO_LLVM_COV_TARGET_DIR || true
unset CARGO_LLVM_COV_BUILD_DIR || true
export CARGO_TARGET_DIR="${PROFDIR%/*}"
echo "==> report"
cargo llvm-cov report --branch --remap-path-prefix --json --output-path "$OUT"
echo "wrote $OUT"
