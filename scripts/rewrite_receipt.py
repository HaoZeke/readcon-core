#!/usr/bin/env python3
"""Rewrite host paths in a coverage receipt and record the rewrite.

Prefixes come from the environment or from arguments. Nothing in this
file names a host user or a cargo install location.
"""

import os
import sys
from pathlib import Path


def prefix_pairs() -> list[tuple[str, str]]:
    pairs = []
    raw = os.environ.get("H2H_PATH_PREFIXES", "")
    for item in raw.split(","):
        item = item.strip()
        if not item or "=" not in item:
            continue
        old, new = item.split("=", 1)
        old = old.strip()
        new = new.strip()
        if old and new and old != new:
            pairs.append((old, new))
    return pairs


def rewrite(text: str, root: str, pairs: list[tuple[str, str]]) -> tuple[str, list[str]]:
    used = []
    if root and root in text:
        text = text.replace(root + "/", "./")
        text = text.replace(root, ".")
        used.append("repository path -> ./")
    for old, new in pairs:
        if old in text:
            text = text.replace(old, new)
            used.append(f"{new} prefix")
    return text, used


def main() -> None:
    path = Path(sys.argv[1])
    root = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("H2H_REPO_ROOT", "")
    text = path.read_text(encoding="utf-8", errors="replace")
    text, used = rewrite(text, root, prefix_pairs())
    note = "rewrote " + (", ".join(used) if used else "no host path")
    marker = "note " + note
    lines = [line for line in text.splitlines(True) if not line.startswith("note rewrote ")]
    text = "".join(lines)
    if "\nexit " in text:
        head, tail = text.rsplit("\nexit ", 1)
        text = head + "\n" + marker + "\nexit " + tail
    else:
        if not text.endswith("\n"):
            text += "\n"
        text += marker + "\n"
    path.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
