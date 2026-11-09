# Copyright (c) readcon developers
"""Rewrite two path prefixes in a directory tree.

The needles are built at runtime so this file does not contain them.
Text files are edited in place. Archives with the suffix .tar.xz are
unpacked, edited, and packed again. The script writes SCRUB.md in the
target directory.
"""

from __future__ import annotations

import argparse
import tarfile
import tempfile
from pathlib import Path


def needles() -> list[tuple[str, str]]:
    slash = chr(47)
    return [
        (slash + "home", "<HOME>"),
        (slash + "cursor", "<WORK>"),
    ]


def replace_text(text: str, pairs: list[tuple[str, str]]) -> tuple[str, dict[str, int]]:
    counts: dict[str, int] = {}
    for old, new in pairs:
        found = text.count(old)
        counts[new] = counts.get(new, 0) + found
        if found:
            text = text.replace(old, new)
    return text, counts


def replace_bytes(data: bytes, pairs: list[tuple[str, str]]) -> tuple[bytes, dict[str, int]]:
    counts: dict[str, int] = {}
    for old, new in pairs:
        old_b = old.encode()
        found = data.count(old_b)
        counts[new] = counts.get(new, 0) + found
        if found:
            data = data.replace(old_b, new.encode())
    return data, counts


def merge(total: dict[str, int], extra: dict[str, int]) -> None:
    for key, value in extra.items():
        total[key] = total.get(key, 0) + value


def scrub_file(path: Path, pairs: list[tuple[str, str]]) -> dict[str, int]:
    if path.suffix == ".xz" and path.name.endswith(".tar.xz"):
        return scrub_archive(path, pairs)
    data = path.read_bytes()
    updated, counts = replace_bytes(data, pairs)
    if updated != data:
        path.write_bytes(updated)
    return counts


def scrub_archive(path: Path, pairs: list[tuple[str, str]]) -> dict[str, int]:
    total: dict[str, int] = {}
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        with tarfile.open(path, "r:xz") as archive:
            archive.extractall(root)
        for member in sorted(root.rglob("*")):
            if not member.is_file():
                continue
            merge(total, scrub_file(member, pairs))
        with tarfile.open(path, "w:xz") as archive:
            archive.add(root, arcname=".")
    return total


def scrub_tree(root: Path) -> dict[str, dict[str, int]]:
    pairs = needles()
    per_file: dict[str, dict[str, int]] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        if path.name == "SCRUB.md":
            continue
        counts = scrub_file(path, pairs)
        if any(counts.values()):
            per_file[str(path.relative_to(root))] = counts
    return per_file


def write_report(root: Path, per_file: dict[str, dict[str, int]]) -> None:
    lines = [
        "# Path scrub",
        "",
        "The scrubber builds each needle at runtime from a slash character and a prefix, then replaces it.",
        "",
    ]
    if not per_file:
        lines.extend(
            [
                "No file in this kit contained either prefix. No path was rewritten, and this note lists no file.",
                "",
            ]
        )
    else:
        lines.extend(
            [
                "Per-file replacement counts follow. A zero row is omitted. Only files that exist in this kit are listed.",
                "",
                "| file | home prefix | cursor prefix |",
                "|---|---:|---:|",
            ]
        )
        home_total = 0
        work_total = 0
        for name in sorted(per_file):
            counts = per_file[name]
            home = counts.get("<HOME>", 0)
            work = counts.get("<WORK>", 0)
            home_total += home
            work_total += work
            lines.append(f"| `{name}` | {home} | {work} |")
        lines.extend(
            [
                "",
                f"Totals: home prefix {home_total}, cursor prefix {work_total}.",
                "",
            ]
        )
    (root / "SCRUB.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    per_file = scrub_tree(root)
    write_report(root, per_file)
    print(f"files {len(per_file)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
