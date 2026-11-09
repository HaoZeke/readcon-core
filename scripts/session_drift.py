#!/usr/bin/env python3
"""Print the session drift section from two results files.

The script checks the sha256 of each input against --expect before it
reads the files. stdout is the report section, verbatim.
"""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import re
import statistics
import subprocess
import tarfile
from pathlib import Path

INSN = re.compile(r"^\s+[0-9a-f]+:\s+\S")

ALPHA = 0.05


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_expect(path: Path) -> dict[str, str]:
    found: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        digest, name = line.split("  ", 1)
        found[name] = digest
    return found


def require_hash(path: Path, expect: dict[str, str]) -> None:
    key = path.as_posix()
    if key not in expect:
        raise SystemExit(f"no expected hash for {key}")
    if sha256(path) != expect[key]:
        raise SystemExit(f"hash mismatch for {key}")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pair_row(data: dict, fmt: str, size: str, peer: str) -> dict:
    for item in data["pairs"]:
        if item["format"] == fmt and item["size"] == size and item["peer"] == peer:
            return item
    raise SystemExit(f"missing pair {fmt} {size} {peer}")


def percent(old: float, new: float) -> float:
    return (new - old) / old * 100.0


def times_larger(old: dict, new: dict) -> float:
    delta = abs(new["ratio_median_of_paired"] - old["ratio_median_of_paired"])
    larger = max(old["ratio_iqr_of_paired"], new["ratio_iqr_of_paired"])
    if larger == 0:
        raise SystemExit("IQR is zero")
    return delta / larger


def arm_name(arm: str) -> str:
    names = {
        "readcon_extxyz": "CI readcon",
        "readcon_extxyz_188": "rustc 1.88.0",
        "readcon_extxyz_r27": "r2.7 rebuild",
        "readcon_pypi": "published wheel",
        "ase_extxyz": "ASE",
        "chemfiles": "chemfiles",
        "ovito": "OVITO",
        "mdanalysis": "MDAnalysis",
        "pymatgen": "pymatgen",
        "ase_eon": "ASE",
    }
    return names.get(arm, arm)


def cell_name(fmt: str, size: str, peer: str) -> str:
    kind = {"extxyz": "extxyz", "con": "CON read", "con_write": "CON write"}[fmt]
    return f"{kind} {size} {arm_name(peer)}"


def direction_word(change: float) -> str:
    if change < 0:
        return "faster"
    if change > 0:
        return "slower"
    return "unchanged"


def times_for(data: dict, fmt: str, size: str, arm: str) -> list[float]:
    rows = [s for s in data["samples"] if s["format"] == fmt and s["size"] == size and s["arm"] == arm]
    rows.sort(key=lambda s: s["repeat"])
    return [s["elapsed_s"] for s in rows]


def paired_ratios(data: dict, fmt: str, size: str, peer: str, baseline: str) -> list[float]:
    base = {
        s["repeat"]: s["elapsed_s"]
        for s in data["samples"]
        if s["format"] == fmt and s["size"] == size and s["arm"] == baseline
    }
    rows = [s for s in data["samples"] if s["format"] == fmt and s["size"] == size and s["arm"] == peer]
    rows.sort(key=lambda s: s["repeat"])
    return [s["elapsed_s"] / base[s["repeat"]] for s in rows]


def instruction_count(path: Path) -> int:
    text = subprocess.check_output(
        ["objdump", "-d", "--no-show-raw-insn", str(path)],
        text=True,
        errors="replace",
    )
    return sum(1 for line in text.splitlines() if INSN.match(line))


def status_ok_blocks(path: Path) -> int:
    if path.name.endswith(".tar.xz"):
        with tarfile.open(path) as archive:
            member = next(item for item in archive.getmembers() if item.name.endswith("run.log"))
            text = archive.extractfile(member).read().decode("utf-8", "replace")
    else:
        text = path.read_text(encoding="utf-8", errors="replace")
    return sum(1 for line in text.splitlines() if line == "status ok")


def permutation_p(left: list[float], right: list[float]) -> tuple[float, int, int]:
    """Two-sided exact p for the difference of medians. Returns p, extreme, total."""
    observed = statistics.median(right) - statistics.median(left)
    pool = list(left) + list(right)
    width = len(left)
    extreme = 0
    total = 0
    for comb in itertools.combinations(range(len(pool)), width):
        chosen = set(comb)
        sample_a = [pool[i] for i in comb]
        sample_b = [pool[i] for i in range(len(pool)) if i not in chosen]
        value = statistics.median(sample_b) - statistics.median(sample_a)
        total += 1
        if abs(value) >= abs(observed) - 1e-15:
            extreme += 1
    return extreme / total, extreme, total


