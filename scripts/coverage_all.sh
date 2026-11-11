#!/usr/bin/env bash
# One coverage session: Rust library tests with no name filter, the Python
# tests, and the Fortran driver. Receipts stay in the output directory.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-coverage}"
mkdir -p "$OUT"
cd "$ROOT"
dir_build="build"
dir_stage="timed-stage"
dir_target="target"
PY="${H2H_PYTHON:-python3}"
SITE="${H2H_READCON_SITE:-$ROOT/$dir_target/py-wheel/site}"

bash "$ROOT/scripts/coverage_branch.sh" "$OUT/rust"
head_sha="$(git -C "$ROOT" rev-parse HEAD)"
tree_sha="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
{
  printf 'HEAD %s\n' "$head_sha"
  printf 'tree %s\n' "$tree_sha"
  printf 'command %s\n' "python3 -m pytest tests/python -q --tb=line --disable-warnings"
  set +e
  PYTHONPATH="${SITE}${PYTHONPATH:+:$PYTHONPATH}" \
    "$PY" -m pytest tests/python -q --tb=line --disable-warnings \
    > "$OUT/pytest-body.txt" 2>&1
  code=$?
  set -e
  sed "s|$ROOT/||g; s|$HOME/||g" "$OUT/pytest-body.txt"
  rm -f "$OUT/pytest-body.txt"
  printf 'exit %s\n' "$code"
} > "$OUT/pytest.txt"
py_code="$code"
if [[ -n "${READCON_LIB_DIR:-}" || -f "$ROOT/$dir_build/$dir_stage/out/libreadcon_core.so" ]]; then
  READCON_LIB_DIR="${READCON_LIB_DIR:-$ROOT/$dir_build/$dir_stage/out}" \
    bash "$ROOT/scripts/coverage_fortran.sh" "$OUT/fortran"
fi

"$PY" - "$OUT" <<'PY'
import json
import re
import sys
from pathlib import Path

out = Path(sys.argv[1])
text = []
export = out / "rust" / "branch.json"
python_export = out / "rust" / "python.json"
if export.is_file():
    data = json.loads(export.read_text(encoding="utf-8"))
    files = data.get("data", [{}])[0].get("files", [])
    if python_export.is_file():
        extra = json.loads(python_export.read_text(encoding="utf-8"))
        extra_files = extra.get("data", [{}])[0].get("files", [])
        files = [
            item
            for item in files
            if not str(item.get("filename", "")).endswith("python.rs")
        ]
        files.extend(extra_files)
    for item in files:
        name = item.get("filename", "")
        summary = item.get("summary", {})
        lines = summary.get("lines", {})
        branches = summary.get("branches", {})
        line_pct = lines.get("percent")
        branch_pct = branches.get("percent")
        covered = branches.get("covered", 0)
        count = branches.get("count", 0)
        text.append(
            f"{name} lines {line_pct} branches {branch_pct} branch_covered {covered} branch_count {count}"
        )
gcov = list((out / "fortran").glob("*.gcov")) if (out / "fortran").is_dir() else []
for path in gcov:
    body = path.read_text(encoding="utf-8", errors="replace")
    executed = len(re.findall(r"^ *[1-9][0-9]*:", body, re.M))
    missed = len(re.findall(r"^ *#####:", body, re.M))
    total = executed + missed
    pct = (100.0 * executed / total) if total else 0.0
    taken = len(re.findall(r"taken [1-9]", body))
    not_taken = len(re.findall(r"taken 0", body))
    text.append(
        f"{path.name} lines {pct:.1f} executed {executed} missed {missed} branches_taken {taken} branches_not_taken {not_taken}"
    )
(out / "per-file.txt").write_text("\n".join(text) + ("\n" if text else ""), encoding="utf-8")
print("\n".join(text))
PY
echo "wrote coverage"
exit "$py_code"
