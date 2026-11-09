#!/usr/bin/env bash
# Write COMMANDS.txt for the kit. The tip is the first argument, or HEAD.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIP="${1:-$(git -C "$ROOT" rev-parse HEAD)}"
cat <<EOF
# Iterator branch coverage and the Fortran gcov driver.
# Tip ${TIP} is the local head of h2h-extxyz-types.
# A push needs approval after the box check.

python -m pip install --require-hashes -r requirements.lock
# apt.lock lists gcc, g++, gfortran, python3, xz-utils, binutils, libc6, and make with versions.
# conda.lock records that conda was not used.

# Rust branch coverage. Writes functions.txt, the branch listings, and branch.json.
scripts/coverage_branch.sh iterator_coverage
# cargo +nightly llvm-cov clean --workspace
# cargo +nightly llvm-cov --branch --lib --no-report -- --test-threads=1 c_iter stream::tests open_decoded
# llvm-cov report --show-functions
# llvm-cov show --show-branches=count src/c_iter.rs
# llvm-cov show --show-branches=count src/ffi.rs
# llvm-cov show --show-branches=count src/stream.rs
# llvm-cov show --show-branches=count src/compression.rs
# llvm-cov export --format=text

# Timings. The build runs first, then the bench scripts produce results.json.
scripts/run_timings.sh

# Fortran annotations. Writes the two .gcov files and fortran-procedures.txt.
READCON_LIB_DIR=target/release scripts/coverage_fortran.sh iterator_coverage
# gfortran -c -g -O0 --coverage -cpp fortran/ReadCon/src/readcon.f90 fortran/ReadCon/test/test_iterator_stream.f90
# gcov -b -c readcon.f90 test_iterator_stream.f90
# gcov -f -b -c readcon.f90

# CFG dump with line numbers, plus gcov -a -b -c.
scripts/fortran_dump.sh iterator_coverage
# gfortran --version
# gfortran -c -O0 -g -cpp -fdump-tree-cfg-lineno=cfg.txt fortran/ReadCon/src/readcon.f90
# gcov -a -b -c readcon.f90

# Coverage of the next-run path recorder. Separate from coverage.txt.
scripts/coverage_generator.sh iterator_coverage

# Trailer check, which returns 0:
# git -C . log --format=%B d5e77f997438557c72c3b3e360d446310f85d6b7..HEAD | grep -ci -E '^[A-Za-z-]+: '

# Bundle. It exports refs/heads/h2h-extxyz-types and records the complete history.
# git bundle create candidate.bundle refs/heads/h2h-extxyz-types
# git bundle verify candidate.bundle
EOF
