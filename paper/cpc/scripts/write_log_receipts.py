#!/usr/bin/env python3
"""Regenerate the chemfiles and zstd receipts from the current tests."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from receipt_text import ROOT, clean, tool_path

OUT = ROOT / "paper" / "cpc" / "receipts"


def run(features: list[str], test: str, target: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PATH"] = tool_path()
    env["CARGO_TARGET_DIR"] = str(ROOT / "target" / target)
    env["CC"] = "/usr/bin/gcc"
    env["CXX"] = "/usr/bin/g++"
    env["LIBRARY_PATH"] = "/usr/lib/gcc/x86_64-linux-gnu/14:/usr/lib/x86_64-linux-gnu"
    env.pop("RUSTFLAGS", None)
    env.pop("LLVM_PROFILE_FILE", None)
    cmd = ["cargo", "test", "--lib", "--", "--test-threads=1", test]
    if features:
        cmd[2:2] = ["--features", ",".join(features)]
    return subprocess.run(
        cmd,
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


def write(name: str, result: subprocess.CompletedProcess[str]) -> None:
    path = OUT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    body = clean(
        f"test-log: {name}\nexit: {result.returncode}\n----- test output -----\n{result.stdout}"
    )
    path.write_text(body)
    print(f"{name}: exit {result.returncode}", flush=True)


def main() -> int:
    empty = run(
        ["chemfiles-from-sources"],
        "an_empty_chemfiles_frame_is_a_zero_atom_frame",
        "chfl-receipt",
    )
    write("empty-chemfiles-frame-pass.txt", empty)
    write("empty-chemfiles-frame-abort.txt", empty)
    off = run([], "writers_dlpack_ingest_and_selection_edges", "zstd-off")
    write("zstd-writer-null-fail.txt", off)
    write("zstd-writer-error-pass.txt", off)
    on = run(["zstd"], "writers_dlpack_ingest_and_selection_edges", "zstd-on")
    write("zstd-writer-roundtrip-pass.txt", on)
    return 0


if __name__ == "__main__":
    sys.exit(main())
