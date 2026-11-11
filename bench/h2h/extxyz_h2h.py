# Copyright (c) readcon-core developers
"""Head-to-head extended XYZ reads and writes.

Every arm sees the same files. Small files live in bench/h2h/fixtures.
Medium and large files are written under bench/h2h/work and are not committed.
One warmup call is excluded. The six timed repeats are serial, on one core.
``session`` runs every read arm and every write arm in one process, interleaved
inside each case, then records massif and a fresh-process high-water mark.
"""

from __future__ import annotations

import ctypes
import json
import os
import struct
import subprocess
import sys
import time
from importlib import metadata
from pathlib import Path

import numpy as np
import support

os.environ["OMP_NUM_THREADS"] = "1"
os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["MKL_NUM_THREADS"] = "1"
os.environ["NUMEXPR_NUM_THREADS"] = "1"
os.environ["RAYON_NUM_THREADS"] = "1"

REPEATS = 6
POS_TOL = 1e-9
CELL_TOL = 1e-6
MASS = float("63.546")
SCRIPT = Path(__file__).resolve()
ROOT = support.ROOT
WORK = support.WORK / "extxyz"
FIXTURES = support.BENCH / "fixtures"
LOG_CSV = support.BENCH / "extxyz-logs"
RESULTS = support.BENCH / "extxyz-results.json"
BINARY = ROOT / "target" / "release" / "examples" / "extxyz_time"
VGMARK_SRC = Path(__file__).resolve().parent / "vgmark.c"
VGMARK = WORK / "vgmark.so"
MASSIF_CASES = ("large-1", "large-n")

# name, atoms, frames, committed fixture
CASES = (
    ("small-1", 10, 1, True),
    ("small-n", 10, 8, True),
    ("medium-1", 1000, 1, False),
    ("medium-n", 100, 100, False),
    ("large-1", 1_000_000, 1, False),
    ("large-n", 100, 10_000, False),
)
READ_ARMS = (
    "readcon_rust",
    "readcon_py",
    "ase_extxyz",
    "ase_cextxyz",
    "extxyz",
    "extxyz_ng",
)
WRITE_ARMS = (
    "readcon_rust",
    "readcon_py",
    "ase_extxyz",
    "ase_cextxyz",
    "extxyz",
    "extxyz_ng",
)
ARM_LABEL = {
    "readcon_rust": "readcon-core Rust",
    "readcon_py": "readcon-core Python",
    "ase_extxyz": "ASE extxyz",
    "ase_cextxyz": "ASE cextxyz",
    "extxyz": "extxyz 0.4.6",
    "extxyz_ng": "extxyz-ng 0.0.2",
}


def case_map() -> dict[str, tuple[int, int, bool]]:
    return {name: (atoms, frames, fixture) for name, atoms, frames, fixture in CASES}


def input_path(name: str) -> Path:
    _atoms, _frames, fixture = case_map()[name]
    if fixture:
        return FIXTURES / f"{name}.extxyz"
    return WORK / "inputs" / f"{name}.extxyz"


def atom_forces(index: int) -> tuple[float, float, float]:
    return ((index % 5) * 0.25, (index % 3) * 0.5, float(index % 2))


def write_extxyz(path: Path, n_atoms: int, n_frames: int) -> None:
    """Write the copper grid used by the CON head-to-head, as extended XYZ."""
    side = support.side_for(n_atoms)
    span = float(side)
    coords: list[tuple[float, float, float]] = []
    written = 0
    for z in range(side):
        for y in range(side):
            for x in range(side):
                if written >= n_atoms:
                    break
                coords.append((float(x), float(y), float(z)))
                written += 1
            else:
                continue
            break
        else:
            continue
        break
    lattice = f"{span:.1f} 0.0 0.0 0.0 {span:.1f} 0.0 0.0 0.0 {span:.1f}"
    header = (
        f'Lattice="{lattice}" Properties=species:S:1:pos:R:3:masses:R:1:forces:R:3 '
        'pbc="T T T"\n'
    )
    rows = []
    for index, (x, y, z) in enumerate(coords):
        fx, fy, fz = atom_forces(index)
        rows.append(f"Cu {x:.1f} {y:.1f} {z:.1f} 63.546 {fx:.2f} {fy:.1f} {fz:.1f}\n")
    frame = f"{n_atoms}\n" + header + "".join(rows)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        for _ in range(n_frames):
            handle.write(frame)


def reference(n_atoms: int, n_frames: int) -> dict:
    positions = support.expected_positions(n_atoms)
    forces = np.array([atom_forces(i) for i in range(n_atoms)], dtype=np.float64)
    masses = np.full(n_atoms, MASS, dtype=np.float64)
    span = float(support.side_for(n_atoms))
    return {
        "n_frames": n_frames,
        "n_atoms": n_atoms,
        "positions": np.broadcast_to(positions, (n_frames, n_atoms, 3)).copy(),
        "forces": np.broadcast_to(forces, (n_frames, n_atoms, 3)).copy(),
        "masses": np.broadcast_to(masses, (n_frames, n_atoms)).copy(),
        "species": [["Cu"] * n_atoms for _ in range(n_frames)],
        "lengths": np.tile(
            np.array([span, span, span], dtype=np.float64), (n_frames, 1)
        ),
        "angles": np.tile(
            np.array([90.0, 90.0, 90.0], dtype=np.float64), (n_frames, 1)
        ),
    }


