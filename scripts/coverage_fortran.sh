#!/usr/bin/env bash
# gcov -b -c for the Fortran iterator driver.
#
# Writes readcon.f90.gcov, test_iterator_stream.f90.gcov, and
# fortran-procedures.txt (lines executed, branches taken/total, calls returned).
# READCON_LIB_DIR must contain libreadcon_core.so.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-iterator_coverage}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
LIB="${READCON_LIB_DIR:-$ROOT/target/release}"
if [[ ! -f "$LIB/libreadcon_core.so" ]]; then
  echo "libreadcon_core.so not found in $LIB" >&2
  exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
gfortran -c -g -O0 --coverage -cpp \
  "$ROOT/fortran/ReadCon/src/readcon.f90" \
  "$ROOT/fortran/ReadCon/test/test_iterator_stream.f90"
gfortran -o test_iterator_stream test_iterator_stream.o readcon.o \
  -L"$LIB" -lreadcon_core -ldl -lpthread -lm --coverage
export LD_LIBRARY_PATH="$LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GFORTRAN_ERROR_BACKTRACE=0
./test_iterator_stream
gcov -b -c readcon.f90 test_iterator_stream.f90

python3 - "$ROOT" "$HOME" "$OUT" <<'PY'
import re
import sys
from pathlib import Path

root, home, out = sys.argv[1:]
out = Path(out)
names = (
    "to_c",
    "open_iterator",
    "it_valid",
    "it_next",
    "it_forward",
    "it_skip",
    "it_nth",
    "it_free",
)

def strip_text(text: str) -> str:
    text = text.replace(root + "/", "")
    if home:
        text = text.replace(home + "/", "")
    return text

def blocks(text: str) -> dict[str, str]:
    found = {}
    parts = re.split(r"(?=^function )", text, flags=re.M)
    for part in parts:
        match = re.match(r"function __readcon_MOD_(\w+)", part)
        if match:
            found[match.group(1)] = part
    return found

rows = []
for filename in ("readcon.f90.gcov", "test_iterator_stream.f90.gcov"):
    raw = Path(filename).read_text(encoding="utf-8", errors="replace")
    text = strip_text(raw)
    (out / filename).write_text(text, encoding="utf-8")
    if filename != "readcon.f90.gcov":
        continue
    parsed = blocks(text)
    for name in names:
        body = parsed[name]
        executable = 0
        executed = 0
        for line in body.splitlines():
            head = line[:16]
            if re.match(r"\s*(#####|\d+):\s+\d+:", line):
                executable += 1
                if not head.lstrip().startswith("#####"):
                    executed += 1
        branches = re.findall(r"^branch\s+\d+\s+(taken \d+|never executed)", body, re.M)
        taken = 0
        for item in branches:
            if item.startswith("taken "):
                count = int(item.split()[1])
                if count > 0:
                    taken += 1
        calls = re.findall(r"^call\s+\d+\s+(returned \d+|never executed)", body, re.M)
        returned = sum(1 for item in calls if item.startswith("returned ") and int(item.split()[1]) > 0)
        rows.append((name, executed, executable, taken, len(branches), returned, len(calls)))

lines = [
    "procedure       lines_executed  branches_taken  calls_returned",
]
for name, executed, executable, taken, branches, returned, calls in rows:
    lines.append(
        f"{name:<16} {executed}/{executable:<14} {taken}/{branches:<14} {returned}/{calls}"
    )
(out / "fortran-procedures.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
print("\n".join(lines))
PY
echo "wrote $OUT"
