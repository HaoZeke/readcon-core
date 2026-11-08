#!/usr/bin/env python3
"""The checksum of the binary image against CRC-32C as RFC 3720 defines it.

CRC-32C is CRC-32/ISCSI of the reveng catalogue: the reflected Castagnoli
polynomial 0x82F63B78, the register set to 0xFFFFFFFF before the first byte
and complemented after the last. This script

1. checks an independent CRC-32C written here against the five vectors of
   RFC 3720, Appendix B.4, and the check value of ``123456789``;
2. searches the library sources for ``crc32c_update``, ``crc32c_sse`` and
   ``crc32c_table``, the functions of the older image;
3. writes a CON-bin image with ``write_from_con`` and compares bytes 32 to
   40 of its header with the payload offset and with CRC-32C of the payload;
4. flips one payload byte and reads the image again.

The record is ``data/claimcheck/crc32c.json``.
"""

from __future__ import annotations

import json
import os
import shutil
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cc_common as cc

os.environ.setdefault("BENCH_WORK", str(cc.WORK / "rust"))
import rust_crate

POLY = 0x82F63B78
TABLE = []
for _i in range(256):
    _c = _i
    for _ in range(8):
        _c = (_c >> 1) ^ POLY if _c & 1 else _c >> 1
    TABLE.append(_c)

PDU = bytes.fromhex(
    "01c00000"
    "00000000"
    "00000000"
    "00000000"
    "14000000"
    "00000400"
    "00000014"
    "00000018"
    "28000000"
    "00000000"
    "02000000"
    "00000000"
)
VECTORS = {
    "rfc3720_zeros": (bytes(32), 0x8A9136AA),
    "rfc3720_ones": (b"\xff" * 32, 0x62A8AB43),
    "rfc3720_incrementing": (bytes(range(32)), 0x46DD794E),
    "rfc3720_decrementing": (bytes(range(31, -1, -1)), 0x113FDB5C),
    "rfc3720_read10_pdu": (PDU, 0xD9963A56),
    "check_123456789": (b"123456789", 0xE3069283),
}
ITEMS = ("fn crc32c_update", "fn crc32c_sse", "unsafe fn crc32c_sse", "fn crc32c_table")
INPUT = "resources/test/tiny_cuh2.con"


def crc32c(data: bytes, init: int = 0xFFFFFFFF, xorout: int = 0xFFFFFFFF) -> int:
    c = init
    for b in data:
        c = TABLE[(c ^ b) & 0xFF] ^ (c >> 8)
    return c ^ xorout


def source_hits(src: Path) -> dict:
    hits = {item: [] for item in ITEMS}
    for path in sorted((src / "src").rglob("*.rs")):
        text = path.read_text()
        rel = str(path.relative_to(src))
        for item in ITEMS:
            count = sum(1 for line in text.splitlines() if line.startswith(item))
            if count:
                hits[item].append({"file": rel, "lines": count})
    return hits


def header_layout(src: Path) -> dict:
    text = (src / "src" / "conbin.rs").read_text()
    return {
        "file": "src/conbin.rs",
        "magic": "CONBIN01" if 'b"CONBIN01"' in text else None,
        "payload_offset_at_bytes_32": "out[32..40].copy_from_slice(&payload_off.to_le_bytes())"
        in text,
        "names_crc32c": "crc32c" in text.lower(),
    }


def main() -> int:
    vectors = {name: crc32c(data) == want for name, (data, want) in VECTORS.items()}
    if not all(vectors.values()):
        raise SystemExit(f"the independent CRC-32C misses {vectors}")
    src = cc.readcon_src()
    hits = source_hits(src)
    layout = header_layout(src)
    work = cc.WORK / "crc32c"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    built = rust_crate.build("conbin_crc", HERE / "rust" / "conbin_crc.rs", [])
    con = src / INPUT
    image = work / "tiny.conbin"
    written = json.loads(
        cc.run([built["path"], "write", con, image])["stdout"].strip().splitlines()[-1]
    )
    raw = image.read_bytes()
    payload_off = struct.unpack_from("<Q", raw, 32)[0]
    payload = raw[payload_off:]
    stored = struct.unpack_from("<I", raw, 32)[0]
    flipped = bytearray(raw)
    # The high byte of the first little-endian binary64, so the change is visible in the sum.
    flipped[payload_off + 7] ^= 0x01
    bad = work / "tiny-flipped.conbin"
    bad.write_bytes(flipped)
    reread = json.loads(
        cc.run([built["path"], "read", bad])["stdout"].strip().splitlines()[-1]
    )
    qemu = os.environ.get("CC_QEMU") or shutil.which("qemu-x86_64")
    cc.write_record(
        "crc32c",
        {
            "claim": "The CON-bin header stores a payload offset, not a CRC-32C of the payload.",
            "independent_crc32c_matches_rfc3720": vectors,
            "source_functions": hits,
            "header": layout,
            "build": built["record"],
            "image": {
                "input": INPUT,
                "bytes": len(raw),
                "write": written,
                "payload_offset": payload_off,
                "payload_offset_matches_write": payload_off == written["payload_off"],
                "bytes_32_are_crc32c_of_payload": stored == crc32c(payload),
                "bytes_32_are_unconditioned_crc": stored == crc32c(payload, 0, 0),
                "flipped_payload_still_reads": reread.get("ok") is True,
                "flipped_position_sum_changed": reread.get("position_sum")
                != written["position_sum"],
            },
            "qemu": {
                "present": bool(qemu),
                "ran": False,
                "reason": "the library sources have no crc32c_sse function to emulate",
            },
        },
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
