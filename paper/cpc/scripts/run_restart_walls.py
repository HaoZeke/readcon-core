#!/usr/bin/env python3
"""Time Table 12 resume routes: wall seconds and NEB force evaluations.

Runs the same interrupted climbing-image protocol as ``restart_formats.py`` /
``cloud-run/run_05.py`` for the routes printed in Table~\\ref{tab:restartfmt},
logs wall-clock seconds (perf_counter) and the number of ``NEB.get_forces``
calls on the resume leg, and writes:

* ``paper/cpc/data/cloud-runs-2/05/restart_formats.csv`` — merged columns
  ``wall_s`` and ``force_calls`` onto every Table-12 route (other CSV rows
  keep empty wall/force cells).
* ``paper/cpc/data/cloud-runs-2/05/restart_walls.csv`` — one row per timed
  route (committed companion).

Requires the venv that has ASE + readcon 0.17.2. Set ``READCON_CORE_ROOT`` /
``READCON_CORE_LIB`` when a specific build must be named in ``build.txt``.

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
REPO = ROOT.parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
os.chdir(ROOT)

import restart_formats as rf  # noqa: E402

OUT = Path(os.environ.get("RUN05_OUT", ROOT / "data" / "cloud-runs-2" / "05"))
# Routes printed in Table 12 (main text), matching gen_cloud_tables.main_names.
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
    """Wrap ASE NEB so each get_forces call is counted."""

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


def time_route(name: str, fn, eps, ref) -> dict:
    row = dict(route=name, wall_s="", force_calls="", error="")
    t0 = time.perf_counter()
    try:
        if name == "uninterrupted":
            # Wall and forces for the full uninterrupted climb (same as Table 12).
            imgs = rf.make_band(eps)
            res = run_counted(imgs)
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
            # Refresh physics columns for Table-12 routes from this timed run.
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
    print("building endpoints", flush=True)
    eps = rf.endpoints()
    # Reference uninterrupted band (also timed as the first Table-12 row).
    print("uninterrupted reference", flush=True)
    ref_imgs = rf.make_band(eps)
    ref = run_counted(ref_imgs)
    print(
        f"uninterrupted steps={ref['step']} barrier={ref['barrier']:.9f} "
        f"force_calls={ref['force_calls']}",
        flush=True,
    )

    route_fns = {name: fn for name, fn in rf.ROUTES}
    timed_rows = []
    # uninterrupted
    t0 = time.perf_counter()
    # Re-run so wall covers a fresh climb (ref already measured forces).
    res = run_counted(rf.make_band(eps))
    wall = time.perf_counter() - t0
    timed_rows.append(
        dict(
            route="uninterrupted",
            steps=res["step"],
            converged=res["converged"],
            barrier_eV=f"{res['barrier']:.9f}",
            d_barrier_eV="0",
            max_disp_A="0",
            file_dpos_A="",
            free_modes_mid=rf.free_modes(rf.make_band(eps)[3]),
            wall_s=f"{wall:.6f}",
            force_calls=res["force_calls"],
            error="",
            notes="",
        )
    )
    print(
        f"uninterrupted wall={wall:.3f}s force_calls={res['force_calls']}",
        flush=True,
    )

    for name in TABLE12:
        if name == "uninterrupted":
            continue
        fn = route_fns.get(name)
        if fn is None:
            print(f"skip missing route fn: {name}", flush=True)
            continue
        row = time_route(name, fn, eps, ref)
        timed_rows.append(row)
        print(
            f"{name}: wall={row.get('wall_s')} force_calls={row.get('force_calls')} "
            f"steps={row.get('steps')} err={str(row.get('error',''))[:80]}",
            flush=True,
        )

    walls_path = OUT / "restart_walls.csv"
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
        "error",
        "notes",
    ]
    with walls_path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=wfields)
        w.writeheader()
        for r in timed_rows:
            w.writerow({k: r.get(k, "") for k in wfields})

    timed = {r["route"]: r for r in timed_rows}
    merge_into_restart_csv(timed)

    # Stamp build note when core env is set.
    core = os.environ.get("READCON_CORE_ROOT")
    sha = os.environ.get("READCON_SHA", "")
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
            f"restart walls: pixi run cpc-restart-walls\n"
        )
    (OUT / "restart_walls.json").write_text(
        json.dumps(
            {
                "host": os.uname().nodename,
                "routes": timed_rows,
                "readcon_sha": sha,
            },
            indent=1,
            default=str,
        )
    )
    print(f"wrote {walls_path}", flush=True)
    print(f"merged wall_s/force_calls into {OUT / 'restart_formats.csv'}", flush=True)
    print("Table 12 walls:", flush=True)
    for r in timed_rows:
        print(
            f"  {r['route']}: wall_s={r.get('wall_s')} force_calls={r.get('force_calls')} "
            f"steps={r.get('steps')}",
            flush=True,
        )


if __name__ == "__main__":
    try:
        main()
    except Exception:
        traceback.print_exc()
        sys.exit(1)
