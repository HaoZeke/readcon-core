#!/usr/bin/env python3
"""Write the old-to-new replay pairs as a raw receipt.

The pairs are read from a map file already produced by the replay.
This script does not edit prose.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "paper" / "cpc" / "receipts" / "replay-map.txt"


def main() -> int:
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    pairs = []
    for line in src.read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) == 2 and len(parts[0]) == 40 and len(parts[1]) == 40:
            pairs.append(f"{parts[0]} {parts[1]}")
    OUT.write_text("\n".join(pairs) + ("\n" if pairs else ""))
    print(f"replay-map: {len(pairs)} pairs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
