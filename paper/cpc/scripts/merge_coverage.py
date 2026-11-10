#!/usr/bin/env python3
"""One coverage report from every instrumented binary.

``llvm-cov export`` keeps the first copy of a duplicated symbol. An
integration test that happens to sort first carries ``rkr_mpi_write_block_f``
with counters the merged profile does not match, and the counts from the
test binary that actually called it are discarded. This script asks
``llvm-cov`` for each readcon binary on its own, then keeps the highest
count any binary recorded for a region.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from collections import defaultdict
from pathlib import Path


def line_counts(record: dict) -> dict[int, int]:
    """Highest count of each line that has a counted segment."""
    found: dict[int, int] = defaultdict(int)
    for seg in record["segments"]:
        if seg[3]:
            found[seg[0]] = max(found[seg[0]], seg[2])
    return found


def line_count(document: dict, filename: str, line: int) -> int:
    for record in document["data"][0]["files"]:
        if record["filename"] == filename:
            return line_counts(record).get(line, 0)
    raise KeyError(filename)


def _branch_sides(record: dict) -> dict[tuple, list[int]]:
    sides: dict[tuple, list[int]] = {}
    for branch in record["branches"]:
        key = (branch[0], branch[1], branch[2], branch[3])
        current = sides.get(key)
        if current is None:
            sides[key] = [branch[4], branch[5]]
        else:
            current[0] = max(current[0], branch[4])
            current[1] = max(current[1], branch[5])
    return sides


def _apply_summary(record: dict, before_lines: dict[int, int], before_sides: dict) -> None:
    lines = line_counts(record)
    flipped = [n for n, count in before_lines.items() if count == 0 and lines.get(n, 0) > 0]
    summary = record["summary"]["lines"]
    summary["covered"] = min(summary["count"], summary["covered"] + len(flipped))
    summary["percent"] = 100.0 * summary["covered"] / summary["count"] if summary["count"] else 0.0

    after = _branch_sides(record)
    gained = 0
    branches = record["summary"]["branches"]
    for key, sides in after.items():
        if key not in before_sides:
            branches["count"] += 2
        previous = before_sides.get(key, [0, 0])
        gained += (previous[0] == 0 and sides[0] > 0) + (previous[1] == 0 and sides[1] > 0)
    branches["covered"] = min(branches["count"], branches["covered"] + gained)
    branches["notcovered"] = branches["count"] - branches["covered"]
    branches["percent"] = (
        100.0 * branches["covered"] / branches["count"] if branches["count"] else 0.0
    )


def _raise_counts(base: dict, extra: dict) -> None:
    indexed = {(seg[0], seg[1]): seg for seg in base["segments"]}
    for seg in extra["segments"]:
        found = indexed.get((seg[0], seg[1]))
        if found is None:
            if seg[3] and seg[2] > 0:
                base["segments"].append(seg)
                indexed[(seg[0], seg[1])] = seg
            continue
        if seg[2] > found[2]:
            found[2] = seg[2]
    sides = _branch_sides(base)
    for branch in extra["branches"]:
        key = (branch[0], branch[1], branch[2], branch[3])
        current = sides.get(key)
        if current is None:
            base["branches"].append(branch)
            sides[key] = [branch[4], branch[5]]
            continue
        if branch[4] > current[0] or branch[5] > current[1]:
            current[0] = max(current[0], branch[4])
            current[1] = max(current[1], branch[5])
            for existing in base["branches"]:
                if (existing[0], existing[1], existing[2], existing[3]) == key:
                    existing[4] = current[0]
                    existing[5] = current[1]
                    break


def merge_documents(documents: list[dict]) -> dict:
    """Keep the highest count any document recorded for each region.

    The first document supplies the file list and the summaries.
    Later documents can only raise a count.
    """
    if not documents:
        raise ValueError("no coverage documents")
    merged = json.loads(json.dumps(documents[0]))
    by_name = {record["filename"]: record for record in merged["data"][0]["files"]}
    before_lines = {name: line_counts(record) for name, record in by_name.items()}
    before_sides = {name: _branch_sides(record) for name, record in by_name.items()}
    for document in documents[1:]:
        for record in document["data"][0]["files"]:
            base = by_name.get(record["filename"])
            if base is None:
                copied = json.loads(json.dumps(record))
                merged["data"][0]["files"].append(copied)
                by_name[record["filename"]] = copied
                before_lines[record["filename"]] = {}
                before_sides[record["filename"]] = {}
                continue
            _raise_counts(base, record)
    for name, record in by_name.items():
        _apply_summary(record, before_lines[name], before_sides[name])
    return merged


def _executable(path: Path) -> bool:
    if not path.is_file():
        return False
    if path.suffix in {".d", ".rlib", ".rmeta", ".o"}:
        return False
    name = path.name
    if name.startswith("build_script") or name.startswith("build-script"):
        return False
    return os.access(path, os.X_OK)


def _elf(path: Path) -> bool:
    try:
        with path.open("rb") as handle:
            return handle.read(4) == b"\x7fELF"
    except OSError:
        return False


def discover_objects(root: Path) -> list[Path]:
    """Instrumented readcon binaries, not build-script or CMake helpers."""
    found = []
    for path in root.rglob("*"):
        if not _executable(path) or not _elf(path):
            continue
        text = str(path)
        if "/build/readcon-core/" in text or path.name.startswith("libreadcon_core") or path.name == "readcon-core":
            found.append(path)

    def rank(path: Path) -> tuple:
        name = path.name
        if name.startswith("readcon_core-"):
            group = 0
        elif "libreadcon_core" in name:
            group = 1
        elif name.startswith("readcon_core"):
            group = 2
        else:
            group = 3
        return (group, -path.stat().st_size, str(path))

    return sorted(found, key=rank)


def _export(llvm_cov: str, profdata: Path, objects: list[Path], workspace: Path) -> dict:
    command = [
        llvm_cov,
        "export",
        f"--instr-profile={profdata}",
        "-format=text",
        f"-path-equivalence=.,{workspace}",
    ]
    for path in objects:
        command.extend(["-object", str(path)])
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(completed.stdout)


def write_report(profdata: Path, search_root: Path, output: Path, workspace: Path) -> None:
    llvm_cov = os.environ.get(
        "LLVM_COV",
        "llvm-cov",
    )
    objects = discover_objects(search_root)
    if not objects:
        raise SystemExit(f"no instrumented binaries under {search_root}")
    # The lib test binaries go first so a duplicated symbol keeps the
    # counters from the binary that ran the tests. A later binary can
    # still raise a count that the first one left at zero.
    skeleton = _export(llvm_cov, profdata, objects, workspace)
    readcon = [path for path in objects if "readcon_core" in path.name or "libreadcon" in path.name]
    documents = [skeleton]
    for path in readcon:
        documents.append(_export(llvm_cov, profdata, [path], workspace))
    merged = merge_documents(documents)
    output.write_text(json.dumps(merged))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profdata", type=Path, required=True)
    parser.add_argument("--search-root", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    write_report(args.profdata, args.search_root, args.output, args.workspace)
    print("wrote", args.output)


if __name__ == "__main__":
    main()
