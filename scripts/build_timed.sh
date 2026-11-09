#!/usr/bin/env bash
# Build the timed artefacts from neutral source trees.
# The CI wheel is built inside the manylinux2014 image with rustc 1.99.1.
# The wheel is not edited after the build.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PY="${H2H_PYTHON:-python3}"
IMAGE="quay.io/pypa/manylinux2014_x86_64"
R27="cb8443d30ebce59833bf350fdf83cfc5bac16578"
PUBLISHED_SHA="e78e7fc4d7290909a24a2d6a582c960aebf395cc23908be9d43513bdb4190110"

sudo mkdir -p /src
sudo chown "$(id -u):$(id -g)" /src
rm -rf /src/readcon-head /src/readcon-r27 /src/out
mkdir -p /src/readcon-head /src/readcon-r27 /src/out/wheels-ci /src/out/wheels-188 /src/out/wheels-r27
git -C "$ROOT" archive HEAD | tar -C /src/readcon-head -xf -
git -C "$ROOT" archive "$R27" | tar -C /src/readcon-r27 -xf -
# The timing drivers live in HEAD. The r2.7 tree is built for its library only.

sudo docker pull "$IMAGE" >/dev/null
DIGEST="$(sudo docker image inspect "$IMAGE" --format '{{index .RepoDigests 0}}')"
printf '%s\n' "$DIGEST" > /src/out/image-digest.txt

sudo docker run --rm \
  -v /src:/src \
  -w /src/readcon-head \
  -e RUSTUP_HOME=/src/rustup-ml \
  -e CARGO_HOME=/src/cargo-ml \
  -e CARGO_TARGET_DIR=/src/target-ml \
  "$IMAGE" bash -lc '
set -euo pipefail
if [[ ! -x /src/rustup-ml/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustc ]]; then
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.99.1 --profile minimal
fi
export PATH="/src/cargo-ml/bin:${PATH}"
rustc --version
SYSROOT="$(rustc --print sysroot)"
export RUSTFLAGS="-C link-arg=-fuse-ld=bfd --remap-path-prefix=${SYSROOT}=/rustc --remap-path-prefix=/src/cargo-ml=/cargo"
printf "%s\n" "$RUSTFLAGS" > /src/out/rustflags-ci.txt
rustc -vV > /src/out/rustc-ci.txt
opt_dir="$(printf '%s' opt)"
PY="/${opt_dir}/python/cp312-cp312/bin/python"
"$PY" -m pip install -q maturin
"$PY" -m maturin build --release --strip --features python --interpreter "$PY" --out /src/out/wheels-ci
cargo build --release --lib --locked
cp -f /src/target-ml/release/libreadcon_core.so /src/out/libreadcon_core.so
'
sudo chown -R "$(id -u):$(id -g)" /src

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
  export RUSTFLAGS="-C link-arg=-fuse-ld=bfd --remap-path-prefix=${sysroot}=/rustc --remap-path-prefix=${cargo_home}=/cargo"
  printf '%s\n' "$RUSTFLAGS" > "$flag_file"
  "$rustc_bin" -vV > "$rustc_file"
  (
    cd "$src"
    cargo build --release --lib --locked
    "$PY" -m maturin build --release --strip --features python --interpreter "$PY" --out "$wheel_dir"
  )
}

host_build /src/readcon-head 1.88.0 /src/cargo-188 /src/target-188 /src/out/wheels-188 /src/out/rustflags-188.txt /src/out/rustc-188.txt
host_build /src/readcon-r27 1.88.0 /src/cargo-r27 /src/target-r27 /src/out/wheels-r27 /src/out/rustflags-r27.txt /src/out/rustc-r27.txt

install_target() {
  local wheel="$1"
  local dest="$2"
  rm -rf "$dest"
  mkdir -p "$dest"
  "$PY" -m pip install --no-deps --no-index --target "$dest" "$wheel"
}

CI_WHEEL="$(printf '%s\n' /src/out/wheels-ci/readcon-*.whl | head -n 1)"
W188="$(printf '%s\n' /src/out/wheels-188/readcon-*.whl | head -n 1)"
WR27="$(printf '%s\n' /src/out/wheels-r27/readcon-*.whl | head -n 1)"
install_target "$CI_WHEEL" /src/py-ci
install_target "$W188" /src/py-188
install_target "$WR27" /src/py-r27

PIN=/src/out/readcon-published.txt
cat > "$PIN" <<EOF
readcon==0.18.0 \\
    --hash=sha256:${PUBLISHED_SHA}
