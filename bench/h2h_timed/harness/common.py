# Copyright (c) readcon developers
"""Protocol helpers for the extended-XYZ rerun."""

from __future__ import annotations

import hashlib
import os
import subprocess
from pathlib import Path

import numpy as np

_WORK = Path.home() / "h2h-work"
REPO = _WORK / "readcon-v0180"
DATA = _WORK / "data"
PYTHON = _WORK / "venv" / "bin" / "python"
COORD_TOL = 1e-8
LENGTH_TOL = 1e-8
ANGLE_TOL = 1e-6


def recorded_path(path: Path) -> str:
    """Path relative to the home directory, for the next timed run."""
    import sys

    root = str(Path(__file__).resolve().parents[1])
    if root not in sys.path:
        sys.path.insert(0, root)
    from recorded_path import recorded_path as record

    return record(path)


def g(value: float) -> str:
    return format(float(value), ".17g")


def type7(values: list[float], probability: float) -> float:
    ordered = np.sort(np.asarray(values, dtype=np.float64))
    if ordered.size == 1:
        return float(ordered[0])
    return float(np.quantile(ordered, probability, method="linear"))


def median_iqr(values: list[float]) -> tuple[float, float, float, float]:
    return (
        type7(values, 0.5),
        type7(values, 0.25),
        type7(values, 0.75),
        type7(values, 0.75) - type7(values, 0.25),
    )


def git_capture() -> tuple[str, str]:
    rev = subprocess.check_output(
        ["git", "-C", str(REPO), "rev-parse", "HEAD"], text=True
    ).strip()
    status = subprocess.check_output(
        ["git", "-C", str(REPO), "status", "--porcelain=v1"], text=True
    )
    return rev, status


def cache_fraction(path: Path) -> float:
    import ctypes

    size = path.stat().st_size
    if size <= 0:
        return 1.0
    page = os.sysconf("SC_PAGE_SIZE")
    pages = (size + page - 1) // page
    buf = (ctypes.c_ubyte * pages)()
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mmap.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_long,
    ]
    libc.mmap.restype = ctypes.c_void_p
    libc.munmap.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    libc.munmap.restype = ctypes.c_int
    libc.mincore.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_ubyte),
    ]
    libc.mincore.restype = ctypes.c_int
    fd = os.open(path, os.O_RDONLY)
    try:
        address = libc.mmap(None, size, 1, 2, fd, 0)
        if not address or address == ctypes.c_void_p(-1).value:
            raise OSError(ctypes.get_errno(), "mmap")
        try:
            rc = libc.mincore(ctypes.c_void_p(address), ctypes.c_size_t(size), buf)
            if rc != 0:
                raise OSError(ctypes.get_errno(), "mincore")
        finally:
            libc.munmap(ctypes.c_void_p(address), size)
    finally:
        os.close(fd)
    return sum(1 for byte in buf if byte & 1) / pages


def cache_label(fraction: float) -> str:
    if fraction >= 0.99:
        return "warm"
    if fraction <= 0.01:
        return "cold"
    return "partial"


def read_dump(path: Path) -> dict[str, np.ndarray]:
    raw = path.read_bytes()
    if raw[:4] != b"H2H1":
        raise ValueError(f"bad dump magic in {path}")
    n_frames, n_atoms = np.frombuffer(raw[4:20], dtype=np.uint64)
    n_frames = int(n_frames)
    n_atoms = int(n_atoms)
    offset = 20
    pos_n = n_frames * n_atoms * 3
    lng_n = n_frames * 3
    typ_n = n_frames * n_atoms
    pos = (
        np.frombuffer(raw, dtype=np.float64, count=pos_n, offset=offset)
        .reshape(n_frames, n_atoms, 3)
        .copy()
    )
    offset += pos_n * 8
    lengths = (
        np.frombuffer(raw, dtype=np.float64, count=lng_n, offset=offset)
        .reshape(n_frames, 3)
        .copy()
    )
    offset += lng_n * 8
    angles = (
        np.frombuffer(raw, dtype=np.float64, count=lng_n, offset=offset)
        .reshape(n_frames, 3)
        .copy()
    )
    offset += lng_n * 8
    types = (
        np.frombuffer(raw, dtype=np.int32, count=typ_n, offset=offset)
        .reshape(n_frames, n_atoms)
        .copy()
    )
    return {"positions": pos, "lengths": lengths, "angles": angles, "types": types}


