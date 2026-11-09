# Copyright (c) readcon developers
"""Rerun the one-thread C and Fortran CON read and iterator arms.

Same protocol as the round-1 campaign for these arms: six fresh
processes, odd repeats reversed, taskset on CPU 0, one thread.
"""

from __future__ import annotations

import json
import os
import shlex
import subprocess
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))

from common import (
    DATA,
    cache_fraction,
    cache_label,
    compare_dumps,
    g,
    git_capture,
    host_block,
    median_iqr,
    read_dump,
    script_sha256,
    sha256_file,
    thread_env,
)

_WORK = Path.home() / "h2h-work"
BUILD = Path("/src/out")
LIB = BUILD
LOGS = _WORK / "r29-logs" / "iterator"
DUMPS = _WORK / "data" / "dumps"
SIZES = ("cu32_f1k", "cu32_f10k", "cu32_f100k", "cu1000_f1k")
ARMS = (("readcon_c", "c"), ("readcon_fortran", "fortran"))
REPEATS = 6


def _recorded_path(path: Path) -> str:
    from common import recorded_path

    return recorded_path(path)


def warm(path: Path) -> None:
    with path.open("rb") as handle:
        while handle.read(1 << 20):
            pass


def run_one(kind: str, op: str, path: Path, log: Path, ref: Path | None) -> dict:
    binary = BUILD / ("h2h_c" if kind == "c" else "h2h_fortran")
    DUMPS.mkdir(parents=True, exist_ok=True)
    dump = DUMPS / (str(log.relative_to(LOGS)).replace("/", "_") + ".h2h")
    cmd = [str(binary), op, str(path), str(dump), "1", "0"]
    env = thread_env(1)
    env["LD_LIBRARY_PATH"] = str(LIB) + os.pathsep + env.get("LD_LIBRARY_PATH", "")
    shown = " ".join(shlex.quote(part) for part in ["taskset", "-c", "0", *cmd])
    home = Path.home().resolve().as_posix()
    shown = shown.replace(home + "/", "")
    proc = subprocess.run(
        ["taskset", "-c", "0", *cmd],
        env=env,
        text=True,
        capture_output=True,
    )
    parsed = {}
    for line in proc.stdout.splitlines():
        if " " not in line:
            continue
        key, value = line.split(" ", 1)
        if key == "elapsed_s":
            parsed[key] = float(value)
        elif key in {"VmHWM_KiB", "n_frames", "n_atoms"}:
            parsed[key] = int(value)
        else:
            parsed[key] = value
    rev, status = git_capture()
    fraction = cache_fraction(path)
    lines = [
        "git rev-parse HEAD",
        rev,
        "git status --porcelain=v1",
        status.rstrip("\n"),
        "command",
        shown,
        host_block().rstrip("\n"),
        f"op {op}",
        f"arm native-{kind}",
        "threads 1",
        f"path {_recorded_path(path)}",
        f"path_sha256 {sha256_file(path)}",
        f"cache_fraction_before {g(fraction)}",
        f"cache {cache_label(fraction)}",
        "native_stdout",
        proc.stdout.rstrip("\n"),
    ]
    if proc.returncode != 0:
        lines.append(proc.stderr.rstrip("\n"))
    if op == "read" and dump.exists() and ref is not None:
        checked = compare_dumps(read_dump(dump), read_dump(ref))
        for key, value in checked.items():
            lines.append(f"{key} {g(value) if isinstance(value, float) else value}")
            parsed[key] = value
        dump.unlink()
    if "elapsed_s" in parsed:
        lines.append(f"elapsed_s {g(parsed['elapsed_s'])}")
    lines.append("status ok" if proc.returncode == 0 else "status error")
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text("\n".join(lines) + "\n", encoding="utf-8")
    parsed["returncode"] = proc.returncode
    parsed["command"] = shown
    return parsed


def main() -> None:
    if os.environ.get("H2H_HARNESS_ENTRY") == "1":
        return
    samples = []
    jobs = [("read", size) for size in SIZES] + [("stream", "cu32_f100k")]
    for op, size in jobs:
        path = DATA / f"{size}.con"
        ref = DATA / f"{size}.con.readcon.h2h" if op == "read" else None
        warm(path)
        for repeat in range(REPEATS):
            order = list(ARMS)
            if repeat % 2 == 1:
                order.reverse()
            for arm, kind in order:
                log = LOGS / op / size / arm / f"r{repeat:02d}.log"
                parsed = run_one(kind, op, path, log, ref)
                parsed.update(
                    {
                        "op": op,
                        "arm": arm,
                        "size": size,
                        "repeat": repeat,
                        "log": _recorded_path(log),
                    }
                )
                samples.append(parsed)
                print(
                    f"{op} {size} {arm} r{repeat} "
                    f"{parsed.get('elapsed_s')} hwm={parsed.get('VmHWM_KiB')} "
                    f"frames={parsed.get('n_frames')} correct={parsed.get('correct')} "
                    f"rc={parsed.get('returncode')}",
                    flush=True,
                )
                if parsed.get("returncode") != 0:
                    raise SystemExit(f"sample failed: {log}")
                if op == "read" and parsed.get("correct") is not True:
                    raise SystemExit(f"read mismatch: {log}")
                if op == "stream" and (
                    parsed.get("n_frames") != 100000 or parsed.get("n_atoms") != 32
                ):
                    raise SystemExit(f"stream counts: {log} {parsed}")
    summary = []
    keys = {(s["op"], s["arm"], s["size"]) for s in samples}
    for op, arm, size in sorted(keys):
        rows = [s for s in samples if (s["op"], s["arm"], s["size"]) == (op, arm, size)]
        times = [s["elapsed_s"] for s in rows]
        heights = [s["VmHWM_KiB"] for s in rows]
        med, q1, q3, iqr = median_iqr(times)
        hmed, hq1, hq3, hiqr = median_iqr(heights)
        summary.append(
            {
                "op": op,
                "arm": arm,
                "size": size,
                "n": len(times),
                "median_s": med,
                "q1_s": q1,
                "q3_s": q3,
                "iqr_s": iqr,
                "times_s": times,
                "vmhwm_median_kib": hmed,
                "vmhwm_q1_kib": hq1,
                "vmhwm_q3_kib": hq3,
                "vmhwm_iqr_kib": hiqr,
                "vmhwm_kib": heights,
                "max_abs_dx": max((s.get("max_abs_dx") or 0.0) for s in rows),
                "correct": all(s.get("correct") is True for s in rows) if op == "read" else None,
            }
        )
    rev, status = git_capture()
    out = {
        "commit": rev,
        "git_status": status,
        "host": host_block(),
        "script_sha256": script_sha256(),
        "so_sha256": sha256_file(LIB / "libreadcon_core.so"),
        "binary_sha256": {
            "h2h_c": sha256_file(BUILD / "h2h_c"),
            "h2h_fortran": sha256_file(BUILD / "h2h_fortran"),
            "libreadcon_core.so": sha256_file(LIB / "libreadcon_core.so"),
        },
        "summaries": summary,
        "samples": samples,
    }
    dest = LOGS / "summary.json"
    dest.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
