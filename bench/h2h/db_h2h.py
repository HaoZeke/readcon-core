# Copyright (c) readcon-core developers
"""readcon-db against ASE SQLite and an ASE trajectory.

One session appends the 100000-frame trajectory. One session reads the
same 1000 frames from stores that already hold that trajectory. One
pinned core, one thread, one warmup and six serial repeats. A row is
timed only after a correctness check. RSS and massif are fresh
processes. Append memory is the increase above the loaded input.
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

import compress_h2h
import extxyz_h2h
import support

os.environ["OMP_NUM_THREADS"] = "1"
os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["MKL_NUM_THREADS"] = "1"
os.environ["NUMEXPR_NUM_THREADS"] = "1"
os.environ["RAYON_NUM_THREADS"] = "1"

REPEATS = 6
ROOT = support.ROOT
WORK = support.WORK / "db"
LOG_CSV = support.BENCH / "db-logs"
RESULTS = support.BENCH / "db-results.json"
SCRIPT = Path(__file__).resolve()
ARMS = ("readcon_db", "ase_db", "ase_traj")
SEED = 20261111
N_RANDOM = 1000
CASE = "traj"


def package_version(name: str) -> str:
    try:
        return metadata.version(name)
    except metadata.PackageNotFoundError:
        return "missing"


def indices() -> list[int]:
    frames = compress_h2h.CASES[CASE]["frames"]
    drawn = np.random.default_rng(SEED).choice(frames, size=N_RANDOM, replace=False)
    return [int(value) for value in drawn]


def store_path(arm: str, kind: str, repeat: int | None = None) -> Path:
    if arm == "readcon_db":
        name = "readcon-db" if repeat is None else f"readcon-db-{repeat}"
    elif arm == "ase_db":
        name = "ase.db" if repeat is None else f"ase-{repeat}.db"
    else:
        name = "frames.traj" if repeat is None else f"frames-{repeat}.traj"
    folder = "random" if kind == "random" else "append"
    return WORK / folder / name


def clear(path: Path) -> None:
    if path.is_dir():
        shutil.rmtree(path)
    elif path.is_file():
        path.unlink()
    path.parent.mkdir(parents=True, exist_ok=True)


def load_inputs():
    compress_h2h.ensure_plain(CASE)
    frames = compress_h2h.load_frames(CASE)
    images = [frame.to_ase() for frame in frames]
    return frames, images


def append_store(arm: str, dest: Path, frames, images) -> int:
    clear(dest)
    if arm == "readcon_db":
        from readcon_db import ConCorpus

        db = ConCorpus(os.fspath(dest))
        count = db.append_trajectory_frames(1, frames)
        if int(count) != len(frames):
            raise RuntimeError(f"readcon-db stored {count} frames")
        return sum(path.stat().st_size for path in dest.rglob("*") if path.is_file())
    if arm == "ase_db":
        from ase.db import connect

        with connect(os.fspath(dest)) as db:
            for atoms in images:
                db.write(atoms)
        return dest.stat().st_size
    from ase.io import Trajectory

    traj = Trajectory(os.fspath(dest), "w")
    for atoms in images:
        traj.write(atoms)
    traj.close()
    return dest.stat().st_size


def positions_at(arm: str, dest: Path, frame_index: int) -> np.ndarray:
    if arm == "readcon_db":
        from readcon_db import ConCorpus

        frame = ConCorpus(os.fspath(dest)).get_frame(1, frame_index)
        return np.asarray(frame.xyz, dtype=np.float64)
    if arm == "ase_db":
        from ase.db import connect

        return np.asarray(
            connect(os.fspath(dest)).get(id=frame_index + 1).toatoms().positions,
            dtype=np.float64,
        )
    from ase.io import Trajectory

    traj = Trajectory(os.fspath(dest))
    try:
        return np.asarray(traj[frame_index].positions, dtype=np.float64)
    finally:
        traj.close()


def check_frames(arm: str, dest: Path, picked: list[int]) -> dict:
    grid, _cell = compress_h2h.reference(CASE)
    max_abs = 0.0
    for frame_index in picked:
        got = positions_at(arm, dest, frame_index)
        if got.shape != grid.shape:
            return {
                "correct": False,
                "reason": f"frame {frame_index} shape {got.shape}",
                "max_abs": None,
                "n_atoms": int(got.shape[0]) if got.ndim == 2 else 0,
                "n_frames": compress_h2h.CASES[CASE]["frames"],
                "n_atoms_ok": False,
                "cell_ok": True,
                "coords_ok": False,
                "cell": None,
            }
        max_abs = max(max_abs, float(np.max(np.abs(got - grid))))
    ok = max_abs <= support.COORD_TOL
    return {
        "correct": ok,
        "reason": "ok" if ok else f"coordinate max abs {max_abs:.3e} exceeds 1e-12",
        "max_abs": max_abs,
        "n_atoms": int(grid.shape[0]),
        "n_frames": compress_h2h.CASES[CASE]["frames"],
        "n_atoms_ok": True,
        "cell_ok": True,
        "coords_ok": ok,
        "cell": None,
    }


def read_random(arm: str, dest: Path, picked: list[int]) -> float:
    if arm == "readcon_db":
        from readcon_db import ConCorpus

        db = ConCorpus(os.fspath(dest), readonly=True)
        started = time.perf_counter()
        sink = 0.0
        for frame_index in picked:
            frame = db.get_frame(1, frame_index)
            sink += float(frame.atoms[0].x)
        return time.perf_counter() - started + sink * 0.0
    if arm == "ase_db":
        from ase.db import connect

        db = connect(os.fspath(dest))
        started = time.perf_counter()
        sink = 0.0
        for frame_index in picked:
            sink += float(db.get(id=frame_index + 1).toatoms().positions[0, 0])
        return time.perf_counter() - started + sink * 0.0
    from ase.io import Trajectory

    traj = Trajectory(os.fspath(dest))
    started = time.perf_counter()
    sink = 0.0
    for frame_index in picked:
        sink += float(traj[frame_index].positions[0, 0])
    elapsed = time.perf_counter() - started
    traj.close()
    return elapsed + sink * 0.0


def _rotate(items: list, shift: int) -> list:
    shift %= len(items)
    return items[shift:] + items[:shift]


def failed(arm: str, op: str, reason: str) -> dict:
    row = support.stats_block([0.0])
    row["samples"] = []
    row["n"] = 0
    row.update(
        {
            "arm": arm,
            "op": op,
            "case": CASE,
            "correct": False,
            "reason": reason,
            "max_abs": None,
            "n_atoms": None,
            "n_frames": None,
            "n_atoms_ok": False,
            "cell_ok": False,
            "coords_ok": False,
            "cell": None,
            "rss_kib": None,
            "massif_heap_bytes": None,
            "bytes": None,
            "file": "",
        }
    )
    return row


def emit(name: str, rows: list[dict]) -> None:
    blocks = []
    for row in rows:
        if not row["samples"]:
            blocks.append(json.dumps({k: v for k, v in row.items() if k != "samples"}))
            continue
        extra = {
            "arm": row["arm"],
            "op": row["op"],
            "case": row["case"],
            "bytes": row.get("bytes", ""),
            "rss_kib": row.get("rss_kib", ""),
            "massif_heap_bytes": row.get("massif_heap_bytes", ""),
        }
        blocks.append(support.format_result(row, extra).rstrip("\n"))
    support.LOG_DIR.mkdir(parents=True, exist_ok=True)
    (support.LOG_DIR / f"{name}.stdout").write_text(
        support.log_text(
            ["taskset", "-c", "0", "python3", "bench/h2h/asv/drive.py", name.replace("-", "_")],
            "\n".join(blocks),
            0,
        ),
        encoding="utf-8",
    )
    LOG_CSV.mkdir(parents=True, exist_ok=True)
    lines = ["arm,op,case,repeat,seconds"]
    for row in rows:
        for index, sample in enumerate(row["samples"]):
            lines.append(f"{row['arm']},{row['op']},{row['case']},{index},{sample:.9f}")
    (LOG_CSV / f"{name}.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_append() -> None:
    frames, images = load_inputs()
    picked = [0, len(frames) // 2, len(frames) - 1]
    rows = []
    live = []
    for arm in ARMS:
        dest = store_path(arm, "append", None)
        try:
            append_store(arm, dest, frames, images)
            check = check_frames(arm, dest, picked)
            clear(dest)
        except Exception as exc:  # noqa: BLE001
            rows.append(failed(arm, "append", f"{type(exc).__name__}: {exc}".splitlines()[0][:500]))
            continue
        if not check["correct"] and arm == "readcon_db":
            raise RuntimeError(f"{arm}: {check['reason']}")
        append_store(arm, dest, frames, images)
        clear(dest)
        live.append((arm, check))
    samples = {arm: [] for arm, _check in live}
    sizes = {arm: [] for arm, _check in live}
    for repeat in range(REPEATS):
        for arm, _check in _rotate(live, repeat):
            dest = store_path(arm, "append", repeat)
            started = time.perf_counter()
            nbytes = append_store(arm, dest, frames, images)
            samples[arm].append(time.perf_counter() - started)
            sizes[arm].append(nbytes)
            clear(dest)
    for arm, check in live:
        row = support.stats_block(samples[arm])
        row.update(check)
        row.update(
            {
                "arm": arm,
                "op": "append",
                "case": CASE,
                "bytes": int(np.median(sizes[arm])),
                "file": arm,
                "rss_kib": None,
                "massif_heap_bytes": None,
            }
        )
        rows.append(row)
    del frames, images
    for row in rows:
        if not row["samples"]:
            continue
        row["rss_kib"] = fresh("rss-append", row["arm"])
        row["massif_heap_bytes"] = fresh_massif_append(row["arm"])
    emit("db-append", rows)


def run_random() -> None:
    frames, images = load_inputs()
    picked = indices()
    probe = [picked[0], picked[len(picked) // 2], picked[-1]]
    rows = []
    live = []
    for arm in ARMS:
        dest = store_path(arm, "random")
        try:
            if not _store_ready(arm, dest):
                append_store(arm, dest, frames, images)
            check = check_frames(arm, dest, probe)
        except Exception as exc:  # noqa: BLE001
            rows.append(failed(arm, "random", f"{type(exc).__name__}: {exc}".splitlines()[0][:500]))
            continue
        if not check["correct"] and arm == "readcon_db":
            raise RuntimeError(f"{arm} random: {check['reason']}")
        read_random(arm, dest, picked)
        live.append((arm, dest, check))
    del frames, images
    samples = {arm: [] for arm, _dest, _check in live}
    for repeat in range(REPEATS):
        for arm, dest, _check in _rotate(live, repeat):
            samples[arm].append(read_random(arm, dest, picked))
    for arm, dest, check in live:
        row = support.stats_block(samples[arm])
        row.update(check)
        row.update(
            {
                "arm": arm,
                "op": "random",
                "case": CASE,
                "bytes": _bytes(dest),
                "file": support.rel(dest),
                "n_random": N_RANDOM,
                "seed": SEED,
                "rss_kib": None,
                "massif_heap_bytes": None,
            }
        )
        rows.append(row)
    for row in rows:
        if not row["samples"]:
            continue
        row["rss_kib"] = fresh("rss-random", row["arm"])
        row["massif_heap_bytes"] = fresh_massif_random(row["arm"])
    emit("db-random", rows)


def _store_ready(arm: str, dest: Path) -> bool:
    if arm == "readcon_db":
        return (dest / "data.mdb").is_file()
    return dest.is_file() and dest.stat().st_size > 0


def _bytes(path: Path) -> int:
    if path.is_dir():
        return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())
    return path.stat().st_size if path.is_file() else 0


def fresh(mode: str, arm: str) -> int:
    completed = subprocess.run(
        [sys.executable, str(SCRIPT), mode, arm],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(support.sanitize(completed.stderr or completed.stdout or mode))
    return int(json.loads(completed.stdout.strip().splitlines()[-1])["rss_kib"])


def fresh_massif_append(arm: str) -> int:
    out = WORK / "massif" / f"{arm}-append.out"
    marker = WORK / "massif" / f"{arm}-append.mark"
    return _massif(
        out,
        [sys.executable, str(SCRIPT), "window-append", arm, str(marker)],
        marker,
    )


def fresh_massif_random(arm: str) -> int:
    out = WORK / "massif" / f"{arm}-random.out"
    return _massif(out, [sys.executable, str(SCRIPT), "once-random", arm], None)


def _massif(out: Path, command: list[str], marker: Path | None) -> int:
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
        return compress_h2h.massif_peak(out)
    return extxyz_h2h.write_heap_delta(out, marker)


def hwm_kib() -> int:
    return compress_h2h.hwm_kib()


def window_append(arm: str, marker: Path) -> None:
    frames, images = load_inputs()
    library = extxyz_h2h.vgmark()
    library.watch_start()
    if library.vg_monitor(f"detailed_snapshot {marker}".encode()) != 0:
        raise RuntimeError("snapshot")
    dest = store_path(arm, "append", 99)
    append_store(arm, dest, frames, images)
    print(json.dumps({"rss_kib": int(library.watch_delta_kib())}))
    clear(dest)


def once_random(arm: str) -> None:
    picked = indices()
    dest = store_path(arm, "random")
    elapsed = read_random(arm, dest, picked)
    print(json.dumps({"rss_kib": hwm_kib(), "seconds": elapsed}))


def rss_append(arm: str) -> None:
    window_append(arm, WORK / "massif" / f"rss-{arm}.mark")


def report() -> None:
    rows = []
    for name in ("db-append", "db-random"):
        path = support.LOG_DIR / f"{name}.stdout"
        if not path.is_file():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("{") and line.endswith("}"):
                rows.append(json.loads(line))
    document = {
        "environment": {
            "host": support.hardware_line(),
            "kernel": os.uname().release,
            "python": sys.version.split()[0],
            "numpy": package_version("numpy"),
            "ase": package_version("ase"),
            "readcon": package_version("readcon"),
            "readcon_db": package_version("readcon-db"),
            "git": support.git_line(["rev-parse", "HEAD"]),
            "pin": "taskset -c 0",
            "threads": 1,
            "repeats": REPEATS,
            "warmup": 1,
            "quantile": "type-7",
            "random_frames": N_RANDOM,
            "seed": SEED,
            "ase_db": "connect() held with `with`, the batch write ASE documents",
            "frames": compress_h2h.CASES[CASE]["frames"],
            "atoms": compress_h2h.CASES[CASE]["atoms"],
        },
        "rows": rows,
    }
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    print(RESULTS)


def main() -> None:
    support.ensure_pinned()
    command = sys.argv[1] if len(sys.argv) > 1 else "report"
    if command == "append":
        run_append()
    elif command == "random":
        run_random()
    elif command == "report":
        report()
    elif command == "rss-append":
        rss_append(sys.argv[2])
    elif command == "rss-random":
        once_random(sys.argv[2])
    elif command == "window-append":
        window_append(sys.argv[2], Path(sys.argv[3]))
    elif command == "once-random":
        once_random(sys.argv[2])
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
