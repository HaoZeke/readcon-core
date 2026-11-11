# Copyright (c) readcon-core developers
"""XYZ head-to-head on the copper-grid files and the 32-atom trajectory.

The two copper-grid files are the extended XYZ files written by
``extxyz_h2h.py``: 10^6 atoms in one frame, and 100 atoms in 10000 frames.
The 32-atom trajectory is that same grid, 100000 frames, written as XYZ
with the same species, positions, masses, forces and lattice comment.

One process interleaves every arm. One warmup call is excluded. The six
repeats are serial. RSS and massif come from a fresh process that only reads.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from importlib import metadata
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extxyz_h2h
import support

os.environ["OMP_NUM_THREADS"] = "1"
os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["MKL_NUM_THREADS"] = "1"
os.environ["NUMEXPR_NUM_THREADS"] = "1"
os.environ["RAYON_NUM_THREADS"] = "1"

REPEATS = 6
# Written coordinates are one decimal place. Float32 nanometre round-trips
# stay far inside half of that digit.
POS_TOL = 5e-2
EXACT_TOL = 1e-6
ROOT = support.ROOT
WORK = support.WORK / "traj"
LOG_CSV = support.BENCH / "traj-logs"
RESULTS = support.BENCH / "traj-results.json"
CHEM_TAG = "0.11.0-rc1"
CHEM_COMMIT = "a2319946dcd0a96b055e803e59affed52e83bfb1"
CHEM_SRC = WORK / "chemfiles-src"
CHEM_PREFIX = WORK / "chemfiles-prefix"
CHEM_PY = WORK / "chemfiles-py"
CHEM_SO_NAME = "libchemfiles.so.0.11"

# name, atoms, frames, log stem
CASES = (
    ("grid-1e6", 1_000_000, 1, "traj-grid-1e6"),
    ("grid-10000", 100, 10_000, "traj-grid-10000"),
    ("traj-32", 32, 100_000, "traj-frames-100000"),
)
READ_ARMS = (
    "readcon_rust",
    "readcon_py",
    "ase_iread",
    "chemfiles_010",
    "chemfiles_011",
    "mdanalysis",
    "mdtraj",
)
ARM_LABEL = {
    "readcon_rust": "readcon Rust",
    "readcon_py": "readcon Python",
    "ase_iread": "ASE iread",
    "chemfiles_010": "chemfiles 0.10.4",
    "chemfiles_011": "chemfiles 0.11.0-rc1",
    "mdanalysis": "MDAnalysis XYZReader",
    "mdtraj": "MDTraj load_xyz",
}
# Fields the timed call does not return. The atom lines still contain them.
SKIPPED = {
    "readcon_rust": "",
    "readcon_py": "",
    "ase_iread": "",
    "chemfiles_010": (
        "timed call copies positions and cell lengths; "
        "species, masses and forces are not copied"
    ),
    "chemfiles_011": (
        "timed call copies positions and cell lengths; "
        "species, masses and forces are not copied"
    ),
    "mdanalysis": "positions only; species, masses, forces and the cell are not returned",
    "mdtraj": (
        "positions only, converted from the caller topology; "
        "species tokens, masses, forces and the cell are not returned; "
        "stored coordinates are nanometres"
    ),
}
_TOPOLOGY: dict[int, object] = {}


def case_map() -> dict[str, tuple[int, int, str]]:
    return {name: (atoms, frames, stem) for name, atoms, frames, stem in CASES}


def input_path(name: str) -> Path:
    if name == "grid-1e6":
        return extxyz_h2h.input_path("large-1")
    if name == "grid-10000":
        return extxyz_h2h.input_path("large-n")
    if name == "traj-32":
        return WORK / "inputs" / "frames-100000x32.xyz"
    raise KeyError(name)


def package_version(name: str) -> str:
    try:
        return metadata.version(name)
    except metadata.PackageNotFoundError:
        return "missing"


def cpu_governor() -> str:
    path = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    if not path.is_file():
        return "not visible"
    return path.read_text(encoding="utf-8").strip() or "not visible"


def environment() -> dict:
    return {
        "host": support.hardware_line(),
        "kernel": os.uname().release,
        "cpu_governor": cpu_governor(),
        "python": sys.version.split()[0],
        "rustc": subprocess.check_output(
            ["rustup", "run", "1.91.0", "rustc", "--version"], text=True
        ).strip(),
        "valgrind": subprocess.check_output(
            ["valgrind", "--version"], text=True
        ).strip(),
        "numpy": package_version("numpy"),
        "ase": package_version("ase"),
        "chemfiles": package_version("chemfiles"),
        "chemfiles_011": CHEM_TAG,
        "chemfiles_011_commit": CHEM_COMMIT,
        "MDAnalysis": package_version("MDAnalysis"),
        "mdtraj": package_version("mdtraj"),
        "readcon": package_version("readcon"),
        "git": support.git_line(["rev-parse", "HEAD"]),
        "pin": "taskset -c 0",
        "threads": 1,
        "repeats": REPEATS,
        "warmup": 1,
        "quantile": "type-7",
        "position_tolerance_A": POS_TOL,
        "memory_scope": (
            "fresh-process VmHWM and massif peak of a process that only reads"
        ),
        "not_armed": (
            "GROMACS and zarrtraj are noted in the paper and do not read these XYZ files"
        ),
        "skipped": SKIPPED,
    }


def ensure_rust() -> None:
    if extxyz_h2h.BINARY.is_file():
        return
    subprocess.check_call(
        [
            "rustup",
            "run",
            "1.91.0",
            "cargo",
            "build",
            "--release",
            "--example",
            "extxyz_time",
        ],
        cwd=ROOT,
    )


def _chem_so() -> Path:
    lib = CHEM_PREFIX / "lib" / CHEM_SO_NAME
    if lib.is_file():
        return lib
    alt = CHEM_PREFIX / "lib64" / CHEM_SO_NAME
    if alt.is_file():
        return alt
    raise RuntimeError(f"missing {lib}")


def ensure_chemfiles_011() -> Path:
    """Build chemfiles 0.11.0-rc1 and point a private Python package at it.

    PyPI chemfiles is 0.10.4. The 0.11.0-rc1 C API renames ``nsteps`` to
    ``size``, ``read_step`` to ``read_at``, and frame ``step`` to ``index``,
    and it removes ``chfl_add_configuration``. The Python call sequence is
    the 0.10.4 Trajectory loop with those symbols retargeted.
    """
    so = CHEM_PREFIX / "lib" / CHEM_SO_NAME
    if not so.is_file() and not (CHEM_PREFIX / "lib64" / CHEM_SO_NAME).is_file():
        if not (CHEM_SRC / "CMakeLists.txt").is_file():
            CHEM_SRC.parent.mkdir(parents=True, exist_ok=True)
            subprocess.check_call(
                [
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    CHEM_TAG,
                    "https://github.com/chemfiles/chemfiles.git",
                    str(CHEM_SRC),
                ]
            )
        head = subprocess.check_output(
            ["git", "-C", str(CHEM_SRC), "rev-parse", "HEAD"], text=True
        ).strip()
        if not head.startswith(CHEM_COMMIT):
            raise RuntimeError(f"chemfiles HEAD {head} is not {CHEM_COMMIT}")
        build = WORK / "chemfiles-build"
        env = os.environ.copy()
        env["CC"] = "gcc"
        env["CXX"] = "g++"
        subprocess.check_call(
            [
                "cmake",
                "-S",
                str(CHEM_SRC),
                "-B",
                str(build),
                "-DCMAKE_BUILD_TYPE=Release",
                "-DBUILD_SHARED_LIBS=ON",
                f"-DCMAKE_INSTALL_PREFIX={CHEM_PREFIX}",
            ],
            env=env,
        )
        subprocess.check_call(["cmake", "--build", str(build), "-j", "8"])
        subprocess.check_call(["cmake", "--install", str(build)])
    so = _chem_so()
    stamp = CHEM_PY / ".stamp"
    if stamp.is_file() and stamp.read_text(encoding="utf-8").strip() == str(so):
        return so
    site = Path(metadata.distribution("chemfiles").locate_file("chemfiles"))
    if CHEM_PY.exists():
        shutil.rmtree(CHEM_PY)
    dest = CHEM_PY / "chemfiles"
    shutil.copytree(
        site,
        dest,
        ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "libchemfiles.so"),
    )
    replacements = (
        ("chfl_frame_set_step", "chfl_frame_set_index"),
        ("chfl_frame_step", "chfl_frame_index"),
        ("chfl_trajectory_read_step", "chfl_trajectory_read_at"),
        ("chfl_trajectory_nsteps", "chfl_trajectory_size"),
    )
    for path in dest.glob("*.py"):
        text = path.read_text(encoding="utf-8")
        for old, new in replacements:
            text = text.replace(old, new)
        text = text.replace(
            "    c_lib.chfl_add_configuration.argtypes = [c_char_p]\n"
            "    c_lib.chfl_add_configuration.restype = chfl_status\n"
            "    c_lib.chfl_add_configuration.errcheck = _check_return_code\n",
            "    # chfl_add_configuration was removed in chemfiles 0.11.0-rc1\n",
        )
        path.write_text(text, encoding="utf-8")
    init = dest / "__init__.py"
    init.write_text(
        init.read_text(encoding="utf-8").replace(
            '__version__ = "0.10.4"', '__version__ = "0.11.0-rc1"'
        ),
        encoding="utf-8",
    )
    (dest / "external.py").write_text(
        f'EXTERNAL_CHEMFILES = "{so}"\n', encoding="utf-8"
    )
    stamp.write_text(str(so) + "\n", encoding="utf-8")
    return so


def ensure_inputs() -> None:
    ensure_rust()
    ensure_chemfiles_011()
    extxyz_h2h.ensure_inputs()
    path = input_path("traj-32")
    stamp = path.with_suffix(".stamp")
    want = "32 100000\n"
    if not path.is_file() or not stamp.is_file() or stamp.read_text(encoding="utf-8") != want:
        path.parent.mkdir(parents=True, exist_ok=True)
        extxyz_h2h.write_extxyz(path, 32, 100_000)
        stamp.write_text(want, encoding="utf-8")


def reference(n_atoms: int, n_frames: int) -> dict:
    base = extxyz_h2h.reference(n_atoms, n_frames)
    # One frame of positions is enough. Every frame of these files is that frame.
    base["positions"] = base["positions"][:1]
    base["forces"] = base["forces"][:1]
    base["masses"] = base["masses"][:1]
    base["lengths"] = base["lengths"][:1]
    base["species"] = base["species"][:1]
    return base


def _stack(frames: list[np.ndarray]) -> np.ndarray:
    return np.stack(frames)


def read_readcon(path: Path) -> dict:
    import readcon

    got = readcon.read_extxyz(os.fspath(path))
    positions = np.asarray(got["positions"], dtype=np.float64)
    return {
        "n_frames": int(positions.shape[0]),
        "n_atoms": int(positions.shape[1]),
        "positions": positions,
        "masses": np.asarray(got["masses"], dtype=np.float64),
        "forces": np.asarray(got["arrays"]["forces"], dtype=np.float64),
        "species": [[str(symbol) for symbol in frame] for frame in got["species"]],
        "lengths": np.asarray(got["cell"], dtype=np.float64)[:, :3],
    }


def read_ase(path: Path) -> dict:
    from ase.io import iread

    positions = []
    masses = []
    forces = []
    species = []
    lengths = []
    for atoms in iread(os.fspath(path), index=":", format="extxyz"):
        positions.append(np.asarray(atoms.positions, dtype=np.float64).copy())
        masses.append(np.asarray(atoms.get_masses(), dtype=np.float64).copy())
        forces.append(np.asarray(atoms.get_forces(), dtype=np.float64).copy())
        species.append([str(symbol) for symbol in atoms.get_chemical_symbols()])
        lengths.append(np.asarray(atoms.cell.lengths(), dtype=np.float64).copy())
    return {
        "n_frames": len(positions),
        "n_atoms": int(positions[0].shape[0]),
        "positions": _stack(positions),
        "masses": _stack(masses),
        "forces": _stack(forces),
        "species": species,
        "lengths": _stack(lengths),
    }


def read_chemfiles_here(path: Path) -> dict:
    import chemfiles

    positions = []
    lengths = []
    with chemfiles.Trajectory(os.fspath(path), "r", "XYZ") as traj:
        n_frames = int(traj.nsteps)
        for _ in range(n_frames):
            frame = traj.read()
            positions.append(np.array(frame.positions, dtype=np.float64, copy=True))
            lengths.append(
                np.asarray(frame.cell.lengths, dtype=np.float64).reshape(3).copy()
            )
    return {
        "n_frames": len(positions),
        "n_atoms": int(positions[0].shape[0]),
        "positions": _stack(positions),
        "masses": None,
        "forces": None,
        "species": None,
        "lengths": _stack(lengths),
    }


def read_mdanalysis(path: Path) -> dict:
    from MDAnalysis.coordinates.XYZ import XYZReader

    reader = XYZReader(os.fspath(path))
    frames = [np.array(reader.ts.positions, dtype=np.float64, copy=True)]
    while True:
        try:
            step = reader._read_next_timestep()
        except EOFError:
            break
        frames.append(np.array(step.positions, dtype=np.float64, copy=True))
    positions = _stack(frames)
    return {
        "n_frames": int(positions.shape[0]),
        "n_atoms": int(positions.shape[1]),
        "positions": positions,
        "masses": None,
        "forces": None,
        "species": None,
        "lengths": None,
    }


def _topology(n_atoms: int):
    cached = _TOPOLOGY.get(n_atoms)
    if cached is not None:
        return cached
    from mdtraj.core.element import get_by_symbol
    from mdtraj.core.topology import Topology

    topology = Topology()
    chain = topology.add_chain()
    residue = topology.add_residue("CU", chain)
    element = get_by_symbol("Cu")
    for _ in range(n_atoms):
        topology.add_atom("Cu", element, residue)
    _TOPOLOGY[n_atoms] = topology
    return topology


def read_mdtraj(path: Path, n_atoms: int) -> dict:
    import mdtraj

    trajectory = mdtraj.load_xyz(os.fspath(path), top=_topology(n_atoms))
    # load_xyz stores nanometres. The file is angstroms.
    positions = np.asarray(trajectory.xyz, dtype=np.float64) * 10.0
    return {
        "n_frames": int(positions.shape[0]),
        "n_atoms": int(positions.shape[1]),
        "positions": positions,
        "masses": None,
        "forces": None,
        "species": None,
        "lengths": None,
    }


def read_arm(arm: str, path: Path, n_atoms: int) -> dict:
    if arm == "readcon_rust":
        got = extxyz_h2h.read_rust(path)
        got["lengths"] = np.asarray(got["lengths"], dtype=np.float64)
        return got
    if arm == "readcon_py":
        return read_readcon(path)
    if arm == "ase_iread":
        return read_ase(path)
    if arm == "chemfiles_010":
        return read_chemfiles_here(path)
    if arm == "chemfiles_011":
        return _chemfiles_011_check(path)
    if arm == "mdanalysis":
        return read_mdanalysis(path)
    if arm == "mdtraj":
        return read_mdtraj(path, n_atoms)
    raise SystemExit(f"unknown arm {arm}")


def _max_abs(got: np.ndarray, expect: np.ndarray) -> float:
    return float(np.max(np.abs(np.asarray(got, dtype=np.float64) - expect)))


def compare(got: dict, expect: dict, arm: str) -> dict:
    reasons = []
    n_frames_ok = int(got["n_frames"]) == int(expect["n_frames"])
    n_atoms_ok = int(got["n_atoms"]) == int(expect["n_atoms"])
    if not n_frames_ok:
        reasons.append(f"frames {got['n_frames']} != {expect['n_frames']}")
    if not n_atoms_ok:
        reasons.append(f"atoms {got['n_atoms']} != {expect['n_atoms']}")
    pos = np.asarray(got["positions"], dtype=np.float64)
    ref = expect["positions"][0]
    coords_ok = False
    max_abs = None
    if pos.ndim == 3 and pos.shape[1:] == ref.shape:
        max_abs = _max_abs(pos, ref)
        coords_ok = max_abs <= POS_TOL
        if not coords_ok:
            reasons.append(f"positions max_abs {max_abs:.3e}")
    else:
        reasons.append(f"position shape {getattr(pos, 'shape', None)}")
    lengths = got.get("lengths")
    cell_ok = None
    cell_max = None
    if lengths is None:
        cell_ok = False
    else:
        cell_max = _max_abs(lengths, expect["lengths"][0])
        cell_ok = cell_max <= EXACT_TOL
        if not cell_ok:
            reasons.append(f"cell max_abs {cell_max:.3e}")
    species = got.get("species")
    species_ok = None
    if species is not None:
        species_ok = species == expect["species"] * int(got["n_frames"])
        if not species_ok:
            reasons.append("species")
    masses = got.get("masses")
    masses_ok = None
    if masses is not None:
        masses_ok = _max_abs(masses, expect["masses"][0]) <= EXACT_TOL
        if not masses_ok:
            reasons.append("masses")
    forces = got.get("forces")
    forces_ok = None
    if forces is not None:
        forces_ok = _max_abs(forces, expect["forces"][0]) <= EXACT_TOL
        if not forces_ok:
            reasons.append("forces")
    returned = coords_ok and n_frames_ok and n_atoms_ok
    if arm.startswith("readcon") or arm == "ase_iread":
        returned = bool(
            returned and cell_ok and species_ok and masses_ok and forces_ok
        )
    elif arm.startswith("chemfiles"):
        returned = bool(returned and cell_ok)
    hard = arm.startswith("readcon") and not returned
    note = SKIPPED.get(arm, "")
    if note and reasons:
        reasons.append(note)
    elif note and not returned:
        reasons.append(note)
    cell = None
    if lengths is not None:
        cell = np.asarray(lengths, dtype=np.float64)[0].tolist()
    return {
        "n_frames": int(got["n_frames"]),
        "n_atoms": int(got["n_atoms"]),
        "n_frames_ok": n_frames_ok,
        "n_atoms_ok": n_atoms_ok,
        "coords_ok": coords_ok,
        "max_abs": max_abs,
        "cell": cell,
        "cell_ok": bool(cell_ok) if cell_ok is not None else False,
        "cell_max_abs": cell_max,
        "species_ok": species_ok,
        "masses_ok": masses_ok,
        "forces_ok": forces_ok,
        "correct": returned,
        "hard_fail": hard,
        "skipped": note,
        "reason": "; ".join(reasons) if reasons else "ok",
    }


def safe_read(arm: str, path: Path, n_atoms: int) -> tuple[dict | None, str | None]:
    try:
        return read_arm(arm, path, n_atoms), None
    except (KeyboardInterrupt, SystemExit):
        raise
    except BaseException as exc:  # noqa: BLE001
        return None, f"{type(exc).__name__}: {exc}".splitlines()[0][:500]


class _Pipe:
    def __init__(self, argv: list[str]) -> None:
        self.proc = subprocess.Popen(
            argv,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            cwd=ROOT,
            env=support.thread_env(),
        )

    def call(self, command: str) -> str:
        if self.proc.stdin is None or self.proc.stdout is None:
            raise RuntimeError("pipe is closed")
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            err = ""
            if self.proc.stderr is not None:
                err = self.proc.stderr.read()
            raise RuntimeError(support.sanitize(err or f"pipe closed on {command}"))
        return line.strip()

    def close(self) -> None:
        try:
            self.call("exit")
        except (OSError, RuntimeError):
            pass
        if self.proc.poll() is None:
            self.proc.kill()
        self.proc.wait()


def _rotate(items: list, shift: int) -> list:
    if not items:
        return items
    shift %= len(items)
    return items[shift:] + items[:shift]


def _time_arm(arm: str, path: Path, n_atoms: int, rust: _Pipe, chem: _Pipe) -> float:
    if arm == "readcon_rust":
        return float(rust.call(f"read {support.rel(path)}"))
    if arm == "chemfiles_011":
        return float(chem.call(f"read {path}"))
    started = time.perf_counter()
    read_arm(arm, path, n_atoms)
    return time.perf_counter() - started


def _warmup(arm: str, path: Path, n_atoms: int, rust: _Pipe, chem: _Pipe) -> None:
    if arm == "readcon_rust":
        rust.call(f"warmup-read {support.rel(path)}")
        return
    if arm == "chemfiles_011":
        chem.call(f"warmup {path}")
        return
    read_arm(arm, path, n_atoms)


def _chemfiles_011_check(path: Path) -> dict:
    proc = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), "check-011", str(path)],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if proc.returncode != 0:
        raise RuntimeError(
            support.sanitize(proc.stderr or proc.stdout or "chemfiles 0.11 check")
        )
    payload = json.loads(proc.stdout.strip().splitlines()[-1])
    positions = np.load(payload["positions"])
    lengths = np.load(payload["lengths"])
    return {
        "n_frames": int(positions.shape[0]),
        "n_atoms": int(positions.shape[1]),
        "positions": positions,
        "masses": None,
        "forces": None,
        "species": None,
        "lengths": lengths,
        "library_version": payload.get("version", ""),
    }


def hwm_kib() -> int:
    for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            return int(line.split()[1])
    return 0


def once(arm: str, name: str) -> None:
    """One read in this process. Prints the high-water mark."""
    ensure_inputs()
    _atoms, _frames, _stem = case_map()[name]
    path = input_path(name)
    if arm == "chemfiles_011":
        got = read_chemfiles_here(path)
    elif arm == "readcon_py":
        got = read_readcon(path)
    elif arm == "ase_iread":
        got = read_ase(path)
    elif arm == "chemfiles_010":
        got = read_chemfiles_here(path)
    elif arm == "mdanalysis":
        got = read_mdanalysis(path)
    elif arm == "mdtraj":
        got = read_mdtraj(path, _atoms)
    else:
        raise SystemExit(f"unknown once arm {arm}")
    total = float(np.sum(got["positions"]))
    print(json.dumps({"hwm_kib": hwm_kib(), "sum_pos": total, "n_atoms": got["n_atoms"]}))


def rss_of(arm: str, name: str) -> int:
    if arm == "readcon_rust":
        completed = support.run_binary(
            extxyz_h2h.rust_binary(), ["once", support.rel(input_path(name))]
        )
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or "rust once"))
        payload = json.loads(completed.stdout.strip().splitlines()[-1])
        return int(payload["hwm_kib"])
    argv = [sys.executable, str(Path(__file__).resolve()), "once", arm, name]
    env = support.thread_env()
    if arm == "chemfiles_011":
        env["PYTHONPATH"] = str(CHEM_PY) + os.pathsep + env.get("PYTHONPATH", "")
        argv = [sys.executable, str(Path(__file__).resolve()), "once", "chemfiles_011", name]
    completed = subprocess.run(
        argv, check=False, capture_output=True, text=True, env=env
    )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr or completed.stdout or "rss")
        )
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    return int(payload["hwm_kib"])


def massif_heap(arm: str, name: str) -> int:
    out = WORK / "massif" / f"{arm}-{name}.out"
    out.parent.mkdir(parents=True, exist_ok=True)
    env = support.thread_env()
    if arm == "readcon_rust":
        command = [str(extxyz_h2h.rust_binary()), "once", str(input_path(name))]
    else:
        command = [sys.executable, str(Path(__file__).resolve()), "once", arm, name]
        if arm == "chemfiles_011":
            env["PYTHONPATH"] = str(CHEM_PY) + os.pathsep + env.get("PYTHONPATH", "")
    completed = subprocess.run(
        [
            "valgrind",
            "--tool=massif",
            f"--massif-out-file={out}",
            "--time-unit=B",
            *command,
        ],
        check=False,
        capture_output=True,
        text=True,
        env=env,
    )
    if completed.returncode != 0:
        raise RuntimeError(support.sanitize(completed.stderr[-2000:] or "massif"))
    peak = 0
    for line in out.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("mem_heap_B="):
            peak = max(peak, int(line.split("=", 1)[1]))
    return peak


def failed_row(arm: str, name: str, path: Path, reason: str) -> dict:
    return {
        "samples": [],
        "n": 0,
        "median_s": None,
        "iqr_s": None,
        "q25_s": None,
        "q75_s": None,
        "correct": False,
        "reason": reason,
        "arm": arm,
        "op": "read",
        "case": name,
        "file": support.rel(path),
        "rss_kib": None,
        "massif_heap_bytes": None,
        "skipped": SKIPPED.get(arm, ""),
        "label": ARM_LABEL[arm],
    }


def write_csv(stem: str, rows: list[dict]) -> None:
    LOG_CSV.mkdir(parents=True, exist_ok=True)
    lines = ["arm,op,case,repeat,seconds"]
    for row in rows:
        for index, sample in enumerate(row.get("samples") or []):
            lines.append(
                f"{row['arm']},{row.get('op', 'read')},{row['case']},{index},{sample:.9f}"
            )
    (LOG_CSV / f"{stem}.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def emit(stem: str, rows: list[dict]) -> None:
    blocks = []
    for row in rows:
        if not row.get("samples"):
            blocks.append(json.dumps(row, default=str))
            continue
        blocks.append(
            support.format_result(
                row,
                {
                    "arm": row["arm"],
                    "label": row.get("label", ""),
                    "op": "read",
                    "case": row["case"],
                    "rss_kib": row.get("rss_kib", ""),
                    "massif_heap_bytes": row.get("massif_heap_bytes", ""),
                    "skipped": row.get("skipped", ""),
                },
            ).rstrip("\n")
        )
    support.write_log(
        stem,
        ["taskset", "-c", "0", "python3", "bench/h2h/asv/drive.py", "traj_session"],
        "\n".join(blocks),
        0 if rows and all(row.get("correct") for row in rows) else 1,
    )
    write_csv(stem, rows)


def run_cases(selected: set[str] | None, memory: bool) -> None:
    ensure_inputs()
    (WORK / "environment.json").write_text(
        json.dumps(environment(), indent=2) + "\n", encoding="utf-8"
    )
    rust = _Pipe([str(extxyz_h2h.rust_binary()), "serve"])
    chem = _Pipe(
        [sys.executable, str(Path(__file__).resolve()), "serve-011"]
    )
    grouped: dict[str, list[dict]] = {}
    try:
        for name, atoms, frames, stem in CASES:
            if selected is not None and name not in selected:
                continue
            path = input_path(name)
            expect = reference(atoms, frames)
            # Topology is an input load_xyz requires. Build it before the clock.
            _topology(atoms)
            live = []
            rows = []
            for arm in READ_ARMS:
                got, error = safe_read(arm, path, atoms)
                if error is not None:
                    rows.append(failed_row(arm, name, path, error))
                    print(f"fail {arm} {name}: {error}", file=sys.stderr, flush=True)
                    continue
                check = compare(got, expect, arm)
                del got
                if check["hard_fail"]:
                    raise RuntimeError(f"{arm} {name}: {check['reason']}")
                if not check["correct"]:
                    rows.append(failed_row(arm, name, path, check["reason"]))
                    print(
                        f"incorrect {arm} {name}: {check['reason']}",
                        file=sys.stderr,
                        flush=True,
                    )
                    continue
                _warmup(arm, path, atoms, rust, chem)
                live.append((arm, check))
            samples = {arm: [] for arm, _check in live}
            for repeat in range(REPEATS):
                for arm, _check in _rotate(live, repeat):
                    samples[arm].append(_time_arm(arm, path, atoms, rust, chem))
            for arm, check in live:
                row = support.stats_block(samples[arm])
                row.update(check)
                row.update(
                    {
                        "arm": arm,
                        "label": ARM_LABEL[arm],
                        "op": "read",
                        "case": name,
                        "file": support.rel(path),
                        "rss_kib": None,
                        "massif_heap_bytes": None,
                    }
                )
                rows.append(row)
            grouped[stem] = rows
            print(f"read {name} done", file=sys.stderr, flush=True)
            (WORK / "session-rows.json").write_text(
                json.dumps(grouped, default=str) + "\n", encoding="utf-8"
            )
    finally:
        rust.close()
        chem.close()
    if memory:
        _attach_memory(grouped)
    for stem, rows in grouped.items():
        emit(stem, rows)
    report()


def _attach_memory(grouped: dict[str, list[dict]]) -> None:
    for rows in grouped.values():
        for row in rows:
            if not row.get("samples"):
                continue
            if row.get("rss_kib") is None:
                row["rss_kib"] = rss_of(row["arm"], row["case"])
            if row.get("massif_heap_bytes") is None:
                row["massif_heap_bytes"] = massif_heap(row["arm"], row["case"])
            print(
                f"memory {row['arm']} {row['case']} "
                f"rss={row['rss_kib']} massif={row['massif_heap_bytes']}",
                file=sys.stderr,
                flush=True,
            )
            (WORK / "session-rows.json").write_text(
                json.dumps(grouped, default=str) + "\n", encoding="utf-8"
            )


def resume_memory() -> None:
    grouped = json.loads((WORK / "session-rows.json").read_text(encoding="utf-8"))
    _attach_memory(grouped)
    for stem, rows in grouped.items():
        emit(stem, rows)
    report()


def report() -> None:
    rows = []
    for path in sorted(support.LOG_DIR.glob("traj-*.stdout")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("{") and line.endswith("}"):
                rows.append(json.loads(line))
    env_path = WORK / "environment.json"
    document = {
        "environment": json.loads(env_path.read_text(encoding="utf-8"))
        if env_path.is_file()
        else environment(),
        "rows": rows,
    }
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def _serve_011() -> None:
    sys.path.insert(0, str(CHEM_PY))
    import chemfiles

    version = chemfiles.__version__
    for line in sys.stdin:
        text = line.strip()
        if not text:
            continue
        parts = text.split(maxsplit=1)
        command = parts[0]
        if command == "exit":
            print("ok", flush=True)
            return
        path = parts[1]
        if command == "warmup":
            read_chemfiles_here(Path(path))
            print("ok", flush=True)
            continue
        if command == "read":
            started = time.perf_counter()
            got = read_chemfiles_here(Path(path))
            elapsed = time.perf_counter() - started
            del got
            print(f"{elapsed:.9f}", flush=True)
            continue
        if command == "version":
            print(version, flush=True)
            continue
        raise SystemExit(f"unknown serve command {command}")


def _check_011(path: Path) -> None:
    sys.path.insert(0, str(CHEM_PY))
    import chemfiles

    got = read_chemfiles_here(path)
    out = WORK / "check011"
    out.mkdir(parents=True, exist_ok=True)
    pos_path = out / "positions.npy"
    len_path = out / "lengths.npy"
    np.save(pos_path, got["positions"])
    np.save(len_path, got["lengths"])
    print(
        json.dumps(
            {
                "version": chemfiles.__version__,
                "positions": str(pos_path),
                "lengths": str(len_path),
            }
        ),
        flush=True,
    )


def probe() -> None:
    """Read a two-frame file with every arm and print the position error."""
    ensure_rust()
    ensure_chemfiles_011()
    path = WORK / "probe.xyz"
    path.parent.mkdir(parents=True, exist_ok=True)
    extxyz_h2h.write_extxyz(path, 4, 2)
    expect = reference(4, 2)
    _topology(4)
    rust = _Pipe([str(extxyz_h2h.rust_binary()), "serve"])
    chem = _Pipe([sys.executable, str(Path(__file__).resolve()), "serve-011"])
    try:
        for arm in READ_ARMS:
            got, error = safe_read(arm, path, 4)
            if error is not None:
                print(f"{arm} ERROR {error}")
                continue
            check = compare(got, expect, arm)
            print(
                f"{arm} correct={check['correct']} max_abs={check['max_abs']} "
                f"cell={check['cell_ok']} species={check['species_ok']} "
                f"masses={check['masses_ok']} forces={check['forces_ok']} "
                f"reason={check['reason']}"
            )
            _warmup(arm, path, 4, rust, chem)
            sample = _time_arm(arm, path, 4, rust, chem)
            print(f"{arm} sample {sample:.6f}")
    finally:
        rust.close()
        chem.close()


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit("missing command")
    command = sys.argv[1]
    if command == "serve-011":
        ensure_chemfiles_011()
        _serve_011()
        return
    if command == "check-011":
        ensure_chemfiles_011()
        _check_011(Path(sys.argv[2]))
        return
    if command == "once":
        # A fresh process. Pinning is the caller's taskset, not a re-exec
        # that would drop the PYTHONPATH chemfiles package.
        once(sys.argv[2], sys.argv[3])
        return
    support.ensure_pinned()
    if command == "prepare":
        ensure_inputs()
        return
    if command == "probe":
        probe()
        return
    if command == "report":
        report()
        return
    if command == "session":
        run_cases(None, True)
        return
    if command == "time-only":
        run_cases(None, False)
        return
    if command == "resume-memory":
        resume_memory()
        return
    selected = {
        "grid-1e6": {"grid-1e6"},
        "grid-10000": {"grid-10000"},
        "traj-32": {"traj-32"},
    }
    if command in selected:
        run_cases(selected[command], True)
        return
    raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