def angles_of(vectors: list[np.ndarray]) -> list[float]:
    def angle(left: np.ndarray, right: np.ndarray) -> float:
        cosine = float(
            np.dot(left, right) / (np.linalg.norm(left) * np.linalg.norm(right))
        )
        return float(np.degrees(np.arccos(np.clip(cosine, -1.0, 1.0))))

    first, second, third = vectors
    return [angle(second, third), angle(first, third), angle(first, second)]


def stack_frames(frames: list[dict]) -> dict:
    return {
        "n_frames": len(frames),
        "n_atoms": int(frames[0]["n_atoms"]),
        "positions": np.stack([item["positions"] for item in frames]),
        "masses": np.stack([item["masses"] for item in frames]),
        "forces": np.stack([item["forces"] for item in frames]),
        "species": [item["species"] for item in frames],
        "lengths": np.stack([item["lengths"] for item in frames]),
        "angles": np.stack([item["angles"] for item in frames]),
    }


def frame_from_columns(
    species, positions, masses, forces, vectors: list[np.ndarray]
) -> dict:
    positions = np.asarray(positions, dtype=np.float64)
    return {
        "n_atoms": int(positions.shape[0]),
        "positions": positions,
        "masses": np.asarray(masses, dtype=np.float64),
        "forces": np.asarray(forces, dtype=np.float64),
        "species": [str(item) for item in species],
        "lengths": np.array(
            [float(np.linalg.norm(item)) for item in vectors], dtype=np.float64
        ),
        "angles": np.array(angles_of(vectors), dtype=np.float64),
    }


def read_ase(path: Path, fmt: str) -> dict:
    from ase.io import read as ase_read

    images = ase_read(os.fspath(path), index=":", format=fmt)
    frames = []
    for image in images:
        cell = np.asarray(image.cell.array, dtype=np.float64)
        vectors = [cell[0], cell[1], cell[2]]
        if "forces" in image.arrays:
            forces = np.asarray(image.arrays["forces"], dtype=np.float64)
        else:
            forces = np.asarray(image.get_forces(), dtype=np.float64)
        frames.append(
            frame_from_columns(
                image.get_chemical_symbols(),
                image.positions,
                image.get_masses(),
                forces,
                vectors,
            )
        )
    return stack_frames(frames)


def read_extxyz(path: Path) -> dict:
    import extxyz

    loaded = extxyz.read_dicts(os.fspath(path))
    frames = loaded if isinstance(loaded, list) else [loaded]
    out = []
    for frame in frames:
        cell = np.asarray(frame.cell, dtype=np.float64)
        vectors = [cell[:, 0], cell[:, 1], cell[:, 2]]
        out.append(
            frame_from_columns(
                frame.arrays["species"],
                frame.arrays["pos"],
                frame.arrays["masses"],
                frame.arrays["forces"],
                vectors,
            )
        )
    return stack_frames(out)


def read_ng(path: Path) -> dict:
    import extxyz.extxyz as ng

    out = []
    for frame in ng.read_frames_from_file(os.fspath(path)):
        lattice = np.asarray(frame.info["Lattice"], dtype=np.float64)
        vectors = [lattice[0], lattice[1], lattice[2]]
        out.append(
            frame_from_columns(
                frame.arrs["species"],
                frame.arrs["pos"],
                frame.arrs["masses"],
                frame.arrs["forces"],
                vectors,
            )
        )
    return stack_frames(out)


def read_readcon(path: Path) -> dict:
    import readcon

    got = readcon.read_extxyz(os.fspath(path))
    positions = np.asarray(got["positions"], dtype=np.float64)
    masses = np.asarray(got["masses"], dtype=np.float64)
    cell = np.asarray(got["cell"], dtype=np.float64)
    species = [[str(symbol) for symbol in frame] for frame in got["species"]]
    forces = np.asarray(got["arrays"]["forces"], dtype=np.float64)
    return {
        "n_frames": int(positions.shape[0]),
        "n_atoms": int(positions.shape[1]),
        "positions": positions,
        "masses": masses,
        "forces": forces,
        "species": species,
        "lengths": cell[:, :3],
        "angles": cell[:, 3:],
    }


def rust_binary() -> Path:
    if not BINARY.is_file():
        raise RuntimeError("missing target/release/examples/extxyz_time")
    return BINARY


