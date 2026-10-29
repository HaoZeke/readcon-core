#!/usr/bin/env python3
"""BCa intervals for the e2 thread-scaling speedups via scipy.

Reads ``results/e2_scaling.csv`` (columns repeat, reader, threads, seconds)
and writes ``results/e2_speedup_bca.json`` with the median of paired
one-thread / N-thread ratios and a bias-corrected accelerated 95 percent
interval of that median from ``scipy.stats.bootstrap(..., method="BCa")``.

The manuscript prints the one→eight medians. The one→two and one→four
blocks are recorded so the supplement can quote the same JSON.
"""
from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.stats import bootstrap

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results"
SEED = 20261011
N_BOOT = 9999
READERS = ("rust", "cpp", "python_full")
TARGETS = (2, 4, 8)


def bca_median(samples: np.ndarray) -> tuple[float, float, float]:
    samples = np.asarray(samples, dtype=float)
    theta = float(np.median(samples))
    res = bootstrap(
        (samples,),
        np.median,
        confidence_level=0.95,
        method="BCa",
        n_resamples=N_BOOT,
        random_state=SEED,
    )
    return theta, float(res.confidence_interval.low), float(res.confidence_interval.high)


def paired_speedups(
    by: dict[tuple[str, str], dict[int, float]], reader: str, threads: int
) -> np.ndarray:
    one = by[(reader, "1")]
    other = by[(reader, str(threads))]
    speedups = []
    for rep in sorted(set(one) & set(other)):
        if other[rep] > 0:
            speedups.append(one[rep] / other[rep])
    return np.asarray(speedups, dtype=float)


def main() -> None:
    by: dict[tuple[str, str], dict[int, float]] = defaultdict(dict)
    with (RESULTS / "e2_scaling.csv").open() as handle:
        for row in csv.DictReader(handle):
            if row.get("status") and row["status"] != "ok":
                continue
            by[(row["reader"], row["threads"])][int(row["repeat"])] = float(row["seconds"])

    # Endpoint medians of the one- and eight-thread walls (for prose).
    endpoints: dict[str, dict[str, float]] = {}
    for reader in READERS:
        one_vals = np.asarray(list(by[(reader, "1")].values()), dtype=float)
        eight_vals = np.asarray(list(by[(reader, "8")].values()), dtype=float)
        endpoints[reader] = {
            "one_median_s": float(np.median(one_vals)) if one_vals.size else float("nan"),
            "eight_median_s": float(np.median(eight_vals)) if eight_vals.size else float("nan"),
            "n": int(min(one_vals.size, eight_vals.size)),
        }

    out: dict[str, dict] = {"endpoints": endpoints, "method": "scipy.stats.bootstrap BCa"}
    for reader in READERS:
        out[reader] = {}
        for threads in TARGETS:
            arr = paired_speedups(by, reader, threads)
            med, lo, hi = bca_median(arr)
            block = {
                "speedups": arr.tolist(),
                "n": int(arr.size),
                "median": med,
                "low": lo,
                "high": hi,
                "method": "scipy.stats.bootstrap BCa",
                "n_resamples": N_BOOT,
                "seed": SEED,
                "threads": threads,
                "note": (
                    f"median of paired one-thread/{threads}-thread ratios; "
                    "BCa 95 percent interval"
                ),
            }
            out[reader][str(threads)] = block
            # Flat one→eight keys kept for the main-text check / older readers.
            if threads == 8:
                out[reader].update(
                    {
                        "speedups": block["speedups"],
                        "n": block["n"],
                        "median": med,
                        "low": lo,
                        "high": hi,
                        "method": block["method"],
                        "n_resamples": N_BOOT,
                        "seed": SEED,
                        "note": block["note"],
                    }
                )
            print(reader, f"1→{threads}", f"n={arr.size}", med, lo, hi)

    (RESULTS / "e2_speedup_bca.json").write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
