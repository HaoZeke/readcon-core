#!/usr/bin/env python3
"""The gzip and zstd envelopes of a CON file, made by the gzip and zstd programs and by hand from RFC 1952
and RFC 8878, read by readcon-core.

The paper says a gzip or zstd envelope wraps the CON text, that a reader recognises it by content, that
every gzip member is read in order, and that a build without the ``zstd`` feature refuses zstd input.
RFC 1952 makes a gzip file a series of members, each of which can carry the optional fields FEXTRA, FNAME,
FCOMMENT and FHCRC.  RFC 8878 makes zstd data one or more frames, each a Zstandard frame or a skippable
frame, and ``pzstd``, the parallel compressor of the zstd sources, writes a skippable frame before each
Zstandard frame.  Each envelope below wraps the same frames of ``resources/test/tiny_multi_cuh2.con``,
written again by readcon-core, and is read

- by the Python module (``read_con``, ``read_arrays``, ``count_frames`` and ``iter_con``), whose wheel build
  has the ``zstd`` feature, against the frames of the plain file;
- by ``read_all_frames`` and ``open_decoded`` of the library in its default build and with ``zstd``.

``pzstd`` is built from the zstd release whose ``zstd`` program the pixi environment holds, pinned by the
SHA-256 the release publishes.  The record is ``data/claimcheck/envelopes.json``.
"""

from __future__ import annotations

import gzip
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import zlib
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cc_common as cc  # noqa: E402

os.environ.setdefault("BENCH_WORK", str(cc.WORK / "rust"))
import rust_crate  # noqa: E402

INPUT = "resources/test/tiny_multi_cuh2.con"
SKIPPABLE = 0x184D2A50
FTEXT, FHCRC, FEXTRA, FNAME, FCOMMENT = 1, 2, 4, 8, 16


def zstd_version() -> str:
    text = subprocess.run(
        ["zstd", "--version"], capture_output=True, text=True, check=True
    ).stdout
    return re.search(r"v(\d+\.\d+\.\d+)", text).group(1)


def member(
    data: bytes,
    flags: int = 0,
    name: bytes = b"",
    comment: bytes = b"",
    extra: bytes = b"",
) -> bytes:
    """One gzip member of ``data`` as RFC 1952, section 2.3, lays it out, with the optional fields of
    ``flags``."""
    header = bytes([0x1F, 0x8B, 8, flags]) + struct.pack("<I", 0) + bytes([0, 255])
    if flags & FEXTRA:
        header += struct.pack("<H", len(extra)) + extra
    if flags & FNAME:
        header += name + b"\0"
    if flags & FCOMMENT:
        header += comment + b"\0"
    if flags & FHCRC:
        header += struct.pack("<H", zlib.crc32(header) & 0xFFFF)
    deflate = zlib.compressobj(6, zlib.DEFLATED, -15)
    body = deflate.compress(data) + deflate.flush()
    return header + body + struct.pack("<II", zlib.crc32(data), len(data) & 0xFFFFFFFF)


def skippable(payload: bytes, nibble: int = 0) -> bytes:
    """A skippable frame of RFC 8878, section 3.1.2."""
    return struct.pack("<II", SKIPPABLE + nibble, len(payload)) + payload


def program(cmd: list, stdin: Path | None = None) -> bytes:
    with stdin.open("rb") if stdin else open(os.devnull, "rb") as handle:
        proc = subprocess.run(
            [str(c) for c in cmd], stdin=handle, capture_output=True, check=False
        )
    print(
        "+",
        " ".join(str(c) for c in cmd),
        "<" if stdin else "",
        stdin.name if stdin else "",
        f"-> exit {proc.returncode}",
        flush=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"{cmd[0]} exited {proc.returncode}: {proc.stderr[-2000:]!r}")
    return proc.stdout


def build_pzstd(version: str) -> tuple[Path | None, dict]:
    base = f"https://github.com/facebook/zstd/releases/download/v{version}/zstd-{version}.tar.gz"
    published, got_sum = cc.fetch(base + ".sha256", f"zstd-{version}.tar.gz.sha256")
    pin = published.read_text().split()[0]
    tarball, got = cc.fetch(base, f"zstd-{version}.tar.gz", sha=pin)
    tree = cc.WORK / "pzstd"
    if tree.exists():
        shutil.rmtree(tree)
    tree.mkdir(parents=True)
    with tarfile.open(tarball) as archive:
        archive.extractall(tree, filter="data")
    source = tree / f"zstd-{version}"
    done = cc.run(
        ["make", "-C", source / "contrib" / "pzstd", f"-j{os.cpu_count()}", "pzstd"],
        check=False,
    )
    binary = source / "contrib" / "pzstd" / "pzstd"
    record = {
        "release": f"zstd {version}",
        "downloads": [got_sum, got],
        "build": cc.brief(done),
    }
    if done["exit"] != 0 or not binary.is_file():
        return None, record
    record["pzstd_sha256"] = cc.sha256(binary)
    shown = subprocess.run(
        [binary, "--version"], capture_output=True, text=True, check=False
    )
    record["pzstd_version"] = (shown.stdout + shown.stderr).strip() or None
    return binary, record


