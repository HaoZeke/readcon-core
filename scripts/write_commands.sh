#!/usr/bin/env bash
# Write COMMANDS.txt for the kit. The tip is the first argument, or HEAD.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIP="${1:-$(git -C "$ROOT" rev-parse HEAD)}"
cat <<EOF
# Iterator branch coverage and the Fortran gcov driver.
# Tip ${TIP} is the local head of h2h-extxyz-types.
# The box checkpointed that tip to a fork branch. This workspace did not.

python -m pip install --require-hashes -r requirements.lock
# apt.lock lists gcc, g++, gfortran, python3, xz-utils, binutils, libc6, and make with versions.
# conda.lock records that conda was not used.

# Rust branch coverage. Writes functions.txt, the three branch listings, and branch.json.
scripts/coverage_branch.sh iterator_coverage
# cargo +nightly llvm-cov clean --workspace
# cargo +nightly llvm-cov --branch --lib --no-report -- c_iter --test-threads=1
# llvm-cov report --show-functions
# llvm-cov show --show-branches=count src/c_iter.rs
# llvm-cov show --show-branches=count src/ffi.rs
# llvm-cov show --show-branches=count src/stream.rs
# llvm-cov export --format=text

# Fortran annotations. Writes the two .gcov files and fortran-procedures.txt.
READCON_LIB_DIR=target/release scripts/coverage_fortran.sh iterator_coverage
# gfortran -c -g -O0 --coverage -cpp fortran/ReadCon/src/readcon.f90 fortran/ReadCon/test/test_iterator_stream.f90
# gcov -b -c readcon.f90 test_iterator_stream.f90
# gcov -f -b -c readcon.f90

# Trailer check, which returns 0:
# git -C . log --format=%B d5e77f997438557c72c3b3e360d446310f85d6b7..HEAD | grep -ci -E '^[A-Za-z-]+: '

# Bundle. It exports refs/heads/h2h-extxyz-types and records the complete history.
# git bundle create candidate.bundle refs/heads/h2h-extxyz-types
# git bundle verify candidate.bundle
EOF
