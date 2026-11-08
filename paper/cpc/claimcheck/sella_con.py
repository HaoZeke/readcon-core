"""Sella from a CON file: the README example of Sella 2.6.0, run from memory and from CON files.

The example builds a Cu(111) slab of 5 x 5 x 6 atoms with a Cu adatom at a bridge site, fixes the atoms of the
lower half of the cell and runs ``Sella(...).run(1e-3, 1000)`` with the EMT calculator of ASE.  Sella is
installed by pip from its release on PyPI, the source distribution pinned by SHA-256, into ``WORK/sella/site``.
Three routes run the same search, each run a fresh process in the interleaved rounds of ``interleave``:

* ``memory``: the README example as written.
* ``readcon``: the slab written to a CON file by readcon, the fixed atoms as the mask.  The run reads the file
  with readcon, fixes the atoms that the mask fixes and writes the saddle with readcon.
* ``ase``: the slab written to a CON file by the eon module of ASE and read back by it.

Every run prints its steps, the energy and largest force at the end and the SHA-256 of the start and saddle
positions as binary64, and a cell stops the campaign if two of its runs print different values.  The record
keeps whether each route started from and reached the positions of the README route bit for bit.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
import tempfile
import time
from pathlib import Path

import cc_common as cc
import numpy as np

SDIST = {
    "url": "https://files.pythonhosted.org/packages/source/s/sella/sella-2.6.0.tar.gz",
    "name": "sella-2.6.0.tar.gz",
    "size": 182652,
    "sha": "7effc2a10f6d43c0bbdff11acb1fe36e16424cd9f01746660e87985662713384",
}
ROUTES = ("memory", "readcon", "ase")


def readme_slab():
    from ase.build import add_adsorbate, fcc111

    slab = fcc111("Cu", (5, 5, 6), vacuum=7.5)
    add_adsorbate(slab, "Cu", 2.0, "bridge")
    fixed = [atom.index for atom in slab if atom.position[2] < slab.cell[2, 2] / 2.0]
    return slab, fixed


def digest(positions: np.ndarray) -> str:
    return hashlib.sha256(
        np.ascontiguousarray(positions, dtype=np.float64).tobytes()
    ).hexdigest()


def write_inputs(work: Path) -> dict:
    import ase.io
    import readcon
    from ase.constraints import FixAtoms

    slab, fixed = readme_slab()
    slab.set_constraint(FixAtoms(indices=fixed))
    paths = {"readcon": work / "slab_readcon.con", "ase": work / "slab_ase.con"}
    readcon.write_con(str(paths["readcon"]), [readcon.ConFrame.from_ase(slab)])
    ase.io.write(paths["ase"], slab, format="eon")
    inputs = {}
    for route, path in paths.items():
        back = (
            readcon.read_con(str(path))[0].to_ase()
            if route == "readcon"
            else ase.io.read(path, format="eon")
        )
        inputs[route] = {
            "file": path.name,
            "bytes": path.stat().st_size,
            "sha256": cc.sha256(path),
            "positions_equal": bool(np.array_equal(back.positions, slab.positions)),
            "largest_position_difference": float(
                np.abs(back.positions - slab.positions).max()
            ),
            "cell_equal": bool(np.array_equal(back.cell[:], slab.cell[:])),
            "largest_cell_difference": float(np.abs(back.cell[:] - slab.cell[:]).max()),
            "pbc_written": slab.pbc.tolist(),
            "pbc_read": back.pbc.tolist(),
        }
    return {"atoms": len(slab), "fixed": len(fixed), "inputs": inputs}


def child(route: str, work: Path) -> int:
    import readcon
    from ase.calculators.emt import EMT
    from sella import Constraints, Sella

    start = time.perf_counter()
    if route == "memory":
        atoms, fixed = readme_slab()
    elif route == "readcon":
        frame = readcon.read_con(str(work / "slab_readcon.con"))[0]
        atoms = frame.to_ase()
        fixed = [i for i, atom in enumerate(frame.atoms) if all(atom.fixed)]
    else:
        import ase.io

        atoms = ase.io.read(work / "slab_ase.con", format="eon")
        fixed = sorted(int(i) for c in atoms.constraints for i in c.get_indices())
    atoms.set_constraint()
    start_sha = digest(atoms.positions)
    cons = Constraints(atoms)
    for index in fixed:
        cons.fix_translation(index)
    atoms.calc = EMT()
    out = Path(tempfile.mkdtemp(dir=work))
    dyn = Sella(atoms, constraints=cons, trajectory=str(out / "sella.traj"))
    converged = bool(dyn.run(1e-3, 1000))
    forces = atoms.get_forces()
    free = [i for i in range(len(atoms)) if i not in set(fixed)]
    result = {
        "converged": converged,
        "steps": int(dyn.nsteps),
        "energy": repr(float(atoms.get_potential_energy())),
        "fmax_free": repr(float(np.sqrt((forces[free] ** 2).sum(axis=1)).max())),
        "fixed": len(fixed),
        "pbc": atoms.pbc.tolist(),
        "start_sha256": start_sha,
        "saddle_sha256": digest(atoms.positions),
    }
    if route == "readcon":
        from ase.constraints import FixAtoms

        atoms.set_constraint(FixAtoms(indices=fixed))
        readcon.write_con(str(out / "saddle.con"), [readcon.ConFrame.from_ase(atoms)])
        back = readcon.read_con(str(out / "saddle.con"))[0]
        result["saddle_file_round_trip_exact"] = bool(
            np.array_equal(back.coords_array(), atoms.positions)
        )
        result["saddle_file_mask_kept"] = [
            i for i, atom in enumerate(back.atoms) if all(atom.fixed)
        ] == fixed
    shutil.rmtree(out)
    result["seconds"] = time.perf_counter() - start
    print(json.dumps(result))
    return 0


def main() -> int:
    work = cc.WORK / "sella"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    sdist, got = cc.fetch(SDIST["url"], SDIST["name"], SDIST["sha"], SDIST["size"])
    site = work / "site"
    install = cc.run(
        [sys.executable, "-m", "pip", "install", "--no-deps", "--target", site, sdist]
    )
    shown = cc.run(
        [
            sys.executable,
            "-c",
            "import sella, importlib.metadata as m; print(m.version('sella'))",
        ],
        env={"PYTHONPATH": str(site)},
    )
    inputs = write_inputs(work)
    env = {"PYTHONPATH": str(site)}
    cells = {
        route: (
            lambda route=route: cc.child(
                [sys.executable, Path(__file__).resolve(), "child", route, work],
                env=env,
            )
        )
        for route in ROUTES
    }
    timing = cc.campaign(cells)
    printed = {route: timing["cells"][route]["printed"] for route in ROUTES}
    cc.write_record(
        "sella_con",
        {
            "claim": "Sella runs from a CON file that readcon wrote and reaches the README saddle bit for bit",
            "sella": {"version": shown["stdout"].strip(), "install": cc.brief(install)},
            "example": "README of Sella 2.6.0: Cu(111) 5 x 5 x 6 with a bridge adatom, EMT, Sella(...).run(1e-3, 1000)",
            **inputs,
            "timing": timing,
            "summary": {
                route: {
                    "converged": printed[route]["converged"],
                    "steps": printed[route]["steps"],
                    "energy": printed[route]["energy"],
                    "fmax_free": printed[route]["fmax_free"],
                    "pbc": printed[route]["pbc"],
                    "same_start_as_memory": printed[route]["start_sha256"]
                    == printed["memory"]["start_sha256"],
                    "same_saddle_as_memory": printed[route]["saddle_sha256"]
                    == printed["memory"]["saddle_sha256"],
                    "median_s": timing["cells"][route]["median_s"],
                    "iqr_s": timing["cells"][route]["iqr_s"],
                }
                for route in ROUTES
            },
        },
        [got],
    )
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "child":
        raise SystemExit(child(sys.argv[2], Path(sys.argv[3])))
    raise SystemExit(main())