def python_reads(readcon, path: Path, plain) -> dict:
    out = {}
    try:
        frames = readcon.read_con(str(path))
        same = len(frames) == len(plain) and all(
            np.array_equal(a.coords_array(), b.coords_array())
            for a, b in zip(frames, plain)
        )
        out["read_con"] = {"frames": len(frames), "same_as_plain": same}
    except Exception as error:  # noqa: BLE001
        out["read_con"] = {"error": f"{type(error).__name__}: {error}"}
    try:
        positions = np.asarray(readcon.read_positions(str(path), 1))
        want = np.stack([f.coords_array() for f in plain])
        out["read_positions"] = {
            "frames": int(positions.shape[0]),
            "same_as_plain": bool(np.array_equal(positions, want)),
        }
    except Exception as error:  # noqa: BLE001
        out["read_arrays"] = {"error": f"{type(error).__name__}: {error}"}
    try:
        out["count_frames"] = {"frames": int(readcon.count_frames(str(path)))}
    except Exception as error:  # noqa: BLE001
        out["count_frames"] = {"error": f"{type(error).__name__}: {error}"}
    for label, kwargs in (("iter_con", {}), ("iter_con_stream", {"stream": True})):
        try:
            frames = list(readcon.iter_con(str(path), **kwargs))
            same = len(frames) == len(plain) and all(
                np.array_equal(a.coords_array(), b.coords_array())
                for a, b in zip(frames, plain)
            )
            out[label] = {"frames": len(frames), "same_as_plain": same}
        except Exception as error:  # noqa: BLE001
            out[label] = {"error": f"{type(error).__name__}: {error}"}
    return out


def main() -> int:
    import readcon

    work = cc.WORK / "envelopes"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    src = cc.readcon_src()
    plain_frames = readcon.read_con(str(src / INPUT))
    texts = [readcon.write_con_string([frame]).encode() for frame in plain_frames]
    (work / "plain.con").write_bytes(b"".join(texts))
    for i, text in enumerate(texts):
        (work / f"frame{i}.con").write_bytes(text)
    frame_files = [work / f"frame{i}.con" for i in range(len(texts))]
    plain = readcon.read_con(str(work / "plain.con"))

    version = zstd_version()
    pzstd, pzstd_record = build_pzstd(version)
    envelopes = {
        "plain.con": (work / "plain.con").read_bytes(),
        "plain-named.con.gz": (work / "plain.con").read_bytes(),
        "gzip-program.con.gz": program(["gzip", "-c", work / "plain.con"]),
        "gzip-program-n.con.gz": program(["gzip", "-c", "-n", work / "plain.con"]),
        "gzip-members.con.gz": b"".join(
            program(["gzip", "-c", f]) for f in frame_files
        ),
        "gzip-fields.con.gz": member(
            texts[0],
            FTEXT | FHCRC | FEXTRA | FNAME | FCOMMENT,
            b"frame0.con",
            b"claimcheck",
            b"RC\x04\x00abcd",
        )
        + b"".join(member(t) for t in texts[1:]),
        "gzip-empty-member.con.gz": member(texts[0])
        + gzip.compress(b"", mtime=0)
        + b"".join(member(t) for t in texts[1:]),
        "gzip-content.con": program(["gzip", "-c", "-n", work / "plain.con"]),
        "zstd-program.con.zst": program(["zstd", "-q", "-c", work / "plain.con"]),
        "zstd-stdin.con.zst": program(["zstd", "-q", "-c"], stdin=work / "plain.con"),
        "zstd-frames.con.zst": b"".join(
            program(["zstd", "-q", "-c", f]) for f in frame_files
        ),
        "zstd-skippable-between.con.zst": program(["zstd", "-q", "-c", frame_files[0]])
        + skippable(b"index")
        + b"".join(program(["zstd", "-q", "-c", f]) for f in frame_files[1:]),
        "zstd-skippable-first.con.zst": skippable(b"index")
        + b"".join(program(["zstd", "-q", "-c", f]) for f in frame_files),
        "zstd-content.con": program(["zstd", "-q", "-c", work / "plain.con"]),
    }
    if pzstd:
        envelopes["pzstd.con.zst"] = program(
            [pzstd, "-q", "-p", "2", "-c", work / "plain.con"]
        )
    for name, data in envelopes.items():
        (work / name).write_bytes(data)
    files = [work / name for name in envelopes]

    builds = {}
    for label, features in (("default", []), ("zstd", ["zstd"])):
        built = rust_crate.build(
            f"envelope_read_{label}", HERE / "rust" / "envelope_read.rs", features
        )
        rows = [
            json.loads(line)
            for line in cc.run([built["path"], *files])["stdout"].splitlines()
            if line.startswith("{")
        ]
        builds[label] = {
            "build": built["record"],
            "reads": {Path(row.pop("file")).name: row for row in rows},
        }

    rows = {}
    for path in files:
        data = path.read_bytes()
        rows[path.name] = {
            "bytes": len(data),
            "first_bytes": data[:4].hex(),
            "python": python_reads(readcon, path, plain),
            "library_default": builds["default"]["reads"][path.name],
            "library_zstd": builds["zstd"]["reads"][path.name],
        }
    cc.write_record(
        "envelopes",
        {
            "claim": "A gzip or zstd envelope wraps the CON text and a reader recognises it by content; every gzip "
            "member is read in order; a build without zstd refuses zstd input.",
            "input": INPUT,
            "frames": len(plain),
            "zstd_program": version,
            "gzip_program": subprocess.run(
                ["gzip", "--version"], capture_output=True, text=True, check=True
            ).stdout.splitlines()[0],
            "pzstd": pzstd_record,
            "builds": {label: value["build"] for label, value in builds.items()},
            "envelopes": rows,
        },
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
