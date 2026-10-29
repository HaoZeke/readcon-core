#!/usr/bin/env python3
"""Full-field vs coordinate-accumulation peer timings for the CPC abstract.

Two readcon modes share the same parse:

* ``full_fields`` — ``read_con`` and keep the frame list (mask, id, cell,
  periodic flags are available on each frame).
* ``coords_accum`` — ``read_con`` and accumulate the first atom's x+y+z only.

Peers are chemfiles XYZ (coordinates), h5py (one numeric array) and
MDAnalysis H5MD. Thirty interleaved pairs per (mode, peer). Ratios are
readcon time over peer time. Outputs:

* ``results/repeats_v76_fairness.csv``
* ``results/ratios_v76_fairness.csv``
* ``results/fairness_host.json``
"""
from __future__ import annotations

import csv
import json
import os
import time
from pathlib import Path

import numpy as np
from scipy.stats import norm

os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
os.environ.setdefault("MKL_NUM_THREADS", "1")

import readcon
import chemfiles
import h5py
import MDAnalysis as mda

ROOT = Path(__file__).resolve().parents[3]
RESULTS = Path(__file__).resolve().parent / "results"
REPEATS = int(os.environ.get("FAIR_REPEATS", "30"))
SEED = 20261011
N_BOOT = 9999


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


def cpu_fields() -> dict[str, str | float | int]:
    family = model = mhz = name = ""
    nproc = 0
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("processor"):
            nproc += 1
        elif line.startswith("model name") and not name:
            name = line.split(":", 1)[1].strip()
        elif line.startswith("cpu family") and not family:
            family = line.split(":", 1)[1].strip()
        elif line.startswith("model") and not line.startswith("model name") and not model:
            model = line.split(":", 1)[1].strip()
        elif line.startswith("cpu MHz") and not mhz:
            mhz = line.split(":", 1)[1].strip()
    mem_kb = 0
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemTotal:"):
            mem_kb = int(line.split()[1])
            break
    return {
        "cpu_model": name,
        "family": family,
        "model": model,
        "mhz": float(mhz) if mhz else 0.0,
        "nproc": nproc,
        "mem_kb": mem_kb,
        "os": "Ubuntu 24.04",
        "kernel": os.uname().release,
        "readcon": getattr(readcon, "__version__", "unknown"),
        "sha": os.environ.get("READCON_SHA", ""),
    }


def time_fn(fn):
    t0 = time.perf_counter()
    fn()
    return time.perf_counter() - t0


def main() -> None:
    import run_bench as rb

    src = Path(os.environ.get("READCON_SRC", ROOT))
    paths = rb.prepare_payloads(src)["frames-10000"]
    con = str(paths["readcon"])
    xyz = str(paths["chemfiles_xyz"])
    h5 = paths["h5py"]
    mda_path = str(paths["mda_h5md"])

    def full_fields():
        return len(readcon.read_con(con, n_threads=1))

    def coords_accum():
        frames = readcon.read_con(con, n_threads=1)
        acc = 0.0
        for fr in frames:
            a = fr.atoms[0]
            acc += float(a.x) + float(a.y) + float(a.z)
        return acc

    def chem():
        traj = chemfiles.Trajectory(xyz)
        n = traj.nsteps
        for i in range(n):
            traj.read_step(i)
        traj.close()
        return n

    def h5_one():
        with h5py.File(h5, "r") as handle:
            data = handle["particles/trajectory/position/value"][()]
        return data.shape

    def mda_h5():
        universe = mda.Universe(mda_path)
        n = 0
        for _ in universe.trajectory:
            n += 1
        return n

    full_fields()
    coords_accum()
    chem()
    h5_one()
    mda_h5()

    rows: list[dict] = []
    for peer, pfn in (("chemfiles_xyz", chem), ("h5py", h5_one), ("mda_h5md", mda_h5)):
        for mode, bfn in (("full_fields", full_fields), ("coords_accum", coords_accum)):
            for rep in range(1, REPEATS + 1):
                bt = time_fn(bfn)
                pt = time_fn(pfn)
                rows.append(
                    dict(
                        mode=mode,
                        peer=peer,
                        repeat=rep,
                        readcon_s=bt,
                        peer_s=pt,
                        ratio=bt / pt,
                    )
                )
                print(mode, peer, rep, round(bt, 4), round(pt, 4), round(bt / pt, 3), flush=True)

    RESULTS.mkdir(parents=True, exist_ok=True)
    out = RESULTS / "repeats_v76_fairness.csv"
    with out.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    summary = ["mode,peer,median_readcon_s,median_peer_s,median_ratio,bca_low,bca_high"]
    for mode in ("full_fields", "coords_accum"):
        for peer in ("chemfiles_xyz", "h5py", "mda_h5md"):
            sub = [r for r in rows if r["mode"] == mode and r["peer"] == peer]
            ratios = np.array([r["ratio"] for r in sub], dtype=float)
            med, lo, hi = bca_median(ratios)
            rc = float(np.median([r["readcon_s"] for r in sub]))
            pc = float(np.median([r["peer_s"] for r in sub]))
            summary.append(f"{mode},{peer},{rc},{pc},{med},{lo},{hi}")
            print("SUM", mode, peer, med, lo, hi, flush=True)
    (RESULTS / "ratios_v76_fairness.csv").write_text("\n".join(summary) + "\n")
    host = cpu_fields()
    (RESULTS / "fairness_host.json").write_text(json.dumps(host, indent=2) + "\n")
    print("wrote", out)


if __name__ == "__main__":
    main()
