# Copyright (c) readcon developers
"""One fresh process, one extended-XYZ sample. Import finishes before the clock."""

from __future__ import annotations

import argparse
import importlib
import sys
import time
import traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import numpy as np

from common import (
    ANGLE_TOL,
    COORD_TOL,
    LENGTH_TOL,
    cache_fraction,
    cache_label,
    g,
    git_capture,
    recorded_path,
    sha256_file,
)

def extension_sha256() -> dict[str, dict[str, str]]:
    import importlib

    found: dict[str, dict[str, str]] = {}
    for arm, names in IMPORTS.items():
        files: dict[str, str] = {}
        for name in names:
            module = importlib.import_module(name)
            file_attr = getattr(module, "__file__", None)
            if not file_attr:
                continue
            file = Path(file_attr).resolve()
            root = file.parent
            if file.name.endswith(".so") and file.is_file():
                files[file.name] = sha256_file(file)
            for path in sorted(root.rglob("*.so")):
                files[path.name] = sha256_file(path)
        found[arm] = files
    return found


IMPORTS = {
    "readcon_extxyz": ("readcon",),
    "readcon_py": ("readcon",),
    "ase_extxyz": ("ase", "ase.io"),
    "ase_eon": ("ase", "ase.io"),
    "chemfiles": ("chemfiles",),
    "ovito": ("ovito", "ovito.io"),
    "mdanalysis": ("MDAnalysis",),
    "pymatgen": ("pymatgen", "pymatgen.io.xyz"),
}


def read_extxyz(path: str) -> dict:
    import readcon

    positions, masses, cell, types = readcon.read_extxyz(path)
    assert masses.shape[0] == positions.shape[0]
    assert types.dtype == np.uint64
    return {
        "positions": np.asarray(positions, dtype=np.float64),
        "lengths": np.asarray(cell[:, :3], dtype=np.float64),
        "angles": np.asarray(cell[:, 3:], dtype=np.float64),
        "types": np.asarray(types),
        "types_dtype": str(types.dtype),
        "cell_parsed": True,
    }


def read_ase(path: str) -> dict:
    return _read_ase(path, "extxyz")


def read_ase_eon(path: str) -> dict:
    return _read_ase(path, "eon")


def _read_ase(path: str, fmt: str) -> dict:
    from ase.io import read

    images = read(path, index=":", format=fmt)
    if not isinstance(images, list):
        images = [images]
    n_frames = len(images)
    n_atoms = len(images[0])
    pos = np.empty((n_frames, n_atoms, 3), dtype=np.float64)
    lengths = np.empty((n_frames, 3), dtype=np.float64)
    angles = np.empty((n_frames, 3), dtype=np.float64)
    types = np.empty((n_frames, n_atoms), dtype=np.int64)
    for index, atoms in enumerate(images):
        pos[index] = atoms.positions
        lengths[index] = atoms.cell.lengths()
        angles[index] = atoms.cell.angles()
        types[index] = atoms.numbers
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": True,
    }


def read_chemfiles(path: str) -> dict:
    import chemfiles

    trajectory = chemfiles.Trajectory(path)
    n_frames = int(trajectory.nsteps)
    pos = None
    lengths = np.empty((n_frames, 3), dtype=np.float64)
    angles = np.empty((n_frames, 3), dtype=np.float64)
    types = None
    for index in range(n_frames):
        frame = trajectory.read_step(index)
        xyz = np.array(frame.positions, dtype=np.float64, copy=True)
        if pos is None:
            pos = np.empty((n_frames, xyz.shape[0], 3), dtype=np.float64)
            types = np.empty((n_frames, xyz.shape[0]), dtype=np.int64)
        pos[index] = xyz
        lengths[index] = np.array(frame.cell.lengths, dtype=np.float64, copy=True)
        angles[index] = np.array(frame.cell.angles, dtype=np.float64, copy=True)
        for atom_index, atom in enumerate(frame.atoms):
            types[index, atom_index] = int(atom.atomic_number)
    trajectory.close()
    assert pos is not None and types is not None
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": True,
    }


