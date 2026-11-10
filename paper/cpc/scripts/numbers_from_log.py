#!/usr/bin/env python3
"""Aggregate a reader-exchange log into logs/NUMBERS.tsv.

    python3 paper/cpc/scripts/numbers_from_log.py \\
        paper/cpc/logs/reader-exchange.log paper/cpc/logs/NUMBERS.tsv

Each sample line contributes one repeat. The median and the interquartile
range are the type 7 percentiles. Memory is the median of the sample MiB.
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np

SAMPLE = re.compile(
    r"sample arm=(\S+) .*seconds=([0-9.eE+-]+) mib=([0-9.eE+-]+) "
    r"matched=(\d+) got=(\d+) requested=(\d+) exit=(\d+)"
)


def type7(values: list[float]) -> tuple[float, float]:
    arr = np.asarray(values, dtype=float)
    med = float(np.percentile(arr, 50, method="linear"))
    iqr = float(
        np.percentile(arr, 75, method="linear")
        - np.percentile(arr, 25, method="linear")
    )
    return med, iqr


def rows(log: Path, quantity: str) -> list[str]:
    groups: dict[str, list[tuple[float, float, int, int, int]]] = defaultdict(list)
    for line in log.read_text().splitlines():
        match = SAMPLE.search(line)
        if not match:
            continue
        arm, seconds, mib, matched, got, requested, code = match.groups()
        groups[arm].append(
            (float(seconds), float(mib), int(requested), int(matched), int(code))
        )
    out = [
        "quantity\tarm\tn\tmedian_s\tiqr_s\tmib\trequested\tmatched\tfailed\tlog"
    ]
    rel = "logs/" + log.name
    for arm in sorted(groups):
        samples = groups[arm]
        med, iqr = type7([row[0] for row in samples])
        mib, _ = type7([row[1] for row in samples])
        requested = samples[0][2]
        matched = samples[0][3]
        failed = sum(row[4] != 0 for row in samples)
        out.append(
            f"{quantity}\t{arm}\t{len(samples)}\t{med:.9e}\t{iqr:.9e}\t"
            f"{mib:.6f}\t{requested}\t{matched}\t{failed}\t{rel}"
        )
    return out


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: numbers_from_log.py LOG OUT", file=sys.stderr)
        return 2
    log, dest = Path(sys.argv[1]), Path(sys.argv[2])
    text = "\n".join(rows(log, "exchange")) + "\n"
    dest.write_text(text)
    print(f"wrote {dest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
