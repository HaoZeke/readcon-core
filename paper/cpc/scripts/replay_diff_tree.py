#!/usr/bin/env python3
"""Diff every replayed tree against its prerebase tree.

The map is the table under "Rebase onto the live main" in AFFECTED.md.
Each row is old sha, new sha, old tree, new tree. For every pair this
prints `git diff-tree -r` from the old tree to the new tree.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
AFFECTED = ROOT / "AFFECTED.md"
HEADING = "## Rebase onto the live main"


def pairs() -> list[tuple[str, str, str, str]]:
    lines = AFFECTED.read_text(encoding="utf-8").splitlines()
    try:
        start = lines.index(HEADING)
    except ValueError as exc:
        raise SystemExit(f"{AFFECTED}: missing {HEADING}") from exc
    rows = []
    seen_header = False
    for line in lines[start + 1 :]:
        if not line.strip():
            if seen_header and rows:
                break
            continue
        if line.startswith("old\t"):
            seen_header = True
            continue
        if not seen_header:
            continue
        parts = line.split("\t")
        if len(parts) != 4:
            raise SystemExit(f"bad map row: {line}")
        rows.append((parts[0], parts[1], parts[2], parts[3]))
    if not rows:
        raise SystemExit(f"{AFFECTED}: the rebase map is empty")
    return rows


def main() -> int:
    found = pairs()
    print(f"pairs {len(found)}")
    for old, new, old_tree, new_tree in found:
        print(f"commit {old} {new}")
        proc = subprocess.run(
            ["git", "-C", str(ROOT), "diff-tree", "-r", "--no-commit-id", old_tree, new_tree],
            check=True,
            capture_output=True,
            text=True,
        )
        sys.stdout.write(proc.stdout)
        if not proc.stdout.endswith("\n"):
            print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
