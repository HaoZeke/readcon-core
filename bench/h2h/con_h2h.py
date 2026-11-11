# Copyright (c) readcon-core developers
"""CON and convel head-to-head.

One session per comparison. Arms share the files, one pinned core, one
thread, one warmup and six serial repeats. A row is timed only after a
correctness check. RSS and massif are fresh processes. A write loads
first and reports the increase above that baseline.
"""

from __future__ import annotations

import json
import os
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
ROOT = support.ROOT
WORK = support.WORK / "con"
LOG_CSV = support.BENCH / "con-logs"
RESULTS = support.BENCH / "con-results.json"
SURVEY_SRC = (
    ROOT / "paper" / "cpc" / "src" / "figures" / "generated" / "survey_frame.con"
)
RUST = ROOT / "target" / "release" / "examples" / "con_time"
EON_BUILDS = {
    "eon_579738a": support.WORK / "eon-579738a",
    "eon_v360": support.WORK / "eon-v360",
}
N_ATOMS = 32
N_FRAMES = 100_000
MASS = 63.546


def rel(path: Path) -> str:
    return support.rel(path)


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
        "ovito": package_version("ovito"),
        "readcon": package_version("readcon"),
        "git": support.git_line(["rev-parse", "HEAD"]),
        "eon_579738a": "579738a9387e8d0ea55496f45dd057812cfab892",
        "eon_v360": "02f57df3442252e4cc8e9efaa70fc5b63663fb43",
        "pin": "taskset -c 0",
        "threads": 1,
        "repeats": REPEATS,
        "warmup": 1,
        "quantile": "type-7",
        "memory_scope": (
            "read: fresh-process VmHWM and massif peak of a process that only reads; "
            "write: fresh-process resident and heap increase above the post-load baseline"
        ),
    }


def geometry(n_atoms: int = N_ATOMS) -> tuple[np.ndarray, float]:
    positions = support.expected_positions(n_atoms)
    span = float(support.side_for(n_atoms))
    return positions, span


def write_classic(path: Path, n_frames: int, velocities: bool) -> None:
    positions, span = geometry()
    lines = []
    for frame in range(n_frames):
        lines.append(f"Frame {frame}")
        lines.append("copper grid")
        lines.append(f"{span:.1f} {span:.1f} {span:.1f}")
        lines.append("90.0 90.0 90.0")
        lines.append("0 0")
        lines.append("0 0")
        lines.append("1")
        lines.append(str(N_ATOMS))
        lines.append(f"{MASS}")
        lines.append("Cu")
        lines.append("Coordinates of Component 1")
        for index, (x, y, z) in enumerate(positions):
            lines.append(f"{x:.1f} {y:.1f} {z:.1f} 0 {index}")
        if velocities:
            lines.append("")
            lines.append("Cu")
            lines.append("Velocities of Component 1")
            for index in range(N_ATOMS):
                vx = (index % 3) * 0.01
                vy = (index % 5) * 0.02
                vz = float(index % 2) * 0.03
                lines.append(f"{vx:.2f} {vy:.2f} {vz:.2f} 0 {index}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def ensure_inputs() -> None:
    survey = WORK / "inputs" / "survey.con"
    survey.parent.mkdir(parents=True, exist_ok=True)
    if not survey.is_file():
        survey.write_bytes(SURVEY_SRC.read_bytes())
    con = WORK / "inputs" / "frames-100000x32.con"
    vel = WORK / "inputs" / "frames-100000x32.convel"
    if not con.is_file():
        write_classic(con, N_FRAMES, False)
    if not vel.is_file():
        write_classic(vel, N_FRAMES, True)
    extxyz_h2h.ensure_inputs()


def reference_grid_positions() -> np.ndarray:
    positions, _span = geometry()
    return positions


def rust_samples(mode: str, args: list[str]) -> list[float]:
    completed = support.run_binary(RUST, [mode, *args])
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr or completed.stdout or "rust")
        )
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    return [float(item) for item in payload["samples"]]


