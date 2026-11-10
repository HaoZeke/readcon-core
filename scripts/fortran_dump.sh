#!/usr/bin/env bash
# gfortran 13 CFG with line numbers, and gcov -a -b -c, for to_c and open_iterator.
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
gfortran --version > "$OUT/gfortran-version.txt"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
gfortran -c -O0 -g -cpp -fdump-tree-cfg-lineno=cfg.txt \
  "$ROOT/fortran/ReadCon/src/readcon.f90"
gfortran -c -g -O0 --coverage -cpp \
  "$ROOT/fortran/ReadCon/src/readcon.f90" \
  "$ROOT/fortran/ReadCon/test/test_iterator_stream.f90"
gfortran -o test_iterator_stream test_iterator_stream.o readcon.o \
  -L"$LIB" -lreadcon_core -ldl -lpthread -lm --coverage
export LD_LIBRARY_PATH="$LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GFORTRAN_ERROR_BACKTRACE=0
./test_iterator_stream
gcov -a -b -c readcon.f90
python3 - "$ROOT" "$HOME" "$OUT" <<'PY'
import sys
from pathlib import Path

root, home, out = sys.argv[1:]
out = Path(out)

def strip(text: str) -> str:
    return text.replace(root + "/", "").replace(home + "/", "")

def function_text(text: str, marker: str) -> str:
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"missing {marker}")
    rest = text[start + len(marker):]
    nxt = rest.find("\n;; Function ")
    end = start + len(marker) + (nxt if nxt >= 0 else len(rest))
    return strip(text[start:end].rstrip() + "\n")

cfg = Path("cfg.txt").read_text(encoding="utf-8", errors="replace")
(out / "to_c.cfg").write_text(
    function_text(cfg, ";; Function to_c ("),
    encoding="utf-8",
)
(out / "open_iterator.cfg").write_text(
    function_text(cfg, ";; Function open_iterator ("),
    encoding="utf-8",
)
gcov = strip(Path("readcon.f90.gcov").read_text(encoding="utf-8", errors="replace"))
(out / "readcon.f90.gcov").write_text(gcov, encoding="utf-8")
print("wrote", out)
PY
