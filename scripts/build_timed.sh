#!/usr/bin/env bash
# Build the timed C binary, Fortran binary, shared library, and Python extension
# from this tree. Path prefixes are remapped so the artefacts do not embed a
# home directory or a workspace directory.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${H2H_PYTHON:-python3}"
OUT="${H2H_BUILD:-$("$PY" -c 'from pathlib import Path; print(Path.home() / "h2h-work" / "r28-build")')}"
mkdir -p "$OUT/wheels"
SYSROOT="$(rustc --print sysroot)"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export CARGO_TARGET_DIR="$OUT/cargo-target"
export RUSTFLAGS="--remap-path-prefix=${ROOT}=/src --remap-path-prefix=${CARGO_HOME}=/cargo --remap-path-prefix=${SYSROOT}=/rustc --remap-path-prefix=${HOME}=/src ${RUSTFLAGS:-}"

cd "$ROOT"
cargo build --release --lib --locked
cp -f "$CARGO_TARGET_DIR/release/libreadcon_core.so" "$OUT/libreadcon_core.so"

MAPS=(
  -ffile-prefix-map="${ROOT}=."
  -fdebug-prefix-map="${ROOT}=."
  -fmacro-prefix-map="${ROOT}=."
  -ffile-prefix-map="${HOME}=."
  -fdebug-prefix-map="${HOME}=."
  -fmacro-prefix-map="${HOME}=."
  -ffile-prefix-map="${OUT}=."
  -fdebug-prefix-map="${OUT}=."
  -fmacro-prefix-map="${OUT}=."
)
gcc -O2 -std=c11 -D_POSIX_C_SOURCE=200809L "${MAPS[@]}" \
  -I "$ROOT/include" \
  "$ROOT/bench/h2h_timed/native/time_read.c" \
  -L"$OUT" -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
  -o "$OUT/h2h_c"
# Relative source names, so the runtime locus strings do not embed a directory prefix.
(
  cd "$ROOT"
  gfortran -O2 -cpp "${MAPS[@]}" -J "$OUT" \
    -c fortran/ReadCon/src/readcon.f90 -o "$OUT/readcon.o"
  gfortran -O2 "${MAPS[@]}" -I "$OUT" \
    bench/h2h_timed/native/time_read.f90 "$OUT/readcon.o" \
    -L"$OUT" -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
    -o "$OUT/h2h_fortran"
)

MATURIN="$(cd "$(dirname "$PY")" && pwd)/maturin"
"$MATURIN" build --release --interpreter "$PY" --out "$OUT/wheels"
WHEEL="$(printf '%s\n' "$OUT"/wheels/readcon-*.whl | head -n 1)"
"$PY" -m pip install --force-reinstall --no-deps --no-index "$WHEEL"

{
  rustc --version
  gcc --version
  gfortran --version
  "$PY" --version
} > "$OUT/compilers.txt"

"$PY" - "$OUT" "$ROOT" "$WHEEL" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

out, root, wheel = map(Path, sys.argv[1:])
home = Path.home().resolve().as_posix()

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()

def rel(path: Path) -> str:
    text = path.resolve().as_posix()
    prefix = home + "/"
    if text.startswith(prefix):
        return text[len(prefix):]
    return text

import readcon
package = Path(readcon.__file__).resolve().parent
extension = sorted(package.glob("*.so"))
if len(extension) != 1:
    raise SystemExit(f"expected one extension, found {extension}")
artefacts = {
    "h2h_c": out / "h2h_c",
    "h2h_fortran": out / "h2h_fortran",
    "libreadcon_core.so": out / "libreadcon_core.so",
    "readcon_wheel": wheel,
    extension[0].name: extension[0],
}
record = {
    "commit": subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip(),
    "git_status": subprocess.check_output(["git", "-C", str(root), "status", "--porcelain=v1"], text=True),
    "compilers": (out / "compilers.txt").read_text(encoding="utf-8"),
    "commands": [
        "cargo build --release --lib --locked",
        "gcc -O2 -std=c11 -D_POSIX_C_SOURCE=200809L -ffile-prefix-map -fdebug-prefix-map -fmacro-prefix-map -I include bench/h2h_timed/native/time_read.c -lreadcon_core -o h2h_c",
        "gfortran -O2 -cpp -ffile-prefix-map -fdebug-prefix-map -fmacro-prefix-map -c fortran/ReadCon/src/readcon.f90 -o readcon.o",
        "gfortran -O2 -ffile-prefix-map -fdebug-prefix-map -fmacro-prefix-map bench/h2h_timed/native/time_read.f90 readcon.o -lreadcon_core -o h2h_fortran",
        "maturin build --release --out wheels",
        "pip install --force-reinstall --no-deps --no-index the built readcon wheel",
    ],
    "remap": "rustc --remap-path-prefix and gcc/gfortran -ffile-prefix-map cover the repository, the cargo home, the rustc sysroot, the build directory, and the home directory",
    "artefacts_before": {
        name: {"path": rel(path), "sha256": sha256(path)}
        for name, path in artefacts.items()
    },
}
text = json.dumps(record, indent=2) + "\n"
(out / "build.json").write_text(text, encoding="utf-8")
print(f"wrote {rel(out / 'build.json')}", flush=True)
PY

if [[ "${H2H_REQUIRE_CLEAN:-0}" == "1" ]]; then
  status="$(git -C "$ROOT" status --porcelain=v1)"
  if [[ -n "$status" ]]; then
    echo "build left the worktree dirty" >&2
    echo "$status" >&2
    exit 1
  fi
fi