def design_floor(width: int) -> tuple[float, int, int]:
    return permutation_p(list(range(width)), list(range(1000, 1000 + width)))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("previous", type=Path)
    parser.add_argument("current", type=Path)
    parser.add_argument("--old-extension", type=Path, required=True)
    parser.add_argument("--new-extension", type=Path, required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--expect", type=Path, required=True)
    parser.add_argument("--label", default="r2.10")
    args = parser.parse_args()

    expect = load_expect(args.expect)
    for path in (args.previous, args.current, args.old_extension, args.new_extension, args.log):
        require_hash(path, expect)

    old = load(args.previous)
    new = load(args.current)
    if sha256(args.old_extension) != old["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]:
        raise SystemExit("r2.9 extension does not match its results record")
    if sha256(args.new_extension) != new["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]:
        raise SystemExit("later extension does not match its results record")
    if len(old["samples"]) != len(new["samples"]):
        raise SystemExit("sample counts differ")
    blocks = len(new["samples"])
    if status_ok_blocks(args.log) != blocks:
        raise SystemExit("log blocks differ from the sample count")

    file_size = "cu32_f100k"
    file_format = "extxyz"
    arm_order: list[str] = []
    for sample in old["samples"]:
        if sample["format"] == file_format and sample["size"] == file_size and sample["arm"] not in arm_order:
            arm_order.append(sample["arm"])
    shifts = []
    for arm in arm_order:
        before = statistics.median(times_for(old, file_format, file_size, arm))
        after = statistics.median(times_for(new, file_format, file_size, arm))
        shifts.append((arm, percent(before, after)))
    changed = [
        key
        for key, info in new["build"]["arms"].items()
        if old["build"]["arms"][key]["binary_sha256"] != info["binary_sha256"]
    ]
    slowed = [arm for arm, change in shifts if change > 0]
    quickened = [arm for arm, change in shifts if change < 0]
    same_hash = all(
        data["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]
        == data["build"]["arms"]["readcon_extxyz_r27"]["binary_sha256"]
        for data in (old, new)
    )

    rows = []
    repeat_counts = set()
    for before in old["pairs"]:
        after = pair_row(new, before["format"], before["size"], before["peer"])
        baseline = before["readcon"]
        old_ratios = paired_ratios(old, before["format"], before["size"], before["peer"], baseline)
        new_ratios = paired_ratios(new, before["format"], before["size"], before["peer"], baseline)
        repeat_counts.add(len(old_ratios))
        repeat_counts.add(len(new_ratios))
        factor = times_larger(before, after)
        outside = factor > 1
        p_value, _extreme, _total = permutation_p(old_ratios, new_ratios)
        rows.append(
            {
                "format": before["format"],
                "size": before["size"],
                "peer": before["peer"],
                "name": cell_name(before["format"], before["size"], before["peer"]),
                "old": before["ratio_median_of_paired"],
                "new": after["ratio_median_of_paired"],
                "factor": factor,
                "outside": outside,
                "p": p_value,
            }
        )
    if len(repeat_counts) != 1:
        raise SystemExit(f"repeat counts differ: {repeat_counts}")
    width = repeat_counts.pop()
    floor, floor_extreme, floor_total = design_floor(width)
    cells = len(rows)
    corrected = ALPHA / cells
    survivors = [row for row in rows if row["p"] < corrected]
    screen_only = [row for row in rows if row["outside"] and row["p"] > ALPHA]
    test_only = [row for row in rows if (not row["outside"]) and row["p"] <= ALPHA]

    rebuild_peers = {"readcon_extxyz_188", "readcon_extxyz_r27"}
    rebuild_rows = [row for row in rows if row["outside"] and row["peer"] in rebuild_peers]
    other_rows = [row for row in rows if row["outside"] and row["peer"] not in rebuild_peers]
    rebuild_sizes = sorted({row["size"] for row in rebuild_rows if row["peer"] == "readcon_extxyz_188"})
    second_sizes = sorted({row["size"] for row in rebuild_rows if row["peer"] == "readcon_extxyz_r27"})

    noise = []
    for data in (old, new):
        if data["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"] != data["build"]["arms"]["readcon_extxyz_r27"]["binary_sha256"]:
            continue
        sizes = sorted({row["size"] for row in data["pairs"] if row["format"] == "extxyz"})
        for size in sizes:
            first = pair_row(data, "extxyz", size, "readcon_extxyz_188")
            second = pair_row(data, "extxyz", size, "readcon_extxyz_r27")
            gap = abs(first["ratio_median_of_paired"] - second["ratio_median_of_paired"])
            larger = max(first["ratio_iqr_of_paired"], second["ratio_iqr_of_paired"])
            noise.append(gap / larger)

    wheel_rows = [s for s in new["samples"] if s["arm"] == "readcon_pypi"]
    ci_rows = [s for s in new["samples"] if s["arm"] == "readcon_extxyz" and s["format"] == "extxyz"]
    wheel_omits_types = bool(wheel_rows) and all(s["types_dtype"] == "absent" for s in wheel_rows)
    ci_has_types = bool(ci_rows) and all(s["types_ok"] for s in ci_rows)

    spelled = {"ovito": False}

    def shift_list(arms: list[str]) -> str:
        parts = []
        for arm, change in shifts:
            if arm not in arms:
                continue
            name = arm_name(arm)
            if arm == "ovito" and not spelled["ovito"]:
                name = "Open Visualization Tool (OVITO)"
                spelled["ovito"] = True
            parts.append(f"{name} {change:+.1f} percent")
        return "; ".join(parts)

    session = f"the {args.label} session (commit {new['commit'][:8]})"
    opening = [
        f"The file in this comparison is cu32_f100k.",
        f"The later run is {session}.",
        "The baseline run is r2.9.",
        "A negative percent means the median time got faster.",
    ]
    if quickened:
        opening.append(f"Faster medians: {shift_list(quickened)}.")
    if slowed:
        opening.append(f"Slower medians: {shift_list(slowed)}.")
    paragraphs = [" ".join(opening)]
    if set(slowed) == set(changed) and same_hash and slowed:
        paragraphs.append(
            "The slower medians belong to the arms whose extension hash changed between the sessions. "
            "Within each session those two arms load one shared hash. "
            "The slowdown points at the rebuilt binary rather than at session drift of the host."
        )
    paragraphs.append(
        f"The interquartile range (IQR) rule is a descriptive screen. "
        f"Each cell uses n={width} repeats. "
        f"A cell is outside the screen when its median paired ratio moves by more than one IQR, "
        f"using the larger interquartile range of the two cells. "
        f"With n={width} the screen only describes those repeats."
    )
    paragraphs.append(
        f"An exact two-sided permutation test uses the {width}+{width} paired ratios in each cell. "
        f"The statistic is the difference of the two medians. "
        f"The smallest p this design can give is {floor:.3f} ({floor_extreme}/{floor_total}). "
        f"A Bonferroni correction across {cells} cells uses {corrected:.4f}. "
        + (
            "No cell survives that correction."
            if not survivors and floor > corrected
            else f"{len(survivors)} cells survive that correction."
        )
    )
    if same_hash and noise:
        paragraphs.append(
            "The rustc 1.88.0 arm and the r2.7 rebuild load the same bytes in each session. "
            f"Their median ratios differ by {min(noise):.2f} to {max(noise):.2f} times the larger IQR. "
            "That span is the in-session noise check."
        )
    if rebuild_rows:
        old_count = instruction_count(args.old_extension)
        new_count = instruction_count(args.new_extension)
        same_bytes = args.old_extension.stat().st_size == args.new_extension.stat().st_size
        size_note = (
            f"The two extension files are both {args.old_extension.stat().st_size} bytes. "
            if same_bytes
            else "The two extension files differ in size. "
        )
        paragraphs.append(
            f"The rustc 1.88.0 arm is outside the screen at {len(rebuild_sizes)} sizes. "
            f"The r2.7 rebuild crosses it at {len(second_sizes)} of those sizes. "
            f"Those {len(rebuild_rows)} cells are one build effect, because both arms load one hash per session. "
            + size_note
            + "`objdump -d --no-show-raw-insn` counts "
            f"{old_count} and {new_count} instructions in the r2.9 and r2.10 files."
        )
    if other_rows:
        count = len(other_rows)
        noun = "cell" if count == 1 else "cells"
        verb = "sits" if count == 1 else "sit"
        listed = ", ".join(row["name"] for row in other_rows).replace(
            "CON read", "eOn configuration (CON) read", 1
        )
        paragraphs.append(f"{count} other {noun} {verb} outside the screen: {listed}.")

    ci_hash = new["build"]["arms"]["readcon_extxyz"]["binary_sha256"]
    ci_unchanged = ci_hash == old["build"]["arms"]["readcon_extxyz"]["binary_sha256"]
    con_notes = []
    flagged_con = False
    for row in rows:
        if row["format"] != "con":
            continue
        flagged = row["outside"] or row["p"] <= ALPHA
        headline = row["size"] == "cu32_f100k"
        if not flagged and not headline:
            continue
        readcon_change = percent(
            statistics.median(times_for(old, "con", row["size"], "readcon_py")),
            statistics.median(times_for(new, "con", row["size"], "readcon_py")),
        )
        ase_change = percent(
            statistics.median(times_for(old, "con", row["size"], "ase_eon")),
            statistics.median(times_for(new, "con", row["size"], "ase_eon")),
        )
        if abs(readcon_change) > abs(ase_change):
            side = "readcon's side"
        elif abs(ase_change) > abs(readcon_change):
            side = "ASE's side"
        else:
            side = "both sides equally"
        sentence = (
            f"At {row['size']}, readcon moved {readcon_change:+.1f} percent ({direction_word(readcon_change)}) "
            f"and ASE moved {ase_change:+.1f} percent ({direction_word(ase_change)}). "
            f"The larger shift is on {side}."
        )
        if flagged and ci_unchanged:
            flagged_con = True
            sentence += (
                f" The CON read ratio moved {percent(row['old'], row['new']):+.2f} percent between these two sessions."
            )
        elif headline and ci_unchanged:
            sentence += (
                f" The CON read ratio moved {percent(row['old'], row['new']):+.2f} percent between these two sessions."
            )
        con_notes.append(sentence)
    if con_notes:
        lead = ""
        if not any(row["format"] == "con" for row in other_rows):
            lead = "CON means an eOn configuration file. "
        if ci_unchanged and flagged_con:
            lead += (
                f"The readcon CI binary {ci_hash[:8]} is unchanged between the sessions. "
                "The cause of the CON read moves is unexplained. "
            )
        if lead.strip():
            paragraphs.append(lead.strip())
        paragraphs.append(" ".join(con_notes))
    if screen_only or test_only:
        flagged = ", ".join(f"{row['name']} p={row['p']:.3f}" for row in screen_only)
        missed = ", ".join(f"{row['name']} p={row['p']:.3f}" for row in test_only)
        text = "Across both sessions, the screen and the test disagree. "
        if screen_only:
            text += f"The screen flags {flagged}. "
        if test_only:
            text += f"The test marks {missed}, which the screen leaves inside one IQR."
        if wheel_omits_types and ci_has_types:
            text += (
                " That published-wheel ratio is time only and not equal work, "
                "because the 0.18.0 wheel returns no atomic numbers."
            )
        paragraphs.append(text.strip())

    header = "| cell | r2.9 | r2.10 | screen | x IQR | p |"
    rule = "| --- | ---: | ---: | --- | ---: | ---: |"
    body = []
    for row in rows:
        screen = "more than one IQR" if row["outside"] else "inside one IQR"
        old_text = f"{row['old']:.2f}" if row["old"] >= 10 else f"{row['old']:.3f}"
        new_text = f"{row['new']:.2f}" if row["new"] >= 10 else f"{row['new']:.3f}"
        body.append(
            f"| {row['name']} | {old_text} | {new_text} | {screen} | {row['factor']:.2f} | {row['p']:.3f} |"
        )
    table = "\n".join([header, rule, *body])
    paragraphs.append("Every cell is in this table. The p column is the permutation test.")
    paragraphs.append(f"run.log carries all {blocks} per-sample blocks.")

    blocks_out = ["## Session drift", ""]
    blocks_out.append("\n\n".join(paragraphs[:-1]))
    blocks_out.append("")
    blocks_out.append(table)
    blocks_out.append("")
    blocks_out.append(paragraphs[-1])
    blocks_out.append("")
    print("\n".join(blocks_out), end="")


if __name__ == "__main__":
    main()
