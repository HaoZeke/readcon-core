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

# Fortran annotations. Writes the two .gcov files and fortran-procedures.txt.
READCON_LIB_DIR=target/release scripts/coverage_fortran.sh iterator_coverage

# Trailer check, which returns 0:
# git -C . log --format=%B d5e77f997438557c72c3b3e360d446310f85d6b7..HEAD | grep -ci -E '^[A-Za-z-]+: '

# Bundle. It exports refs/heads/h2h-extxyz-types and records the complete history.
# git bundle create candidate.bundle refs/heads/h2h-extxyz-types
# git bundle verify candidate.bundle
EOF