def native_samples(binary: Path, args: list[str], key: str) -> list[float]:
    completed = support.run_binary(binary, args)
    if completed.returncode != 0:
        raise RuntimeError(
            support.sanitize(completed.stderr or completed.stdout or str(binary))
        )
    if completed.stdout.strip().startswith("{"):
        payload = json.loads(completed.stdout.strip().splitlines()[-1])
        return [float(item) for item in payload["samples"]]
    samples = []
    for line in completed.stdout.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == key:
            samples.append(float(parts[1]))
    if not samples:
        raise RuntimeError(support.sanitize(completed.stdout or "no samples"))
    return samples


def read_readcon(path: Path) -> dict:
    import readcon

    frames = readcon.read_con(os.fspath(path))
    positions = []
    velocities = []
    species = []
    has_v = False
    for frame in frames:
        # Column views. Indexing ``.atoms`` builds one Python object per atom,
        # which dominated a single small convel frame.
        positions.append(np.array(frame.xyz, copy=True))
        species.append(list(frame.symbols))
        vel = frame.vel
        if vel is not None:
            has_v = True
            velocities.append(np.array(vel, copy=True))
    cell = frames[0].cell if frames else None
    return {
        "n_frames": len(frames),
        "n_atoms": len(positions[0]) if positions else 0,
        "positions": np.asarray(positions, dtype=np.float64),
        "velocities": np.asarray(velocities, dtype=np.float64) if has_v else None,
        "species": species,
        "cell": np.asarray(cell, dtype=np.float64) if cell is not None else None,
        "reader": "readcon",
    }


def read_ase_eon(path: Path) -> dict:
    from ase.io import read as ase_read

    images = ase_read(os.fspath(path), index=":", format="eon")
    if not isinstance(images, list):
        images = [images]
    positions = [image.positions for image in images]
    return {
        "n_frames": len(images),
        "n_atoms": int(images[0].positions.shape[0]) if images else 0,
        "positions": np.asarray(positions, dtype=np.float64),
        "velocities": None,
        "species": [list(image.get_chemical_symbols()) for image in images],
        "cell": np.concatenate([images[0].cell.lengths(), images[0].cell.angles()])
        if images
        else None,
        "reader": "ase",
    }


def read_loadcons(path: Path) -> dict:
    sys.path.insert(0, str(support.WORK / "eon-src"))
    from eon.fileio import loadcons

    images = loadcons(os.fspath(path))
    positions = np.asarray([image.r for image in images], dtype=np.float64)
    return {
        "n_frames": len(images),
        "n_atoms": int(positions.shape[1]) if len(images) else 0,
        "positions": positions,
        "velocities": None,
        "species": [list(image.names) for image in images],
        "cell": None,
        "reader": "loadcons",
    }


def ase_velocity_stop(path: Path) -> dict:
    """Where ASE's eon reader stops on a convel file."""
    lines = path.read_text(encoding="utf-8").splitlines()
    try:
        read_ase_eon(path)
    except (ValueError, RuntimeError, StopIteration) as exc:
        text = str(exc).splitlines()[0]
        token = text.split(":")[-1].strip().strip("'\"")
        line_no = None
        for number, line in enumerate(lines, start=1):
            if token and token in line:
                line_no = number
                break
        return {
            "stopped": True,
            "exception": type(exc).__name__,
            "message": text,
            "token": token,
            "line": line_no,
            "line_text": lines[line_no - 1] if line_no else "",
            "note": (
                "read_eon takes the blank line before the velocity block as the "
                "first header line of a second frame. process_header then parses "
                "the velocity title as the cell-length line."
            ),
        }
    return {"stopped": False}


def safe(call):
    try:
        return call(), None
    except (KeyboardInterrupt, SystemExit):
        raise
    except BaseException as exc:  # noqa: BLE001
        return None, f"{type(exc).__name__}: {exc}".splitlines()[0][:500]


def timed(call) -> list[float]:
    call()
    samples = []
    for _ in range(REPEATS):
        started = time.perf_counter()
        call()
        samples.append(time.perf_counter() - started)
    return samples


