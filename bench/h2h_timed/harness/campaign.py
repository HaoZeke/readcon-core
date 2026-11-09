# Copyright (c) readcon developers
"""Extended-XYZ rerun after read_extxyz returns atomic numbers."""

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
    PYTHON,
    cache_fraction,
    cache_label,
    g,
    git_capture,
    host_block,
    median_iqr,
    recorded_path,
    script_sha256,
    thread_env,
)

KIT = Path(__file__).resolve().parents[1]
LOG_ROOT = Path.home() / "h2h-work" / "r28-logs"
SIZES = ("cu32_f1k", "cu32_f10k", "cu32_f100k", "cu1000_f1k")
ARMS = (
    "readcon_extxyz",
    "ase_extxyz",
    "chemfiles",
    "ovito",
    "mdanalysis",
    "pymatgen",
)
RATIO_PEERS = ("ase_extxyz", "chemfiles", "ovito")
NOT_RATIO = {
    "mdanalysis": "MDAnalysis XYZReader reads coordinates and names and does not parse the Lattice line, so the cell is absent.",
    "pymatgen": "pymatgen.io.xyz.XYZ reads species and coordinates and drops the lattice from the comment line.",
}
REPEATS = 6
GROUPS = (
    {
        "format": "extxyz",
        "suffix": ".xyz",
        "truth_suffix": ".truth.h2h",
        "arms": ARMS,
        "baseline": "readcon_extxyz",
        "peers": RATIO_PEERS,
    },
    {
        "format": "con",
        "suffix": ".con",
        "truth_suffix": ".con.readcon.h2h",
        "arms": ("readcon_py", "ase_eon"),
        "baseline": "readcon_py",
        "peers": ("ase_eon",),
    },
)
MUST_MATCH = {
    "readcon_extxyz",
    "ase_extxyz",
    "chemfiles",
    "ovito",
    "readcon_py",
    "ase_eon",
}


def run_sample(arm: str, path: Path, truth: Path, log: Path) -> dict:
    cmd = [
        "taskset",
        "-c",
        "0",
        str(PYTHON),
        str(KIT / "harness" / "sample.py"),
        "--arm",
        arm,
        "--path",
        str(path),
        "--truth",
        str(truth),
        "--log",
        str(log),
    ]
    shown = " ".join(shlex.quote(part) for part in cmd)
    home = Path.home().resolve().as_posix()
    shown = shown.replace(home + "/", "")
    cmd.extend(["--command", shown])
    subprocess.run(cmd, env=thread_env(), check=False)
    data = {"arm": arm, "path": recorded_path(path), "log": recorded_path(log)}
    for line in log.read_text(encoding="utf-8").splitlines():
        if " " not in line:
            continue
        key, value = line.split(" ", 1)
        if key in {"elapsed_s", "max_abs_dx", "length_max", "angle_max", "cache_fraction_before"}:
            data[key] = float(value)
        elif key in {"correct", "frames_ok", "atoms_ok", "types_ok", "cell_parsed"}:
            data[key] = value == "True"
        elif key in {"status", "cache", "types_dtype"}:
            data[key] = value.strip()
    return data


def main() -> None:
    if os.environ.get("H2H_HARNESS_ENTRY") == "1":
        return
    rev, status = git_capture()
    if status != "":
        raise SystemExit(f"worktree is dirty: {status!r}")
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
            cache_notes[f"{fmt}/{size}"] = {"before": before, "after": after, "label": cache_label(after)}
            for repeat in range(REPEATS):
                order = list(group["arms"])
                if repeat % 2 == 1:
                    order.reverse()
                for arm in order:
                    log = LOG_ROOT / fmt / size / arm / f"r{repeat:02d}.log"
                    parsed = run_sample(arm, path, truth, log)
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
                    if s["format"] == group["format"] and s["size"] == size and s["arm"] == arm and "elapsed_s" in s
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
                reason = "a side did not return positions, cell and types within tolerance"
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
                "ratio_of_medians": summary["median_s"] / readcon["median_s"] if readcon["median_s"] else None,
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
    payload = {
        "commit": rev,
        "git_status": status,
        "parent": "d5e77f997438557c72c3b3e360d446310f85d6b7",
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
    }
    out = Path(os.environ.get("H2H_RESULTS", str(KIT / "results.json")))
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out}", flush=True)


if __name__ == "__main__":
    main()
