#!/usr/bin/env python3
"""Build and check the C++ readers of readcon-core 0.17.4 and chemfiles for the timing campaign.

``bench/reads.py`` times the C++ cells of the 0.17.4 tables in the same
schedule as the Python cells; this script builds their two programs with the
C++ compiler of the Python environment that runs it (``sys.prefix``, the
``default`` environment of pixi.toml):

* ``readcon_read``   ``native/readcon_read.cpp`` over the C ABI static library
                     of ``READCON_SRC``, built with the ``parallel`` feature;
* ``chemfiles_read`` ``native/chemfiles_read.cpp`` against ``CHEMFILES_PREFIX``
                     (default the environment).

Each cell is one process (``native/protocol.hpp``) pinned with ``taskset`` to
one core or eight: it reads the ten-frame warm-up file of its format untimed,
prints READY, reads the payload on the next line of its input, holds every
frame and record, sums the checksums, frees the frames and prints the seconds
from that line until the frames are freed.  One run of each cell must print
the checksums of its payload's reference in ``payloads.json``: integers
exactly, floats to ``REL_TOL``.

Records (``BENCH_RESULTS``, default ``results/r0174``), under ``cpp/``:
``checks.json`` (the line each cell printed, the reference and the verdict) and
``build.json`` (commands, compiler, library digest, artifact SHA-256), the
second with host paths replaced as in ``portable.py``.

    READCON_SRC=... RC_PAYLOADS=... python cpp_reads.py
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "scripts"))
sys.path.insert(0, str(HERE))
import portable  # noqa: E402
from provenance import library_digest  # noqa: E402

ENV = Path(sys.prefix).resolve()
NATIVE = HERE / "native"
OUT = Path(os.environ.get("BENCH_NATIVE_OUT", HERE / "work" / "native0174"))
RESULTS = Path(os.environ.get("BENCH_RESULTS", HERE / "results" / "r0174")) / "cpp"
PAYLOADS = Path(os.environ.get("RC_PAYLOADS", HERE / "work" / "r0174"))
CXXFLAGS = ["-O3", "-std=c++17", "-DNDEBUG"]


def compiler() -> str:
    """The C++ compiler for the two reader programs.

    ``CXX`` wins when it names a real file. Otherwise the conda compiler in
    this Python environment is used, and otherwise ``g++`` or ``c++`` on
    ``PATH``.
    """
    chosen = os.environ.get("CXX", "").strip()
    if chosen and Path(chosen).is_file():
        return chosen
    bundled = ENV / "bin" / "x86_64-conda-linux-gnu-c++"
    if bundled.is_file():
        return str(bundled)
    found = shutil.which("g++") or shutil.which("c++")
    if not found:
        raise SystemExit("no C++ compiler")
    return found


def chemfiles_source() -> Path:
    """The chemfiles tree shipped inside the chemfiles-sys crate Cargo downloaded."""
    cargo_home = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    found = sorted(
        cargo_home.glob("registry/src/*/chemfiles-sys-*/chemfiles/CMakeLists.txt")
    )
    if not found:
        raise SystemExit(
            "chemfiles sources are not in the cargo registry; build the chemfiles feature once"
        )
    return found[-1].parent


def install_chemfiles(out: Path, log: list, env: dict) -> Path:
    """Build chemfiles into ``out`` and return the prefix that contains chemfiles.hpp."""
    prefix = out / "chemfiles"
    header = prefix / "include" / "chemfiles.hpp"
    if header.is_file() and any((prefix / "lib").glob("libchemfiles.*")):
        return prefix
    source_link = out / "chemfiles-src"
    if source_link.is_symlink() or source_link.exists():
        source_link.unlink()
    source_link.symlink_to(chemfiles_source(), target_is_directory=True)
    build = out / "chemfiles-build"
    cxx = compiler()
    run(
        [
            "cmake",
            "-S",
            str(source_link),
            "-B",
            str(build),
            f"-DCMAKE_INSTALL_PREFIX={prefix}",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=ON",
            f"-DCMAKE_CXX_COMPILER={cxx}",
            f"-DCMAKE_C_COMPILER={shutil.which('gcc') or 'gcc'}",
        ],
        log,
        env=env,
    )
    run(
        [
            "cmake",
            "--build",
            str(build),
            "--target",
            "install",
            "--parallel",
            str(os.cpu_count() or 1),
        ],
        log,
        env=env,
    )
    if not header.is_file():
        raise SystemExit("the chemfiles install did not write chemfiles.hpp")
    lib_dir = prefix / "lib"
    if not lib_dir.is_dir():
        alt = prefix / "lib64"
        if alt.is_dir():
            lib_dir.symlink_to(alt, target_is_directory=True)
    if not any(lib_dir.glob("libchemfiles.*")):
        raise SystemExit("the chemfiles install did not write libchemfiles")
    return prefix
FEATURES = ["parallel"]
REL_TOL = 1e-12
CHECKED = {
    "moc": (
        "frames",
        "atoms",
        "sum_positions",
        "sum_velocities",
        "n_fixed",
        "sum_ids",
        "sum_numbers",
        "sum_cell",
        "n_pbc",
    ),
    "coords": ("frames", "atoms", "sum_positions", "sum_numbers"),
}
# name, program, contract, payload, warm-up file, threads
CELLS = (
    ("readcon_cpp_moc_1", "readcon_read", "moc", "traj.con", "warm.con", 1),
    ("readcon_cpp_moc_8", "readcon_read", "moc", "traj.con", "warm.con", 8),
    ("chemfiles_cpp_moc_1", "chemfiles_read", "moc", "traj.extxyz", "warm.extxyz", 1),
    (
        "readcon_cpp_coords_1",
        "readcon_read",
        "coords",
        "traj_coords.con",
        "warm_coords.con",
        1,
    ),
    ("chemfiles_cpp_coords_1", "chemfiles_read", "coords", "traj.xyz", "warm.xyz", 1),
)
PROGRAMS = {
    "readcon_read": OUT / "readcon_read",
    "chemfiles_read": OUT / "chemfiles_read",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(cmd: list[str], log: list[list[str]], **kw) -> str:
    print("+", " ".join(cmd), flush=True)
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False, **kw)
    if proc.returncode != 0:
        raise SystemExit(f"failed: {' '.join(cmd)}\n{proc.stdout}{proc.stderr}")
    log.append(cmd)
    return proc.stdout + proc.stderr


def environment(threads: int) -> dict:
    knobs = (
        "RAYON_NUM_THREADS",
        "OMP_NUM_THREADS",
        "OPENBLAS_NUM_THREADS",
        "MKL_NUM_THREADS",
    )
    return {**os.environ, **{knob: str(threads) for knob in knobs}}


def argv(cell: tuple) -> list[str]:
    _, program, contract, payload, warm, threads = cell
    cores = "0" if threads == 1 else f"0-{threads - 1}"
    return [
        "taskset",
        "-c",
        cores,
        str(PROGRAMS[program]),
        contract,
        str(PAYLOADS / payload),
        str(PAYLOADS / warm),
        str(threads),
    ]


def disagreements(contract: str, got: dict, want: dict) -> list[str]:
    bad = []
    for key in CHECKED[contract]:
        value = want[key]
        ok = (
            got.get(key) == value
            if isinstance(value, int)
            else abs(got.get(key, float("nan")) - value) <= REL_TOL * abs(value)
        )
        if not ok:
            bad.append(f"{key} {got.get(key)!r} != {value!r}")
    return bad


def check(cell: tuple, references: dict, files: dict) -> dict:
    name, _, contract, payload, _, threads = cell
    proc = subprocess.run(
        argv(cell),
        input="\n",
        capture_output=True,
        text=True,
        check=True,
        env=environment(threads),
    )
    lines = proc.stdout.splitlines()
    got = json.loads(lines[-1])
    want = {
        key: references[files[payload]["payload"]][key] for key in CHECKED[contract]
    }
    return {
        "cell": name,
        "ready": lines[0],
        "printed": got,
        "reference": want,
        "disagreements": disagreements(contract, got, want),
    }


def main() -> None:
    src = Path(os.environ.get("READCON_SRC", "").strip() or HERE.parents[2]).resolve()
    OUT.mkdir(parents=True, exist_ok=True)
    RESULTS.mkdir(parents=True, exist_ok=True)
    log: list[list[str]] = []
    env = {
        **os.environ,
        "PATH": f"{ENV / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}",
    }
    if os.environ.get("CHEMFILES_PREFIX", "").strip():
        prefix = Path(os.environ["CHEMFILES_PREFIX"]).resolve()
    else:
        prefix = install_chemfiles(OUT, log, env)
    cxx = compiler()
    target = OUT / "target-capi"
    run(
        [
            str(ENV / "bin" / "cargo"),
            "build",
            "--release",
            "--lib",
            "--features",
            ",".join(FEATURES),
            "--manifest-path",
            str(src / "Cargo.toml"),
        ],
        log,
        env={**env, "CARGO_TARGET_DIR": str(target)},
    )
    static = target / "release" / "libreadcon_core.a"
    rpath = f"-Wl,-rpath,{ENV / 'lib'}"
    run(
        [
            cxx,
            *CXXFLAGS,
            f"-I{NATIVE}",
            f"-I{src / 'include'}",
            str(NATIVE / "readcon_read.cpp"),
            str(static),
            "-lpthread",
            "-ldl",
            "-lm",
            rpath,
            "-o",
            str(PROGRAMS["readcon_read"]),
        ],
        log,
        env=env,
    )
    run(
        [
            cxx,
            *CXXFLAGS,
            f"-I{NATIVE}",
            f"-I{prefix / 'include'}",
            str(NATIVE / "chemfiles_read.cpp"),
            f"-L{prefix / 'lib'}",
            "-lchemfiles",
            f"-Wl,-rpath,{prefix / 'lib'}",
            rpath,
            "-o",
            str(PROGRAMS["chemfiles_read"]),
        ],
        log,
        env=env,
    )

    manifest = json.loads((RESULTS.parent / "payloads.json").read_text())
    checks = [check(cell, manifest["references"], manifest["files"]) for cell in CELLS]
    (RESULTS / "checks.json").write_text(json.dumps(checks, indent=2) + "\n")
    failed = [c for c in checks if c["disagreements"]]
    if failed:
        raise SystemExit(f"checksums disagree: {json.dumps(failed, indent=2)}")

    chemfiles_version = ""
    for line in (
        (prefix / "include" / "chemfiles" / "config.h").read_text().splitlines()
    ):
        if "CHEMFILES_VERSION " in line:
            chemfiles_version = line.split()[-1].strip('"')
    record = {
        "readcon_src": str(src),
        "library_digest": library_digest(src),
        "cargo_features": FEATURES,
        "rustc": subprocess.run(
            [str(ENV / "bin" / "rustc"), "--version"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip(),
        "cxx": subprocess.run(
            [cxx, "--version"], capture_output=True, text=True, check=True
        ).stdout.splitlines()[0],
        "cxxflags": CXXFLAGS,
        "chemfiles_version": chemfiles_version,
        "cells": [
            dict(
                zip(
                    ("name", "program", "contract", "payload", "warm", "threads"),
                    c,
                    strict=True,
                )
            )
            for c in CELLS
        ],
        "commands": log,
        "sources": {
            p.name: sha256(p)
            for p in (
                NATIVE / "protocol.hpp",
                NATIVE / "readcon_read.cpp",
                NATIVE / "chemfiles_read.cpp",
                Path(__file__),
            )
        },
        "artifacts": {p.name: sha256(p) for p in (*PROGRAMS.values(), static)},
    }
    (RESULTS / "build.json").write_text(
        json.dumps(portable.portable(record), indent=2) + "\n"
    )
    print("wrote", RESULTS)


if __name__ == "__main__":
    main()