def emit(name: str, rows: list[dict]) -> None:
    blocks = []
    for row in rows:
        if not row.get("samples"):
            blocks.append(json.dumps(row, default=str))
            continue
        blocks.append(
            support.format_result(
                row,
                {
                    "arm": row.get("arm", ""),
                    "op": row.get("op", ""),
                    "case": row.get("case", ""),
                    "rss_kib": row.get("rss_kib", ""),
                    "massif_heap_bytes": row.get("massif_heap_bytes", ""),
                },
            ).rstrip("\n")
        )
    support.write_log(
        name,
        ["taskset", "-c", "0", "python3", "bench/h2h/con_h2h.py", name],
        "\n".join(blocks),
        0 if rows and all(row.get("correct") for row in rows) else 1,
    )
    LOG_CSV.mkdir(parents=True, exist_ok=True)
    lines = ["arm,op,case,repeat,seconds"]
    for row in rows:
        for index, sample in enumerate(row.get("samples") or []):
            lines.append(
                f"{row.get('arm')},{row.get('op')},{row.get('case')},{index},{sample:.9f}"
            )
    (LOG_CSV / f"{name}.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def failed(arm: str, op: str, case: str, reason: str, **extra) -> dict:
    row = {
        "samples": [],
        "n": 0,
        "median_s": None,
        "iqr_s": None,
        "correct": False,
        "reason": reason,
        "arm": arm,
        "op": op,
        "case": case,
        "rss_kib": None,
        "massif_heap_bytes": None,
    }
    row.update(extra)
    return row


def finish(
    samples: list[float],
    arm: str,
    op: str,
    case: str,
    correct: bool,
    reason: str,
    **extra,
):
    row = support.stats_block(samples)
    row.update(
        {
            "arm": arm,
            "op": op,
            "case": case,
            "correct": correct,
            "reason": reason,
            "rss_kib": None,
            "massif_heap_bytes": None,
        }
    )
    row.update(extra)
    return row


def positions_ok(got, expect: np.ndarray, tol: float = 1e-6) -> bool:
    if got is None or got.get("positions") is None:
        return False
    pos = np.asarray(got["positions"], dtype=np.float64)
    if pos.shape[1:] != expect.shape:
        return False
    return float(np.max(np.abs(pos - expect))) <= tol


def run_convel_read() -> list[dict]:
    path = WORK / "inputs" / "survey.con"
    stop = ase_velocity_stop(path)
    (WORK / "ase-velocity-stop.json").write_text(
        json.dumps(stop, indent=2) + "\n", encoding="utf-8"
    )
    rows = []
    expect = read_readcon(path)

    def rust_call():
        return rust_samples("time", [rel(path), str(REPEATS)])

    rows.append(
        finish(
            rust_call(),
            "readcon_rust",
            "read",
            "survey",
            True,
            "ok",
            velocities=True,
        )
    )
    rows.append(
        finish(
            timed(lambda: read_readcon(path)),
            "readcon_py",
            "read",
            "survey",
            True,
            "ok",
            velocities=True,
        )
    )
    c_samples = native_samples(
        support.WORK / "time_c",
        ["read", rel(path), rel(WORK / "outs" / "c-survey.con"), str(REPEATS)],
        "read",
    )
    rows.append(
        finish(c_samples, "readcon_c", "read", "survey", True, "ok", velocities=True)
    )
    got, err = safe(lambda: read_ase_eon(path))
    if err:
        rows.append(failed("ase_eon", "read", "survey", err, detail=stop))
    else:
        rows.append(
            finish(
                timed(lambda: read_ase_eon(path)),
                "ase_eon",
                "read",
                "survey",
                False,
                "read velocities",
            )
        )
    got, err = safe(lambda: read_loadcons(path))
    if err:
        rows.append(failed("loadcons", "read", "survey", err))
    else:
        ok = got["n_atoms"] == expect["n_atoms"] and got["n_frames"] == 1
        rows.append(
            finish(
                timed(lambda: read_loadcons(path)),
                "loadcons",
                "read",
                "survey",
                ok,
                "ok" if ok else "mismatch",
                velocities=False,
            )
        )
    for arm, root in EON_BUILDS.items():
        binary = root / "con_eon_time"
        if not binary.is_file():
            rows.append(failed(arm, "read", "survey", "missing con_eon_time"))
            continue
        try:
            samples = native_samples(binary, ["read", str(path), str(REPEATS)], "read")
        except RuntimeError as exc:
            rows.append(failed(arm, "read", "survey", str(exc)))
            continue
        rows.append(finish(samples, arm, "read", "survey", True, "ok"))
    for row in rows:
        if row.get("samples"):
            row["rss_kib"] = read_rss(row["arm"], path, "survey")
            row["massif_heap_bytes"] = massif_read(row["arm"], path)
    emit("con-convel-read", rows)
    return rows


def read_rss(arm: str, path: Path, case: str) -> int | None:
    if arm == "readcon_rust":
        completed = support.run_binary(RUST, ["once", rel(path)])
    elif arm == "readcon_c":
        completed = support.run_binary(
            support.WORK / "time_c",
            ["read", rel(path), rel(WORK / "outs" / "rss.con"), "1"],
        )
        if completed.returncode != 0:
            return None
        for line in reversed(completed.stdout.splitlines()):
            if line.startswith("hwm_kib"):
                return int(line.split()[1])
        return None
    elif arm.startswith("eon_"):
        binary = EON_BUILDS[arm] / "con_eon_time"
        completed = support.run_binary(binary, ["once", str(path)])
    else:
        completed = subprocess.run(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                "once-read",
                arm,
                str(path),
            ],
            check=False,
            capture_output=True,
            text=True,
            env=support.thread_env(),
        )
    if completed.returncode != 0:
        return None
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    return int(payload["hwm_kib"])


