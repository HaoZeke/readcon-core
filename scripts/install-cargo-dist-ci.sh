#!/usr/bin/env bash
# Install cargo-dist 0.28.0 from the GitHub Release after SHA-256 verification.
# Checksums are the per-artifact .sha256 files from
# https://github.com/axodotdev/cargo-dist/releases/tag/v0.28.0
set -euo pipefail

VERSION="0.28.0"
DEST="${CARGO_HOME:-${HOME}/.cargo}/bin"
mkdir -p "${DEST}"

os="$(uname -s)"
arch="$(uname -m)"
triple=""
ext="tar.xz"
sha=""

is_musl() {
  if [[ -e /lib/ld-musl-x86_64.so.1 || -e /lib/ld-musl-aarch64.so.1 ]]; then
    return 0
  fi
  if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
    return 0
  fi
  return 1
}

case "${os}" in
  Linux)
    case "${arch}" in
      x86_64)
        if is_musl; then
          triple="x86_64-unknown-linux-musl"
          sha="0aea6d50e86c0b9d4f91022577df84a96c4ff405cfc317472890662c2af1df35"
        else
          triple="x86_64-unknown-linux-gnu"
          sha="c5da0fc4e782315e860bf5d1fb5f9a35e0e78c2d61f27662dfb096cf43de12d8"
        fi
        ;;
      aarch64)
        if is_musl; then
          triple="aarch64-unknown-linux-musl"
          sha="31a445ab8a584dc9384c92372045de471d2d214b797f0cf300db3c003c627b02"
        else
          triple="aarch64-unknown-linux-gnu"
          sha="96ac038f1c01a1d3aeed56668c6fb60f9303770d40b3cdfe1c1a5224a2823060"
        fi
        ;;
      *)
        echo "unsupported Linux arch for cargo-dist ${VERSION}: ${arch}" >&2
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "${arch}" in
      x86_64)
        triple="x86_64-apple-darwin"
        sha="de231817ab627c605f4e8aeca409db164b0b749f57b0df5e37a88ff805109698"
        ;;
      arm64)
        triple="aarch64-apple-darwin"
        sha="436e9d1e503b106e938ac8e5e8218d5ad12b161430c8a1f874934271a1f869e9"
        ;;
      *)
        echo "unsupported macOS arch for cargo-dist ${VERSION}: ${arch}" >&2
        exit 1
        ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    triple="x86_64-pc-windows-msvc"
    ext="zip"
    sha="8d92e7a9542692bbaae85bdb52eee6234627067eb0700841dcb36d89896fd9ca"
    ;;
  *)
    echo "unsupported OS for cargo-dist ${VERSION}: ${os}" >&2
    exit 1
    ;;
esac

if [[ "${ext}" == "zip" ]]; then
  archive="cargo-dist-${triple}.zip"
else
  archive="cargo-dist-${triple}.tar.xz"
fi

base="https://github.com/axodotdev/cargo-dist/releases/download/v${VERSION}"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

curl --proto '=https' --tlsv1.2 -fsSL -o "${tmp}/${archive}" "${base}/${archive}"

got=""
if command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "${tmp}/${archive}" | awk '{print $1}')"
else
  got="$(shasum -a 256 "${tmp}/${archive}" | awk '{print $1}')"
fi
if [[ "${got}" != "${sha}" ]]; then
  echo "SHA-256 mismatch for ${archive}: got ${got}, expected ${sha}" >&2
  exit 1
fi

if [[ "${ext}" == "zip" ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -o -q -d "${tmp}" "${tmp}/${archive}"
  else
    python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" \
      "${tmp}/${archive}" "${tmp}"
  fi
  src=""
  if [[ -f "${tmp}/dist.exe" ]]; then
    src="${tmp}/dist.exe"
  else
    src="$(find "${tmp}" -name 'dist.exe' -type f | head -n 1)"
  fi
  [[ -n "${src}" ]] || { echo "dist.exe missing from ${archive}" >&2; exit 1; }
  install -m 0755 "${src}" "${DEST}/dist.exe"
  # Workflow cache/upload looks for ~/.cargo/bin/dist
  cp -f "${DEST}/dist.exe" "${DEST}/dist"
else
  tar -xJf "${tmp}/${archive}" -C "${tmp}"
  src=""
  if [[ -f "${tmp}/dist" ]]; then
    src="${tmp}/dist"
  else
    src="$(find "${tmp}" -name dist -type f | head -n 1)"
  fi
  [[ -n "${src}" ]] || { echo "dist missing from ${archive}" >&2; exit 1; }
  install -m 0755 "${src}" "${DEST}/dist"
fi

if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "${DEST}" >> "${GITHUB_PATH}"
fi
echo "installed cargo-dist ${VERSION} (${triple}) to ${DEST}"