def read_rust(path: Path) -> dict:
    dump = WORK / "dump" / (path.stem + ".bin")
    dump.parent.mkdir(parents=True, exist_ok=True)
    completed = support.run_binary(
        rust_binary(), ["check", support.rel(path), support.rel(dump)]
    )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(
                completed.stderr or completed.stdout or "rust check failed"
            )
        )
    blob = dump.read_bytes()
    if blob[:4] != b"RCX2":
        raise RuntimeError("rust dump header does not match")
    n_frames, n_atoms = struct.unpack_from("<II", blob, 4)
    offset = 12
    count = n_frames * n_atoms * 3
    positions = np.frombuffer(blob, dtype="<f8", count=count, offset=offset).reshape(
        n_frames, n_atoms, 3
    )
    offset += count * 8
    masses = np.frombuffer(
        blob, dtype="<f8", count=n_frames * n_atoms, offset=offset
    ).reshape(n_frames, n_atoms)
    offset += n_frames * n_atoms * 8
    cell = np.frombuffer(blob, dtype="<f8", count=n_frames * 6, offset=offset).reshape(
        n_frames, 6
    )
    offset += n_frames * 6 * 8
    (n_species,) = struct.unpack_from("<I", blob, offset)
    offset += 4
    flat = []
    for _ in range(n_species):
        (nbytes,) = struct.unpack_from("<I", blob, offset)
        offset += 4
        flat.append(blob[offset : offset + nbytes].decode("utf-8"))
        offset += nbytes
    species = [flat[start : start + n_atoms] for start in range(0, len(flat), n_atoms)]
    (has_forces,) = struct.unpack_from("<I", blob, offset)
    offset += 4
    forces = None
    if has_forces:
        raw = blob[offset : offset + n_frames * n_atoms * 3 * 8]
        forces = np.frombuffer(raw, dtype="<f8").reshape(n_frames, n_atoms, 3).copy()
    return {
        "n_frames": int(n_frames),
        "n_atoms": int(n_atoms),
        "positions": positions.copy(),
        "masses": masses.copy(),
        "forces": forces,
        "species": species,
        "lengths": cell[:, :3].copy(),
        "angles": cell[:, 3:].copy(),
    }


def read_arm(arm: str, path: Path) -> dict:
    if arm == "readcon_rust":
        return read_rust(path)
    if arm == "readcon_py":
        return read_readcon(path)
    if arm == "ase_extxyz":
        return read_ase(path, "extxyz")
    if arm == "ase_cextxyz":
        return read_ase(path, "cextxyz")
    if arm == "extxyz":
        return read_extxyz(path)
    if arm == "extxyz_ng":
        return read_ng(path)
    raise SystemExit(f"unknown arm {arm}")


def load_for_write(arm: str, path: Path):
    if arm == "readcon_py":
        import readcon

        return readcon.read_extxyz(os.fspath(path))
    if arm == "readcon_rust":
        return path
    if arm == "ase_extxyz":
        from ase.io import read as ase_read

        return ase_read(os.fspath(path), index=":", format="extxyz")
    if arm == "ase_cextxyz":
        from ase.io import read as ase_read

        return ase_read(os.fspath(path), index=":", format="cextxyz")
    if arm == "extxyz":
        import extxyz

        loaded = extxyz.read_dicts(os.fspath(path))
        return loaded if isinstance(loaded, list) else [loaded]
    if arm == "extxyz_ng":
        import extxyz.extxyz as ng

        return list(ng.read_frames_from_file(os.fspath(path)))
    raise SystemExit(f"{arm} does not write extended XYZ")