def once_write(arm: str, src: str, dest: str, marker: Path) -> None:
    import readcon
    from ase.io import read as ase_read
    from ase.io import write as ase_write

    library = extxyz_h2h.vgmark()
    if arm == "readcon_py":
        frames = readcon.read_con(src)
        library.watch_start()
        if library.vg_monitor(f"detailed_snapshot {marker}".encode()) != 0:
            raise RuntimeError("snapshot")
        readcon.write_con(dest, frames)
    elif arm == "ase_eon":
        if "survey" in Path(src).name or "survey" in dest:
            images, _frame = survey_atoms()
        else:
            images = ase_read(src, index=":", format="eon")
        library.watch_start()
        if library.vg_monitor(f"detailed_snapshot {marker}".encode()) != 0:
            raise RuntimeError("snapshot")
        ase_write(dest, images, format="eon")
    else:
        raise SystemExit(arm)
    print(json.dumps({"rss_kib": int(library.watch_delta_kib())}))


def once_read(arm: str, path: str) -> None:
    if arm == "readcon_py":
        read_readcon(Path(path))
    elif arm == "ase_eon":
        read_ase_eon(Path(path))
    elif arm == "loadcons":
        read_loadcons(Path(path))
    elif arm == "chemfiles":
        read_chemfiles(Path(path))
    elif arm == "ovito":
        read_ovito(Path(path))
    else:
        raise SystemExit(arm)
    print(json.dumps({"hwm_kib": hwm_kib()}))


def hwm_kib() -> int:
    for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            return int(line.split()[1])
    return 0


def read_chemfiles(path: Path) -> dict:
    import chemfiles

    positions = []
    cell = None
    shape = None
    has_velocities = None
    with chemfiles.Trajectory(os.fspath(path), "r", "XYZ") as traj:
        n_frames = traj.nsteps
        for _ in range(n_frames):
            frame = traj.read()
            positions.append(np.array(frame.positions, copy=True))
            cell = np.array(frame.cell.lengths, copy=True)
            shape = int(frame.cell.shape)
            has_velocities = bool(frame.has_velocities())
    return {
        "n_frames": len(positions),
        "n_atoms": int(positions[0].shape[0]) if positions else 0,
        "positions": np.asarray(positions),
        "cell_lengths": cell,
        "cell_shape": shape,
        "has_velocities": has_velocities,
        "pbc": None,
        "reader": "chemfiles",
    }


def read_ovito(path: Path) -> dict:
    from ovito.io import import_file

    pipeline = import_file(os.fspath(path))
    positions = []
    cell = None
    pbc = None
    for index in range(pipeline.source.num_frames):
        data = pipeline.compute(index)
        positions.append(np.array(data.particles.positions, copy=True))
        cell = np.array(data.cell[:, :3], copy=True)
        pbc = [bool(data.cell.pbc[i]) for i in range(3)]
    return {
        "n_frames": len(positions),
        "n_atoms": int(positions[0].shape[0]) if positions else 0,
        "positions": np.asarray(positions),
        "cell_matrix": cell,
        "pbc": pbc,
        "reader": "ovito",
    }


