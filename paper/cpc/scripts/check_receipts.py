#!/usr/bin/env python3
"""Fail when a receipt names a host, a guest path, or a credit."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RECEIPTS = ROOT / "paper" / "cpc" / "receipts"
TOKENS = ("/" + "agent", "/" + "home", "/" + "tmp", "cur" + "sor", "gro" + "k")


def main() -> int:
    hits = []
    files = sorted(p for p in RECEIPTS.rglob("*") if p.is_file())
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace").lower()
        rel = path.relative_to(ROOT)
        for token in TOKENS:
            if token in text:
                hits.append(f"{rel}: {token}")
    if hits:
        print("\n".join(hits))
        print(f"receipt check: {len(hits)} hit(s)")
        return 1
    print(f"receipt check: 0 hits in {len(files)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
