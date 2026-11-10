#!/usr/bin/env python3
"""Pack the measurement records into records.tar.xz and records.sha256.

    python3 paper/cpc/scripts/pack_records.py --check
    python3 paper/cpc/scripts/pack_records.py --write

--check compares the archive to records.sha256.
--write rebuilds the archive from the loose files named by the current
archive, with a fixed timestamp, then rewrites the sidecar.
"""

from __future__ import annotations

import hashlib
import io
import sys
import tarfile
from pathlib import Path

CPC = Path(__file__).resolve().parents[1]
ARCHIVE = CPC / "records.tar.xz"
SIDE = CPC / "records.sha256"
# 8 November 2026 00:00:00 UTC, the same epoch regen_paper.py uses.
EPOCH = 1794096000


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check() -> int:
    recorded = SIDE.read_text().split()[0]
    actual = digest(ARCHIVE)
    if recorded != actual:
        print(f"records.sha256 {recorded} archive {actual}", file=sys.stderr)
        return 1
    print(f"records.tar.xz {actual}")
    return 0


def write() -> int:
    raw = ARCHIVE.read_bytes()
    src = tarfile.open(fileobj=io.BytesIO(raw), mode="r:xz")
    out = io.BytesIO()
    dest = tarfile.open(fileobj=out, mode="w:xz")
    for member in src.getmembers():
        payload = src.extractfile(member)
        data = payload.read() if payload is not None else b""
        rel = member.name[2:] if member.name.startswith("./") else member.name
        loose = CPC / rel
        if loose.is_file():
            data = loose.read_bytes()
        info = tarfile.TarInfo(member.name)
        info.size = len(data)
        info.mtime = EPOCH
        info.uid = 0
        info.gid = 0
        info.mode = member.mode
        info.type = member.type
        dest.addfile(info, io.BytesIO(data))
    dest.close()
    src.close()
    ARCHIVE.write_bytes(out.getvalue())
    text = f"{digest(ARCHIVE)}  records.tar.xz\n"
    SIDE.write_text(text)
    print(text.strip())
    return 0


def main() -> int:
    if "--write" in sys.argv:
        return write()
    return check()


if __name__ == "__main__":
    raise SystemExit(main())