def run_frames_read() -> list[dict]:
    path = WORK / "inputs" / "frames-100000x32.con"
    expect = reference_grid_positions()
    rows = []
    c_samples = native_samples(
        support.WORK / "time_c",
        ["read", rel(path), rel(WORK / "outs" / "c-frames.con"), str(REPEATS)],
        "read",
    )
    rows.append(finish(c_samples, "readcon_c", "read", "frames-100000x32", True, "ok"))
    f_samples = native_samples(
        support.WORK / "time_f",
        ["read", rel(path), rel(WORK / "outs" / "f-frames.con"), str(REPEATS + 1)],
        "read",
    )
    rows.append(
        finish(f_samples[1:], "readcon_fortran", "read", "frames-100000x32", True, "ok")
    )
    for arm, root in EON_BUILDS.items():
        binary = root / "con_eon_time"
        if not binary.is_file():
            rows.append(failed(arm, "read", "frames-100000x32", "missing con_eon_time"))
            continue
        try:
            samples = native_samples(binary, ["read", str(path), str(REPEATS)], "read")
        except RuntimeError as exc:
            rows.append(failed(arm, "read", "frames-100000x32", str(exc)))
            continue
        rows.append(finish(samples, arm, "read", "frames-100000x32", True, "ok"))
    for arm, reader in (("chemfiles", read_chemfiles), ("ovito", read_ovito)):
        _got, err = safe(lambda reader=reader: reader(path))
        rows.append(failed(arm, "read", "frames-100000x32", err or "opened a CON file"))
    got = read_readcon(path)
    if not positions_ok(got, expect):
        raise RuntimeError("generated CON does not match the copper grid")
    for row in rows:
        if row.get("samples") and row["case"] == "frames-100000x32":
            row["rss_kib"] = read_rss(row["arm"], path, row["case"])
            if row["arm"] in ("readcon_c", "readcon_fortran") or row["arm"].startswith(
                "eon_"
            ):
                row["massif_heap_bytes"] = massif_read(row["arm"], path)
    emit("con-frames-read", rows)
    return rows


def massif_command(command: list[str], out: Path, marker: Path | None) -> int:
    out.parent.mkdir(parents=True, exist_ok=True)
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
        raise RuntimeError(support.sanitize(completed.stderr[-1500:] or "massif"))
    if marker is None:
        peak = 0
        for line in out.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("mem_heap_B="):
                peak = max(peak, int(line.split("=", 1)[1]))
        return peak
    return extxyz_h2h.write_heap_delta(out, marker)


def massif_read(arm: str, path: Path) -> int:
    out = WORK / "massif" / f"{arm}-read-{path.name}.out"
    if arm == "readcon_rust":
        command = [str(RUST), "once", rel(path)]
    elif arm == "readcon_c":
        command = [
            str(support.WORK / "time_c"),
            "read",
            rel(path),
            rel(WORK / "outs" / "m.con"),
            "1",
        ]
    elif arm == "readcon_fortran":
        command = [
            str(support.WORK / "time_f"),
            "read",
            rel(path),
            rel(WORK / "outs" / "m.con"),
            "1",
        ]
    elif arm.startswith("eon_"):
        command = [str(EON_BUILDS[arm] / "con_eon_time"), "once", str(path)]
    else:
        command = [
            sys.executable,
            str(Path(__file__).resolve()),
            "once-read",
            arm,
            str(path),
        ]
    return massif_command(command, out, None)


def survey_atoms():
    import readcon

    frame = readcon.read_con(os.fspath(WORK / "inputs" / "survey.con"))[0]
    return frame.to_ase(), frame


def write_survey_extxyz(path: Path) -> None:
    from ase.io import write as ase_write

    atoms, _frame = survey_atoms()
    path.parent.mkdir(parents=True, exist_ok=True)
    ase_write(os.fspath(path), atoms, format="extxyz")


