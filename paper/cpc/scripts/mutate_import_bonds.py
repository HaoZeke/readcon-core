#!/usr/bin/env python3
"""Break one import path, run its test, and restore the file."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from receipt_text import ROOT, clean, tool_path
SOURCE = ROOT / "src" / "chemfiles_import_imp.rs"
OUT = ROOT / "paper" / "cpc" / "receipts" / "import"

MUTATIONS = [
    (
        "bond-index",
        "guess_bonds_adds_water_topology",
        "let i = unsafe { *id_to_data_idx.get_unchecked(b.i as usize) };",
        "let i = 1;",
        "left == right",
    ),
    (
        "origin-z",
        "imports_xyz_from_temp_file_via_chemfiles_io",
        'a.symbol.as_ref() == "O" && a.x.abs() < 1e-6 && a.y.abs() < 1e-6 && a.z.abs() < 1e-6',
        'a.symbol.as_ref() == "O" && a.x.abs() < 1e-6 && a.y.abs() < 1e-6 && a.z.abs() < -1.0',
        "expected O at the origin",
    ),
]


def run_test(name: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PATH"] = tool_path()
    env["CARGO_TARGET_DIR"] = str(ROOT / "target" / "chfl2")
    env["CC"] = "/usr/bin/gcc"
    env["CXX"] = "/usr/bin/g++"
    env["LIBRARY_PATH"] = "/usr/lib/gcc/x86_64-linux-gnu/14:/usr/lib/x86_64-linux-gnu"
    env.pop("RUSTFLAGS", None)
    env.pop("LLVM_PROFILE_FILE", None)
    return subprocess.run(
        [
            "cargo",
            "test",
            "--features",
            "chemfiles-from-sources",
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
