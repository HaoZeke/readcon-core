"""Print the session drift section from two results files.

The script checks the sha256 of each input against --expect before it
reads the files. stdout is the report section, verbatim.
"""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import math
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
        "readcon_extxyz": "continuous integration (CI) readcon",
        "readcon_extxyz_188": "rustc 1.88.0",
        "readcon_extxyz_r27": "r2.7 rebuild",
        "readcon_pypi": "published wheel",
        "ase_extxyz": "Atomic Simulation Environment (ASE)",
        "chemfiles": "chemfiles",
        "ovito": "Open Visualization Tool (OVITO)",
        "mdanalysis": "MDAnalysis",
        "pymatgen": "pymatgen",
        "ase_eon": "Atomic Simulation Environment (ASE)",
    }
    return names.get(arm, arm)


def short_peer(peer: str) -> str:
    return {
        "readcon_extxyz": "ci",
        "readcon_extxyz_188": "r188",
        "readcon_extxyz_r27": "r27",
        "readcon_pypi": "pypi",
        "ase_extxyz": "ase",
        "chemfiles": "chem",
        "ovito": "ovito",
        "pymatgen": "pymat",
        "mdanalysis": "mda",
        "ase_eon": "eon",
    }.get(peer, peer)


def short_cell(fmt: str, size: str, peer: str) -> str:
    tag = {"extxyz": "xyz", "con": "con", "con_write": "wcon"}[fmt]
    return f"{size} {tag} {short_peer(peer)}"


def cell_name(fmt: str, size: str, peer: str) -> str:
    kind = {
        "extxyz": "extended XYZ format",
        "con": "eOn configuration (CON) read",
        "con_write": "eOn configuration (CON) write",
    }[fmt]
    return f"{kind} {size} {arm_name(peer)}"


def direction_word(change: float) -> str:
    if change < 0:
        return "faster"
    if change > 0:
        return "slower"
    return "unchanged"


def times_for(data: dict, fmt: str, size: str, arm: str) -> list[float]:
    rows = [
        s
        for s in data["samples"]
        if s["format"] == fmt and s["size"] == size and s["arm"] == arm
    ]
    rows.sort(key=lambda s: s["repeat"])
    return [s["elapsed_s"] for s in rows]


def paired_ratios(
    data: dict, fmt: str, size: str, peer: str, baseline: str
) -> list[float]:
    base = {
        s["repeat"]: s["elapsed_s"]
        for s in data["samples"]
        if s["format"] == fmt and s["size"] == size and s["arm"] == baseline
    }
    rows = [
        s
        for s in data["samples"]
        if s["format"] == fmt and s["size"] == size and s["arm"] == peer
    ]
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
    if path.name.endswith((".tar", ".tar.xz")):
        with tarfile.open(path) as archive:
            member = next(
                item for item in archive.getmembers() if item.name.endswith("run.log")
            )
            text = archive.extractfile(member).read().decode("utf-8", "replace")
    else:
        text = path.read_text(encoding="utf-8", errors="replace")
    return sum(1 for line in text.splitlines() if line.startswith("command taskset"))


def mean_log(values: list[float]) -> float:
    return sum(math.log(value) for value in values) / len(values)


def permutation_p(left: list[float], right: list[float]) -> tuple[float, int, int]:
    """Two-sided exact p for the difference of mean log ratios."""
    observed = mean_log(right) - mean_log(left)
    pool = list(left) + list(right)
    width = len(left)
    extreme = 0
    total = 0
    for comb in itertools.combinations(range(len(pool)), width):
        chosen = set(comb)
        sample_a = [pool[i] for i in comb]
        sample_b = [pool[i] for i in range(len(pool)) if i not in chosen]
        value = mean_log(sample_b) - mean_log(sample_a)
        total += 1
        if abs(value) >= abs(observed) - 1e-12:
            extreme += 1
    return extreme / total, extreme, total