def inspect_survey_extxyz(path: Path) -> dict:
    import readcon
    from ase.io import read as ase_read

    header = path.read_text(encoding="utf-8").splitlines()[1]
    data = readcon.read_extxyz(os.fspath(path))
    image = ase_read(os.fspath(path), format="extxyz")
    velocities = image.get_velocities()
    chem, chem_err = safe(lambda: read_chemfiles(path))
    ovito, ovito_err = safe(lambda: read_ovito(path))
    return {
        "header": header,
        "readcon_columns": list(data["arrays"]),
        "readcon_cell": np.asarray(data["cell"]).tolist(),
        "ase_pbc": [bool(value) for value in image.pbc],
        "ase_cell_lengths": image.cell.lengths().tolist(),
        "ase_velocity_row": None if velocities is None else velocities[4].tolist(),
        "chemfiles_error": chem_err,
        "chemfiles_pbc": None if chem is None else chem.get("pbc"),
        "chemfiles_lengths": None
        if chem is None or chem.get("cell_lengths") is None
        else np.asarray(chem["cell_lengths"]).tolist(),
        "ovito_error": ovito_err,
        "ovito_pbc": None if ovito is None else ovito.get("pbc"),
    }


def run_writes() -> list[dict]:
    import readcon
    from ase.io import write as ase_write

    rows = []
    jobs = (
        ("survey-con", WORK / "inputs" / "survey.con", False),
        ("survey-convel", WORK / "inputs" / "survey.con", True),
        ("frames-con", WORK / "inputs" / "frames-100000x32.con", False),
        ("frames-convel", WORK / "inputs" / "frames-100000x32.convel", True),
    )
    for case, src, convel in jobs:
        frames = readcon.read_con(os.fspath(src))
        suffix = "convel" if convel else "con"

        def write_py(frames=frames, case=case, suffix=suffix):
            readcon.write_con(
                os.fspath(WORK / "outs" / f"readcon-py-{case}.{suffix}"), frames
            )

        rows.append(finish(timed(write_py), "readcon_py", "write", case, True, "ok"))
        try:
            samples = rust_samples(
                "write",
                [
                    rel(src),
                    rel(WORK / "outs" / f"readcon-rust-{case}.{suffix}"),
                    str(REPEATS),
                ],
            )
            rows.append(finish(samples, "readcon_rust", "write", case, True, "ok"))
        except RuntimeError as exc:
            rows.append(failed("readcon_rust", "write", case, str(exc)))
        if case.startswith("survey"):
            atoms, _frame = survey_atoms()
            images = atoms
        else:
            from ase.io import read as ase_read

            images, err = safe(
                lambda src=src: ase_read(os.fspath(src), index=":", format="eon")
            )
            if err:
                rows.append(failed("ase_eon", "write", case, "load " + err))
                images = None
        if (
            images is not None
            and case.startswith("survey")
            or (images is not None and not case.startswith("survey"))
        ):
            dest = WORK / "outs" / f"ase-{case}.{suffix}"

            def write_ase(images=images, dest=dest):
                ase_write(os.fspath(dest), images, format="eon")

            _got, err = safe(write_ase)
            if err:
                rows.append(failed("ase_eon", "write", case, err))
            else:
                rows.append(
                    finish(timed(write_ase), "ase_eon", "write", case, True, "wrote")
                )
        for arm, root in EON_BUILDS.items():
            binary = root / "con_eon_time"
            if not binary.is_file():
                rows.append(failed(arm, "write", case, "missing con_eon_time"))
                continue
            mode = "write-convel" if convel else "write-con"
            dest = WORK / "outs" / f"{arm}-{case}.{suffix}"
            try:
                samples = native_samples(
                    binary, [mode, str(src), str(dest), str(REPEATS)], "write"
                )
            except RuntimeError as exc:
                rows.append(failed(arm, "write", case, str(exc)))
                continue
            rows.append(finish(samples, arm, "write", case, True, "ok"))
    attach_write_memory(rows)
    emit("con-write", rows)
    return rows


