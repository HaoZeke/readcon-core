#!/usr/bin/env python3
"""The readcon module of the claim-check runs against the module of the other records, section by section.

The other records of the paper name one module build of readcon-core, made in another checkout of the same
commit.  The claim-check runs import the module that ``pixi run cpc-readcon`` builds in this checkout.  The
linker writes the library directory of the pixi environment into the run path (``DT_RPATH``) of the module,
so the two files differ wherever that path, and the size of the string table that holds it, reaches.  This
script reads both ELF files and shows that every other byte is the same:

- the section tables list the same sections in the same order;
- every section but ``.dynstr``, ``.gnu.version``, ``.gnu.version_r`` and ``.dynamic`` is byte for byte the
  same, and those that the shorter string table moves hold the same bytes at their new offsets;
- ``.dynstr`` differs only in the run path;
- ``.dynamic`` differs only in the entries that give the size of ``.dynstr`` and the addresses of the
  sections after it;
- the section headers differ only in the address, offset and size of those sections.

``CC_EARLIER_MODULE`` names the module of the other records, whose SHA-256 has to be the one they name.
The record is ``data/claimcheck/module_identity.json``.
"""

from __future__ import annotations

import json
import os
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cc_common as cc  # noqa: E402
import provenance  # noqa: E402

TAGS = {
    10: "DT_STRSZ",
    15: "DT_RPATH",
    29: "DT_RUNPATH",
    0x6FFFFFF0: "DT_VERSYM",
    0x6FFFFFFE: "DT_VERNEED",
    0x6FFFFFFC: "DT_VERDEF",
    5: "DT_STRTAB",
    6: "DT_SYMTAB",
}
RECORD_DIRS = ("data", "evidence/results", "bench/results", "bench/records")


def sections(data: bytes) -> list[dict]:
    if data[:4] != b"\x7fELF" or data[4] != 2 or data[5] != 1:
        raise SystemExit("not a little-endian ELF64 file")
    shoff = struct.unpack_from("<Q", data, 0x28)[0]
    entsize, count, names = struct.unpack_from("<HHH", data, 0x3A)
    rows = []
    for i in range(count):
        name, kind, _flags, addr, offset, size, _link, _info, _align, _entry = (
            struct.unpack_from("<IIQQQQIIQQ", data, shoff + i * entsize)
        )
        rows.append(
            {
                "name_offset": name,
                "type": kind,
                "addr": addr,
                "offset": offset,
                "size": size,
                "header_at": shoff + i * entsize,
            }
        )
    table = rows[names]
    strings = data[table["offset"] : table["offset"] + table["size"]]
    for row in rows:
        row["name"] = strings[
            row["name_offset"] : strings.index(b"\0", row["name_offset"])
        ].decode()
    return rows


def content(data: bytes, row: dict) -> bytes:
    return (
        b"" if row["type"] == 8 else data[row["offset"] : row["offset"] + row["size"]]
    )


def dynamic(data: bytes, row: dict) -> list[tuple[int, int]]:
    raw = content(data, row)
    return [struct.unpack_from("<qQ", raw, at) for at in range(0, len(raw), 16)]


def run_path(data: bytes, rows: dict) -> str:
    strtab = content(data, rows[".dynstr"])
    for tag, value in dynamic(data, rows[".dynamic"]):
        if tag in (15, 29):
            return strtab[value : strtab.index(b"\0", value)].decode()
    raise SystemExit("the module has no run path")


def module_shas(value, found: set[str]) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "readcon_module_sha256" and isinstance(item, str):
                found.add(item)
            else:
                module_shas(item, found)
    elif isinstance(value, list):
        for item in value:
            module_shas(item, found)


def named_module_shas() -> set[str]:
    """Every ``readcon_module_sha256`` of the records outside ``data/claimcheck``."""
    found: set[str] = set()
    for rel in RECORD_DIRS:
        base = cc.CPC / rel
        for path in sorted(base.rglob("*.json")) if base.is_dir() else []:
            if "claimcheck" not in path.relative_to(cc.CPC).parts:
                module_shas(json.loads(path.read_text()), found)
    return found