def design_floor(left_n: int, right_n: int) -> tuple[float, int, int]:
    left = [math.exp(index) for index in range(left_n)]
    # exp(1000) overflows. A gap of 20 still puts every right-hand
    # value above every left-hand value. Unequal sizes leave one
    # fully separated assignment, so the floor is that one split.
    # Equal sizes leave two assignments: that split and the swap.
    right = [math.exp(20 + index) for index in range(right_n)]
    return permutation_p(left, right)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("previous", type=Path)
    parser.add_argument("current", type=Path)
    parser.add_argument("--old-extension", type=Path, required=True)
    parser.add_argument("--new-extension", type=Path, required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--expect", type=Path, required=True)
    parser.add_argument("--label", default="later")
    parser.add_argument("--baseline", default="earlier")
    args = parser.parse_args()

    expect = load_expect(args.expect)
    for path in (
        args.previous,
        args.current,
        args.old_extension,
        args.new_extension,
        args.log,
    ):
        require_hash(path, expect)

    old = load(args.previous)
    new = load(args.current)
    if (
        sha256(args.old_extension)
        != old["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]
    ):
        raise SystemExit("baseline extension does not match its results record")
    if (
        sha256(args.new_extension)
        != new["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]
    ):
        raise SystemExit("later extension does not match its results record")
    blocks = len(new["samples"])
    if status_ok_blocks(args.log) != blocks:
        raise SystemExit("log blocks differ from the sample count")

    file_size = "cu32_f100k"
    file_format = "extxyz"
    arm_order: list[str] = []
    for sample in old["samples"]:
        if (
            sample["format"] == file_format
            and sample["size"] == file_size
            and sample["arm"] not in arm_order
        ):
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
    old_counts: set[int] = set()
    new_counts: set[int] = set()
    for before in old["pairs"]:
        after = pair_row(new, before["format"], before["size"], before["peer"])
        baseline = before["readcon"]
        old_ratios = paired_ratios(
            old, before["format"], before["size"], before["peer"], baseline
        )
        new_ratios = paired_ratios(
            new, before["format"], before["size"], before["peer"], baseline
        )
        old_counts.add(len(old_ratios))
        new_counts.add(len(new_ratios))
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
    if len(old_counts) != 1 or len(new_counts) != 1:
        raise SystemExit(f"repeat counts differ: {old_counts} {new_counts}")
    old_n = old_counts.pop()
    new_n = new_counts.pop()
    floor, floor_extreme, floor_total = design_floor(old_n, new_n)
    cells = len(rows)
    corrected = ALPHA / cells
    uncorrected = [row for row in rows if row["p"] <= ALPHA]
    survivors = [row for row in rows if row["p"] < corrected]
    screen_only = [row for row in rows if row["outside"] and row["p"] > ALPHA]
    test_only = [row for row in rows if (not row["outside"]) and row["p"] <= ALPHA]

    rebuild_peers = {"readcon_extxyz_188", "readcon_extxyz_r27"}
    rebuild_rows = [
        row for row in rows if row["outside"] and row["peer"] in rebuild_peers
    ]
    other_rows = [
        row for row in rows if row["outside"] and row["peer"] not in rebuild_peers
    ]
    rebuild_sizes = sorted(
        {row["size"] for row in rebuild_rows if row["peer"] == "readcon_extxyz_188"}
    )
    second_sizes = sorted(
        {row["size"] for row in rebuild_rows if row["peer"] == "readcon_extxyz_r27"}
    )

    noise = []
    for data in (old, new):
        if (
            data["build"]["arms"]["readcon_extxyz_188"]["binary_sha256"]
            != data["build"]["arms"]["readcon_extxyz_r27"]["binary_sha256"]
        ):
            continue
        sizes = sorted(
            {row["size"] for row in data["pairs"] if row["format"] == "extxyz"}
        )
        for size in sizes:
            first = pair_row(data, "extxyz", size, "readcon_extxyz_188")
            second = pair_row(data, "extxyz", size, "readcon_extxyz_r27")
            gap = abs(
                first["ratio_median_of_paired"] - second["ratio_median_of_paired"]
            )
            larger = max(first["ratio_iqr_of_paired"], second["ratio_iqr_of_paired"])
            noise.append(gap / larger)

    wheel_rows = [s for s in new["samples"] if s["arm"] == "readcon_pypi"]
    ci_rows = [
        s
        for s in new["samples"]
        if s["arm"] == "readcon_extxyz" and s["format"] == "extxyz"
    ]
    wheel_omits_types = bool(wheel_rows) and all(
        s["types_dtype"] == "absent" for s in wheel_rows
    )
    ci_has_types = bool(ci_rows) and all(s["types_ok"] for s in ci_rows)

    spelled = {"ovito": False, "ase": False, "ci": False}

    def shift_sentences(arms: list[str], word: str) -> list[str]:
        found = []
        for arm, change in shifts:
            if arm not in arms:
                continue
            name = arm_name(arm)
            if arm == "ovito" and not spelled["ovito"]:
                name = "Open Visualization Tool (OVITO)"
                spelled["ovito"] = True
            elif arm in {"ase_extxyz", "ase_eon"} and not spelled["ase"]:
                name = "Atomic Simulation Environment (ASE)"
                spelled["ase"] = True
            elif arm == "readcon_extxyz" and not spelled["ci"]:
                name = "continuous integration (CI) readcon"
                spelled["ci"] = True
            found.append(f"{name} is {abs(change):.1f} percent {word}.")
        return found

    session = f"the {args.label} session (commit {new['commit'][:8]})"
    opening = [
        "The file in this comparison is cu32_f100k.",
        f"The later run is {session}.",
        f"The baseline run is {args.baseline}.",
        "A negative percent means the median time got faster.",
    ]
    opening.extend(shift_sentences(quickened, "faster"))
    opening.extend(shift_sentences(slowed, "slower"))
    paragraphs = list(opening)
    if set(slowed) == set(changed) and same_hash and slowed:
        paragraphs.append(
            "The slower medians belong to the arms whose extension hash changed."
        )
        paragraphs.append("Within each session those two arms load one shared hash.")
        paragraphs.append("The slowdown points at the rebuilt binary.")
    if old_n == new_n:
        repeat_sentence = f"Both sessions use {old_n} repeats."
    else:
        repeat_sentence = (
            f"The earlier cells use {old_n} repeats and the later cells use {new_n} repeats."
        )
    paragraphs.append("The interquartile range (IQR) is a descriptive screen.")
    paragraphs.append(repeat_sentence)
    paragraphs.append("A cell is outside the screen when its paired ratio moves too far.")
    paragraphs.append("The limit is the larger of the two interquartile ranges.")
    paragraphs.append(
        f"A permutation test uses the {old_n} and {new_n} paired ratios."
    )
    paragraphs.append("The statistic is the difference of the mean log ratios.")
    paragraphs.append(
        f"The smallest p this design can give is {floor:.6f} ({floor_extreme}/{floor_total})."
    )
    paragraphs.append(f"Uncorrected cells at p<=0.05: {len(uncorrected)}.")
    paragraphs.append(
        f"Cells below the Bonferroni level {corrected:.6f}: {len(survivors)}."
    )
    if same_hash and noise:
        paragraphs.append(
            "The rustc 1.88.0 arm and the r2.7 rebuild load the same bytes in each session."
        )
        paragraphs.append(
            f"Their median ratios differ by {min(noise):.2f} to {max(noise):.2f} times the larger IQR."
        )
        paragraphs.append("That span is the in-session noise check.")
    if rebuild_rows:
        old_count = instruction_count(args.old_extension)
        new_count = instruction_count(args.new_extension)
        same_bytes = (
            args.old_extension.stat().st_size == args.new_extension.stat().st_size
        )
        size_note = (
            f"The two extension files are both {args.old_extension.stat().st_size} bytes. "
            if same_bytes
            else "The two extension files differ in size. "
        )
        claim = ""
        if same_hash:
            claim = (
                f" Those {len(rebuild_rows)} cells are one build effect, "
                "because both arms load one hash per session."
            )

        def size_phrase(count: int) -> str:
            if count == 1:
                return "1 size"
            return f"{count} sizes"

        if len(rebuild_sizes) == 0:
            paragraphs.append(
                "Every rustc 1.88.0 ratio stays inside one interquartile range."
            )
        else:
            paragraphs.append(
                "The rustc 1.88.0 ratios leave one interquartile range at "
                f"{size_phrase(len(rebuild_sizes))}."
            )
        paragraphs.append(
            "The rebuilt r2.7 extension leaves that range at "
            f"{size_phrase(len(second_sizes))}."
        )
        if claim:
            paragraphs.append(claim.strip())
        if same_bytes:
            paragraphs.append(
                f"The two extension files are both {args.old_extension.stat().st_size} bytes."
            )
        else:
            paragraphs.append("The two extension files differ in size.")
        paragraphs.append(
            f"objdump counts {old_count} instructions in the {args.baseline} file."
        )
        paragraphs.append(
            f"It counts {new_count} instructions in the {args.label} file."
        )
    if other_rows:
        paragraphs.append("The interquartile range also flags more cells.")
        for row in other_rows:
            paragraphs.append(f"It flags {short_cell(row['format'], row['size'], row['peer'])}.")

    bench_names = []
    label_names = []
    session_names = []
    readcon_peers = {
        "readcon_extxyz",
        "readcon_extxyz_188",
        "readcon_extxyz_r27",
        "readcon_pypi",
    }
    for row in rows:
        if not row["outside"]:
            continue
        if row["peer"] == "ovito" or (
            row["format"] == "con" and row["peer"] == "ase_eon"
        ):
            bench_names.append(short_cell(row["format"], row["size"], row["peer"]))
        elif row["peer"] in readcon_peers:
            old_hash = old["build"]["arms"][row["peer"]]["binary_sha256"]
            new_hash = new["build"]["arms"][row["peer"]]["binary_sha256"]
            if old_hash != new_hash:
                label_names.append(row["name"])
            else:
                session_names.append(short_cell(row["format"], row["size"], row["peer"]))
        else:
            session_names.append(short_cell(row["format"], row["size"], row["peer"]))

    def joined(names: list[str]) -> str:
        if not names:
            return "none"
        return ", ".join(names).replace("CON read", "eOn configuration (CON) read", 1)

    ovito_name = "OVITO" if spelled["ovito"] else "Open Visualization Tool (OVITO)"
    if bench_names:
        paragraphs.append("Bench code accounts for " + joined(bench_names) + ".")
    if any(name.endswith(" ovito") for name in bench_names):
        paragraphs.append(f"{ovito_name} builds its type table once per file.")
    if any(name.endswith(" eon") for name in bench_names):
        paragraphs.append("The eOn reader maps each distinct symbol once.")
    if label_names:
        paragraphs.append(
            "A changed readcon archive label accounts for " + joined(label_names) + "."
        )
    if session_names:
        paragraphs.append("The session moved these cells.")
        for name in session_names:
            paragraphs.append(f"It moved {name}.")

    ci_hash = new["build"]["arms"]["readcon_extxyz"]["binary_sha256"]
    ci_unchanged = ci_hash == old["build"]["arms"]["readcon_extxyz"]["binary_sha256"]
    con_notes = []
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
            side = "readcon"
        elif abs(ase_change) > abs(readcon_change):
            side = "the Atomic Simulation Environment (ASE)"
        else:
            side = "both readers equally"
        move = percent(row["old"], row["new"])
        con_notes.append(f"At {row['size']} the larger change is on {side}.")
        con_notes.append(f"The paired ratio moves by {move:+.2f} percent.")
    if con_notes:
        paragraphs.append("The eOn configuration (CON) reads move as follows.")
        paragraphs.extend(con_notes)
    if screen_only or test_only:
        def p_text(value: float) -> str:
            if value >= 0.001:
                return f"{value:.3f}"
            return f"{value:.6f}"

        if screen_only or test_only:
            paragraphs.append("The screen and the test disagree.")
        for row in screen_only:
            paragraphs.append(
                "The screen flags "
                f"{short_cell(row['format'], row['size'], row['peer'])} "
                f"at p={p_text(row['p'])}."
            )
        for row in test_only:
            paragraphs.append(
                "The test marks "
                f"{short_cell(row['format'], row['size'], row['peer'])} "
                f"at p={p_text(row['p'])}."
            )
        if test_only:
            paragraphs.append("The screen leaves those cells inside one IQR.")
        cited = screen_only + test_only
        if wheel_omits_types and ci_has_types and any(
            row["peer"] == "readcon_pypi" for row in cited
        ):
            paragraphs.append(
                "That published-wheel ratio is time only and not equal work."
            )
            paragraphs.append(
                "The 0.18.0 wheel returns no atomic numbers."
            )

    header = f"| cell | {args.baseline} | {args.label} | screen | x IQR | p |"
    rule = "| --- | ---: | ---: | --- | ---: | ---: |"
    body = []
    for row in rows:
        screen = "outside" if row["outside"] else "inside"
        old_text = f"{row['old']:.2f}" if row["old"] >= 10 else f"{row['old']:.3f}"
        new_text = f"{row['new']:.2f}" if row["new"] >= 10 else f"{row['new']:.3f}"
        if row["p"] >= 0.001:
            p_text = f"{row['p']:.3f}"
        else:
            p_text = f"{row['p']:.6f}"
        body.append(
            "| "
            f"{short_cell(row['format'], row['size'], row['peer'])} "
            f"| {old_text} | {new_text} | {screen} | {row['factor']:.2f} | {p_text} |"
        )
    table = "\n".join([header, rule, *body])
    paragraphs.append("xyz is an extended XYZ format read.")
    paragraphs.append("con is an eOn configuration (CON) read.")
    paragraphs.append("wcon is a CON write.")
    paragraphs.append("The table writes rustc 1.88.0 as r188.")
    paragraphs.append("It writes the r2.7 rebuild as r27.")
    paragraphs.append(
        "Other peers are ci, pypi, ase, chem, ovito, pymat, mda, and eon."
    )
    paragraphs.append("Every cell is in this table.")
    paragraphs.append("The p column is the permutation test.")
    paragraphs.append(f"run.log carries all {blocks} per-sample blocks.")

    blocks_out = []
    blocks_out.append("\n\n".join(paragraphs[:-1]))
    blocks_out.append("")
    blocks_out.append(table)
    blocks_out.append("")
    blocks_out.append(paragraphs[-1])
    blocks_out.append("")
    print("\n".join(blocks_out), end="")


if __name__ == "__main__":
    main()