def attach_write_memory(rows: list[dict]) -> None:
    library = str(extxyz_h2h.ensure_vgmark())
    sources = {
        "survey-con": (WORK / "inputs" / "survey.con", "con", False),
        "survey-convel": (WORK / "inputs" / "survey.con", "convel", True),
        "frames-con": (WORK / "inputs" / "frames-100000x32.con", "con", False),
        "frames-convel": (WORK / "inputs" / "frames-100000x32.convel", "convel", True),
    }
    for row in rows:
        if not row.get("samples"):
            continue
        src, suffix, convel = sources[row["case"]]
        dest = WORK / "outs" / f"mem-{row['arm']}-{row['case']}.{suffix}"
        marker = WORK / "massif" / f"{row['arm']}-{row['case']}.mark"
        marker.parent.mkdir(parents=True, exist_ok=True)
        arm = row["arm"]
        try:
            if arm == "readcon_rust":
                completed = support.run_binary(
                    RUST, ["write-once", rel(src), rel(dest), library, str(marker)]
                )
            elif arm.startswith("eon_"):
                kind = "convel" if convel else "con"
                completed = support.run_binary(
                    EON_BUILDS[arm] / "con_eon_time",
                    ["write-once", str(src), str(dest), kind, library, str(marker)],
                )
            else:
                completed = subprocess.run(
                    [
                        sys.executable,
                        str(Path(__file__).resolve()),
                        "once-write",
                        arm,
                        str(src),
                        str(dest),
                        str(marker),
                    ],
                    check=False,
                    capture_output=True,
                    text=True,
                    env=support.thread_env(),
                )
            if completed.returncode == 0:
                payload = json.loads(completed.stdout.strip().splitlines()[-1])
                row["rss_kib"] = int(payload["rss_kib"])
            if row["case"].startswith("frames"):
                if arm == "readcon_rust":
                    command = [
                        str(RUST),
                        "write-once",
                        rel(src),
                        rel(dest),
                        library,
                        str(marker),
                    ]
                elif arm.startswith("eon_"):
                    kind = "convel" if convel else "con"
                    command = [
                        str(EON_BUILDS[arm] / "con_eon_time"),
                        "write-once",
                        str(src),
                        str(dest),
                        kind,
                        library,
                        str(marker),
                    ]
                else:
                    command = [
                        sys.executable,
                        str(Path(__file__).resolve()),
                        "once-write",
                        arm,
                        str(src),
                        str(dest),
                        str(marker),
                    ]
                row["massif_heap_bytes"] = massif_command(
                    command, WORK / "massif" / f"{arm}-{row['case']}.out", marker
                )
        except (OSError, RuntimeError, json.JSONDecodeError) as exc:
            row["reason"] = f"{row.get('reason') or ''} memory {exc}".strip()


def run_survey_extxyz() -> list[dict]:
    path = WORK / "outs" / "survey.extxyz"
    write_survey_extxyz(path)
    text = path.read_text(encoding="utf-8")
    lattice = ""
    for line in text.splitlines():
        if "Lattice=" in line:
            lattice = line
            break
    rows = []

    def read_survey_readcon():
        import readcon

        data = readcon.read_extxyz(os.fspath(path))
        return {
            "n_atoms": int(data["positions"].shape[1]),
            "n_frames": int(data["positions"].shape[0]),
            "positions": data["positions"],
            "pbc": None,
            "velocities": data["arrays"].get("momenta"),
            "columns": list(data["arrays"]),
        }

    def read_survey_ase():
        from ase.io import read as ase_read

        image = ase_read(os.fspath(path), format="extxyz")
        return {
            "n_atoms": len(image),
            "n_frames": 1,
            "positions": image.positions.reshape(1, len(image), 3),
            "pbc": [bool(value) for value in image.pbc],
            "velocities": image.get_velocities(),
        }

    readers = (
        ("readcon_py", read_survey_readcon),
        ("ase_extxyz", read_survey_ase),
        ("chemfiles", lambda: read_chemfiles(path)),
        ("ovito", lambda: read_ovito(path)),
    )
    detail = {"lattice_line": lattice}
    (WORK / "survey-extxyz-header.txt").write_text(lattice + "\n", encoding="utf-8")
    for arm, reader in readers:
        got, err = safe(reader)
        if err:
            rows.append(failed(arm, "read", "survey-extxyz", err, lattice_line=lattice))
            detail[arm] = {"error": err}
            continue
        rows.append(
            finish(
                timed(reader),
                arm,
                "read",
                "survey-extxyz",
                True,
                "ok",
                n_atoms=got.get("n_atoms"),
                n_frames=got.get("n_frames"),
                pbc=got.get("pbc"),
                has_velocities=got.get("velocities") is not None,
            )
        )
        detail[arm] = {
            "n_atoms": got.get("n_atoms"),
            "pbc": got.get("pbc"),
            "cell_lengths": None
            if got.get("cell_lengths") is None
            else np.asarray(got["cell_lengths"]).tolist(),
            "has_velocities": got.get("velocities") is not None,
        }
    detail["inspect"] = inspect_survey_extxyz(path)
    (WORK / "survey-extxyz-readers.json").write_text(
        json.dumps(detail, indent=2, default=str) + "\n", encoding="utf-8"
    )
    emit("con-survey-extxyz", rows)
    return rows