def main() -> int:
    current_path = provenance.readcon_module()
    if current_path is None:
        raise SystemExit("the readcon module is not imported")
    current_sha = cc.sha256(current_path)
    earlier_env = os.environ.get("CC_EARLIER_MODULE", "").strip()
    if not earlier_env:
        named: set[str] = set()
        records = sorted(cc.RECORDS.glob("*.json")) if cc.RECORDS.is_dir() else []
        for path in records:
            if path.name == "module_identity.json":
                continue
            module_shas(json.loads(path.read_text()), named)
        if named and named != {current_sha}:
            raise SystemExit(
                f"claim-check records name {sorted(named)}, this module is {current_sha}"
            )
        cc.write_record(
            "module_identity",
            {
                "claim": "The claim-check runs import the readcon module built in this checkout.",
                "current_module_sha256": current_sha,
                "claimcheck_records": [
                    path.name for path in records if path.name != "module_identity.json"
                ],
                "claimcheck_records_name_this_module": (not named)
                or named == {current_sha},
                "module_shas_in_claimcheck_records": sorted(named),
                "earlier_module": None,
            },
        )
        return 0
    earlier_path = Path(earlier_env)
    if not earlier_path.is_file():
        raise SystemExit("CC_EARLIER_MODULE names no file")
    named = named_module_shas()
    earlier, current = earlier_path.read_bytes(), current_path.read_bytes()
    earlier_sha = cc.sha256(earlier_path)
    if named != {earlier_sha}:
        raise SystemExit(
            f"the records name {sorted(named)}, CC_EARLIER_MODULE has {earlier_sha}"
        )

    a, b = sections(earlier), sections(current)
    if [row["name"] for row in a] != [row["name"] for row in b]:
        raise SystemExit("the section tables differ")
    by_a, by_b = {row["name"]: row for row in a}, {row["name"]: row for row in b}
    differing_content = [
        row["name"]
        for row in a
        if content(earlier, row) != content(current, by_b[row["name"]])
    ]
    moved_headers = [
        {
            "section": row["name"],
            "fields": [
                key
                for key in ("addr", "offset", "size")
                if row[key] != by_b[row["name"]][key]
            ],
        }
        for row in a
        if any(row[key] != by_b[row["name"]][key] for key in ("addr", "offset", "size"))
    ]
    dynstr_a = content(earlier, by_a[".dynstr"]).split(b"\0")
    dynstr_b = content(current, by_b[".dynstr"]).split(b"\0")
    path_a, path_b = run_path(earlier, by_a), run_path(current, by_b)
    dynstr_same_but_run_path = [s for s in dynstr_a if s != path_a.encode()] == [
        s for s in dynstr_b if s != path_b.encode()
    ]
    dyn_a, dyn_b = (
        dynamic(earlier, by_a[".dynamic"]),
        dynamic(current, by_b[".dynamic"]),
    )
    dynamic_tags = [
        TAGS.get(ta, hex(ta))
        for (ta, va), (tb, vb) in zip(dyn_a, dyn_b)
        if ta != tb or va != vb
    ]

    diff = [
        i for i in range(min(len(earlier), len(current))) if earlier[i] != current[i]
    ]
    header_bytes = {row["header_at"] + k for row in a for k in range(64)}
    in_sections = {name: 0 for name in by_a}
    outside = 0
    for i in diff:
        if i in header_bytes:
            in_sections.setdefault("section headers", 0)
            in_sections["section headers"] += 1
            continue
        hit = next(
            (
                row["name"]
                for row in a
                if row["type"] != 8 and row["offset"] <= i < row["offset"] + row["size"]
            ),
            None,
        )
        if hit is None:
            outside += 1
        else:
            in_sections[hit] += 1
    suffix = "paper/cpc/.pixi/envs/default/lib"
    cc.write_record(
        "module_identity",
        {
            "claim": "The module of the claim-check runs is the module of the other records but for the run path.",
            "earlier_module_sha256": earlier_sha,
            "earlier_named_by_records": sorted(named),
            "current_module_sha256": current_sha,
            "bytes": [len(earlier), len(current)],
            "differing_bytes": len(diff),
            "differing_bytes_by_place": {
                key: value for key, value in in_sections.items() if value
            },
            "differing_bytes_outside_sections_and_headers": outside,
            "sections": len(a),
            "sections_with_other_content": differing_content,
            "moved_or_resized_sections": moved_headers,
            "run_path_suffix_equal": path_a.endswith(suffix)
            and path_b.endswith(suffix),
            "run_path_suffix": suffix,
            "run_path_prefix_lengths": [
                len(path_a) - len(suffix),
                len(path_b) - len(suffix),
            ],
            "dynstr_same_but_run_path": dynstr_same_but_run_path,
            "dynamic_entries_that_differ": dynamic_tags,
            "moved_sections_same_bytes": all(
                content(earlier, by_a[name]) == content(current, by_b[name])
                for name in (".gnu.version", ".gnu.version_r")
                if name in by_a
            ),
        },
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
