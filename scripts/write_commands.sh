#!/usr/bin/env bash
# Print the commands for this branch. The tip is the first argument, or HEAD.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIP="${1:-$(git -C "$ROOT" rev-parse HEAD)}"
REF="${2:-fix/h2h-independent-bench}"
cat <<EOF
# Tip ${TIP} is the local head of ${REF}.

python -m pip install --require-hashes -r bench/requirements.lock

# Build the timed wheels and extensions, then run the session.
# The repeat count is 10.
H2H_REPEATS=10 scripts/run_timings.sh

# Session drift against the baseline results.
python3 scripts/session_drift.py drift/baseline-results.json results.json --old-extension drift/baseline-188.so --new-extension timed/readcon-188.so --log samples.tar --expect drift/SHA256 --label later --baseline earlier

# Coverage of the Rust library, the Python tests, and readcon.f90.
scripts/coverage_all.sh coverage

# Bundle.
# git bundle create candidate.bundle refs/heads/${REF}
# git bundle verify candidate.bundle
EOF
