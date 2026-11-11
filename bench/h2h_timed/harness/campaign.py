# Copyright (c) readcon developers
"""Extended-XYZ rerun after read_extxyz returns atomic numbers."""

from __future__ import annotations

import json
import sys
import os
import shlex
import subprocess
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from common import (
    DATA,
    PYTHON,
    REPO,
    cache_fraction,
    cache_label,
    git_capture,
    host_block,
    median_iqr,
    recorded_path,
    script_sha256,
    sha256_file,
    thread_env,
)

KIT = Path(__file__).resolve().parents[1]
LOG_ROOT = Path(os.environ.get("H2H_OUT", str(Path.home() / "h2h-work" / "h2h-logs")))
SIZES = ("cu32_f1k", "cu32_f10k", "cu32_f100k", "cu1000_f1k")
ARMS = (
    "readcon_extxyz",
    "readcon_extxyz_188",
    "readcon_extxyz_r27",
    "readcon_pypi",
    "ase_extxyz",
    "chemfiles",
    "ovito",
    "mdanalysis",
    "pymatgen",
)
SITES = {
    "readcon_extxyz": "py-ci",
    "readcon_extxyz_188": "py-188",
    "readcon_extxyz_r27": "py-r27",
    "readcon_pypi": "py-0180",
    "readcon_py": "py-ci",
}
RATIO_PEERS = ("ase_extxyz", "chemfiles", "ovito")
NOT_RATIO = {
    "mdanalysis": "MDAnalysis XYZReader reads coordinates and names and does not parse the Lattice line, so the cell is absent.",
    "pymatgen": "pymatgen.io.xyz.XYZ reads species and coordinates and drops the lattice from the comment line.",
}
REPEATS = int(os.environ.get("H2H_REPEATS", "10"))
GROUPS = (
    {
        "format": "extxyz",
        "suffix": ".xyz",
        "truth_suffix": ".truth.h2h",
        "arms": ARMS,
        "baseline": "readcon_extxyz",
        "peers": RATIO_PEERS
        + ("readcon_extxyz_188", "readcon_extxyz_r27", "readcon_pypi"),
        "op": "read",
    },
    {
        "format": "con",
        "suffix": ".con",
        "truth_suffix": ".con.readcon.h2h",
        "arms": ("readcon_py", "ase_eon"),
        "baseline": "readcon_py",
        "peers": ("ase_eon",),
        "op": "read",
    },
    {
        "format": "con_write",
        "suffix": ".con",
        "truth_suffix": ".con.readcon.h2h",
        "arms": ("readcon_py", "ase_eon"),
        "baseline": "readcon_py",
        "peers": ("ase_eon",),
        "op": "write",
    },
)
MUST_MATCH = {
    "readcon_extxyz",
    "readcon_extxyz_188",
    "readcon_extxyz_r27",
    "ase_extxyz",
    "chemfiles",
    "ovito",
    "readcon_py",
    "ase_eon",
}


def stage_dir() -> Path:
    build = os.environ.get("H2H_BUILD", "")
    if build:
        return Path(build).resolve().parent
    return REPO / "build" / "timed-stage"