def compare_dumps(result: dict, reference: dict) -> dict:
    got = result["positions"]
    exp = reference["positions"]
    report = {
        "frames_ok": got.shape[0] == exp.shape[0],
        "atoms_ok": got.shape == exp.shape,
        "correct": False,
    }
    if got.shape != exp.shape:
        report["max_abs_dx"] = None
        return report
    report["max_abs_dx"] = float(np.max(np.abs(got - exp))) if got.size else 0.0
    report["length_max"] = float(
        np.max(np.abs(result["lengths"] - reference["lengths"]))
    )
    report["angle_max"] = float(np.max(np.abs(result["angles"] - reference["angles"])))
    report["types_ok"] = bool(np.array_equal(result["types"], reference["types"]))
    report["correct"] = (
        report["max_abs_dx"] <= COORD_TOL
        and report["length_max"] <= LENGTH_TOL
        and report["angle_max"] <= ANGLE_TOL
        and report["types_ok"]
    )
    return report


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def timed_wheel_record() -> dict[str, dict[str, str]]:
    import importlib.metadata as metadata

    wanted = ("numpy", "ase", "chemfiles", "ovito", "MDAnalysis", "pymatgen")
    lock = (REPO / "requirements.lock").read_text(encoding="utf-8")
    found: dict[str, dict[str, str]] = {}
    lines = lock.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index].strip()
        if "==" in line and not line.startswith("#"):
            name, version = line.split("==", 1)
            name = name.strip()
            version = version.split("\\")[0].strip()
            digest = ""
            if index + 1 < len(lines) and "--hash=sha256:" in lines[index + 1]:
                digest = lines[index + 1].split("sha256:", 1)[1].strip().split()[0]
            if name in wanted:
                installed = metadata.version(name)
                if installed != version:
                    raise SystemExit(f"{name} installed {installed} lock {version}")
                found[name] = {"version": version, "wheel_sha256": digest}
        index += 1
    missing = [name for name in wanted if name not in found]
    if missing:
        raise SystemExit(f"lock is missing {missing}")
    return found


def script_sha256(directory: Path | None = None) -> dict[str, str]:
    root = directory or Path(__file__).resolve().parent
    return {
        path.name: sha256_file(path)
        for path in sorted(root.glob("*.py"))
        if path.is_file()
    }


def host_block() -> str:
    lscpu = subprocess.check_output(["lscpu"], text=True)
    cpuinfo = Path("/proc/cpuinfo").read_text(encoding="utf-8")

    def field(name: str) -> str:
        for line in cpuinfo.splitlines():
            key, _, value = line.partition(":")
            if key.strip() == name:
                return value.strip()
        return ""

    flags = field("flags")
    hypervisor = "present" if "hypervisor" in flags.split() else "absent"
    return (
        "lscpu\n"
        f"{lscpu}"
        "cpuinfo model_name\n"
        f"{field('model name')}\n"
        "cpuinfo family\n"
        f"{field('cpu family')}\n"
        "cpuinfo model\n"
        f"{field('model')}\n"
        "cpuinfo stepping\n"
        f"{field('stepping')}\n"
        "cpuinfo hypervisor_flag\n"
        f"{hypervisor}\n"
    )


def thread_env(threads: int = 1) -> dict[str, str]:
    env = os.environ.copy()
    for key in (
        "OMP_NUM_THREADS",
        "OPENBLAS_NUM_THREADS",
        "MKL_NUM_THREADS",
        "NUMEXPR_NUM_THREADS",
        "RAYON_NUM_THREADS",
        "VECLIB_MAXIMUM_THREADS",
        "BLIS_NUM_THREADS",
    ):
        env[key] = "1"
    env["RAYON_NUM_THREADS"] = str(threads)
    env["H2H_REPO"] = str(REPO)
    return env