def write_arm(arm: str, payload, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        path.unlink()
    if arm == "readcon_py":
        import readcon

        readcon.write_extxyz(os.fspath(path), payload)
        return
    if arm == "readcon_rust":
        raise SystemExit("the rust writer is timed in extxyz_time")
    if arm in ("ase_extxyz", "ase_cextxyz"):
        from ase.io import write as ase_write

        fmt = "extxyz" if arm == "ase_extxyz" else "cextxyz"
        ase_write(os.fspath(path), payload, format=fmt)
        return
    if arm == "extxyz":
        import extxyz

        extxyz.write_dicts(path, payload)
        return
    if arm == "extxyz_ng":
        import extxyz.extxyz as ng

        # write_frames rejects a Python iterator of Frame in 0.0.2.
        with path.open("wb") as handle:
            for frame in payload:
                ng.write_frame(handle, frame)
        return
    raise SystemExit(f"{arm} does not write extended XYZ")


def _field_ok(got, expect, tol: float) -> tuple[bool, float | None]:
    if got is None:
        return False, None
    delta = float(np.max(np.abs(np.asarray(got, dtype=np.float64) - expect)))
    return delta <= tol, delta


def compare(got: dict, expect: dict) -> dict:
    reasons = []
    atoms_ok = (
        got["n_atoms"] == expect["n_atoms"] and got["n_frames"] == expect["n_frames"]
    )
    if not atoms_ok:
        reasons.append(f"atoms {got['n_atoms']} frames {got['n_frames']}")
    lengths_ok, length_delta = _field_ok(got["lengths"], expect["lengths"], CELL_TOL)
    angles_ok, angle_delta = _field_ok(got["angles"], expect["angles"], CELL_TOL)
    cell_ok = lengths_ok and angles_ok
    if not cell_ok:
        reasons.append(f"cell length {length_delta} angle {angle_delta}")
    coords_ok, max_abs = _field_ok(got["positions"], expect["positions"], POS_TOL)
    if not coords_ok:
        reasons.append(f"positions max abs {max_abs}")
    masses_ok, mass_delta = _field_ok(got["masses"], expect["masses"], POS_TOL)
    if got["masses"] is None:
        reasons.append("masses not returned")
    elif not masses_ok:
        reasons.append(f"masses max abs {mass_delta}")
    if got["species"] is None:
        species_ok = False
        reasons.append("species not returned")
    else:
        species_ok = got["species"] == expect["species"]
        if not species_ok:
            reasons.append("species mismatch")
    forces_ok, force_delta = _field_ok(got["forces"], expect["forces"], POS_TOL)
    if got["forces"] is None:
        reasons.append("forces not returned")
    elif not forces_ok:
        reasons.append(f"forces max abs {force_delta}")
    hard_fail = not (
        atoms_ok and cell_ok and coords_ok and masses_ok and species_ok and forces_ok
    )
    cell = np.concatenate([got["lengths"][0], got["angles"][0]])
    return {
        "n_atoms": got["n_atoms"],
        "n_frames": got["n_frames"],
        "cell": [float(v) for v in cell],
        "max_abs": max_abs,
        "mass_max_abs": mass_delta,
        "force_max_abs": force_delta,
        "n_atoms_ok": atoms_ok,
        "cell_ok": cell_ok,
        "coords_ok": coords_ok,
        "masses_ok": masses_ok,
        "species_ok": species_ok,
        "forces_ok": forces_ok,
        "correct": atoms_ok
        and cell_ok
        and coords_ok
        and masses_ok
        and species_ok
        and forces_ok,
        "hard_fail": hard_fail,
        "reason": "; ".join(reasons) if reasons else "ok",
    }


def timed(call, repeats: int = REPEATS) -> tuple[list[float], object]:
    warmed = call()
    samples = []
    last = warmed
    for _ in range(repeats):
        started = time.perf_counter()
        last = call()
        samples.append(time.perf_counter() - started)
    return samples, last


def hwm_kib() -> int:
    for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            return int(line.split()[1])
    return 0


def package_version(name: str) -> str:
    try:
        return metadata.version(name)
    except metadata.PackageNotFoundError:
        return "missing"


def cpu_governor() -> str:
    path = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    if not path.is_file():
        return "not visible"
    text = path.read_text(encoding="utf-8").strip()
    return text or "not visible"


def environment() -> dict:
    rustc = subprocess.check_output(
        ["rustup", "run", "1.91.0", "rustc", "--version"], text=True
    ).strip()
    valgrind = subprocess.check_output(["valgrind", "--version"], text=True).strip()
    readcon_version = package_version("readcon")
    return {
        "host": support.hardware_line(),
        "kernel": os.uname().release,
        "cpu_governor": cpu_governor(),
        "hardware": support.hardware_line(),
        "python": sys.version.split()[0],
        "rustc": rustc,
        "valgrind": valgrind,
        "numpy": package_version("numpy"),
        "ase": package_version("ase"),
        "ase-extxyz": package_version("ase-extxyz"),
        "extxyz": package_version("extxyz"),
        "extxyz-ng": package_version("extxyz-ng"),
        "readcon": readcon_version,
        "git": support.git_line(["rev-parse", "HEAD"]),
        "pin": "taskset -c 0",
        "session": "one process; arms interleaved within each case",
        "threads": 1,
        "repeats": REPEATS,
        "warmup": 1,
        "quantile": "type-7",
        "pos_tol": POS_TOL,
        "cell_tol": CELL_TOL,
    }


def ensure_inputs() -> None:
    for name, atoms, frames, _fixture in CASES:
        path = input_path(name)
        if not path.is_file():
            write_extxyz(path, atoms, frames)


def safe_read(arm: str, path: Path) -> tuple[dict | None, str | None]:
    try:
        return read_arm(arm, path), None
    except (KeyboardInterrupt, SystemExit):
        raise
    except BaseException as exc:  # noqa: BLE001  pyo3 panics are not Exception
        return None, f"{type(exc).__name__}: {exc}".splitlines()[0][:500]


def failed_row(arm: str, op: str, name: str, path: Path, reason: str) -> dict:
    return {
        "samples": [],
        "n": 0,
        "median_s": None,
        "q25_s": None,
        "q75_s": None,
        "iqr_s": None,
        "n_atoms": None,
        "n_frames": None,
        "cell": None,
        "max_abs": None,
        "n_atoms_ok": False,
        "cell_ok": False,
        "coords_ok": False,
        "masses_ok": False,
        "species_ok": False,
        "forces_ok": False,
        "correct": False,
        "hard_fail": False,
        "reason": reason,
        "arm": arm,
        "op": op,
        "case": name,
        "file": support.rel(path),
        "rss_kib": None,
        "massif_heap_bytes": None,
    }


def check_all() -> list[dict]:
    findings = []
    for name, atoms, frames, _fixture in CASES:
        path = input_path(name)
        expect = reference(atoms, frames)
        for arm in READ_ARMS:
            got, error = safe_read(arm, path)
            if error is not None:
                findings.append(failed_row(arm, "read", name, path, error))
                continue
            row = compare(got, expect)
            row.update(
                {"arm": arm, "op": "read", "case": name, "file": support.rel(path)}
            )
            findings.append(row)
            if row["hard_fail"] and arm.startswith("readcon"):
                raise RuntimeError(f"{arm} {name}: {row['reason']}")
    return findings


def ng_file_limit() -> dict:
    """Largest generated frame extxyz-ng reads, and the next size it rejects."""
    import extxyz.extxyz as ng

    lines = input_path("medium-1").read_bytes().splitlines()
    header, atoms = lines[1], lines[2:]
    found = {
        "reads_bytes": None,
        "rejects_bytes": None,
        "reads_atoms": None,
        "rejects_atoms": None,
    }
    for count in range(200, 280):
        body = b"\n".join([str(count).encode(), header, *atoms[:count]]) + b"\n"
        path = WORK / "ng-limit.extxyz"
        path.write_bytes(body)
        try:
            list(ng.read_frames_from_file(str(path)))
        except (KeyboardInterrupt, SystemExit):
            raise
        except BaseException:  # noqa: BLE001  pyo3 panics are not Exception
            found["rejects_bytes"] = len(body)
            found["rejects_atoms"] = count
            break
        found["reads_bytes"] = len(body)
        found["reads_atoms"] = count
    return found


def prepare() -> None:
    ensure_inputs()
    findings = check_all()
    limit = ng_file_limit()
    WORK.mkdir(parents=True, exist_ok=True)
    (WORK / "ng-limit.json").write_text(
        json.dumps(limit, indent=2) + "\n", encoding="utf-8"
    )
    (WORK / "correctness.json").write_text(
        json.dumps(findings, indent=2) + "\n", encoding="utf-8"
    )
    (WORK / "environment.json").write_text(
        json.dumps(environment(), indent=2) + "\n", encoding="utf-8"
    )
    (WORK / "prepare.ok").write_text("ok\n", encoding="utf-8")


def write_csv(name: str, rows: list[dict]) -> None:
    LOG_CSV.mkdir(parents=True, exist_ok=True)
    lines = ["arm,op,case,repeat,seconds"]
    for row in rows:
        for index, sample in enumerate(row["samples"]):
            lines.append(f"{row['arm']},{row['op']},{row['case']},{index},{sample:.9f}")
    (LOG_CSV / f"{name}.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def emit(name: str, command: list[str], rows: list[dict]) -> None:
    blocks = []
    for row in rows:
        if not row["samples"]:
            blocks.append(json.dumps(row, default=str))
            continue
        extra = {
            "arm": row["arm"],
            "op": row["op"],
            "case": row["case"],
            "file": row.get("file", ""),
            "rss_kib": row.get("rss_kib", ""),
            "massif_heap_bytes": row.get("massif_heap_bytes", ""),
        }
        blocks.append(support.format_result(row, extra).rstrip("\n"))
    support.LOG_DIR.mkdir(parents=True, exist_ok=True)
    (support.LOG_DIR / f"{name}.stdout").write_text(
        support.log_text(
            command, "\n".join(blocks), 0 if all(row["correct"] for row in rows) else 1
        ),
        encoding="utf-8",
    )
    write_csv(name, rows)


def rss_of(arm: str, name: str) -> int:
    if arm == "readcon_rust":
        completed = support.run_binary(
            rust_binary(), ["once", support.rel(input_path(name))]
        )
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or "rust once failed"))
        payload = json.loads(completed.stdout.strip().splitlines()[-1])
        return int(payload["hwm_kib"])
    completed = subprocess.run(
        [sys.executable, str(SCRIPT), "once", arm, name],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr or completed.stdout or "rss failed")
        )
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    return int(payload["hwm_kib"])


