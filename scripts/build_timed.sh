#!/usr/bin/env bash
# Build the timed artefacts from neutral source trees.
# The CI wheel is built inside the manylinux2014 image with rustc 1.99.1.
# The wheel is not edited after the build.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
dir_build="build"
dir_stage="timed-stage"
stage="${H2H_STAGE_ROOT:-$ROOT/$dir_build/$dir_stage}"
PY="${H2H_PYTHON:-python3}"
# Host shells may set this to 1. That adds debuginfo on top of Cargo.toml release.
export CARGO_PROFILE_RELEASE_DEBUG=false
DIGEST="sha256:62eeff30ef134a85ec0f97109e008a319b0315ca5e8a6d754ee771556c02622a"
IMAGE="quay.io/pypa/manylinux2014_x86_64@${DIGEST}"
R27="cb8443d30ebce59833bf350fdf83cfc5bac16578"
PUBLISHED_SHA="e78e7fc4d7290909a24a2d6a582c960aebf395cc23908be9d43513bdb4190110"

mkdir -p "${stage}/head" "${stage}/r27" "${stage}/out"
rm -rf ${stage}/head ${stage}/r27 ${stage}/out
mkdir -p ${stage}/head ${stage}/r27 ${stage}/out/wheels-ci ${stage}/out/wheels-188 ${stage}/out/wheels-r27
git -C "$ROOT" archive HEAD | tar -C ${stage}/head -xf -
git -C "$ROOT" archive "$R27" | tar -C ${stage}/r27 -xf -
HEAD_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
"$PY" - ${stage}/head "head.${HEAD_COMMIT:0:8}" <<'PY'
import pathlib, re, sys
src, label = sys.argv[1:]
root = pathlib.Path(src)
cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
found = re.search(r'(?m)^version = "([^"]+)"', cargo)
if found is None:
    raise SystemExit("crate version missing")
version = found.group(1)
path = root / "pyproject.toml"
text = path.read_text(encoding="utf-8")
new, count = re.subn(
    r'(?m)^version = "[^"]+"',
    f'version = "{version}+{label}"',
    text,
    count=1,
)
if count != 1:
    raise SystemExit("version line missing")
path.write_text(new, encoding="utf-8")
PY
"$PY" - ${stage}/r27 "r27.${R27:0:8}" <<'PY'
import pathlib, re, sys
src, label = sys.argv[1:]
root = pathlib.Path(src)
cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
found = re.search(r'(?m)^version = "([^"]+)"', cargo)
if found is None:
    raise SystemExit("crate version missing")
version = found.group(1)
path = root / "pyproject.toml"
text = path.read_text(encoding="utf-8")
new, count = re.subn(
    r'(?m)^version = "[^"]+"',
    f'version = "{version}+{label}"',
    text,
    count=1,
)
if count != 1:
    raise SystemExit("version line missing")
path.write_text(new, encoding="utf-8")
PY
# The timing drivers live in HEAD. The r2.7 tree is built for its library only.

sudo docker pull "$IMAGE" >/dev/null
printf '%s\n' "$IMAGE" > ${stage}/out/image-digest.txt

sudo docker run --rm \
  -v "${stage}:/work" \
  -w /work/head \
  -e RUSTUP_HOME=/work/rustup \
  -e CARGO_HOME=/work/cargo \
  -e CARGO_TARGET_DIR=/work/target \
  "$IMAGE" bash -lc '
set -euo pipefail
if [[ ! -x /work/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustc ]]; then
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.99.1 --profile minimal
fi
export PATH="/work/cargo/bin:${PATH}"
export CARGO_PROFILE_RELEASE_DEBUG=false
rustc --version
SYSROOT="$(rustc --print sysroot)"
export RUSTFLAGS="-C link-arg=-fuse-ld=bfd --remap-path-prefix=${SYSROOT}=/rustc --remap-path-prefix=/work/cargo=/cargo"
printf "%s\n" "--remap-path-prefix=<sysroot>=/rustc --remap-path-prefix=<cargo-home>=/cargo" > /work/out/rustflags-ci.txt
rustc -vV > /work/out/rustc-ci.txt
PY=""
for candidate in /*/python/cp312-cp312/bin/python3.12 /*/_internal/cpython-3.12*/bin/python3.12; do
  if [[ -f "$candidate" ]]; then
    PY="$candidate"
    break
  fi
