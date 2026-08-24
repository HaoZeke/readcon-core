#!/usr/bin/env bash
# Install cargo-dist 0.28.0 from the GitHub release tarball after SHA-256 verify.
# Hashes: https://github.com/axodotdev/cargo-dist/releases/download/v0.28.0/sha256.sum
# Version must match dist-workspace.toml cargo-dist-version.
set -euo pipefail

DIST_VERSION="v0.28.0"
DEST="${CARGO_HOME:-$HOME/.cargo}/bin"

verify_sha256() {
  local hash="$1" file="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    echo "${hash}  ${file}" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    echo "${hash}  ${file}" | shasum -a 256 -c -
  else
    python3 -c '
import hashlib, sys
got = hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest()
want = sys.argv[2]
if got != want:
    raise SystemExit(f"SHA-256 mismatch: {got} != {want}")
' "$file" "$hash"
  fi
}

os="$(uname -s)"
arch="$(uname -m)"
libc="gnu"
if [[ -f /lib/ld-musl-x86_64.so.1 || -f /lib/ld-musl-aarch64.so.1 ]]; then
  libc="musl"
fi

triple=""
ext="tar.xz"
case "$os" in
  Linux)
    case "$arch" in
      x86_64) triple="x86_64-unknown-linux-${libc}" ;;
      aarch64|arm64) triple="aarch64-unknown-linux-${libc}" ;;
    esac
    ;;
  Darwin)
    case "$arch" in
      x86_64) triple="x86_64-apple-darwin" ;;
      arm64) triple="aarch64-apple-darwin" ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    triple="x86_64-pc-windows-msvc"
    ext="zip"
    ;;
esac

if [[ -z "$triple" ]]; then
  echo "unsupported cargo-dist host: $os $arch" >&2
  exit 1
fi

# sha256.sum from the v0.28.0 GitHub Release (binary-mode * prefix stripped).
case "$triple" in
  aarch64-apple-darwin) hash="436e9d1e503b106e938ac8e5e8218d5ad12b161430c8a1f874934271a1f869e9" ;;
  aarch64-unknown-linux-gnu) hash="96ac038f1c01a1d3aeed56668c6fb60f9303770d40b3cdfe1c1a5224a2823060" ;;
  aarch64-unknown-linux-musl) hash="31a445ab8a584dc9384c92372045de471d2d214b797f0cf300db3c003c627b02" ;;
  x86_64-apple-darwin) hash="de231817ab627c605f4e8aeca409db164b0b749f57b0df5e37a88ff805109698" ;;
  x86_64-pc-windows-msvc) hash="8d92e7a9542692bbaae85bdb52eee6234627067eb0700841dcb36d89896fd9ca" ;;
  x86_64-unknown-linux-gnu) hash="c5da0fc4e782315e860bf5d1fb5f9a35e0e78c2d61f27662dfb096cf43de12d8" ;;
  x86_64-unknown-linux-musl) hash="0aea6d50e86c0b9d4f91022577df84a96c4ff405cfc317472890662c2af1df35" ;;
  *)
    echo "no pinned SHA-256 for cargo-dist $triple" >&2
    exit 1
    ;;
esac

archive="cargo-dist-${triple}.${ext}"
url="https://github.com/axodotdev/cargo-dist/releases/download/${DIST_VERSION}/${archive}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

curl --retry 5 --retry-delay 2 --proto '=https' --tlsv1.2 -LsSf -o "${work}/${archive}" "$url"
verify_sha256 "$hash" "${work}/${archive}"

tar -xf "${work}/${archive}" -C "$work"
bin=""
while IFS= read -r -d '' f; do
  bin="$f"
  break
done < <(find "$work" -type f \( -name dist -o -name dist.exe \) -print0)
if [[ -z "$bin" ]]; then
  echo "cargo-dist archive missing dist binary" >&2
  exit 1
fi

mkdir -p "$DEST"
if command -v install >/dev/null 2>&1; then
  install -m 755 "$bin" "${DEST}/$(basename "$bin")"
else
  cp -f "$bin" "${DEST}/$(basename "$bin")"
  chmod 755 "${DEST}/$(basename "$bin")"
fi
# cargo-dist 0.28 installs as `dist`; keep a cargo-dist name for older docs.
if [[ "$(basename "$bin")" == "dist" ]]; then
  ln -sfn dist "${DEST}/cargo-dist"
elif [[ "$(basename "$bin")" == "dist.exe" ]]; then
  cp -f "${DEST}/dist.exe" "${DEST}/cargo-dist.exe"
fi

if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "${DEST}" >> "$GITHUB_PATH"
fi
export PATH="${DEST}:${PATH}"
echo "installed cargo-dist ${DIST_VERSION} (${triple}) -> ${DEST}"
