#!/usr/bin/env python3
"""Break one distrib_read branch, run its test, and restore the file.

Each receipt is the broken replacement, the raw test output, and a
check that the source bytes match the copy taken before the edit.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from receipt_text import ROOT, clean, tool_path, versions
SOURCE = ROOT / "src" / "distrib_read.rs"
OUT = ROOT / "paper" / "cpc" / "receipts" / "distrib_read"

# (name, test filter, old, new, needle that must appear in the failure)
MUTATIONS = [
    (
        "span-length",
        "one_rank_reads_each_mode_and_the_export",
        "if !matches!(next.forward_fast(), Some(Ok(()))) || next.byte_pos() != span {",
        "if !matches!(next.forward_fast(), Some(Ok(()))) {",
        "a shorter frame in the next span is not one tile",
    ),
    (
        "lock-interrupted",
        "one_rank_reads_each_mode_and_the_export",
        """        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return false;
        }""",
        """        if true {
            return false;
        }""",
        "an alarm while waiting for the lock is retried",
    ),
    (
        "null-frame-count",
        "a_null_frame_count_must_fail_the_c_reader",
        '"a null frame count must fail the C reader"',
        '"a null frame count was accepted"',
        "a null frame count must fail the C reader",
    ),
    (
        "block-past-2gib",
        "one_rank_cannot_take_a_block_over_2_gib",
        'Err("one rank cannot take a block over 2 GiB".into())',
        'Err("the oversized block was accepted".into())',
        "one rank cannot take a block over 2 GiB",
    ),
    (
        "point-to-point",
        "a_large_file_uses_point_to_point",
        """    if data_len > i32::MAX as u64 {
        Ok(())
    } else {
        Ok(())
    }""",
        """    if data_len > i32::MAX as u64 {
        Err("point-to-point send disabled".into())
    } else {
        Ok(())
    }""",
        "point-to-point send disabled",
    ),
    (
        "empty-library-path",
        "two_ranks_read_and_write_every_mode",
        "                            if !built.is_empty() {",
        "                            if false && !built.is_empty() {",
        "a seeded library path keeps a separator",
    ),
    (
        "profile-flag",
        "two_ranks_read_and_write_every_mode",
        '                if std::env::var_os("LLVM_PROFILE_FILE").is_some() {',
        '                if false && std::env::var_os("LLVM_PROFILE_FILE").is_some() {',
        "the profile flag follows the variable",
    ),
]


def run_test(name: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PATH"] = tool_path()
    env["CARGO_TARGET_DIR"] = str(ROOT / "target" / "lib-tests")
    env.pop("RUSTFLAGS", None)
    env.pop("LLVM_PROFILE_FILE", None)
    env["OMPI_ALLOW_RUN_AS_ROOT"] = "1"
    env["OMPI_ALLOW_RUN_AS_ROOT_CONFIRM"] = "1"
    env["LIBRARY_PATH"] = "/usr/lib/x86_64-linux-gnu"
    return subprocess.run(
        [
            "cargo",
            "test",
            "--features",
            "mpi",
            "--lib",
            "--",
            "--test-threads=1",
            name,
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


def main() -> int:
    original = SOURCE.read_text()
    OUT.mkdir(parents=True, exist_ok=True)
    failed = 0
    for name, test, old, new, needle in MUTATIONS:
        if original.count(old) != 1:
            print(f"{name}: the broken side is not unique ({original.count(old)})", flush=True)
            failed += 1
            continue
        SOURCE.write_text(original.replace(old, new, 1))
        try:
            result = run_test(test)
        finally:
            SOURCE.write_text(original)
        restored = SOURCE.read_text() == original
        body = (
            f"broken-side: {name}\n"
            f"test: {test}\n"
            f"needle: {needle}\n"
            f"exit: {result.returncode}\n"
            f"restored: {restored}\n"
            "----- test output -----\n"
            f"{result.stdout}"
            f"{versions()}"
        )
        (OUT / f"{name}.txt").write_text(clean(body))
        ok = result.returncode != 0 and needle in result.stdout and restored
        print(f"{name}: exit {result.returncode} restored {restored} ok {ok}", flush=True)
        if not ok:
            failed += 1
    print("RESTORED", SOURCE.read_text() == original, flush=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
