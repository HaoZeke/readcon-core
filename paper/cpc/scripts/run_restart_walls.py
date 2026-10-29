#!/usr/bin/env python3
"""Time Table 12 resume routes: wall seconds and NEB force evaluations.

Repeats each Table-12 route ``WALL_REPEATS`` times (default 10) and writes the
median wall and median force-call count. Physics columns (steps, barrier, …)
come from the first successful repeat.

Outputs:

* ``paper/cpc/data/cloud-runs-2/05/restart_formats.csv`` — merged ``wall_s`` /
  ``force_calls``
* ``paper/cpc/data/cloud-runs-2/05/restart_walls.csv`` — one summary row per route
* ``paper/cpc/data/cloud-runs-2/05/restart_walls_repeats.csv`` — every repeat

Driver: ``pixi run cpc-restart-walls``.
"""
from __future__ import annotations

import csv
import json
import os
import sys
import tempfile
import time
import traceback
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "data" / "cloud-runs-2" / "cloud-run"))
os.chdir(ROOT)

import restart_formats as rf  # noqa: E402

OUT = Path(os.environ.get("RUN05_OUT", ROOT / "data" / "cloud-runs-2" / "05"))
WALL_REPEATS = int(os.environ.get("WALL_REPEATS", "10"))

TABLE12 = [
    "uninterrupted",
    "CON 3.1, shortest round trip",
    "CON 3.1, six decimals",
    "CON 3.1 read by the ASE eon reader",
    "same, periodicity restored by hand",
    "extended XYZ (ASE)",
    "ASE trajectory",
    "POSCAR (ASE)",
    "legacy CON (ASE eon writer)",
    "ASE trajectory, id array",
    "H5MD MDAnalysis to MDAnalysis",
    "H5MD MDAnalysis to h5py",
    "H5MD h5py to h5py",
    "H5MD h5py to MDAnalysis",
]


class CountingNEB:
    def __init__(self, neb):
        self._neb = neb
        self.force_calls = 0

    def get_forces(self, *a, **k):
        self.force_calls += 1
        return self._neb.get_forces(*a, **k)

    def __getattr__(self, name):
        return getattr(self._neb, name)


def run_counted(imgs, start=0, stop=None):
    from ase.mep import NEB
    from ase.calculators.emt import EMT

    for im in imgs:
        im.calc = EMT()
    raw = NEB(imgs, climb=True, method="improvedtangent")
    neb = CountingNEB(raw)
    step = start
    fmax = float("inf")
    while True:
        f = neb.get_forces()
        fmax = float(np.sqrt((f**2).sum(axis=1)).max())
        if fmax < rf.FMAX or step >= rf.NMAX or (stop is not None and step >= stop):
            break
        neb.set_positions(neb.get_positions() + rf.ALPHA * f)
        step += 1
    e = np.array([im.get_potential_energy() for im in imgs])
    frc = [im.get_forces(apply_constraint=False).copy() for im in imgs]
    return dict(
        step=step,
        fmax=fmax,
        barrier=float(e.max() - e[0]),
        converged=bool(fmax < rf.FMAX),
        pos=[im.get_positions().copy() for im in imgs],
        energies=e.tolist(),
        forces=frc,
        force_calls=neb.force_calls,
    )


def with_ids(imgs):
    out = []
    for im in imgs:
        a = im.copy()
        a.set_array("id", np.array([17 + 3 * i for i in range(len(a))], dtype=np.int64))
        out.append(a)
    return out


def traj_ids(imgs, d):
    import ase.io

    out = []
    for i, im in enumerate(with_ids(imgs)):
        p = d / f"img{i}.traj"
        ase.io.write(str(p), im, format="traj")
        out.append(ase.io.read(str(p), format="traj"))
    return out


def h5_fns():
    import run_05 as r5

    def make(wr, rd):
        def go(imgs, d):
            return r5.h5_roundtrip(wr, rd, imgs)

        return go

    return {
        "H5MD MDAnalysis to MDAnalysis": make(r5.w_mda_convention, r5.fm.r_mda("H5MD")),
        "H5MD MDAnalysis to h5py": make(r5.w_mda_convention, r5.r_h5py),
        "H5MD h5py to h5py": make(r5.w_h5py, r5.r_h5py),
        "H5MD h5py to MDAnalysis": make(r5.w_h5py, r5.fm.r_mda("H5MD")),
    }