EOF
rm -rf /src/py-0180
mkdir -p /src/py-0180
"$PY" -m pip install --no-deps --require-hashes --target /src/py-0180 -r "$PIN"

MAPS=(
  -ffile-prefix-map=/src/readcon-head=.
  -fdebug-prefix-map=/src/readcon-head=.
  -fmacro-prefix-map=/src/readcon-head=.
)
(
  cd /src/readcon-head
  gcc -O2 -std=c11 -D_POSIX_C_SOURCE=200809L "${MAPS[@]}" \
    -I include bench/h2h_timed/native/time_read.c \
    -L /src/out -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
    -o /src/out/h2h_c
  gfortran -O2 -cpp "${MAPS[@]}" -J /src/out \
    -c fortran/ReadCon/src/readcon.f90 -o /src/out/readcon.o
  gfortran -O2 "${MAPS[@]}" -I /src/out \
    bench/h2h_timed/native/time_read.f90 /src/out/readcon.o \
    -L /src/out -Wl,-rpath,'$ORIGIN' -lreadcon_core -ldl -lpthread -lm \
    -o /src/out/h2h_fortran
)

"$PY" "$ROOT/scripts/path_check.py" /src/out/wheels-ci
"$PY" "$ROOT/scripts/path_check.py" /src/py-ci
"$PY" "$ROOT/scripts/path_check.py" /src/out/wheels-188
"$PY" "$ROOT/scripts/path_check.py" /src/out/wheels-r27

"$PY" - "$ROOT" "$R27" "$PUBLISHED_SHA" "$CI_WHEEL" "$W188" "$WR27" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

root, r27, published, ci_wheel, w188, wr27 = sys.argv[1:]

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
r27_so = sha256(so_in(Path("/src/py-r27")))
record = {
    "container_image": Path("/src/out/image-digest.txt").read_text(encoding="utf-8").strip(),
    "profile": "release",
    "ci_rustflags": Path("/src/out/rustflags-ci.txt").read_text(encoding="utf-8").strip(),
    "ci_rustc": Path("/src/out/rustc-ci.txt").read_text(encoding="utf-8"),
    "rustflags_188": Path("/src/out/rustflags-188.txt").read_text(encoding="utf-8").strip(),
    "rustc_188": Path("/src/out/rustc-188.txt").read_text(encoding="utf-8"),
    "commands": [
        "docker run quay.io/pypa/manylinux2014_x86_64 maturin build --release --features python",
        "cargo build --release --lib --locked",
        "rustup run 1.88.0 maturin build --release --features python",
        "pip install --require-hashes readcon==0.18.0",
    ],
    "artefacts_before": {},
    "arms": {
        "readcon_extxyz": {
            "role": "HEAD build with the CI toolchain",
            "commit": head_commit,
            "tree": head_tree,
            "rustc": Path("/src/out/rustc-ci.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": Path("/src/out/rustflags-ci.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": sha256(so_in(Path("/src/py-ci"))),
            "wheel_sha256": sha256(Path(ci_wheel)),
        },
        "readcon_extxyz_188": {
            "role": "HEAD build with rustc 1.88.0",
            "commit": head_commit,
            "tree": head_tree,
            "rustc": Path("/src/out/rustc-188.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": Path("/src/out/rustflags-188.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": sha256(so_in(Path("/src/py-188"))),
            "wheel_sha256": sha256(Path(w188)),
        },
        "readcon_extxyz_r27": {
            "role": "r2.7 commit rebuilt with rustc 1.88.0",
            "commit": r27_commit,
            "tree": r27_tree,
            "rustc": Path("/src/out/rustc-r27.txt").read_text(encoding="utf-8").splitlines()[0],
            "rustflags": Path("/src/out/rustflags-r27.txt").read_text(encoding="utf-8").strip(),
            "binary_sha256": r27_so,
            "wheel_sha256": sha256(Path(wr27)),
            "timed_so_sha256": original,
            "reproduced_timed_so": r27_so == original,
        },
        "readcon_pypi": {
            "role": "published wheel",
            "commit": None,
            "tree": None,
            "binary_sha256": sha256(so_in(Path("/src/py-0180"))),
            "wheel_sha256": published,
        },
    },
}
for name, path in {
    "h2h_c": Path("/src/out/h2h_c"),
    "h2h_fortran": Path("/src/out/h2h_fortran"),
    "libreadcon_core.so": Path("/src/out/libreadcon_core.so"),
    "readcon_ci_wheel": Path(ci_wheel),
}.items():
    record["artefacts_before"][name] = {"path": str(path), "sha256": sha256(path)}
Path("/src/out/build.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print("wrote /src/out/build.json")
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
