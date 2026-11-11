#!/usr/bin/env bash
# Branch coverage of the next-run path recorder.
# Writes generator_coverage.txt next to the given directory.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-iterator_coverage}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
cd "$ROOT/bench/h2h_timed"
python3 -m coverage run --branch --source=recorded_path -m pytest -q test_recorded_path.py
python3 -m coverage report -m --include='recorded_path.py' > "$OUT/generator_coverage.txt"
echo "wrote $OUT/generator_coverage.txt"