def time_route_once(name: str, fn, eps, ref) -> dict:
    row = dict(route=name, wall_s="", force_calls="", error="")
    t0 = time.perf_counter()
    try:
        if name == "uninterrupted":
            res = run_counted(rf.make_band(eps))
            wall = time.perf_counter() - t0
            row.update(
                steps=res["step"],
                converged=res["converged"],
                barrier_eV=f"{res['barrier']:.9f}",
                d_barrier_eV="0",
                max_disp_A="0",
                file_dpos_A="",
                free_modes_mid=rf.free_modes(rf.make_band(eps)[3]),
                wall_s=f"{wall:.6f}",
                force_calls=res["force_calls"],
                notes="",
            )
            return row

        imgs = rf.make_band(eps)
        part = run_counted(imgs, stop=rf.K)
        with tempfile.TemporaryDirectory() as td:
            back = fn(imgs, Path(td))
        fm = rf.free_modes(back[3])
        fdp = max(
            float(np.abs(b.get_positions() - a.get_positions()).max())
            for a, b in zip(imgs, back)
        )
        res = run_counted(back, start=part["step"])
        wall = time.perf_counter() - t0
        disp = max(float(np.abs(a - b).max()) for a, b in zip(res["pos"], ref["pos"]))
        row.update(
            steps=res["step"],
            converged=res["converged"],
            barrier_eV=f"{res['barrier']:.9f}",
            d_barrier_eV=f"{res['barrier'] - ref['barrier']:.2e}",
            max_disp_A=f"{disp:.2e}",
            file_dpos_A=f"{fdp:.1e}",
            free_modes_mid=fm,
            wall_s=f"{wall:.6f}",
            force_calls=res["force_calls"],
            notes="",
        )
    except BaseException as e:
        wall = time.perf_counter() - t0
        row.update(
            steps="",
            converged="",
            barrier_eV="",
            d_barrier_eV="",
            max_disp_A="",
            file_dpos_A="",
            free_modes_mid="",
            wall_s=f"{wall:.6f}",
            force_calls="",
            error=f"{type(e).__name__}: {e}"[:300],
            notes="",
        )
    return row


def median_summary(name: str, repeats: list[dict]) -> dict:
    ok = [r for r in repeats if not r.get("error")]
    walls = [float(r["wall_s"]) for r in repeats if r.get("wall_s") not in ("", None)]
    forces = [
        float(r["force_calls"])
        for r in ok
        if r.get("force_calls") not in ("", None)
    ]
    base = dict(ok[0]) if ok else dict(repeats[0])
    base["route"] = name
    base["wall_s"] = f"{float(np.median(walls)):.6f}" if walls else ""
    base["force_calls"] = str(int(round(float(np.median(forces))))) if forces else ""
    base["n_repeats"] = len(repeats)
    base["n_ok"] = len(ok)
    base["notes"] = (
        f"median of {len(walls)} walls"
        + (f"; first error {repeats[0].get('error','')[:80]}" if not ok and repeats else "")
    )
    if not ok and repeats:
        base["error"] = repeats[0].get("error", "")
    elif ok:
        base["error"] = ""
    return base