def massif_heap(arm: str, name: str) -> int:
    out = WORK / "massif" / f"{arm}-{name}.out"
    out.parent.mkdir(parents=True, exist_ok=True)
    if arm == "readcon_rust":
        command = [str(rust_binary()), "once", str(input_path(name))]
    else:
        command = [sys.executable, str(SCRIPT), "once", arm, name]
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
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr[-2000:] or "massif failed")
        )
    peak = 0
    for line in out.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("mem_heap_B="):
            peak = max(peak, int(line.split("=", 1)[1]))
    return peak


def ensure_vgmark() -> Path:
    VGMARK.parent.mkdir(parents=True, exist_ok=True)
    if not VGMARK.is_file() or VGMARK.stat().st_mtime < VGMARK_SRC.stat().st_mtime:
        completed = subprocess.run(
            [
                "gcc",
                "-shared",
                "-fPIC",
                "-O2",
                "-pthread",
                "-o",
                str(VGMARK),
                str(VGMARK_SRC),
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            raise RuntimeError(completed.stderr or "gcc vgmark failed")
    return VGMARK


def vgmark():
    library = ctypes.CDLL(str(ensure_vgmark()))
    library.watch_start.restype = None
    library.watch_delta_kib.restype = ctypes.c_long
    library.vg_monitor.argtypes = [ctypes.c_char_p]
    library.vg_monitor.restype = ctypes.c_int
    return library


def snapshot_heaps(path: Path) -> list[tuple[int, int]]:
    found = []
    stamp = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("time="):
            stamp = int(line.split("=", 1)[1])
        elif line.startswith("mem_heap_B=") and stamp is not None:
            found.append((stamp, int(line.split("=", 1)[1])))
            stamp = None
    return found


def write_heap_delta(main: Path, marker: Path) -> int:
    marked = snapshot_heaps(marker)
    if not marked:
        raise RuntimeError(f"missing marker snapshot {marker}")
    mark_time, mark_heap = marked[-1]
    after = [heap for when, heap in snapshot_heaps(main) if when >= mark_time]
    peak = max(after) if after else mark_heap
    return max(0, peak - mark_heap)


def write_window(arm: str, name: str, marker: Path) -> None:
    """Load the input, then measure only the write."""
    payload = load_for_write(arm, input_path(name))
    library = vgmark()
    library.watch_start()
    if library.vg_monitor(f"detailed_snapshot {marker}".encode()) != 0:
        raise RuntimeError("snapshot")
    out = WORK / "outs" / f"window-{arm}-{name}.extxyz"
    write_arm(arm, payload, out)
    print(json.dumps({"rss_kib": int(library.watch_delta_kib())}))


def write_rss(arm: str, name: str) -> int:
    """Fresh process. The load sits outside the measured resident window."""
    out = WORK / "outs" / f"rss-{arm}-{name}.extxyz"
    marker = WORK / "massif" / f"{arm}-write-rss-{name}.mark"
    marker.parent.mkdir(parents=True, exist_ok=True)
    library = ensure_vgmark()
    if arm == "readcon_rust":
        completed = support.run_binary(
            rust_binary(),
            [
                "write-once",
                support.rel(input_path(name)),
                support.rel(out),
                str(library),
                str(marker),
            ],
        )
    else:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "write-window", arm, name, str(marker)],
            check=False,
            capture_output=True,
            text=True,
            env=support.thread_env(),
        )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr or completed.stdout or "write rss failed")
        )
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    return int(payload["rss_kib"])


