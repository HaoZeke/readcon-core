#!/usr/bin/env python3
"""Break one python error arm, run its test, and restore the file."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from receipt_text import ROOT, clean, tool_path
SOURCE = ROOT / "src" / "python.rs"
OUT = ROOT / "paper" / "cpc" / "receipts" / "python"

MUTATIONS = [
    (
        "missing-positions",
        "a_missing_position_file_and_a_bad_forward_name_the_error",
        "crate::iterators::read_positions_with_threads(Path::new(&path_owned), n_threads)\n                .map_err(|e| e.to_string())",
        "crate::iterators::read_positions_with_threads(Path::new(&path_owned), n_threads)\n                .map_err(|_e| \"wrong\".to_string())",
        "OSError: wrong",
    ),
    (
        "skip-error",
        "a_missing_position_file_and_a_bad_forward_name_the_error",
        'Some(Err(e)) => Err(PyIOError::new_err(format!("skip error: {e}"))),',
        'Some(Err(e)) => Err(PyIOError::new_err(format!("other: {e}"))),',
        "OSError: other:",
    ),
    (
        "velocity-row",
        "every_stored_row_matches_the_shape_that_names_it",
        "            data.push(atom.vx.unwrap_or(0.0));\n            data.push(atom.vy.unwrap_or(0.0));\n            data.push(atom.vz.unwrap_or(0.0));",
        "            data.push(0.0);\n            data.push(atom.vy.unwrap_or(0.0));\n            data.push(atom.vz.unwrap_or(0.0));",
        "0.4",
    ),
    (
        "missing-j",
        "every_stored_row_matches_the_shape_that_names_it",
        'PyValueError::new_err("bonds entry missing j")',
        'PyValueError::new_err("bonds entry missing endpoint")',
        "missing endpoint",
    ),
    (
        "bond-index",
        "a_bond_index_that_is_not_an_integer_is_rejected",
        '.ok_or_else(|| PyValueError::new_err("bonds entry missing i"))?\n                    .extract()?;',
        '.ok_or_else(|| PyValueError::new_err("bonds entry missing i"))?\n                    .extract().map_err(|_| PyValueError::new_err("bond index type"))?;',
        "bond index type",
    ),
    (
        "conbin-source",
        "a_missing_conbin_source_names_the_write_error",
        'crate::conbin::write_from_con(Path::new(&con_path), Path::new(&out_path), binary32)\n                .map_err(|e| e.to_string())',
        'crate::conbin::write_from_con(Path::new(&con_path), Path::new(&out_path), binary32)\n                .map_err(|_e| "conbin write".to_string())',
        "conbin write",
    ),
    (
        "mpi-read",
        "a_missing_mpi_path_names_the_read_and_the_write",
        'crate::distrib_read::read_block(&c, Path::new(path), mode, n_threads)\n        .map_err(|e| PyIOError::new_err(e.to_string()))?',
        'crate::distrib_read::read_block(&c, Path::new(path), mode, n_threads)\n        .map_err(|_e| PyIOError::new_err("mpi read"))?',
        "mpi read",
    ),
    (
        "mpi-write",
        "a_missing_mpi_path_names_the_read_and_the_write",
        'crate::distrib_write::write_block(&c, Path::new(path), frames, mode, precision)\n        .map(|_| ())\n        .map_err(|e| PyIOError::new_err(e.to_string()))',
        'crate::distrib_write::write_block(&c, Path::new(path), frames, mode, precision)\n        .map(|_| ())\n        .map_err(|_e| PyIOError::new_err("mpi write"))',
        "mpi write",
    ),
]


def run_test(name: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PATH"] = tool_path()
    env["CARGO_TARGET_DIR"] = str(ROOT / "target" / "py-mpi")
    env["LIBRARY_PATH"] = "/usr/lib/x86_64-linux-gnu"
    env["RUSTFLAGS"] = "-C link-arg=-lpython3.12"
    env.pop("LLVM_PROFILE_FILE", None)
    env["OMPI_ALLOW_RUN_AS_ROOT"] = "1"
    env["OMPI_ALLOW_RUN_AS_ROOT_CONFIRM"] = "1"
    return subprocess.run(
        [
            "cargo",
            "test",
            "--features",
            "python,mpi",
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
