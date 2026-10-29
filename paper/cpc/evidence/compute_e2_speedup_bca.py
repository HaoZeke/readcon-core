#!/usr/bin/env python3
"""BCa intervals for the five-repeat e2 thread-scaling speedups.

Reads ``results/e2_scaling.csv`` (columns repeat, reader, threads, seconds)
and writes ``results/e2_speedup_bca.json`` with the median of paired
one-thread/eight-thread ratios and a bias-corrected accelerated 95 percent
interval of that median, 9999 resamples.
"""
from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.stats import norm

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results"
SEED = 20261011
N_BOOT = 9999
READERS = ("rust", "cpp", "python_full")


def bca_median(samples: np.ndarray) -> tuple[float, float, float]:
    samples = np.asarray(samples, dtype=float)
    n = samples.size
    theta = float(np.median(samples))
    rng = np.random.default_rng(SEED)
    boots = np.array([np.median(rng.choice(samples, size=n, replace=True)) for _ in range(N_BOOT)])
    prop = float(np.mean(boots < theta))
    prop = min(max(prop, 1 / (N_BOOT + 1)), 1 - 1 / (N_BOOT + 1))
    z0 = float(norm.ppf(prop))
    jack = np.array([np.median(np.delete(samples, i)) for i in range(n)])
    diff = jack.mean() - jack
    denom = np.sum(diff**2) ** 1.5
    acc = float(np.sum(diff**3) / (6 * denom)) if denom else 0.0

    def level(p: float) -> float:
        z = float(norm.ppf(p))
        numer = z0 + z
        dz = 1 - acc * numer
        return float(np.quantile(boots, norm.cdf(z0 + numer / dz) if dz else p))

    return theta, level(0.025), level(0.975)


def main() -> None:
    by: dict[tuple[str, str], dict[int, float]] = defaultdict(dict)
    with (RESULTS / "e2_scaling.csv").open() as handle:
        for row in csv.DictReader(handle):
            if row.get("status") and row["status"] != "ok":
                continue
            by[(row["reader"], row["threads"])][int(row["repeat"])] = float(row["seconds"])
    out: dict[str, dict] = {}
    for reader in READERS:
        one = by[(reader, "1")]
        eight = by[(reader, "8")]
        speedups = []
        for rep in sorted(set(one) & set(eight)):
            if eight[rep] > 0:
                speedups.append(one[rep] / eight[rep])
        arr = np.array(speedups, dtype=float)
        med, lo, hi = bca_median(arr)
        out[reader] = {
            "speedups": speedups,
            "median": med,
            "low": lo,
            "high": hi,
            "note": "median of paired one-thread/eight-thread ratios; BCa 95 percent interval",
        }
        print(reader, med, lo, hi)
    (RESULTS / "e2_speedup_bca.json").write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