def run_sample(
    arm: str, path: Path, truth: Path, log: Path, op: str, dest: Path | None
) -> dict:
    cmd = [
        "taskset",
        "-c",
        "0",
        str(PYTHON),
        str(KIT / "harness" / "sample.py"),
        "--op",
        op,
        "--arm",
        arm,
        "--path",
        str(path),
        "--truth",
        str(truth),
        "--log",
        str(log),
    ]
    if dest is not None:
        cmd.extend(["--dest", str(dest)])
    display = [
        "taskset",
        "-c",
        "0",
        "python3",
        "bench/h2h_timed/harness/sample.py",
        "--op",
        op,
        "--arm",
        arm,
        "--path",
        recorded_path(path),
        "--truth",
        recorded_path(truth),
        "--log",
        recorded_path(log),
    ]
    if dest is not None:
        display.extend(["--dest", recorded_path(dest)])
    shown = " ".join(shlex.quote(part) for part in display)
    cmd.extend(["--command", shown])
    env = thread_env()
    site_name = SITES.get(arm)
    if site_name:
        env["H2H_READCON_SITE"] = str(stage_dir() / site_name)
    subprocess.run(cmd, env=env, check=False)
    data = {"arm": arm, "path": recorded_path(path), "log": recorded_path(log)}
    for line in log.read_text(encoding="utf-8").splitlines():
        if " " not in line:
            continue
        key, value = line.split(" ", 1)
        if key in {
            "elapsed_s",
            "max_abs_dx",
            "length_max",
            "angle_max",
            "cache_fraction_before",
        }:
            data[key] = float(value)
        elif key in {"correct", "frames_ok", "atoms_ok", "types_ok", "cell_parsed"}:
            data[key] = value == "True"
        elif key in {
            "status",
            "cache",
            "types_dtype",
            "readcon_file",
            "readcon_so_sha256",
        }:
            data[key] = value.strip()
    return data


def input_files() -> list[Path]:
    found: list[Path] = []
    for group in GROUPS:
        for size in SIZES:
            for suffix in (group["suffix"], group["truth_suffix"]):
                path = DATA / f"{size}{suffix}"
                if path.is_file() and path not in found:
                    found.append(path)
    return found


def hash_inputs() -> dict[str, str]:
    return {recorded_path(path): sha256_file(path) for path in input_files()}


def per_core_load() -> dict:
    stat = Path("/proc/stat").read_text(encoding="utf-8")
    cores = [line for line in stat.splitlines() if line.startswith("cpu")]
    loadavg = Path("/proc/loadavg").read_text(encoding="utf-8").strip()
    return {"loadavg": loadavg, "proc_stat": cores}


def session_point(label: str) -> dict:
    boot = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="utf-8").strip()
    uptime = Path("/proc/uptime").read_text(encoding="utf-8").split()[0]
    wall = datetime.now(timezone.utc).isoformat(timespec="seconds")
    listing = subprocess.check_output(["lscpu"], text=True)
    load = per_core_load()
    print(f"session {label} boot_id {boot}", flush=True)
    print(f"session {label} uptime_s {uptime}", flush=True)
    print(f"session {label} wall_utc {wall}", flush=True)
    print(f"session {label} loadavg {load['loadavg']}", flush=True)
    for line in load["proc_stat"]:
        print(f"session {label} proc_stat {line}", flush=True)
    print(f"session {label} lscpu", flush=True)
    sys.stdout.write(listing if listing.endswith("\n") else listing + "\n")
    print(f"session {label} lscpu_end", flush=True)
    return {
        "boot_id": boot,
        "uptime_s": uptime,
        "wall_utc": wall,
        "lscpu": listing,
        "loadavg": load["loadavg"],
        "proc_stat": load["proc_stat"],
    }


