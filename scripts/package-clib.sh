#!/usr/bin/env bash
# Assemble a prebuilt C ABI tarball: headers + cdylib + pkg-config.
#
# cargo-c copies include/ (Cargo.toml generation=false). cbindgen must not run.
#
# Layout (prefix root, no wrapper directory):
#   include/readcon-core.h
#   include/readcon-core.hpp
#   include/readcon-metatensor.h
#   lib/libreadcon_core.so|dylib  (or lib/readcon_core.dll.lib on Windows)
#   lib/pkgconfig/readcon-core.pc
#   bin/readcon_core.dll          (Windows only)
#
# Usage:
#   scripts/package-clib.sh OUTPUT_DIR [TARGET]
#
# Env:
#   CARGO_C_FEATURES  optional cargo features (empty = lean).
#                     chemfiles is refused when TARGET is Windows.
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 OUTPUT_DIR [TARGET]" >&2
    exit 2
fi

OUTPUT_DIR="$1"
shift
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -1)"
if [[ -z "${VERSION}" ]]; then
    echo "package-clib: could not read version from Cargo.toml" >&2
    exit 1
fi

if [[ $# -ge 1 && -n "${1:-}" ]]; then
    TARGET="$1"
else
    TARGET="$(rustc -vV | awk '/^host:/{print $2}')"
fi
if [[ -z "${TARGET}" ]]; then
    echo "package-clib: rustc host target is empty" >&2
    exit 1
fi

if ! grep -q 'generation = false' "$ROOT_DIR/Cargo.toml"; then
    echo "package-clib: Cargo.toml capi.header.generation must stay false (cbindgen must not run)" >&2
    exit 1
fi

FEATURES="${CARGO_C_FEATURES:-}"
case "$TARGET" in
    *windows*)
        if [[ "${FEATURES}" == *chemfiles* ]]; then
            echo "package-clib: Windows chemfiles is not in this tarball" >&2
            exit 1
        fi
        ;;
esac

if cargo cinstall -h >/dev/null 2>&1; then
    CINSTALL=(cargo cinstall)
elif command -v cargo-cinstall >/dev/null 2>&1; then
    CINSTALL=(cargo-cinstall)
else
    echo "package-clib: cargo-c (cargo cinstall) is required" >&2
    exit 1
fi

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
    fi
}

rewrite_pc() {
    local pc="$1"
    local tmp
    tmp="$(mktemp)"
    awk '
        /^prefix=/ { print "prefix=${pcfiledir}/../.."; next }
        /^exec_prefix=/ { print "exec_prefix=${prefix}"; next }
        /^libdir=/ { print "libdir=${prefix}/lib"; next }
        /^includedir=/ { print "includedir=${prefix}/include"; next }
        { print }
    ' "$pc" > "$tmp"
    mv "$tmp" "$pc"
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
DEST="${TMP_DIR}/prefix"
mkdir -p "$DEST"

cd "$ROOT_DIR"

CINSTALL_ARGS=(
    --release
    --locked
    --library-type cdylib
    --prefix "$DEST"
    --libdir "$DEST/lib"
    --target "$TARGET"
)
if [[ -n "${FEATURES}" ]]; then
    CINSTALL_ARGS+=(--features "${FEATURES}")
fi

"${CINSTALL[@]}" "${CINSTALL_ARGS[@]}"

if [[ ! -f "$DEST/include/readcon-core.h" ]]; then
    echo "package-clib: missing $DEST/include/readcon-core.h" >&2
    exit 1
fi
if ! cmp -s "$ROOT_DIR/include/readcon-core.h" "$DEST/include/readcon-core.h"; then
    echo "package-clib: installed header differs from shipped include/readcon-core.h (cbindgen must not run)" >&2
    diff -u "$ROOT_DIR/include/readcon-core.h" "$DEST/include/readcon-core.h" >&2 || true
    exit 1
fi
if [[ ! -f "$DEST/lib/pkgconfig/readcon-core.pc" ]]; then
    echo "package-clib: missing $DEST/lib/pkgconfig/readcon-core.pc" >&2
    exit 1
fi

LIB_FOUND=0
for cand in \
    "$DEST/lib/libreadcon_core.so" \
    "$DEST/lib/libreadcon_core.dylib" \
    "$DEST/lib/readcon_core.dll" \
    "$DEST/lib/readcon_core.dll.lib" \
    "$DEST/lib/readcon_core.lib" \
    "$DEST/bin/readcon_core.dll"; do
    if [[ -e "$cand" ]]; then
        LIB_FOUND=1
        break
    fi
done
if [[ "$LIB_FOUND" -ne 1 ]]; then
    echo "package-clib: no cdylib under $DEST/lib or $DEST/bin" >&2
    find "$DEST" -type f | sort >&2 || true
    exit 1
fi

rewrite_pc "$DEST/lib/pkgconfig/readcon-core.pc"

ARCHIVE_NAME="readcon-core-clib-${VERSION}-${TARGET}"
TAR_ITEMS=(include lib)
if [[ -d "$DEST/bin" ]]; then
    TAR_ITEMS+=(bin)
fi

tar -C "$DEST" -cf "${TMP_DIR}/${ARCHIVE_NAME}.tar" "${TAR_ITEMS[@]}"
gzip -9 "${TMP_DIR}/${ARCHIVE_NAME}.tar"
cp "${TMP_DIR}/${ARCHIVE_NAME}.tar.gz" "${OUTPUT_DIR}/${ARCHIVE_NAME}.tar.gz"

SHA="$(sha256_file "${OUTPUT_DIR}/${ARCHIVE_NAME}.tar.gz")"
echo "${OUTPUT_DIR}/${ARCHIVE_NAME}.tar.gz"
echo "sha256:${SHA}"
echo "${SHA}" > "${OUTPUT_DIR}/${ARCHIVE_NAME}.tar.gz.sha256"
echo "target:${TARGET}"
echo "Fill julia/ReadCon/Artifacts.toml.in sha256 for ${TARGET} with ${SHA}"