def massif_write(arm: str, name: str) -> int:
    """Fresh process. Heap growth after the post-load snapshot."""
    out = WORK / "massif" / f"{arm}-write-{name}.out"
    marker = WORK / "massif" / f"{arm}-write-{name}.mark"
    out.parent.mkdir(parents=True, exist_ok=True)
    library = ensure_vgmark()
    dest = WORK / "outs" / f"massif-{arm}-{name}.extxyz"
    if arm == "readcon_rust":
        command = [
            str(rust_binary()),
            "write-once",
            str(input_path(name)),
            str(dest),
            str(library),
            str(marker),
        ]
    else:
        command = [
            sys.executable,
            str(SCRIPT),
            "write-window",
            arm,
            name,
            str(marker),
        ]
    completed = subprocess.run(
        [
            "valgrind",
            "--tool=massif",
            f"--massif-out-file={out}",
            "--time-unit=B",
            "--detailed-freq=1",
            "--max-snapshots=200",
            *command,
        ],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr[-2000:] or "write massif failed")
        )
    return write_heap_delta(out, marker)


def run_read(arm: str) -> None:
    rows = []
    for name, atoms, frames, _fixture in CASES:
        path = input_path(name)
        expect = reference(atoms, frames)
        got, error = safe_read(arm, path)
        if error is not None:
            rows.append(failed_row(arm, "read", name, path, error))
            continue
        check = compare(got, expect)
        if check["hard_fail"] and arm.startswith("readcon"):
            raise RuntimeError(f"{arm} {name}: {check['reason']}")

        def call(path: Path = path, arm: str = arm) -> dict:
            return read_arm(arm, path)

        if arm == "readcon_rust":
            completed = support.run_binary(
                rust_binary(), ["time", support.rel(path), str(REPEATS)]
            )
            if completed.returncode != 0:
                raise RuntimeError(
                    support.sanitize(completed.stderr or "rust time failed")
                )
            payload = json.loads(completed.stdout.strip().splitlines()[-1])
            samples = [float(item) for item in payload["samples"]]
        else:
            samples, _last = timed(call)
        row = support.stats_block(samples)
        row.update(check)
        row.update(
            {
                "arm": arm,
                "op": "read",
                "case": name,
                "file": support.rel(path),
                "rss_kib": rss_of(arm, name),
                "massif_heap_bytes": massif_heap(arm, name)
                if name in MASSIF_CASES
                else None,
            }
        )
        rows.append(row)
    emit(
        f"extxyz-{arm}-read",
        ["taskset", "-c", "0", "python3", "bench/h2h/extxyz_h2h.py", "read", arm],
        rows,
    )


def run_write(arm: str) -> None:
    rows = []
    note = ""
    if arm == "extxyz_ng":
        note = "write_frame loop; write_frames rejects a Python iterator of Frame"
    for name, atoms, frames, _fixture in CASES:
        path = input_path(name)
        try:
            payload = load_for_write(arm, path)
        except (KeyboardInterrupt, SystemExit):
            raise
        except BaseException as exc:  # noqa: BLE001  pyo3 panics are not Exception
            reason = f"{type(exc).__name__}: {exc}".splitlines()[0][:500]
            rows.append(failed_row(arm, "write", name, path, reason))
            continue
        out = WORK / "outs" / f"{arm}-{name}.extxyz"

        def call(payload=payload, out: Path = out, arm: str = arm) -> None:
            write_arm(arm, payload, out)

        if arm == "readcon_rust":
            completed = support.run_binary(
                rust_binary(),
                ["write", support.rel(path), support.rel(out), str(REPEATS)],
            )
            if completed.returncode != 0:
                raise RuntimeError(
                    support.sanitize(
                        completed.stderr or completed.stdout or "rust write failed"
                    )
                )
            payload_row = json.loads(completed.stdout.strip().splitlines()[-1])
            samples = [float(item) for item in payload_row["samples"]]
        else:
            samples, _last = timed(call)
        got, error = safe_read(arm, out)
        if error is not None:
            row = support.stats_block(samples)
            row.update(
                {
                    "correct": False,
                    "reason": "wrote; reread " + error,
                    "arm": arm,
                    "op": "write",
                    "case": name,
                    "file": support.rel(out),
                    "rss_kib": hwm_kib(),
                    "massif_heap_bytes": None,
                    "note": note,
                }
            )
            rows.append(row)
            continue
        check = compare(got, reference(atoms, frames))
        row = support.stats_block(samples)
        row.update(check)
        row.update(
            {
                "arm": arm,
                "op": "write",
                "case": name,
                "file": support.rel(out),
                "rss_kib": write_rss(arm, name),
                "massif_heap_bytes": massif_write(arm, name)
                if name in MASSIF_CASES
                else None,
                "note": note,
            }
        )
        rows.append(row)
    emit(
        f"extxyz-{arm}-write",
        ["taskset", "-c", "0", "python3", "bench/h2h/extxyz_h2h.py", "write", arm],
        rows,
    )