def main() -> None:
    if os.environ.get("H2H_HARNESS_ENTRY") == "1":
        return
    rev, status = git_capture()
    if status != "":
        raise SystemExit(f"worktree is dirty: {status!r}")
    session_start = session_point("start")
    inputs_before = hash_inputs()
    build_path = Path(os.environ["H2H_BUILD"]) / "build.json"
    build = json.loads(build_path.read_text(encoding="utf-8"))
    so_before = {}
    for key, info in build["arms"].items():
        digest = sha256_file(REPO / info["binary_path"])
        if digest != info["binary_sha256"]:
            raise SystemExit(f"before hash mismatch for {key}")
        so_before[key] = digest
    samples = []
    cache_notes = {}
    for group in GROUPS:
        fmt = group["format"]
        for size in SIZES:
            path = DATA / f"{size}{group['suffix']}"
            truth = DATA / f"{size}{group['truth_suffix']}"
            before = cache_fraction(path)
            with path.open("rb") as handle:
                while handle.read(1 << 20):
                    pass
            after = cache_fraction(path)
            cache_notes[f"{fmt}/{size}"] = {
                "before": before,
                "after": after,
                "label": cache_label(after),
            }
            for repeat in range(REPEATS):
                order = list(group["arms"])
                if repeat % 2 == 1:
                    order.reverse()
                for arm in order:
                    log = LOG_ROOT / fmt / size / arm / f"r{repeat:02d}.log"
                    dest = None
                    if group["op"] == "write":
                        label = os.environ.get("H2H_SESSION") or rev[:12]
                        scratch = Path(
                            os.environ.get("H2H_SCRATCH", str(REPO / "target" / "h2h-writes"))
                        )
                        dest = scratch / label / fmt / size / arm / f"r{repeat:02d}.con"
                        dest.parent.mkdir(parents=True, exist_ok=True)
                    parsed = run_sample(arm, path, truth, log, group["op"], dest)
                    parsed.update(size=size, repeat=repeat, format=fmt)
                    samples.append(parsed)
                    print(
                        f"read {fmt} {size} {arm} r{repeat} {parsed.get('elapsed_s')} {parsed.get('correct')} {parsed.get('status')}",
                        flush=True,
                    )
                    if arm in MUST_MATCH and parsed.get("correct") is not True:
                        raise SystemExit(f"read mismatch: {log}")
    summaries = []
    for group in GROUPS:
        for size in SIZES:
            for arm in group["arms"]:
                rows = [
                    s
                    for s in samples
                    if s["format"] == group["format"]
                    and s["size"] == size
                    and s["arm"] == arm
                    and "elapsed_s" in s
                ]
                times = [s["elapsed_s"] for s in rows]
                med, q1, q3, iqr = median_iqr(times)
                summaries.append(
                    {
                        "arm": arm,
                        "size": size,
                        "format": group["format"],
                        "n": len(times),
                        "median_s": med,
                        "q1_s": q1,
                        "q3_s": q3,
                        "iqr_s": iqr,
                        "correct": all(s.get("correct") for s in rows),
                        "max_abs_dx": max((s.get("max_abs_dx") or 0.0) for s in rows),
                        "types_ok": all(s.get("types_ok") for s in rows),
                        "cell_parsed": all(s.get("cell_parsed") for s in rows),
                        "not_ratio_reason": NOT_RATIO.get(arm),
                    }
                )
    pairs = []
    slower = []
    for group in GROUPS:
        fmt = group["format"]
        baseline = group["baseline"]
        base = {
            s["size"]: s
            for s in summaries
            if s["format"] == fmt and s["arm"] == baseline
        }
        for summary in summaries:
            if summary["format"] != fmt or summary["arm"] not in group["peers"]:
                continue
            readcon = base[summary["size"]]
            reason = None
            if not summary["correct"] or not readcon["correct"]:
                reason = (
                    "a side did not return positions, cell and types within tolerance"
                )
            ratios = []
            for repeat in range(REPEATS):
                peer = next(
                    s
                    for s in samples
                    if s["format"] == fmt
                    and s["size"] == summary["size"]
                    and s["arm"] == summary["arm"]
                    and s["repeat"] == repeat
                )
                own = next(
                    s
                    for s in samples
                    if s["format"] == fmt
                    and s["size"] == summary["size"]
                    and s["arm"] == baseline
                    and s["repeat"] == repeat
                )
                if "elapsed_s" in peer and "elapsed_s" in own and own["elapsed_s"] > 0:
                    ratios.append(peer["elapsed_s"] / own["elapsed_s"])
            ratio_median, ratio_q1, ratio_q3, ratio_iqr = (
                median_iqr(ratios) if ratios else (None, None, None, None)
            )
            record = {
                "format": fmt,
                "size": summary["size"],
                "peer": summary["arm"],
                "readcon": baseline,
                "fair": reason is None,
                "reason": reason,
                "peer_median_s": summary["median_s"],
                "readcon_median_s": readcon["median_s"],
                "peer_iqr_s": summary["iqr_s"],
                "readcon_iqr_s": readcon["iqr_s"],
                "peer_q1_s": summary["q1_s"],
                "peer_q3_s": summary["q3_s"],
                "readcon_q1_s": readcon["q1_s"],
                "readcon_q3_s": readcon["q3_s"],
                "ratio_median_of_paired": ratio_median,
                "ratio_q1_of_paired": ratio_q1,
                "ratio_q3_of_paired": ratio_q3,
                "ratio_iqr_of_paired": ratio_iqr,
                "ratio_of_medians": summary["median_s"] / readcon["median_s"]
                if readcon["median_s"]
                else None,
                "correct": reason is None,
            }
            if record["fair"]:
                if readcon["q1_s"] > summary["q3_s"]:
                    record["verdict"] = "readcon slower"
                elif readcon["q3_s"] < summary["q1_s"]:
                    record["verdict"] = "readcon faster"
                elif readcon["median_s"] > summary["median_s"]:
                    record["verdict"] = "tie, readcon median slower, IQRs overlap"
                elif readcon["median_s"] < summary["median_s"]:
                    record["verdict"] = "tie, readcon median faster, IQRs overlap"
                else:
                    record["verdict"] = "tie"
                if readcon["median_s"] > summary["median_s"]:
                    slower.append(record)
            else:
                record["verdict"] = "not a head-to-head"
            pairs.append(record)
    arm_key = {
        "readcon_extxyz": "readcon_extxyz",
        "readcon_extxyz_188": "readcon_extxyz_188",
        "readcon_extxyz_r27": "readcon_extxyz_r27",
        "readcon_pypi": "readcon_pypi",
        "readcon_py": "readcon_extxyz",
    }
    loaded = {}
    extension = {}
    for arm, key in arm_key.items():
        rows = [s for s in samples if s["arm"] == arm and s.get("readcon_so_sha256")]
        if len(rows) < REPEATS:
            raise SystemExit(f"missing loaded hash for {arm}")
        found = {s["readcon_so_sha256"] for s in rows}
        if len(found) != 1:
            raise SystemExit(f"mixed loaded hashes for {arm}: {found}")
        got = found.pop()
        info = build["arms"][key]
        expected = info["binary_sha256"]
        binary = REPO / info["binary_path"]
        before = so_before[key]
        after = sha256_file(binary)
        if got != expected or before != expected or after != expected:
            raise SystemExit(
                f"{arm} hash mismatch loaded {got} record {expected} before {before} after {after}"
            )
        loaded[arm] = {
            "sha256": got,
            "path": rows[0]["readcon_file"],
            "before": before,
            "after": after,
            "n": len(rows),
            "commit": info["commit"],
            "tree": info["tree"],
        }
        extension[arm] = {Path(rows[0]["readcon_file"]).name: got}
    inputs_after = hash_inputs()
    if inputs_after != inputs_before:
        raise SystemExit("input hash changed during the run")
    session_end = session_point("end")
    tree = subprocess.check_output(
        ["git", "-C", str(REPO), "rev-parse", "HEAD^{tree}"], text=True
    ).strip()
    payload = {
        "commit": rev,
        "tree": tree,
        "git_status": status,
        "parent": subprocess.check_output(
            ["git", "-C", str(REPO), "rev-parse", "HEAD^"], text=True
        ).strip(),
        "write_extxyz": False,
        "repeats": REPEATS,
        "cache": cache_notes,
        "samples": samples,
        "summaries": summaries,
        "pairs": pairs,
        "slower": slower,
        "not_ratio": NOT_RATIO,
        "host": host_block(),
        "script_sha256": script_sha256(),
        "loaded_so": loaded,
        "extension_sha256": extension,
        "inputs": {"before": inputs_before, "after": inputs_after},
        "session": {"start": session_start, "end": session_end},
    }
    out = Path(os.environ.get("H2H_RESULTS", str(KIT / "results.json")))
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {recorded_path(out)}", flush=True)


if __name__ == "__main__":
    main()