done
"$PY" -m pip install -q maturin
"$PY" -m maturin build --release --strip --features python --interpreter "$PY" --out /work/out/wheels-ci
cargo build --release --lib --locked
cp -f /work/target/release/libreadcon_core.so /work/out/libreadcon_core.so
' > "${stage}/out/ci-build.log" 2>&1
sudo chown -R "$(id -u):$(id -g)" "$stage"

host_build() {
  local src="$1"
  local toolchain="$2"
  local cargo_home="$3"
  local target="$4"
  local wheel_dir="$5"
  local flag_file="$6"
  local rustc_file="$7"
  local rustc_bin
  rustc_bin="$(rustup which --toolchain "$toolchain" rustc)"
  local sysroot
  sysroot="$("$rustc_bin" --print sysroot)"
  export CARGO_HOME="$cargo_home"
  export CARGO_TARGET_DIR="$target"
  export RUSTUP_TOOLCHAIN="$toolchain"
  export CARGO_PROFILE_RELEASE_DEBUG=false
  export RUSTFLAGS="-C link-arg=-fuse-ld=bfd --remap-path-prefix=${sysroot}=/rustc --remap-path-prefix=${cargo_home}=/cargo"
  printf '%s\n' "--remap-path-prefix=<sysroot>=/rustc --remap-path-prefix=<cargo-home>=/cargo" > "$flag_file"
  "$rustc_bin" -vV > "$rustc_file"
  (
    cd "$src"
    cargo build --release --lib --locked
    "$PY" -m maturin build --release --strip --features python --interpreter "$PY" --out "$wheel_dir"
  )
}

host_build ${stage}/head 1.88.0 ${stage}/cargo-188 ${stage}/target-188 ${stage}/out/wheels-188 ${stage}/out/rustflags-188.txt ${stage}/out/rustc-188.txt > "${stage}/out/host-188.log" 2>&1
host_build ${stage}/r27 1.88.0 ${stage}/cargo-r27 ${stage}/target-r27 ${stage}/out/wheels-r27 ${stage}/out/rustflags-r27.txt ${stage}/out/rustc-r27.txt > "${stage}/out/host-r27.log" 2>&1

install_target() {
  local wheel="$1"
  local dest="$2"
  rm -rf "$dest"
  mkdir -p "$dest"
  "$PY" -m pip install --no-deps --no-index --target "$dest" "$wheel"
}

CI_WHEEL="$(printf '%s\n' ${stage}/out/wheels-ci/readcon-*.whl | head -n 1)"
W188="$(printf '%s\n' ${stage}/out/wheels-188/readcon-*.whl | head -n 1)"
WR27="$(printf '%s\n' ${stage}/out/wheels-r27/readcon-*.whl | head -n 1)"
install_target "$CI_WHEEL" ${stage}/py-ci > "${stage}/out/install-ci.log" 2>&1
install_target "$W188" ${stage}/py-188 > "${stage}/out/install-188.log" 2>&1
install_target "$WR27" ${stage}/py-r27 > "${stage}/out/install-r27.log" 2>&1

PIN=${stage}/out/readcon-published.txt
cat > "$PIN" <<EOF
readcon==0.18.0 \\
    --hash=sha256:${PUBLISHED_SHA}
EOF
rm -rf ${stage}/py-0180
mkdir -p ${stage}/py-0180
"$PY" -m pip install --no-deps --require-hashes --target ${stage}/py-0180 -r "$PIN" > "${stage}/out/install-published.log" 2>&1
"$PY" -m pip uninstall -y readcon >/dev/null

MAPS=(
  -ffile-prefix-map=${stage}/head=.
  -fdebug-prefix-map=${stage}/head=.
  -fmacro-prefix-map=${stage}/head=.
)
(
  cd ${stage}/head
  gcc -O2 -std=c11 -D_POSIX_C_SOURCE=200809L "${MAPS[@]}" \
    -I include bench/h2h_timed/native/time_read.c \
    -L ${stage}/out -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
    -o ${stage}/out/h2h_c
  gfortran -O2 -cpp "${MAPS[@]}" -J ${stage}/out \
    -c fortran/ReadCon/src/readcon.f90 -o ${stage}/out/readcon.o
  gfortran -O2 "${MAPS[@]}" -I ${stage}/out \
    bench/h2h_timed/native/time_read.f90 ${stage}/out/readcon.o \
    -L ${stage}/out -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
    -o ${stage}/out/h2h_fortran
) > "${stage}/out/native-build.log" 2>&1


"$PY" - "$ROOT" "$stage" "$R27" "$PUBLISHED_SHA" "$CI_WHEEL" "$W188" "$WR27" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

root, stage, r27, published, ci_wheel, w188, wr27 = sys.argv[1:]
base = Path(stage)

