#!/usr/bin/env bash
# Structural gate: release.yml actions are SHA-pinned; installers are verified.
# Reproducer: rg -n 'uses:.*@(v[0-9]|stable|main|master)' .github/workflows/release.yml
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WF="$ROOT/.github/workflows/release.yml"
fail=0

die() { echo "ERROR: $*" >&2; fail=1; }
ok() { echo "OK: $*"; }

[[ -f "$WF" ]] || { echo "ERROR: missing $WF" >&2; exit 1; }

uses_lines=()
while IFS= read -r line; do
  uses_lines+=("$line")
done < <(grep -E '^[[:space:]]+-?[[:space:]]*uses:' "$WF" || true)

if [[ ${#uses_lines[@]} -eq 0 ]]; then
  die "release.yml has no uses: lines"
fi

sha_re='^[0-9a-f]{40}$'
for line in "${uses_lines[@]}"; do
  ref="${line##*@}"
  ref="${ref%%#*}"
  ref="${ref//[[:space:]]/}"
  if [[ ! "$ref" =~ $sha_re ]]; then
    die "uses: is not a 40-char SHA: $line"
    continue
  fi
  if [[ ! "$line" =~ \#[[:space:]]*v ]]; then
    die "uses: missing readable version comment: $line"
    continue
  fi
  ok "pinned $ref"
done

if grep -nE 'uses:.*@(v[0-9]|stable|main|master)' "$WF"; then
  die "mutable uses: tag remains (want empty rg of @(vN|stable|main|master))"
else
  ok "no mutable uses: tags"
fi

if grep -nE 'cargo-dist-installer\.sh[[:space:]]*\|' "$WF"; then
  die "cargo-dist installer is piped to a shell"
else
  ok "no cargo-dist installer pipe"
fi

if grep -nE 'sh\.rustup\.rs[[:space:]]*\|' "$WF"; then
  die "rustup.sh is piped to a shell"
else
  ok "no rustup.sh pipe"
fi

# Integrity path: checksums in the workflow or in a script it invokes.
scan_files=("$WF")
while IFS= read -r rel; do
  [[ -f "$ROOT/$rel" ]] && scan_files+=("$ROOT/$rel")
done < <(grep -oE 'scripts/[[:alnum:]_./-]+\.sh' "$WF" | sort -u || true)

dist_ok=0
rust_ok=0
for f in "${scan_files[@]}"; do
  if grep -qE 'sha256sum -c|shasum -a 256 -c' "$f" \
    && grep -qE 'cargo-dist-|axodotdev/cargo-dist' "$f"; then
    dist_ok=1
  fi
  if grep -qE 'dtolnay/rust-toolchain@[0-9a-f]{40}' "$f"; then
    rust_ok=1
  fi
  if grep -qE 'sha256sum -c|shasum -a 256 -c' "$f" \
    && grep -qE 'rustup-init|static\.rust-lang\.org/rustup' "$f"; then
    rust_ok=1
  fi
done

if [[ "$dist_ok" -eq 1 ]]; then
  ok "cargo-dist install verifies a SHA-256"
else
  die "cargo-dist install has no SHA-256 verify (checksum or pinned artifact)"
fi

if [[ "$rust_ok" -eq 1 ]]; then
  ok "Rust toolchain install is checksummed or a SHA-pinned action"
else
  die "Rust install has no checksum / pinned toolchain action"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "check_release_workflow_pins: FAILED" >&2
  exit 1
fi
echo "check_release_workflow_pins: all checks passed"
