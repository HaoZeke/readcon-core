#!/usr/bin/env bash
# Structural gate: PR-triggered workflows must not grant Pages or OIDC
# at workflow scope. Job-level writes stay on trusted publication jobs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WF_DIR="$ROOT/.github/workflows"
fail=0

die() { echo "ERROR: $*" >&2; fail=1; }
ok() { echo "OK: $*"; }

# Body of a top-level YAML key (inline remainder plus indented children).
top_level_block() {
  local file="$1" key="$2"
  awk -v key="$key" '
    BEGIN { collecting = 0 }
    /^[[:space:]]*#/ && !collecting { next }
    /^[A-Za-z0-9_-]+:/ {
      split($0, parts, ":")
      if (collecting) exit
      if (parts[1] == key) {
        collecting = 1
        rest = $0
        sub(/^[^:]+:[[:space:]]*/, "", rest)
        if (rest != "") print rest
        next
      }
    }
    collecting { print }
  ' "$file"
}

# pull_request / pull_request_target as an `on:` trigger, not a later if:.
on_is_pr_triggered() {
  local on="$1"
  echo "$on" | grep -qE '^[[:space:]]*pull_request(-target)?[[:space:]]*:' && return 0
  echo "$on" | grep -qE '^[[:space:]]*-[[:space:]]*['\''"]?pull_request(-target)?['\''"]?[[:space:]]*$' && return 0
  echo "$on" | grep -qE '(^|[[:space:]\[,])pull_request(-target)?($|[[:space:]\,\]])' && return 0
  return 1
}

perm_has_write() {
  local perms="$1" name="$2"
  echo "$perms" | grep -qE "^[[:space:]]*[\"']?${name}[\"']?[[:space:]]*:[[:space:]]*[\"']?write[\"']?"
}

[[ -d "$WF_DIR" ]] || die "missing $WF_DIR"

shopt -s nullglob
for wf in "$WF_DIR"/*.yml "$WF_DIR"/*.yaml; do
  base="$(basename "$wf")"
  on="$(top_level_block "$wf" on)"
  perms="$(top_level_block "$wf" permissions)"
  if on_is_pr_triggered "$on"; then
    bad=0
    if perm_has_write "$perms" "pages"; then
      die "$base: workflow-scope pages: write on a pull_request workflow"
      bad=1
    fi
    if perm_has_write "$perms" "id-token"; then
      die "$base: workflow-scope id-token: write on a pull_request workflow"
      bad=1
    fi
    if [[ "$bad" -eq 0 ]]; then
      ok "$base: PR trigger has no workflow-scope pages/id-token write"
    fi
  else
    ok "$base: not PR-triggered"
  fi
done
shopt -u nullglob

if [[ "$fail" -ne 0 ]]; then
  echo "check_workflow_permissions: FAILED" >&2
  exit 1
fi
echo "check_workflow_permissions: all checks passed"
