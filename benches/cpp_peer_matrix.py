#!/usr/bin/env python3
"""Build the four compiler cells of the head-to-head C++ campaign.

Each cell is g++ or clang++ at -O2 or -O3, linked to the readcon static
library and a chemfiles 0.10.4 build configured with that compiler and
optimisation. The driver is ``benches/cpp_peer_driver.cpp``.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

CELLS = (
    ("gcc", "-O2", "gcc-o2"),
    ("gcc", "-O3", "gcc-o3"),
    ("clang", "-O2", "clang-o2"),
    ("clang", "-O3", "clang-o3"),
)

SOURCE = (
    Path.home()
    / ".cargo"
    / "registry"
    / "src"
    / "index.crates.io-1949cf8c6b5b557f"
    / "chemfiles-sys-0.10.41"
    / "chemfiles"
)


def compiler_version(compiler: str) -> str:
    out = subprocess.run(
        [compiler, "--version"], check=True, capture_output=True, text=True
    ).stdout
    return out.splitlines()[0].strip()


def ensure_chemfiles(compiler: str, opt: str, source: Path, build: Path) -> Path:
    """Configure and install chemfiles when this compiler and opt have no library yet."""
    if not source.is_dir():
        source.parent.mkdir(parents=True, exist_ok=True)
        if source.exists() or source.is_symlink():
            source.unlink()
        source.symlink_to(SOURCE, target_is_directory=True)
    lib = build / "lib" / "libchemfiles.so"
    if lib.is_file():
        return build
    build.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    clang = Path(compiler).name.startswith("clang")
    if clang:
        libstd = "/usr/lib/gcc/x86_64-linux-gnu/13"
        env["LIBRARY_PATH"] = libstd + os.pathsep + env.get("LIBRARY_PATH", "")
    flags = f"{opt} -DNDEBUG"
    if clang:
        flags += " -stdlib=libstdc++ -I/usr/include/c++/13 -I/usr/include/x86_64-linux-gnu/c++/13"
    subprocess.run(
        [
            "cmake",
            "-S",
            str(source),
            "-B",
            str(build / "build"),
            f"-DCMAKE_INSTALL_PREFIX={build}",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=ON",
            f"-DCMAKE_CXX_COMPILER={compiler}",
            f"-DCMAKE_CXX_FLAGS_RELEASE={flags}",
        ],
        check=True,
        env=env,
    )
    subprocess.run(
        ["cmake", "--build", str(build / "build"), "--target", "install", "-j", str(os.cpu_count() or 2)],
        check=True,
        env=env,
    )
    return build


def build_cell(
    name: str,
    compiler: str,
    opt: str,
    source: Path,
    chem_build: Path,
    library_dir: Path,
    out_dir: Path,
    gcc_major: str,
) -> tuple[Path, Path]:
    del name, gcc_major
    prefix = ensure_chemfiles(compiler, opt, source, chem_build)
    binary = out_dir / f"peer-{compiler.split('/')[-1]}-{opt.lstrip('-')}"
    static = library_dir / "libreadcon_core.a"
    if not static.is_file():
        static = library_dir / "deps" / "libreadcon_core.a"
    driver = Path(__file__).resolve().parent / "cpp_peer_driver.cpp"
    include = Path(__file__).resolve().parents[1] / "include"
    subprocess.run(
        [
            compiler,
            opt,
            "-std=c++17",
            "-DNDEBUG",
            *(
                [
                    "-stdlib=libstdc++",
                    "-I/usr/include/c++/13",
                    "-I/usr/include/x86_64-linux-gnu/c++/13",
                ]
                if Path(compiler).name.startswith("clang")
                else []
            ),
            f"-I{include}",
            f"-I{prefix / 'include'}",
            str(driver),
            str(static),
            f"-L{prefix / 'lib'}",
            "-lchemfiles",
            f"-Wl,-rpath,{prefix / 'lib'}",
            "-lpthread",
            "-ldl",
            "-lm",
            *(
                ["-L/usr/lib/gcc/x86_64-linux-gnu/13"]
                if Path(compiler).name.startswith("clang")
                else []
            ),
            "-o",
            str(binary),
        ],
        check=True,
    )
    return binary, prefix
