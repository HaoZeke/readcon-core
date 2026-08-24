#!/usr/bin/env bash
# Structural gate: packaging/wrapdb/readcon-core.wrap source_hash matches
# the documented v0.14.7 cxx tarball SHA-256.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WRAP="$ROOT/packaging/wrapdb/readcon-core.wrap"
# Documented SHA-256 of readcon-core-cxx-0.14.7.tar.gz
WANT="14710ec007b2131d2c0e13931bf3e7e443ce87627ace2fb21497a05ea0b5df43"
fail=0

die() { echo "ERROR: $*" >&2; fail=1; }
ok() { echo "OK: $*"; }

[[ -f "$WRAP" ]] || { die "missing $WRAP"; echo "check_wrap_hash: FAILED" >&2; exit 1; }

hash_line="$(sed -n 's/^source_hash[[:space:]]*=[[:space:]]*//p' "$WRAP" | head -1)"
got="${hash_line#sha256:}"
got="${got//[[:space:]]/}"

if [[ "$got" == "$WANT" ]]; then
  ok "wrap source_hash matches v0.14.7 cxx tarball $WANT"
else
  die "wrap source_hash is ${got:-<empty>}, want $WANT"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "check_wrap_hash: FAILED" >&2
  exit 1
fi
echo "check_wrap_hash: all checks passed"