def run_grid_extxyz() -> list[dict]:
    rows = []
    for name, atoms, frames, _fixture in extxyz_h2h.CASES:
        path = extxyz_h2h.input_path(name)
        expect = extxyz_h2h.reference(atoms, frames)
        for arm, reader in (
            ("chemfiles", read_chemfiles),
            ("ovito", read_ovito),
        ):
            got, err = safe(lambda reader=reader, path=path: reader(path))
            if err:
                rows.append(failed(arm, "read", name, err))
                continue
            ok = positions_ok(got, expect["positions"][0])
            rows.append(
                finish(
                    timed(lambda reader=reader, path=path: reader(path)),
                    arm,
                    "read",
                    name,
                    ok,
                    "ok" if ok else "coordinates differ",
                    pbc=got.get("pbc"),
                )
            )
    for row in rows:
        if not row.get("samples"):
            continue
        path = extxyz_h2h.input_path(row["case"])
        row["rss_kib"] = read_rss(row["arm"], path, row["case"])
        if row["case"] in ("large-1", "large-n"):
            row["massif_heap_bytes"] = massif_read(row["arm"], path)
    emit("con-grid-extxyz", rows)
    return rows


def main_command(command: str) -> None:
    sys.argv = [sys.argv[0], command]
    main()


def main() -> None:
    if len(sys.argv) >= 2 and sys.argv[1] == "once-read":
        once_read(sys.argv[2], sys.argv[3])
        return
    if len(sys.argv) >= 2 and sys.argv[1] == "once-write":
        once_write(sys.argv[2], sys.argv[3], sys.argv[4], Path(sys.argv[5]))
        return
    support.ensure_pinned()
    command = sys.argv[1] if len(sys.argv) > 1 else "session"
    ensure_inputs()
    support.compile_tools()
    WORK.mkdir(parents=True, exist_ok=True)
    (WORK / "environment.json").write_text(
        json.dumps(environment(), indent=2) + "\n", encoding="utf-8"
    )
    if command in ("session", "convel"):
        run_convel_read()
    if command in ("session", "frames"):
        run_frames_read()
    if command in ("session", "write"):
        run_writes()
    if command in ("session", "survey-extxyz"):
        run_survey_extxyz()
    if command in ("session", "grid"):
        run_grid_extxyz()
    if command in ("session", "report"):
        report()
    elif command not in (
        "session",
        "convel",
        "frames",
        "write",
        "survey-extxyz",
        "grid",
        "report",
    ):
        raise SystemExit(f"unknown command {command}")


def report() -> None:
    rows = []
    for path in sorted(support.LOG_DIR.glob("con-*.stdout")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("{") and line.endswith("}"):
                rows.append(json.loads(line))
    document = {
        "environment": json.loads(
            (WORK / "environment.json").read_text(encoding="utf-8")
        )
        if (WORK / "environment.json").is_file()
        else environment(),
        "ase_velocity_stop": json.loads(
            (WORK / "ase-velocity-stop.json").read_text(encoding="utf-8")
        )
        if (WORK / "ase-velocity-stop.json").is_file()
        else {},
        "rows": rows,
    }
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
