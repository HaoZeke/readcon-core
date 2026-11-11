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
case "$OUT" in
  /*) ;;
  *) OUT="$ROOT/$OUT" ;;
esac
LIB="${READCON_LIB_DIR:-$ROOT/target/release}"
if [[ ! -f "$LIB/libreadcon_core.so" ]]; then
  echo "libreadcon_core.so was not found" >&2
  exit 1
fi

dir_target="target"
WORK="$dir_target/fortran-cov-work"
rm -rf "$WORK"
mkdir -p "$WORK"
cd "$ROOT"
gfortran -c -g -O0 --coverage -cpp -J "$WORK" -o "$WORK/readcon.o" \
  fortran/ReadCon/src/readcon.f90
gfortran -c -g -O0 --coverage -cpp -I "$WORK" -o "$WORK/test_iterator_stream.o" \
  fortran/ReadCon/test/test_iterator_stream.f90
gfortran -o "$WORK/test_iterator_stream" "$WORK/test_iterator_stream.o" "$WORK/readcon.o" \
  -L"$LIB" -lreadcon_core -ldl -lpthread -lm --coverage
export LD_LIBRARY_PATH="$LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GFORTRAN_ERROR_BACKTRACE=0
( cd "$WORK" && ./test_iterator_stream )
gcov -b -c -o "$WORK" fortran/ReadCon/src/readcon.f90 fortran/ReadCon/test/test_iterator_stream.f90
gcov -f -b -c -o "$WORK" fortran/ReadCon/src/readcon.f90 > "$WORK/gcov-functions.txt"
cp "$WORK/gcov-functions.txt" "$OUT/gcov-functions.txt"

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

def blocks(text: str) -> dict[str, str]:
    found = {}
    parts = re.split(r"(?=^function )", text, flags=re.M)
    for part in parts:
        match = re.match(r"function __readcon_MOD_(\w+)", part)
        if match:
            found[match.group(1)] = part
    return found

summary = {}
function_text = (out / "gcov-functions.txt").read_text(encoding="utf-8", errors="replace")
for match in re.finditer(
    r"Function '__readcon_MOD_(\w+)'\nLines executed:([0-9.]+)% of (\d+)",
    function_text,
):
    name, percent, total = match.group(1), float(match.group(2)), int(match.group(3))
    executed = int(round(percent / 100.0 * total))
    summary[name] = (executed, total)

rows = []
wanted = {
    "readcon.f90.gcov": "readcon.f90.gcov",
    "test_iterator_stream.f90.gcov": "test_iterator_stream.f90.gcov",
}
found_gcov = {path.name: path for path in Path(".").glob("*.gcov")}
for filename, dest_name in wanted.items():
    match = next((path for name, path in found_gcov.items() if name.endswith(filename)), None)
    if match is None:
        raise SystemExit(f"missing {filename}")
    text = match.read_text(encoding="utf-8", errors="replace")
    (out / dest_name).write_text(text, encoding="utf-8")
    if filename != "readcon.f90.gcov":
        continue
    parsed = blocks(text)
    for name in names:
        body = parsed[name]
        executed, executable = summary[name]
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
rm -f readcon.f90.gcov test_iterator_stream.f90.gcov gcov-functions.txt
echo "wrote fortran coverage"
