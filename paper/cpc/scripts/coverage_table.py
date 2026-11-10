#!/usr/bin/env python3
"""Write COVERAGE_TABLE.md from a cargo llvm-cov --branch JSON export.

The hit, total, and percent columns are the file summary in that JSON.
A file is OPEN when its line percent is under 98 or its branch percent
is under 98. Branch percent is not applied when the file has no branches.
"""

import argparse
import json
import sys
from collections import defaultdict


def ranges(lines):
    if not lines:
        return "none"
    lines = sorted(set(lines))
    spans = []
    start = prev = lines[0]
    for line in lines[1:]:
        if line == prev + 1:
            prev = line
            continue
        spans.append((start, prev))
        start = prev = line
    spans.append((start, prev))
    parts = []
    for start, end in spans:
        parts.append(str(start) if start == end else f"{start}-{end}")
    return ", ".join(parts)


def uncovered_lines(segments):
    by_line = defaultdict(list)
    for seg in segments:
        if len(seg) < 6:
            continue
        by_line[seg[0]].append(seg)
    missed = []
    for line, segs in by_line.items():
        entries = [seg for seg in segs if seg[3] and seg[4] and not seg[5]]
        if entries and all(seg[2] == 0 for seg in entries):
            missed.append(line)
    return sorted(missed)


def uncovered_branches(records):
    groups = defaultdict(list)
    for rec in records:
        if len(rec) < 6:
            continue
        groups[(rec[0], rec[1], rec[2], rec[3])].append(rec)
    missed = []
    for key, recs in groups.items():
        best = max(recs, key=lambda rec: ((rec[4] > 0) + (rec[5] > 0), rec[4] + rec[5]))
        if best[4] == 0:
            missed.append((key[0], key[1], "true"))
        if best[5] == 0:
            missed.append((key[0], key[1], "false"))
    missed.sort()
    return missed


def rel_name(filename, root):
    if root and filename.startswith(root):
        filename = filename[len(root) :]
        if filename.startswith("/"):
            filename = filename[1:]
    return filename


def percent_text(summary):
    count = summary["count"]
    if count == 0:
        return "n/a"
    return f"{summary['percent']:.2f}"


def is_open(summary):
    lines = summary["lines"]
    branches = summary["branches"]
    line_open = lines["count"] == 0 or lines["percent"] < 98
    branch_open = branches["count"] > 0 and branches["percent"] < 98
    return line_open or branch_open


def render(data, root, measured):
    files = []
    for block in data["data"]:
        files.extend(block["files"])
    files.sort(key=lambda item: rel_name(item["filename"], root))
    rows = []
    open_files = []
    for item in files:
        name = rel_name(item["filename"], root)
        if "/src/" not in f"/{name}" and not name.startswith("src/"):
            continue
        summary = item["summary"]
        status = "OPEN" if is_open(summary) else "met"
        rows.append(
            (
                name,
                summary["lines"]["covered"],
                summary["lines"]["count"],
                percent_text(summary["lines"]),
                summary["branches"]["covered"],
                summary["branches"]["count"],
                percent_text(summary["branches"]),
                status,
            )
        )
        if status == "OPEN":
            open_files.append(item)
    lines = [
        "# Coverage table",
        "",
        "Generated from a `cargo llvm-cov --branch` JSON export.",
        "Hit, total, and percent are that export's file summary.",
        "OPEN means lines or branches are under 98 percent.",
        "A file with no branches is not failed for branches.",
        "",
        f"Measured commit: `{measured}`",
        "",
        "Command:",
        "",
        "```",
        "cargo llvm-cov --workspace --all-targets \\",
        "  --features parallel,zstd,grammar --branch --no-fail-fast \\",
        "  --json --output-path coverage.json",
        "```",
        "",
        "| File | Lines hit | Lines total | Lines % | Branches hit | Branches total | Branches % | Status |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row[0]} | {row[1]} | {row[2]} | {row[3]} | {row[4]} | {row[5]} | {row[6]} | {row[7]} |"
        )
    lines.append("")
    lines.append(f"OPEN files: {len(open_files)}. Measured files: {len(rows)}.")
    lines.append("")
    for item in open_files:
        name = rel_name(item["filename"], root)
        summary = item["summary"]
        line_miss = summary["lines"]["count"] - summary["lines"]["covered"]
        branch_miss = summary["branches"].get(
            "notcovered", summary["branches"]["count"] - summary["branches"]["covered"]
        )
        missed_lines = uncovered_lines(item.get("segments") or [])
        missed_branches = uncovered_branches(item.get("branches") or [])
        lines.append(f"## {name}")
        lines.append("")
        line_word = "line" if line_miss == 1 else "lines"
        branch_word = "branch" if branch_miss == 1 else "branches"
        lines.append(
            f"Status: OPEN. Summary misses {line_miss} {line_word} and {branch_miss} {branch_word}."
        )
        lines.append("")
        lines.append("Uncovered lines:")
        lines.append("")
        lines.append(ranges(missed_lines))
        lines.append("")
        lines.append("Uncovered branches (line:column, side not taken):")
        lines.append("")
        if not missed_branches:
            lines.append("none")
        else:
            for line_no, col, side in missed_branches:
                lines.append(f"- {line_no}:{col} {side}")
        if line_miss != len(missed_lines) or branch_miss != len(missed_branches):
            lines.append("")
            lines.append(
                "The summary counts each instantiation. "
                "The locations above are the merged map, where a side is listed "
                "only when the best copy still leaves it at zero."
            )
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("json_path")
    parser.add_argument("--output", required=True)
    parser.add_argument("--root", default="")
    parser.add_argument("--measured", default="HEAD")
    args = parser.parse_args()
    with open(args.json_path, encoding="utf-8") as handle:
        data = json.load(handle)
    text = render(data, args.root.rstrip("/"), args.measured)
    with open(args.output, "w", encoding="utf-8") as handle:
        handle.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
