# Copyright (c) readcon-core developers
"""Compressed CON head-to-head.

One session per comparison. Arms share the files, one pinned core, one
thread, one warmup and six serial repeats. A row is timed only after a
correctness check. RSS and massif are fresh processes. A write loads
first and reports the increase above that baseline.

Codecs are whatever ``detect_compression_from_extension`` returns other
than none. The rust writer selects the codec from the path the same way.
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
WORK = support.WORK / "compress"
LOG_CSV = support.BENCH / "compress-logs"
RESULTS = support.BENCH / "compress-results.json"
RUST = ROOT / "target" / "release" / "examples" / "compress_time"
CLI = ROOT / "target" / "release" / "readcon-core"
SCRIPT = Path(__file__).resolve()

CASES = {
    "traj": {"atoms": 32, "frames": 100_000, "plain": "traj.con"},
    "cu": {"atoms": 1_000_000, "frames": 1, "plain": "cu.con"},
}


def package_version(name: str) -> str:
    try:
        return metadata.version(name)
    except metadata.PackageNotFoundError:
        return "missing"


def plain_path(case: str) -> Path:
    return WORK / "inputs" / CASES[case]["plain"]


def compressed_path(case: str, codec: str) -> Path:
    suffix = {"gzip": ".con.gz", "zstd": ".con.zst"}[codec]
    stem = Path(CASES[case]["plain"]).stem
    return WORK / "inputs" / f"{stem}{suffix}"


def xyz_path(case: str) -> Path:
    stem = Path(CASES[case]["plain"]).stem
    return WORK / "inputs" / f"{stem}.xyz.gz"


def reference(case: str) -> tuple[np.ndarray, np.ndarray]:
    spec = CASES[case]
    positions = support.expected_positions(spec["atoms"])
    cell = support.expected_cell(spec["atoms"])
    return positions, cell


def ensure_plain(case: str) -> None:
    path = plain_path(case)
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_file():
        return
    spec = CASES[case]
    if case == "traj":
        import con_h2h

        con_h2h.write_classic(path, spec["frames"], False)
    else:
        support.write_con(path, spec["atoms"], spec["frames"])


def rust_binary() -> Path:
    if not RUST.is_file():
        raise RuntimeError(f"missing {RUST}")
    return RUST


class RustPipe:
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
            raise RuntimeError(support.sanitize(err or f"rust serve closed on {command}"))
        return line.strip()

    def close(self) -> None:
        try:
            self.call("exit")
        except (OSError, RuntimeError):
            pass
        if self.proc.poll() is None:
            self.proc.kill()
        self.proc.wait()


def discover_codecs() -> list[str]:
    completed = support.run_binary(rust_binary(), ["codecs"])
    if completed.returncode != 0:
        raise RuntimeError(support.sanitize(completed.stderr or completed.stdout or "codecs"))
    payload = json.loads(completed.stdout.strip().splitlines()[-1])
    codecs = list(payload["codecs"])
    if not codecs:
        raise RuntimeError("compression_from_extension returned no codec")
    return codecs


def ensure_xyz(case: str) -> None:
    path = xyz_path(case)
    if path.is_file() and path.stat().st_size > 0:
        return
    import gzip

    positions, _cell = reference(case)
    spec = CASES[case]
    path.parent.mkdir(parents=True, exist_ok=True)
    with gzip.open(path, "wt", encoding="utf-8", compresslevel=6) as handle:
        for frame in range(spec["frames"]):
            handle.write(f"{spec['atoms']}\n")
            handle.write(f"copper grid frame {frame}\n")
            for x, y, z in positions:
                handle.write(f"Cu {x:.6f} {y:.6f} {z:.6f}\n")


def ensure_compressed(codecs: list[str]) -> None:
    rust = RustPipe()
    try:
        for case in CASES:
            ensure_plain(case)
            src = plain_path(case)
            for codec in codecs:
                dest = compressed_path(case, codec)
                if dest.is_file() and dest.stat().st_size > 0:
                    continue
                rust.call(f"warmup-write {support.rel(src)} {support.rel(dest)}")
    finally:
        rust.close()


def load_readcon(path: Path):
    import readcon

    return readcon.read_con(os.fspath(path), n_threads=1)


def frame_positions(frame) -> np.ndarray:
    return np.asarray(frame.xyz, dtype=np.float64)


def frame_cell(frame) -> np.ndarray:
    return np.concatenate(
        [np.asarray(frame.cell, dtype=np.float64), np.asarray(frame.angles, dtype=np.float64)]
    )


def judge(positions: np.ndarray | None, cell: np.ndarray | None, case: str) -> dict:
    spec = CASES[case]
    grid, expect_cell = reference(case)
    reasons = []
    n_atoms = 0 if positions is None else int(positions.shape[1])
    n_frames = 0 if positions is None else int(positions.shape[0])
    atoms_ok = n_atoms == spec["atoms"]
    frames_ok = n_frames == spec["frames"]
    if not atoms_ok:
        reasons.append(f"atom count {n_atoms} does not match {spec['atoms']}")
    if not frames_ok:
        reasons.append(f"frame count {n_frames} does not match {spec['frames']}")
    if cell is None:
        cell_ok = False
        cell_list = None
        reasons.append("cell is missing")
    else:
        cell_arr = np.asarray(cell, dtype=np.float64).reshape(-1)
        cell_list = [float(value) for value in cell_arr[:6]]
        if cell_arr.size < 6:
            cell_ok = False
            reasons.append("cell is missing")
        elif float(np.max(np.abs(cell_arr[:6] - expect_cell))) > support.CELL_TOL:
            err = float(np.max(np.abs(cell_arr[:6] - expect_cell)))
            cell_ok = False
            reasons.append(f"cell max abs {err:.3e} exceeds 1e-12")
        else:
            cell_ok = True
    max_abs = None
    coords_ok = False
    if positions is None:
        reasons.append("coordinates are missing")
    elif positions.shape != (spec["frames"], spec["atoms"], 3):
        reasons.append(
            f"coordinate shape {tuple(positions.shape)} does not match "
            f"{(spec['frames'], spec['atoms'], 3)}"
        )
    else:
        max_abs = float(np.max(np.abs(positions - grid)))
        coords_ok = max_abs <= support.COORD_TOL
        if not coords_ok:
            reasons.append(f"coordinate max abs {max_abs:.3e} exceeds 1e-12")
    return {
        "correct": atoms_ok and frames_ok and cell_ok and coords_ok,
        "n_atoms_ok": atoms_ok,
        "n_frames_ok": frames_ok,
        "cell_ok": cell_ok,
        "coords_ok": coords_ok,
        "max_abs": max_abs,
        "n_atoms": n_atoms,
        "n_frames": n_frames,
        "cell": cell_list,
        "reason": "; ".join(reasons),
    }


def stack_readcon(frames) -> tuple[np.ndarray, np.ndarray | None]:
    if not frames:
        return np.zeros((0, 0, 3)), None
    positions = np.empty((len(frames), len(frames[0].atoms), 3), dtype=np.float64)
    for index, frame in enumerate(frames):
        positions[index] = frame_positions(frame)
    return positions, frame_cell(frames[0])


def read_readcon(path: Path) -> tuple[np.ndarray, np.ndarray | None]:
    return stack_readcon(load_readcon(path))


def read_xyz_stream(path: Path) -> tuple[np.ndarray, None]:
    """Coordinate check that does not use ASE's xyz reader.

    That reader pops the front of the line list, so a 100000-frame file
    does not return.
    """
    import gzip

    opener = gzip.open if path.name.endswith(".gz") else open
    blocks = []
    with opener(path, "rt", encoding="utf-8") as handle:
        while True:
            header = handle.readline()
            if header == "":
                break
            count = int(header)
            handle.readline()
            block = np.empty((count, 3), dtype=np.float64)
            for index in range(count):
                parts = handle.readline().split()
                block[index, 0] = float(parts[-3])
                block[index, 1] = float(parts[-2])
                block[index, 2] = float(parts[-1])
            blocks.append(block)
    if not blocks:
        return np.zeros((0, 0, 3)), None
    return np.stack(blocks), None


def read_ase(path: Path, fmt: str) -> tuple[np.ndarray, np.ndarray | None]:
    from ase.io import read as ase_read

    images = ase_read(os.fspath(path), index=":", format=fmt)
    if not isinstance(images, list):
        images = [images]
    if not images:
        return np.zeros((0, 0, 3)), None
    positions = np.asarray([image.positions for image in images], dtype=np.float64)
    cell = np.concatenate([images[0].cell.lengths(), images[0].cell.angles()])
    return positions, cell


def write_readcon_py(frames, dest: Path) -> None:
    import readcon

    dest.parent.mkdir(parents=True, exist_ok=True)
    readcon.write_con(os.fspath(dest), frames)


def write_ase(images, dest: Path, fmt: str) -> None:
    from ase.io import write as ase_write

    dest.parent.mkdir(parents=True, exist_ok=True)
    ase_write(os.fspath(dest), images, format=fmt)


def pipe_read(path: Path) -> subprocess.CompletedProcess[str]:
    gzip = subprocess.Popen(
        ["gzip", "-dc", os.fspath(path)],
        stdout=subprocess.PIPE,
        env=support.thread_env(),
    )
    reader = subprocess.Popen(
        [str(CLI), "-"],
        stdin=gzip.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=support.thread_env(),
    )
    if gzip.stdout is not None:
        gzip.stdout.close()
    stdout, stderr = reader.communicate()
    gzip.wait()
    return subprocess.CompletedProcess(
        ["gzip", "-dc", os.fspath(path), "|", str(CLI), "-"],
        reader.returncode if gzip.returncode == 0 else gzip.returncode,
        stdout,
        stderr,
    )


def pipe_counts(stdout: str) -> tuple[int, int]:
    frames = 0
    atoms = 0
    for line in stdout.splitlines():
        if "Successfully parsed" in line:
            frames = int(line.split()[3])
        if line.strip().startswith("- Total atoms:"):
            atoms = int(line.split(":")[1])
    return frames, atoms


def failed_row(arm: str, op: str, case: str, path: Path, reason: str) -> dict:
    row = support.stats_block([0.0])
    row["samples"] = []
    row["n"] = 0
    row.update(
        {
            "correct": False,
            "n_atoms_ok": False,
            "cell_ok": False,
            "coords_ok": False,
            "max_abs": None,
            "n_atoms": None,
            "n_frames": None,
            "cell": None,
            "reason": reason,
            "arm": arm,
            "op": op,
            "case": case,
            "file": support.rel(path),
            "bytes": path.stat().st_size if path.is_file() else None,
            "rss_kib": None,
            "massif_heap_bytes": None,
        }
    )
    return row


def file_sizes(codecs: list[str]) -> dict:
    sizes = {}
    for case in CASES:
        plain = plain_path(case)
        entry = {"plain": plain.stat().st_size}
        for codec in codecs:
            path = compressed_path(case, codec)
            entry[codec] = path.stat().st_size if path.is_file() else None
        xyz = xyz_path(case)
        entry["xyz_gz"] = xyz.stat().st_size if xyz.is_file() else None
        sizes[case] = entry
    return sizes


def ensure_inputs() -> list[str]:
    codecs = discover_codecs()
    for case in CASES:
        ensure_plain(case)
        ensure_xyz(case)
    ensure_compressed(codecs)
    return codecs


def read_arms(codecs: list[str]) -> list[tuple[str, str, Path]]:
    arms = []
    for codec in codecs:
        arms.append((f"readcon_rust_{codec}", codec, None))
        arms.append((f"readcon_py_{codec}", codec, None))
    arms.append(("ase_eon_gz", "gzip", "eon"))
    arms.append(("ase_xyz_gz", "gzip", "xyz"))
    arms.append(("gzip_dc", "gzip", "pipe"))
    return arms


def write_arms(codecs: list[str]) -> list[str]:
    arms = []
    for codec in codecs:
        arms.append(f"readcon_rust_{codec}")
        arms.append(f"readcon_py_{codec}")
    arms.append("ase_eon_gz")
    arms.append("ase_xyz_gz")
    return arms


def _rotate(items: list, shift: int) -> list:
    if not items:
        return items
    shift %= len(items)
    return items[shift:] + items[:shift]


def hwm_kib() -> int:
    for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            return int(line.split()[1])
    return 0


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
            "file": row.get("file", ""),
            "bytes": row.get("bytes", ""),
            "rss_kib": row.get("rss_kib", ""),
            "massif_heap_bytes": row.get("massif_heap_bytes", ""),
        }
        blocks.append(support.format_result(row, extra).rstrip("\n"))
    support.LOG_DIR.mkdir(parents=True, exist_ok=True)
    command = [
        "taskset",
        "-c",
        "0",
        "python3",
        "bench/h2h/asv/drive.py",
        f"compress_{name.replace('compress-', '').replace('-', '_')}",
    ]
    (support.LOG_DIR / f"{name}.stdout").write_text(
        support.log_text(
            command, "\n".join(blocks), 0 if all(row.get("correct") or not str(row["arm"]).startswith("readcon") for row in rows) else 1
        ),
        encoding="utf-8",
    )
    LOG_CSV.mkdir(parents=True, exist_ok=True)
    lines = ["arm,op,case,repeat,seconds"]
    for row in rows:
        for index, sample in enumerate(row["samples"]):
            lines.append(
                f"{row['arm']},{row['op']},{row['case']},{index},{sample:.9f}"
            )
    (LOG_CSV / f"{name}.csv").write_text("\n".join(lines) + "\n", encoding="utf-8")


def check_read(arm: str, case: str, path: Path) -> dict:
    if arm == "gzip_dc":
        completed = pipe_read(path)
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or completed.stdout or "pipe"))
        frames, atoms = pipe_counts(completed.stdout or "")
        spec = CASES[case]
        if frames != spec["frames"] or atoms != spec["atoms"]:
            raise RuntimeError(f"pipe parsed {frames} frames and {atoms} atoms")
        positions, cell = read_readcon(path)
        return judge(positions, cell, case)
    if arm.startswith("readcon"):
        positions, cell = read_readcon(path)
        return judge(positions, cell, case)
    fmt = "eon" if "eon" in arm else "xyz"
    positions, cell = read_ase(path, fmt)
    return judge(positions, cell, case)


def time_read(arm: str, path: Path, rust: RustPipe) -> float:
    if arm.startswith("readcon_rust"):
        return float(rust.call(f"read {support.rel(path)}"))
    if arm == "gzip_dc":
        started = time.perf_counter()
        completed = pipe_read(path)
        elapsed = time.perf_counter() - started
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or "pipe"))
        return elapsed
    if arm.startswith("readcon_py"):
        started = time.perf_counter()
        frames = load_readcon(path)
        elapsed = time.perf_counter() - started
        _sink = float(frames[-1].atoms[0].x)
        return elapsed
    fmt = "eon" if "eon" in arm else "xyz"
    started = time.perf_counter()
    images = __import__("ase.io", fromlist=["read"]).read(
        os.fspath(path), index=":", format=fmt
    )
    elapsed = time.perf_counter() - started
    if not isinstance(images, list):
        images = [images]
    _sink = float(images[-1].positions[0, 0])
    return elapsed


def run_read(case: str) -> None:
    codecs = ensure_inputs()
    rows = []
    live = []
    rust = RustPipe()
    try:
        for arm, codec, _kind in read_arms(codecs):
            path = xyz_path(case) if arm.startswith("ase_xyz") else compressed_path(case, codec)
            if arm.startswith("ase_xyz") and case == "traj":
                rows.append(
                    failed_row(
                        arm,
                        "read",
                        case,
                        path,
                        "ASE xyz read does not finish: read_xyz pops the front of the line list",
                    )
                )
                continue
            if arm.startswith("ase_xyz") and not path.is_file():
                rows.append(failed_row(arm, "read", case, path, "xyz.gz is missing"))
                continue
            try:
                check = check_read(arm, case, path)
            except Exception as exc:  # noqa: BLE001
                rows.append(
                    failed_row(arm, "read", case, path, f"{type(exc).__name__}: {exc}".splitlines()[0][:500])
                )
                continue
            if not check["correct"] and arm.startswith("readcon"):
                raise RuntimeError(f"{arm} {case}: {check['reason']}")
            if arm.startswith("readcon_rust"):
                rust.call(f"warmup-read {support.rel(path)}")
            else:
                time_read(arm, path, rust)
            live.append((arm, path, check))
        samples = {arm: [] for arm, _path, _check in live}
        print(f"read {case} timing {len(live)} arms", file=sys.stderr, flush=True)
        for repeat in range(REPEATS):
            for arm, path, _check in _rotate(live, repeat):
                samples[arm].append(time_read(arm, path, rust))
        for arm, path, check in live:
            row = support.stats_block(samples[arm])
            row.update(check)
            row.update(
                {
                    "arm": arm,
                    "op": "read",
                    "case": case,
                    "file": support.rel(path),
                    "bytes": path.stat().st_size,
                    "rss_kib": None,
                    "massif_heap_bytes": None,
                }
            )
            rows.append(row)
    finally:
        rust.close()
    for row in rows:
        if not row["samples"]:
            continue
        print(f"memory read {case} {row['arm']}", file=sys.stderr, flush=True)
        row["rss_kib"] = fresh_rss("read", case, row["arm"])
        row["massif_heap_bytes"] = fresh_massif("read", case, row["arm"])
    emit(f"compress-{case}-read", rows)


def load_frames(case: str):
    return load_readcon(plain_path(case))


def load_ase_images(case: str):
    frames = load_frames(case)
    return [frame.to_ase() for frame in frames]


def write_dest(case: str, arm: str) -> Path:
    if arm.startswith("ase_xyz"):
        return WORK / "outs" / f"{arm}-{case}.xyz.gz"
    if "zstd" in arm:
        return WORK / "outs" / f"{arm}-{case}.con.zst"
    return WORK / "outs" / f"{arm}-{case}.con.gz"


def time_write(arm: str, case: str, frames, images, dest: Path, rust: RustPipe) -> float:
    if arm.startswith("readcon_rust"):
        return float(
            rust.call(f"write {support.rel(plain_path(case))} {support.rel(dest)}")
        )
    if arm.startswith("readcon_py"):
        started = time.perf_counter()
        write_readcon_py(frames, dest)
        return time.perf_counter() - started
    fmt = "xyz" if arm.startswith("ase_xyz") else "eon"
    started = time.perf_counter()
    write_ase(images, dest, fmt)
    return time.perf_counter() - started


def run_write(case: str) -> None:
    codecs = ensure_inputs()
    frames = load_frames(case)
    images = load_ase_images(case)
    (WORK / "outs").mkdir(parents=True, exist_ok=True)
    rows = []
    live = []
    rust = RustPipe()
    try:
        for arm in write_arms(codecs):
            dest = write_dest(case, arm)
            try:
                if arm.startswith("readcon_rust"):
                    rust.call(
                        f"warmup-write {support.rel(plain_path(case))} {support.rel(dest)}"
                    )
                else:
                    time_write(arm, case, frames, images, dest, rust)
            except Exception as exc:  # noqa: BLE001
                rows.append(
                    failed_row(
                        arm,
                        "write",
                        case,
                        dest,
                        f"{type(exc).__name__}: {exc}".splitlines()[0][:500],
                    )
                )
                continue
            try:
                if arm.startswith("ase_xyz"):
                    positions, cell = read_xyz_stream(dest)
                elif arm.startswith("ase_eon"):
                    positions, cell = read_ase(dest, "eon")
                else:
                    positions, cell = read_readcon(dest)
                check = judge(positions, cell, case)
            except Exception as exc:  # noqa: BLE001
                check = {
                    "correct": False,
                    "n_atoms_ok": False,
                    "cell_ok": False,
                    "coords_ok": False,
                    "max_abs": None,
                    "n_atoms": None,
                    "n_frames": None,
                    "cell": None,
                    "reason": "wrote; reread "
                    + f"{type(exc).__name__}: {exc}".splitlines()[0][:400],
                }
            if not check["correct"] and arm.startswith("readcon"):
                raise RuntimeError(f"{arm} write {case}: {check['reason']}")
            live.append((arm, dest, check))
        samples = {arm: [] for arm, _dest, _check in live}
        print(f"write {case} timing {len(live)} arms", file=sys.stderr, flush=True)
        for repeat in range(REPEATS):
            for arm, dest, _check in _rotate(live, repeat):
                samples[arm].append(time_write(arm, case, frames, images, dest, rust))
        for arm, dest, check in live:
            row = support.stats_block(samples[arm])
            row.update(check)
            row.update(
                {
                    "arm": arm,
                    "op": "write",
                    "case": case,
                    "file": support.rel(dest),
                    "bytes": dest.stat().st_size if dest.is_file() else None,
                    "rss_kib": None,
                    "massif_heap_bytes": None,
                }
            )
            rows.append(row)
        if case:
            xyz = xyz_path(case)
            produced = write_dest(case, "ase_xyz_gz")
            if produced.is_file():
                xyz.write_bytes(produced.read_bytes())
    finally:
        rust.close()
        del frames
        del images
    for row in rows:
        if not row["samples"]:
            continue
        print(f"memory write {case} {row['arm']}", file=sys.stderr, flush=True)
        row["rss_kib"] = fresh_rss("write", case, row["arm"])
        row["massif_heap_bytes"] = fresh_massif("write", case, row["arm"])
    emit(f"compress-{case}-write", rows)


def fresh_rss(op: str, case: str, arm: str) -> int:
    if op == "read" and arm.startswith("readcon_rust"):
        codec = "zstd" if arm.endswith("zstd") else "gzip"
        path = xyz_path(case) if arm.startswith("ase_xyz") else compressed_path(case, codec)
        completed = support.run_binary(rust_binary(), ["once", support.rel(path)])
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or "rust once"))
        return int(json.loads(completed.stdout.strip().splitlines()[-1])["hwm_kib"])
    if op == "write" and arm.startswith("readcon_rust"):
        dest = write_dest(case, arm)
        library = extxyz_h2h.ensure_vgmark()
        marker = WORK / "massif" / f"{arm}-write-rss-{case}.mark"
        marker.parent.mkdir(parents=True, exist_ok=True)
        completed = support.run_binary(
            rust_binary(),
            [
                "write-once",
                support.rel(plain_path(case)),
                support.rel(dest),
                str(library),
                str(marker),
            ],
        )
        if completed.returncode != 0:
            raise RuntimeError(support.sanitize(completed.stderr or completed.stdout or "write rss"))
        return int(json.loads(completed.stdout.strip().splitlines()[-1])["rss_kib"])
    completed = subprocess.run(
        [sys.executable, str(SCRIPT), "rss", op, case, arm],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(support.sanitize(completed.stderr or completed.stdout or "rss"))
    return int(json.loads(completed.stdout.strip().splitlines()[-1])["rss_kib"])


def fresh_massif(op: str, case: str, arm: str) -> int:
    out = WORK / "massif" / f"{arm}-{op}-{case}.out"
    out.parent.mkdir(parents=True, exist_ok=True)
    marker = None
    if op == "read" and arm.startswith("readcon_rust"):
        codec = "zstd" if arm.endswith("zstd") else "gzip"
        path = compressed_path(case, codec)
        command = [str(rust_binary()), "once", str(path)]
    elif op == "write" and arm.startswith("readcon_rust"):
        marker = WORK / "massif" / f"{arm}-write-{case}.mark"
        library = extxyz_h2h.ensure_vgmark()
        command = [
            str(rust_binary()),
            "write-once",
            str(plain_path(case)),
            str(write_dest(case, arm)),
            str(library),
            str(marker),
        ]
    elif op == "read" and arm == "gzip_dc":
        command = [sys.executable, str(SCRIPT), "massif-pipe", case]
    else:
        if op == "write":
            marker = WORK / "massif" / f"{arm}-write-{case}.mark"
            command = [sys.executable, str(SCRIPT), "write-window", case, arm, str(marker)]
        else:
            command = [sys.executable, str(SCRIPT), "once-read", case, arm]
    prefix = [
        "valgrind",
        "--tool=massif",
        f"--massif-out-file={out}",
        "--time-unit=B",
        "--detailed-freq=1",
        "--max-snapshots=200",
    ]
    if arm == "gzip_dc" and op == "read":
        gzip = subprocess.Popen(
            ["gzip", "-dc", os.fspath(compressed_path(case, "gzip"))],
            stdout=subprocess.PIPE,
            env=support.thread_env(),
        )
        proc = subprocess.run(
            [*prefix, str(CLI), "-"],
            stdin=gzip.stdout,
            check=False,
            capture_output=True,
            text=True,
            env=support.thread_env(),
        )
        if gzip.stdout is not None:
            gzip.stdout.close()
        gzip.wait()
        if proc.returncode != 0 or gzip.returncode != 0:
            raise RuntimeError(support.sanitize(proc.stderr[-1500:] or "pipe massif"))
        return massif_peak(out)
    completed = subprocess.run(
        [*prefix, *command],
        check=False,
        capture_output=True,
        text=True,
        env=support.thread_env(),
    )
    if completed.returncode != 0:
        raise RuntimeError(support.sanitize(completed.stderr[-1500:] or "massif"))
    if marker is None:
        return massif_peak(out)
    return extxyz_h2h.write_heap_delta(out, marker)


def massif_peak(path: Path) -> int:
    peak = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("mem_heap_B="):
            peak = max(peak, int(line.split("=", 1)[1]))
    return peak


def rss_command(op: str, case: str, arm: str) -> None:
    if op == "read":
        if arm == "gzip_dc":
            gzip_proc = subprocess.Popen(
                ["gzip", "-dc", os.fspath(compressed_path(case, "gzip"))],
                stdout=subprocess.PIPE,
            )
            reader = subprocess.Popen(
                [str(CLI), "-"],
                stdin=gzip_proc.stdout,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            if gzip_proc.stdout is not None:
                gzip_proc.stdout.close()
            peak = 0
            while reader.poll() is None:
                status = Path(f"/proc/{reader.pid}/status")
                try:
                    text = status.read_text(encoding="utf-8")
                except FileNotFoundError:
                    break
                for line in text.splitlines():
                    if line.startswith("VmHWM:"):
                        peak = max(peak, int(line.split()[1]))
                time.sleep(0.01)
            reader.wait()
            gzip_proc.wait()
            if reader.returncode != 0 or gzip_proc.returncode != 0:
                raise RuntimeError(f"pipe rss exit {reader.returncode}")
            print(json.dumps({"rss_kib": peak}))
            return
        codec = "zstd" if arm.endswith("zstd") else "gzip"
        path = xyz_path(case) if arm.startswith("ase_xyz") else compressed_path(case, codec)
        if arm.startswith("readcon_py"):
            frames = load_readcon(path)
            _sink = float(frames[-1].atoms[0].x)
        else:
            fmt = "xyz" if arm.startswith("ase_xyz") else "eon"
            images = __import__("ase.io", fromlist=["read"]).read(
                os.fspath(path), index=":", format=fmt
            )
            if not isinstance(images, list):
                images = [images]
            _sink = float(images[-1].positions[0, 0])
        print(json.dumps({"rss_kib": hwm_kib(), "sink": _sink}))
        return
    marker = WORK / "massif" / f"rss-{arm}-{case}.mark"
    write_window(case, arm, marker)
    return


def write_window(case: str, arm: str, marker: Path) -> None:
    frames = load_frames(case) if arm.startswith("readcon") else None
    images = None if frames is not None else load_ase_images(case)
    library = extxyz_h2h.vgmark()
    library.watch_start()
    if library.vg_monitor(f"detailed_snapshot {marker}".encode()) != 0:
        raise RuntimeError("snapshot")
    dest = write_dest(case, arm)
    if frames is not None:
        write_readcon_py(frames, dest)
    else:
        fmt = "xyz" if arm.startswith("ase_xyz") else "eon"
        write_ase(images, dest, fmt)
    print(json.dumps({"rss_kib": int(library.watch_delta_kib())}))


def once_read(case: str, arm: str) -> None:
    rss_command("read", case, arm)


def environment(codecs: list[str]) -> dict:
    return {
        "host": support.hardware_line(),
        "kernel": os.uname().release,
        "python": sys.version.split()[0],
        "rustc": subprocess.check_output(
            ["rustup", "run", "1.91.0", "rustc", "--version"], text=True
        ).strip(),
        "valgrind": subprocess.check_output(["valgrind", "--version"], text=True).strip(),
        "gzip": subprocess.check_output(["gzip", "--version"], text=True).splitlines()[0],
        "numpy": package_version("numpy"),
        "ase": package_version("ase"),
        "readcon": package_version("readcon"),
        "git": support.git_line(["rev-parse", "HEAD"]),
        "codecs": codecs,
        "codec_source": "detect_compression_from_extension",
        "pin": "taskset -c 0",
        "threads": 1,
        "repeats": REPEATS,
        "warmup": 1,
        "quantile": "type-7",
        "memory_scope": (
            "read: fresh-process VmHWM and massif peak; "
            "write: fresh-process resident and heap increase above the post-load baseline; "
            "gzip -dc | readcon-core - is the decompression baseline"
        ),
    }


def parse_logs() -> list[dict]:
    rows = []
    for path in sorted(support.LOG_DIR.glob("compress-*.stdout")):
        text = path.read_text(encoding="utf-8")
        for line in text.splitlines():
            if line.startswith("{") and line.endswith("}"):
                rows.append(json.loads(line))
            elif line.startswith("repeat "):
                continue
        block = []
        for line in text.splitlines():
            if line.startswith("repeat ") or line.startswith("sample ") or line.startswith("median_s "):
                block.append(line)
        # Rows already stored by format_result are one JSON object per failed
        # arm. Successful arms are the key/value blocks. Rebuild those.
    rebuilt = []
    for path in sorted(support.LOG_DIR.glob("compress-*.stdout")):
        case_op = path.name.replace("compress-", "").replace(".stdout", "")
        chunks = []
        current = []
        body = False
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("git rev-parse") or line.startswith("command") or line.startswith("hardware ") or line.startswith("pinned ") or line.startswith("exit "):
                continue
            if line.startswith("{") and line.endswith("}"):
                rebuilt.append(json.loads(line))
                continue
            if line.startswith("repeat "):
                if current:
                    chunks.append(current)
                current = [line]
                body = True
                continue
            if body:
                current.append(line)
        if current:
            chunks.append(current)
        for chunk in chunks:
            row = {"samples": []}
            for line in chunk:
                parts = line.split()
                if line.startswith("repeat "):
                    row["n"] = int(parts[1])
                elif line.startswith("sample "):
                    row["samples"].append(float(parts[1]))
                elif line.startswith("median_s "):
                    row["median_s"] = float(parts[1])
                elif line.startswith("iqr_s "):
                    row["iqr_s"] = float(parts[1])
                elif line.startswith("q25_s "):
                    row["q25_s"] = float(parts[1])
                elif line.startswith("q75_s "):
                    row["q75_s"] = float(parts[1])
                elif line.startswith("arm "):
                    row["arm"] = parts[1]
                elif line.startswith("op "):
                    row["op"] = parts[1]
                elif line.startswith("case "):
                    row["case"] = parts[1]
                elif line.startswith("file "):
                    row["file"] = parts[1]
                elif line.startswith("bytes "):
                    row["bytes"] = None if parts[1] == "None" else int(float(parts[1]))
                elif line.startswith("rss_kib "):
                    row["rss_kib"] = None if parts[1] in ("", "None") else int(float(parts[1]))
                elif line.startswith("massif_heap_bytes "):
                    row["massif_heap_bytes"] = None if parts[1] in ("", "None") else int(float(parts[1]))
                elif line.startswith("correct "):
                    row["correct"] = parts[1] == "1"
                elif line.startswith("max_abs "):
                    row["max_abs"] = None if parts[1] == "missing" else float(parts[1])
                elif line.startswith("reason "):
                    row["reason"] = line[len("reason ") :]
            if row.get("samples"):
                rebuilt.append(row)
        _ = case_op
    return rebuilt


def report() -> None:
    codecs = discover_codecs() if RUST.is_file() else []
    document = {
        "environment": environment(codecs),
        "file_sizes": file_sizes(codecs) if codecs else {},
        "rows": parse_logs(),
    }
    RESULTS.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    print(RESULTS)


def main() -> None:
    support.ensure_pinned()
    command = sys.argv[1] if len(sys.argv) > 1 else "report"
    if command == "read":
        run_read(sys.argv[2])
    elif command == "write":
        run_write(sys.argv[2])
    elif command == "report":
        report()
    elif command == "rss":
        rss_command(sys.argv[2], sys.argv[3], sys.argv[4])
    elif command == "once-read":
        once_read(sys.argv[2], sys.argv[3])
    elif command == "write-window":
        write_window(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
    elif command == "codecs":
        print(json.dumps(discover_codecs()))
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
