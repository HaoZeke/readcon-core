"""Physical executable-line score from per-object llvm-cov export JSON.

A source line is executable when any object maps a counted segment onto
it, and covered when any object's count on that line is positive.
Display text such as ``784k`` is checked against the integer in the
export. A missing object, a missing source, or a count that is not an
integer is an error.
"""

from __future__ import annotations

import json
from pathlib import Path


class ScoreError(Exception):
    """The inventory or an export cannot be scored."""


def display_count(count: int) -> str:
    """The token ``llvm-cov show`` prints for an integer count.

    Values under 1000 are decimal. Larger values use a 1000-based suffix
    and truncate, matching ``784346`` to ``784k`` and ``1566582`` to
    ``1.56M``.
    """
    if count < 0:
        raise ScoreError(f"negative count {count}")
    if count < 1000:
        return str(count)
    value = float(count)
    suffix = ""
    for mark in ("k", "M", "G", "T", "P", "E"):
        value /= 1000.0
        suffix = mark
        if value < 1000.0:
            break
    if value >= 100.0:
        shown = str(int(value))
    elif value >= 10.0:
        shown = f"{int(value * 10.0) / 10.0:.1f}"
    else:
        shown = f"{int(value * 100.0) / 100.0:.2f}"
    return shown + suffix


def _count(value: object, where: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ScoreError(f"unknown count at {where}: {value!r}")
    if value < 0:
        raise ScoreError(f"negative count at {where}: {value}")
    return value


def line_hits(export: dict, source: str) -> dict[int, int]:
    """Maximum count of each physical line of ``source`` in one export."""
    found = False
    lines: dict[int, int] = {}
    data = export.get("data")
    if not isinstance(data, list) or not data:
        raise ScoreError(f"{source}: export has no data")
    for entry in data:
        files = entry.get("files")
        if not isinstance(files, list):
            raise ScoreError(f"{source}: export files are not a list")
        for record in files:
            name = record.get("filename")
            if not isinstance(name, str) or not name.endswith(source):
                continue
            found = True
            segments = record.get("segments")
            if not isinstance(segments, list):
                raise ScoreError(f"{source}: segments missing in {name}")
            for index, segment in enumerate(segments):
                if not isinstance(segment, list) or len(segment) < 4:
                    raise ScoreError(f"{source}: short segment {index}")
                if segment[3] is not True:
                    continue
                line = segment[0]
                if isinstance(line, bool) or not isinstance(line, int):
                    raise ScoreError(f"{source}: bad line {line!r}")
                count = _count(segment[2], f"{source}:{line}")
                if count > lines.get(line, 0):
                    lines[line] = count
    if not found:
        raise ScoreError(f"missing source {source}")
    return lines


def union_lines(exports: list[dict], sources: list[str]) -> dict[str, dict]:
    """Union ``exports``, one document per object. Every source is required."""
    if not exports:
        raise ScoreError("no objects")
    if not sources:
        raise ScoreError("no sources")
    scored: dict[str, dict] = {}
    for source in sources:
        best: dict[int, int] = {}
        for export in exports:
            for line, count in line_hits(export, source).items():
                if count > best.get(line, 0):
                    best[line] = count
        missed = sorted(line for line, count in best.items() if count == 0)
        hit = len(best) - len(missed)
        scored[source] = {
            "hit": hit,
            "total": len(best),
            "miss": missed,
        }
    return scored


def check_display(raw: int, shown: str) -> None:
    """Fail when ``shown`` is not the display form of ``raw``."""
    expect = display_count(raw)
    if shown != expect:
        raise ScoreError(f"display {shown!r} is not {expect!r} for {raw}")


def load_inventory(path: Path) -> dict:
    """Read an inventory. Each object must name an export file that exists."""
    document = json.loads(path.read_text(encoding="utf-8"))
    objects = document.get("objects")
    sources = document.get("sources")
    if not isinstance(objects, list) or not objects:
        raise ScoreError("inventory has no objects")
    if not isinstance(sources, list) or not sources:
        raise ScoreError("inventory has no sources")
    exports = []
    for obj in objects:
        ident = obj.get("id")
        digest = obj.get("digest")
        export_path = obj.get("export")
        if not ident or not digest or not export_path:
            raise ScoreError(f"object missing id, digest, or export: {obj!r}")
        file = Path(export_path)
        if not file.is_file():
            raise ScoreError(f"missing export for {ident}")
        exports.append(json.loads(file.read_text(encoding="utf-8")))
    return {"exports": exports, "sources": sources, "objects": objects}


def score_inventory(path: Path) -> dict[str, dict]:
    """Score the inventory at ``path``."""
    loaded = load_inventory(path)
    return union_lines(loaded["exports"], loaded["sources"])
