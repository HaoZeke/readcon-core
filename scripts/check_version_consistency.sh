#!/usr/bin/env bash
# Structural gate: language-binding package versions stay in lockstep
# with Cargo.toml [package].version.
#
# Sources: Cargo.toml, pyproject.toml (+ chemfiles extra pin),
# pyproject.chemfiles.toml, meson.build, pixi.toml,
# julia/ReadCon/Project.toml, fortran/ReadCon/fpm.toml, CITATION.cff.
#
# Intentionally lagged bindings go in the ALLOWLIST case below with a reason.
set -euo pipefail
ROOT="${READCON_VERSION_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$ROOT"

toml_version() {
  sed -n 's/^[[:space:]]*version = "\([^"]*\)".*/\1/p' "$1" | head -1
}

meson_version() {
  sed -n "s/^[[:space:]]*version:[[:space:]]*['\"]\\([^'\"]*\\)['\"].*/\\1/p" "$1" | head -1
}

cff_version() {
  awk '
    /^[[:space:]]*cff-version:/ { next }
    /^[[:space:]]*version:[[:space:]]*/ {
      sub(/^[[:space:]]*version:[[:space:]]*/, "")
      gsub(/["'\'']/, "")
      print
      exit
    }
  ' "$1"
}

chemfiles_pin() {
  sed -n 's/.*readcon-chemfiles==\([0-9][0-9.]*\).*/\1/p' "$1" | head -1
}

# Named allowlist: return 0 to skip a label. Empty: every binding must match.
allowlisted() {
  case "$1" in
    # julia/ReadCon/Project.toml) return 0 ;;  # independent Julia registry release
    *) return 1 ;;
  esac
}

CARGO="$(toml_version Cargo.toml)"
if [[ -z "${CARGO}" ]]; then
  echo "ERROR: Cargo.toml: missing package version" >&2
  exit 1
fi

fail=0
check() {
  local label="$1" got="$2"
  if allowlisted "$label"; then
    echo "SKIP: ${label}=${got} (allowlisted)"
    return
  fi
  if [[ -z "${got}" ]]; then
    echo "ERROR: ${label}: missing version" >&2
    fail=1
    return
  fi
  if [[ "${got}" == "${CARGO}" ]]; then
    echo "OK: ${label}=${got}"
  else
    echo "ERROR: ${label}: version ${got} != Cargo.toml ${CARGO}" >&2
    fail=1
  fi
}

check "pyproject.toml" "$(toml_version pyproject.toml)"
check "pyproject.toml[chemfiles]" "$(chemfiles_pin pyproject.toml)"
check "pyproject.chemfiles.toml" "$(toml_version pyproject.chemfiles.toml)"
check "meson.build" "$(meson_version meson.build)"
check "pixi.toml" "$(toml_version pixi.toml)"
check "julia/ReadCon/Project.toml" "$(toml_version julia/ReadCon/Project.toml)"
check "fortran/ReadCon/fpm.toml" "$(toml_version fortran/ReadCon/fpm.toml)"
check "CITATION.cff" "$(cff_version CITATION.cff)"

if [[ "${fail}" -ne 0 ]]; then
  echo "check_version_consistency: FAILED" >&2
  exit 1
fi
echo "check_version_consistency: all bindings match Cargo.toml ${CARGO}"