def merge_into_restart_csv(timed: dict[str, dict]) -> None:
    path = OUT / "restart_formats.csv"
    if not path.exists():
        raise SystemExit(f"missing {path}; run cloud-run/run_05.py first or write a stub")
    with path.open(newline="") as fh:
        rows = list(csv.DictReader(fh))
    fields = list(rows[0].keys()) if rows else []
    for col in ("wall_s", "force_calls"):
        if col not in fields:
            fields.append(col)
    for row in rows:
        hit = timed.get(row["route"])
        if hit:
            row["wall_s"] = hit.get("wall_s", "")
            row["force_calls"] = hit.get("force_calls", "")
            for k in (
                "steps",
                "converged",
                "barrier_eV",
                "d_barrier_eV",
                "max_disp_A",
                "file_dpos_A",
                "free_modes_mid",
                "error",
            ):
                if k in hit and hit[k] != "":
                    row[k] = hit[k]
        else:
            row.setdefault("wall_s", "")
            row.setdefault("force_calls", "")
    with path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields)
        w.writeheader()
        w.writerows(rows)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"WALL_REPEATS={WALL_REPEATS}", flush=True)
    print("building endpoints", flush=True)
    eps = rf.endpoints()
    print("uninterrupted reference", flush=True)
    ref = run_counted(rf.make_band(eps))
    print(
        f"uninterrupted steps={ref['step']} barrier={ref['barrier']:.9f} "
        f"force_calls={ref['force_calls']}",
        flush=True,
    )

    route_fns = {name: fn for name, fn in rf.ROUTES}
    route_fns["ASE trajectory, id array"] = traj_ids
    try:
        route_fns.update(h5_fns())
    except Exception as e:
        print(f"H5MD routes unavailable: {e}", flush=True)

    all_repeats: list[dict] = []
    timed_rows: list[dict] = []
    for name in TABLE12:
        fn = None if name == "uninterrupted" else route_fns.get(name)
        if name != "uninterrupted" and fn is None:
            print(f"skip missing route fn: {name}", flush=True)
            continue
        reps = []
        for i in range(WALL_REPEATS):
            row = time_route_once(name, fn, eps, ref)
            row["repeat"] = i
            reps.append(row)
            all_repeats.append(dict(row))
            print(
                f"{name}[{i}]: wall={row.get('wall_s')} force_calls={row.get('force_calls')} "
                f"steps={row.get('steps')} err={str(row.get('error',''))[:60]}",
                flush=True,
            )
        summary = median_summary(name, reps)
        timed_rows.append(summary)
        print(
            f"{name} MEDIAN wall={summary['wall_s']} force_calls={summary['force_calls']} "
            f"n={summary['n_repeats']}",
            flush=True,
        )

    wfields = [
        "route",
        "steps",
        "converged",
        "barrier_eV",
        "d_barrier_eV",
        "max_disp_A",
        "file_dpos_A",
        "free_modes_mid",
        "wall_s",
        "force_calls",
        "n_repeats",
        "n_ok",
        "error",
        "notes",
    ]
    walls_path = OUT / "restart_walls.csv"
    with walls_path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=wfields)
        w.writeheader()
        for r in timed_rows:
            w.writerow({k: r.get(k, "") for k in wfields})

    rfields = [
        "route",
        "repeat",
        "steps",
        "converged",
        "barrier_eV",
        "d_barrier_eV",
        "max_disp_A",
        "file_dpos_A",
        "free_modes_mid",
        "wall_s",
        "force_calls",
        "error",
        "notes",
    ]
    with (OUT / "restart_walls_repeats.csv").open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=rfields)
        w.writeheader()
        for r in all_repeats:
            w.writerow({k: r.get(k, "") for k in rfields})

    timed = {r["route"]: r for r in timed_rows}
    merge_into_restart_csv(timed)

    sha = os.environ.get("READCON_SHA", "")
    core = os.environ.get("READCON_CORE_ROOT")
    if core and Path(core).exists() and not sha:
        try:
            import subprocess

            sha = subprocess.check_output(
                ["git", "-C", core, "rev-parse", "HEAD"], text=True
            ).strip()
        except Exception:
            sha = ""
    if sha:
        (OUT / "build.txt").write_text(
            f"readcon-core 0.17.2 commit {sha}\n"
            f"restart walls: pixi run cpc-restart-walls (WALL_REPEATS={WALL_REPEATS})\n"
        )
    (OUT / "restart_walls.json").write_text(
        json.dumps(
            {
                "host": os.uname().nodename,
                "wall_repeats": WALL_REPEATS,
                "routes": timed_rows,
                "readcon_sha": sha,
            },
            indent=1,
            default=str,
        )
    )
    print(f"wrote {walls_path}", flush=True)
    for r in timed_rows:
        print(
            f"  {r['route']}: wall_s={r.get('wall_s')} force_calls={r.get('force_calls')} "
            f"n={r.get('n_repeats')}",
            flush=True,
        )


if __name__ == "__main__":
    try:
        main()
    except Exception:
        traceback.print_exc()
        sys.exit(1)