def _cell_from_columns(matrix: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    a, b, c = matrix[:, 0], matrix[:, 1], matrix[:, 2]
    la, lb, lc = np.linalg.norm(a), np.linalg.norm(b), np.linalg.norm(c)

    def angle(u, v, lu, lv):
        cosine = np.clip(np.dot(u, v) / (lu * lv), -1.0, 1.0)
        return np.degrees(np.arccos(cosine))

    return (
        np.array([la, lb, lc]),
        np.array([angle(b, c, lb, lc), angle(a, c, la, lc), angle(a, b, la, lb)]),
    )


def read_ovito(path: str) -> dict:
    from ovito.io import import_file

    pipeline = import_file(path, sort_particles=False)
    n_frames = int(pipeline.num_frames)
    first = pipeline.compute(0)
    n_atoms = int(first.particles.count)
    pos = np.empty((n_frames, n_atoms, 3), dtype=np.float64)
    lengths = np.empty((n_frames, 3), dtype=np.float64)
    angles = np.empty((n_frames, 3), dtype=np.float64)
    types = np.empty((n_frames, n_atoms), dtype=np.int64)
    for index in range(n_frames):
        data = pipeline.compute(index)
        pos[index] = np.asarray(data.particles.positions)
        matrix = np.asarray(data.cell.matrix, dtype=np.float64)
        lengths[index], angles[index] = _cell_from_columns(matrix[:, :3])
        prop = data.particles.particle_types
        assert prop is not None
        names = {}
        for particle_type in prop.types:
            atomic = getattr(particle_type, "atomic_number", None)
            if atomic:
                names[int(particle_type.id)] = int(atomic)
            else:
                assert str(particle_type.name) == "Cu"
                names[int(particle_type.id)] = 29
        ids = np.asarray(prop, dtype=np.int64)
        for atom_index, type_id in enumerate(ids):
            types[index, atom_index] = names[int(type_id)]
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": True,
    }


def read_mdanalysis(path: str) -> dict:
    import MDAnalysis as mda

    universe = mda.Universe(path)
    n_frames = len(universe.trajectory)
    n_atoms = len(universe.atoms)
    names = [str(name) for name in universe.atoms.names]
    pos = np.empty((n_frames, n_atoms, 3), dtype=np.float64)
    lengths = np.empty((n_frames, 3), dtype=np.float64)
    angles = np.empty((n_frames, 3), dtype=np.float64)
    types = np.empty((n_frames, n_atoms), dtype=np.int64)
    z = {"Cu": 29, "H": 1, "O": 8}
    for atom_index, name in enumerate(names):
        assert name in z
        types[:, atom_index] = z[name]
    finite = True
    for index, timestep in enumerate(universe.trajectory):
        pos[index] = timestep.positions
        dimensions = timestep.dimensions
        if dimensions is None:
            lengths[index] = np.nan
            angles[index] = np.nan
            finite = False
        else:
            lengths[index] = dimensions[:3]
            angles[index] = dimensions[3:6]
            if not np.any(np.abs(dimensions[:3]) > 0):
                finite = False
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": finite,
    }


def read_pymatgen(path: str) -> dict:
    from pymatgen.io.xyz import XYZ

    molecules = XYZ.from_file(path).all_molecules
    n_frames = len(molecules)
    n_atoms = len(molecules[0])
    pos = np.empty((n_frames, n_atoms, 3), dtype=np.float64)
    lengths = np.full((n_frames, 3), np.nan, dtype=np.float64)
    angles = np.full((n_frames, 3), np.nan, dtype=np.float64)
    types = np.empty((n_frames, n_atoms), dtype=np.int64)
    cell_parsed = True
    for index, molecule in enumerate(molecules):
        lattice = getattr(molecule, "lattice", None)
        if lattice is None:
            cell_parsed = False
        else:
            lengths[index], angles[index] = _cell_from_columns(np.asarray(lattice.matrix, dtype=np.float64).T)
        for atom_index, site in enumerate(molecule):
            pos[index, atom_index] = site.coords
            types[index, atom_index] = int(site.specie.Z)
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": cell_parsed,
    }


def read_readcon_con(path: str) -> dict:
    import readcon

    frames = readcon.read_con(path, n_threads=1)
    n_frames = len(frames)
    if n_frames == 0:
        raise RuntimeError("read_con returned no frames")
    n_atoms = int(np.asarray(frames[0].xyz).shape[0])
    pos = np.empty((n_frames, n_atoms, 3), dtype=np.float64)
    lengths = np.empty((n_frames, 3), dtype=np.float64)
    angles = np.empty((n_frames, 3), dtype=np.float64)
    types = np.empty((n_frames, n_atoms), dtype=np.int64)
    atomic_number = readcon.symbol_to_atomic_number
    for index, frame in enumerate(frames):
        pos[index] = np.asarray(frame.xyz, dtype=np.float64)
        lengths[index] = np.asarray(frame.cell, dtype=np.float64)
        angles[index] = np.asarray(frame.angles, dtype=np.float64)
        atoms = frame.atoms
        if len(atoms) != n_atoms:
            raise RuntimeError("atom count changed between frames")
        for atom_index, atom in enumerate(atoms):
            types[index, atom_index] = int(atomic_number(atom.symbol))
    return {
        "positions": pos,
        "lengths": lengths,
        "angles": angles,
        "types": types,
        "types_dtype": str(types.dtype),
        "cell_parsed": True,
    }


