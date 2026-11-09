# Copyright (c) readcon developers
"""Search a tree, unpacking wheels, zips, and tars, for forbidden path prefixes.

The needles are built at runtime so this file does not contain them.
"""

from __future__ import annotations

import argparse
import io
import tarfile
import zipfile
from pathlib import Path


def needles() -> list[bytes]:
    slash = chr(47).encode()
    return [slash + name.encode() for name in ("home", "cursor", "opt", "agent")]


def walk(name: str, data: bytes):
    lower = name.lower()
    if lower.endswith(".whl") or lower.endswith(".zip"):
        try:
            archive = zipfile.ZipFile(io.BytesIO(data))
        except zipfile.BadZipFile:
            yield name, data
            return
        for info in archive.infolist():
            if info.is_dir():
                continue
            inner = archive.read(info)
            yield from walk(f"{name}:{info.filename}", inner)
        return
    if lower.endswith(".tar") or lower.endswith(".tar.gz") or lower.endswith(".tgz") or lower.endswith(".tar.xz") or lower.endswith(".tar.bz2"):
        try:
            archive = tarfile.open(fileobj=io.BytesIO(data), mode="r:*")
        except tarfile.TarError:
            yield name, data
            return
        for member in archive.getmembers():
            if not member.isfile():
                continue
            extracted = archive.extractfile(member)
            if extracted is None:
                continue
            yield from walk(f"{name}:{member.name}", extracted.read())
        return
    yield name, data


def scan(root: Path) -> list[str]:
    hits = []
    patterns = needles()
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        if path.name in {"path_check.py", "scrub_paths.py"}:
            continue
        data = path.read_bytes()
        for name, payload in walk(str(path.relative_to(root)), data):
            for pattern in patterns:
                count = payload.count(pattern)
                if count:
                    hits.append(f"{name} {pattern.decode()} {count}")
    return hits


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    hits = scan(args.root.resolve())
    if hits:
        print("\n".join(hits))
        return 1
    print("path check clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