def shown(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(Path(root).resolve()).as_posix()
    except ValueError:
        return path.name

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()

def so_in(site: Path) -> Path:
    found = sorted(site.glob("readcon/*.so"))
    if len(found) != 1:
        raise SystemExit(f"extension count in {site}: {found}")
    return found[0]

def meta(commit: str) -> tuple[str, str]:
    tree = subprocess.check_output(["git", "-C", root, "rev-parse", f"{commit}^{{tree}}"], text=True).strip()
    return commit, tree

head = subprocess.check_output(["git", "-C", root, "rev-parse", "HEAD"], text=True).strip()
head_commit, head_tree = meta(head)
r27_commit, r27_tree = meta(r27)
original = "9d61dfdf9968d83d7cbd0ffaf3fa620f59e84ba3de5f6e1becb4bc088271d04e"
r27_so_path = so_in(base / "py-r27")
ci_so_path = so_in(base / "py-ci")
so_188_path = so_in(base / "py-188")
pypi_so_path = so_in(base / "py-0180")
r27_so = sha256(r27_so_path)
record = {
    "container_image": (base / "out/image-digest.txt").read_text(encoding="utf-8").strip(),
    "profile": "release",
    "ci_rustflags": (base / "out/rustflags-ci.txt").read_text(encoding="utf-8").strip(),
    "ci_rustc": (base / "out/rustc-ci.txt").read_text(encoding="utf-8"),
    "rustflags_188": (base / "out/rustflags-188.txt").read_text(encoding="utf-8").strip(),
    "rustc_188": (base / "out/rustc-188.txt").read_text(encoding="utf-8"),
    "commands": [
        "docker run quay.io/pypa/manylinux2014_x86_64@sha256:62eeff30ef134a85ec0f97109e008a319b0315ca5e8a6d754ee771556c02622a maturin build --release --strip --features python",
        "cargo build --release --lib --locked",
        "rustup run 1.88.0 maturin build --release --strip --features python",
        "pip install --require-hashes readcon==0.18.0",
    ],
    "artefacts_before": {},
    "arms": {
        "readcon_extxyz": {
            "role": "HEAD build with the CI toolchain",
            "commit": head_commit,
            "tree": head_tree,
            "rustc": (base / "out/rustc-ci.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": (base / "out/rustflags-ci.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": sha256(ci_so_path),
            "binary_path": shown(ci_so_path),
            "wheel_sha256": sha256(Path(ci_wheel)),
            "wheel_path": shown(Path(ci_wheel)),
        },
        "readcon_extxyz_188": {
            "role": "HEAD build with rustc 1.88.0",
            "commit": head_commit,
            "tree": head_tree,
            "rustc": (base / "out/rustc-188.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": (base / "out/rustflags-188.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": sha256(so_188_path),
            "binary_path": shown(so_188_path),
            "wheel_sha256": sha256(Path(w188)),
            "wheel_path": shown(Path(w188)),
        },
        "readcon_extxyz_r27": {
            "role": "r2.7 commit rebuilt with rustc 1.88.0",
            "commit": r27_commit,
            "tree": r27_tree,
            "rustc": (base / "out/rustc-r27.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": (base / "out/rustflags-r27.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": r27_so,
            "binary_path": shown(r27_so_path),
            "wheel_sha256": sha256(Path(wr27)),
            "wheel_path": shown(Path(wr27)),
            "timed_so_sha256": original,
            "reproduced_timed_so": r27_so == original,
        },
        "readcon_pypi": {
            "role": "published wheel",
            "commit": None,
            "tree": None,
            "binary_sha256": sha256(pypi_so_path),
            "binary_path": shown(pypi_so_path),
            "wheel_sha256": published,
        },
    },
}
for name, path in {
    "h2h_c": base / "out/h2h_c",
    "h2h_fortran": base / "out/h2h_fortran",
    "libreadcon_core.so": base / "out/libreadcon_core.so",
    "readcon_ci_wheel": Path(ci_wheel),
}.items():
    record["artefacts_before"][name] = {"path": shown(path), "sha256": sha256(path)}
(base / "out/build.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print("wrote build.json")
print("r27 reproduced", r27_so == original)
PY

if [[ "${H2H_REQUIRE_CLEAN:-0}" == "1" ]]; then
  status="$(git -C "$ROOT" status --porcelain=v1)"
  if [[ -n "$status" ]]; then
    echo "build left the worktree dirty" >&2
    echo "$status" >&2
    exit 1
  fi
fi