READERS = {
    "readcon_extxyz": read_extxyz,
    "readcon_py": read_readcon_con,
    "ase_extxyz": read_ase,
    "ase_eon": read_ase_eon,
    "chemfiles": read_chemfiles,
    "ovito": read_ovito,
    "mdanalysis": read_mdanalysis,
    "pymatgen": read_pymatgen,
}


def compare(result: dict, truth: dict) -> dict:
    got = result["positions"]
    exp = truth["positions"]
    report = {
        "frames_ok": got.shape[0] == exp.shape[0],
        "atoms_ok": got.shape[:2] == exp.shape[:2],
        "cell_parsed": bool(result["cell_parsed"]),
        "types_dtype": result["types_dtype"],
    }
    if got.shape != exp.shape:
        report.update(max_abs_dx=None, length_max=None, angle_max=None, types_ok=False, correct=False)
        return report
    report["max_abs_dx"] = float(np.max(np.abs(got - exp)))
    report["length_max"] = float(np.max(np.abs(result["lengths"] - truth["lengths"])))
    report["angle_max"] = float(np.max(np.abs(result["angles"] - truth["angles"])))
    report["types_ok"] = bool(
        np.array_equal(result["types"].astype(np.int64), truth["types"].astype(np.int64))
    )
    report["correct"] = (
        report["cell_parsed"]
        and report["types_ok"]
        and report["max_abs_dx"] <= COORD_TOL
        and report["length_max"] <= LENGTH_TOL
        and report["angle_max"] <= ANGLE_TOL
    )
    return report


def load_truth(path: Path) -> dict:
    raw = path.read_bytes()
    assert raw[:4] == b"H2H1"
    n_frames, n_atoms = np.frombuffer(raw[4:20], dtype=np.uint64)
    n_frames = int(n_frames)
    n_atoms = int(n_atoms)
    offset = 20
    pos_n = n_frames * n_atoms * 3
    lng_n = n_frames * 3
    positions = np.frombuffer(raw, dtype=np.float64, count=pos_n, offset=offset).reshape(n_frames, n_atoms, 3).copy()
    offset += pos_n * 8
    lengths = np.frombuffer(raw, dtype=np.float64, count=lng_n, offset=offset).reshape(n_frames, 3).copy()
    offset += lng_n * 8
    angles = np.frombuffer(raw, dtype=np.float64, count=lng_n, offset=offset).reshape(n_frames, 3).copy()
    offset += lng_n * 8
    types = np.frombuffer(raw, dtype=np.int32, count=n_frames * n_atoms, offset=offset).reshape(n_frames, n_atoms).copy()
    return {"positions": positions, "lengths": lengths, "angles": angles, "types": types}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--arm", required=True)
    parser.add_argument("--path", required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--command", default="")
    args = parser.parse_args()
    lines = []
    rev, status = git_capture()
    lines.append("git rev-parse HEAD")
    lines.append(rev)
    lines.append("git status --porcelain=v1")
    lines.append(status.rstrip("\n"))
    lines.append("command")
    lines.append(args.command or " ".join(sys.argv))
    lines.append(f"arm {args.arm}")
    lines.append(f"path {recorded_path(Path(args.path))}")
    fraction = cache_fraction(Path(args.path))
    lines.append(f"cache_fraction_before {g(fraction)}")
    lines.append(f"cache {cache_label(fraction)}")
    lines.append(f"coord_tol {g(COORD_TOL)}")
    lines.append(f"length_tol {g(LENGTH_TOL)}")
    lines.append(f"angle_tol {g(ANGLE_TOL)}")
    try:
        for name in IMPORTS[args.arm]:
            importlib.import_module(name)
        t0 = time.perf_counter()
        result = READERS[args.arm](args.path)
        elapsed = time.perf_counter() - t0
        checked = compare(result, load_truth(Path(args.truth)))
        lines.append(f"elapsed_s {g(elapsed)}")
        for key, value in checked.items():
            lines.append(f"{key} {g(value) if isinstance(value, float) else value}")
        lines.append("status ok")
        code = 0 if status == "" else 3
    except Exception:
        lines.append("status error")
        lines.append(traceback.format_exc())
        code = 1
    text = "\n".join(lines) + "\n"
    log = Path(args.log)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(text, encoding="utf-8")
    sys.stdout.write(text)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