def once(arm: str, name: str) -> None:
    got = read_arm(arm, input_path(name))
    _used = float(np.sum(got["positions"]))
    print(
        json.dumps({"hwm_kib": hwm_kib(), "sum_pos": _used, "n_atoms": got["n_atoms"]})
    )


def parse_logs() -> list[dict]:
    rows = []
    for path in sorted(support.LOG_DIR.glob("extxyz-*.stdout")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("{") and line.endswith("}"):
                rows.append(json.loads(line))
    return rows


def report() -> None:
    env_path = WORK / "environment.json"
    correct_path = WORK / "correctness.json"
    limit_path = WORK / "ng-limit.json"
    document = {
        "environment": json.loads(env_path.read_text(encoding="utf-8"))
        if env_path.is_file()
        else environment(),
        "ng_file_limit": json.loads(limit_path.read_text(encoding="utf-8"))
        if limit_path.is_file()
        else {},
        "correctness": json.loads(correct_path.read_text(encoding="utf-8"))
        if correct_path.is_file()
        else [],
        "rows": parse_logs(),
    }
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


class RustPipe:
    """One warm ``extxyz_time serve`` process for single timed calls."""

    def __init__(self) -> None:
        self.proc = subprocess.Popen(
            [str(rust_binary()), "serve"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            cwd=ROOT,
            env=support.thread_env(),
        )

    def call(self, command: str) -> str:
        if self.proc.stdin is None or self.proc.stdout is None:
            raise RuntimeError("rust serve is closed")
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            err = ""
            if self.proc.stderr is not None:
                err = self.proc.stderr.read()
            raise RuntimeError(
                support.sanitize(err or f"rust serve closed on {command}")
            )
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


def _time_read(arm: str, path: Path, rust: RustPipe) -> float:
    if arm == "readcon_rust":
        return float(rust.call(f"read {support.rel(path)}"))
    started = time.perf_counter()
    read_arm(arm, path)
    return time.perf_counter() - started


def _time_write(arm: str, payload, dest: Path, src: Path, rust: RustPipe) -> float:
    if arm == "readcon_rust":
        return float(rust.call(f"write {support.rel(src)} {support.rel(dest)}"))
    started = time.perf_counter()
    write_arm(arm, payload, dest)
    return time.perf_counter() - started


def _attach_memory(rows: list[dict], op: str) -> None:
    for row in rows:
        if not row["samples"]:
            continue
        name = row["case"]
        arm = row["arm"]
        row["rss_kib"] = write_rss(arm, name) if op == "write" else rss_of(arm, name)
        if name in MASSIF_CASES:
            row["massif_heap_bytes"] = (
                massif_write(arm, name) if op == "write" else massif_heap(arm, name)
            )


def run_session() -> None:
    """Time every read and write arm in one process, then measure memory."""
    prepare()
    rust = RustPipe()
    read_rows = {arm: [] for arm in READ_ARMS}
    write_rows = {arm: [] for arm in WRITE_ARMS}
    try:
        for name, atoms, frames, _fixture in CASES:
            path = input_path(name)
            expect = reference(atoms, frames)
            live = []
            for arm in READ_ARMS:
                got, error = safe_read(arm, path)
                if error is not None:
                    read_rows[arm].append(failed_row(arm, "read", name, path, error))
                    continue
                check = compare(got, expect)
                del got
                if check["hard_fail"] and arm.startswith("readcon"):
                    raise RuntimeError(f"{arm} {name}: {check['reason']}")
                if arm == "readcon_rust":
                    rust.call(f"warmup-read {support.rel(path)}")
                else:
                    read_arm(arm, path)
                live.append((arm, check))
            samples = {arm: [] for arm, _check in live}
            for repeat in range(REPEATS):
                for arm, _check in _rotate(live, repeat):
                    samples[arm].append(_time_read(arm, path, rust))
            for arm, check in live:
                row = support.stats_block(samples[arm])
                row.update(check)
                row.update(
                    {
                        "arm": arm,
                        "op": "read",
                        "case": name,
                        "file": support.rel(path),
                        "rss_kib": None,
                        "massif_heap_bytes": None,
                    }
                )
                read_rows[arm].append(row)
            print(f"read {name} done", file=sys.stderr, flush=True)

        for name, atoms, frames, _fixture in CASES:
            path = input_path(name)
            expect = reference(atoms, frames)
            live = []
            for arm in WRITE_ARMS:
                dest = WORK / "outs" / f"{arm}-{name}.extxyz"
                try:
                    payload = load_for_write(arm, path)
                except (KeyboardInterrupt, SystemExit):
                    raise
                except BaseException as exc:  # noqa: BLE001  pyo3 panics are not Exception
                    reason = f"{type(exc).__name__}: {exc}".splitlines()[0][:500]
                    write_rows[arm].append(failed_row(arm, "write", name, path, reason))
                    continue
                if arm == "readcon_rust":
                    rust.call(f"warmup-write {support.rel(path)} {support.rel(dest)}")
                else:
                    write_arm(arm, payload, dest)
                note = ""
                if arm == "extxyz_ng":
                    note = "write_frame loop; write_frames rejects a Python iterator of Frame"
                live.append((arm, payload, dest, note))
            samples = {arm: [] for arm, _payload, _dest, _note in live}
            for repeat in range(REPEATS):
                for arm, payload, dest, _note in _rotate(live, repeat):
                    samples[arm].append(_time_write(arm, payload, dest, path, rust))
            for arm, _payload, dest, note in live:
                got, error = safe_read(arm, dest)
                if error is not None:
                    row = support.stats_block(samples[arm])
                    row.update(
                        {
                            "correct": False,
                            "reason": "wrote; reread " + error,
                            "arm": arm,
                            "op": "write",
                            "case": name,
                            "file": support.rel(dest),
                            "rss_kib": None,
                            "massif_heap_bytes": None,
                            "note": note,
                        }
                    )
                    write_rows[arm].append(row)
                    continue
                check = compare(got, expect)
                del got
                if check["hard_fail"] and arm.startswith("readcon"):
                    raise RuntimeError(f"{arm} write {name}: {check['reason']}")
                row = support.stats_block(samples[arm])
                row.update(check)
                row.update(
                    {
                        "arm": arm,
                        "op": "write",
                        "case": name,
                        "file": support.rel(dest),
                        "rss_kib": None,
                        "massif_heap_bytes": None,
                        "note": note,
                    }
                )
                write_rows[arm].append(row)
            print(f"write {name} done", file=sys.stderr, flush=True)
    finally:
        rust.close()

    (WORK / "session-rows.json").write_text(
        json.dumps({"read": read_rows, "write": write_rows}, indent=2) + "\n",
        encoding="utf-8",
    )
    for arm, rows in read_rows.items():
        _attach_memory(rows, "read")
        emit(
            f"extxyz-{arm}-read",
            [
                "taskset",
                "-c",
                "0",
                "python3",
                "bench/h2h/extxyz_h2h.py",
                "session",
            ],
            rows,
        )
        print(f"memory read {arm} done", file=sys.stderr, flush=True)
    for arm, rows in write_rows.items():
        _attach_memory(rows, "write")
        emit(
            f"extxyz-{arm}-write",
            [
                "taskset",
                "-c",
                "0",
                "python3",
                "bench/h2h/extxyz_h2h.py",
                "session",
            ],
            rows,
        )
        print(f"memory write {arm} done", file=sys.stderr, flush=True)
    report()


def refresh_memory() -> None:
    """Replace RSS and massif. Timed samples stay as they are."""
    ensure_vgmark()
    document = json.loads(RESULTS.read_text(encoding="utf-8"))
    only_arm = sys.argv[2] if len(sys.argv) > 2 else ""
    only_case = sys.argv[3] if len(sys.argv) > 3 else ""
    rows = document["rows"]
    for row in rows:
        if not row.get("samples"):
            continue
        if only_arm and row["arm"] != only_arm:
            continue
        if only_case and row["case"] != only_case:
            continue
        arm = row["arm"]
        name = row["case"]
        if row["op"] == "read":
            row["rss_kib"] = rss_of(arm, name)
            row["massif_heap_bytes"] = (
                massif_heap(arm, name) if name in MASSIF_CASES else None
            )
            row["memory_scope"] = "read"
        elif row["op"] == "write":
            row["rss_kib"] = write_rss(arm, name)
            row["massif_heap_bytes"] = (
                massif_write(arm, name) if name in MASSIF_CASES else None
            )
            row["memory_scope"] = "write"
        print(
            f"memory {row['op']} {arm} {name} "
            f"rss={row['rss_kib']} massif={row['massif_heap_bytes']}",
            file=sys.stderr,
            flush=True,
        )
        (WORK / "memory-checkpoint.json").write_text(
            json.dumps(document) + "\n", encoding="utf-8"
        )
    if only_arm or only_case:
        return
    document["environment"]["memory_scope"] = (
        "read: fresh-process VmHWM and massif peak of a process that only reads; "
        "write: fresh-process resident and heap increase above the post-load baseline"
    )
    document["environment"]["memory_git"] = support.git_line(["rev-parse", "HEAD"])
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    grouped: dict[tuple[str, str], list[dict]] = {}
    for row in rows:
        grouped.setdefault((row["arm"], row["op"]), []).append(row)
    for (arm, op), group in grouped.items():
        emit(
            f"extxyz-{arm}-{op}",
            ["taskset", "-c", "0", "python3", "bench/h2h/extxyz_h2h.py", "memory"],
            group,
        )


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit("missing command")
    command = sys.argv[1]
    if command == "once":
        once(sys.argv[2], sys.argv[3])
        return
    if command == "write-once":
        write_window(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
        return
    if command == "write-window":
        write_window(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
        return
    support.ensure_pinned()
    if command == "prepare":
        prepare()
    elif command == "read":
        run_read(sys.argv[2])
    elif command == "write":
        run_write(sys.argv[2])
    elif command == "fixtures":
        for name, atoms, frames, fixture in CASES:
            if fixture:
                write_extxyz(FIXTURES / f"{name}.extxyz", atoms, frames)
    elif command == "report":
        report()
    elif command == "session":
        run_session()
    elif command == "memory":
        refresh_memory()
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
