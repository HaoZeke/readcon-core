#!/usr/bin/env bash
# Install rustup-init 1.29.0 from the rustup archive after SHA-256 verify.
# Skips when cargo is already on PATH (GitHub-hosted runners).
# rustup-init hashes: https://static.rust-lang.org/rustup/archive/1.29.0/<triple>/rustup-init.sha256
set -euo pipefail

if command -v cargo >/dev/null 2>&1; then
  echo "cargo already installed; skipping rustup-init"
  exit 0
fi

RUSTUP_VERSION="1.29.0"
# Matches Cargo.toml rust-version (1.88).
TOOLCHAIN="${READCON_RUST_TOOLCHAIN:-1.88.0}"

os="$(uname -s)"
arch="$(uname -m)"
libc="gnu"
if [[ -f /lib/ld-musl-x86_64.so.1 || -f /lib/ld-musl-aarch64.so.1 ]]; then
  libc="musl"
fi

triple=""
case "$os" in
  Linux)
    case "$arch" in
      x86_64) triple="x86_64-unknown-linux-${libc}" ;;
      aarch64|arm64) triple="aarch64-unknown-linux-${libc}" ;;
    esac
    ;;
esac

if [[ -z "$triple" ]]; then
  echo "unsupported rustup-init host: $os $arch (container path is Linux-only)" >&2
  exit 1
fi

case "$triple" in
  x86_64-unknown-linux-gnu) hash="4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10" ;;
  aarch64-unknown-linux-gnu) hash="9732d6c5e2a098d3521fca8145d826ae0aaa067ef2385ead08e6feac88fa5792" ;;
  x86_64-unknown-linux-musl) hash="9cd3fda5fd293890e36ab271af6a786ee22084b5f6c2b83fd8323cec6f0992c1" ;;
  aarch64-unknown-linux-musl) hash="88761caacddb92cd79b0b1f939f3990ba1997d701a38b3e8dd6746a562f2a759" ;;
  *)
    echo "no pinned SHA-256 for rustup-init $triple" >&2
    exit 1
    ;;
esac

verify_sha256() {
  local want="$1" file="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    echo "${want}  ${file}" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    echo "${want}  ${file}" | shasum -a 256 -c -
  else
    python3 -c '
import hashlib, sys
got = hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest()
want = sys.argv[2]
if got != want:
    raise SystemExit(f"SHA-256 mismatch: {got} != {want}")
' "$file" "$want"
  fi
}

url="https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${triple}/rustup-init"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --retry 5 --retry-delay 2 --proto '=https' --tlsv1.2 -LsSf -o "${work}/rustup-init" "$url"
verify_sha256 "$hash" "${work}/rustup-init"
chmod +x "${work}/rustup-init"
"${work}/rustup-init" -y --default-toolchain "$TOOLCHAIN" --profile minimal --no-modify-path

cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "${cargo_bin}" >> "$GITHUB_PATH"
fi
export PATH="${cargo_bin}:${PATH}"
echo "installed rustup ${RUSTUP_VERSION} + toolchain ${TOOLCHAIN} (${triple})"
