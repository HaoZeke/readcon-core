"""Periodicity on a file that is not periodic on every axis.

The Rust frame stores ``pbc`` in metadata. ``FrameHeader::pbc_effective``
uses that array, and a missing key is periodic on every axis. The C++
header reads the same JSON through ``ConFrame::pbc``. This test writes
one partially periodic file and one file with no ``pbc`` key, then
compares the Python reader with a C++ program that opens those files.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import readcon

ROOT = Path(__file__).resolve().parents[2]
INCLUDE = ROOT / "include"
ARCHIVE = ROOT / "target" / "release" / "libreadcon_core.a"

PROBE = r"""
#include "readcon-core.hpp"
#include <cstdio>
int main(int argc, char **argv) {
    if (argc < 2)
        return 2;
    for (int i = 1; i < argc; ++i) {
        auto frames = readcon::read_all_frames(argv[i], 1);
        if (frames.empty())
            return 3;
        auto p = frames[0].pbc();
        std::printf("%d %d %d\n", (int)p[0], (int)p[1], (int)p[2]);
    }
    return 0;
}
"""


def _frame(flags):
    atom = readcon.Atom(symbol="Cu", x=0.1, y=0.2, z=0.3, atom_id=1)
    frame = readcon.ConFrame(cell=[3.0, 4.0, 5.0], angles=[90.0, 90.0, 90.0], atoms=[atom])
    if flags is not None:
        frame.metadata = {"pbc": flags}
    return frame


def _python_flags(path: Path):
    frame = readcon.read_con(str(path))[0]
    return [bool(bit) for bit in frame.to_ase().pbc]


def test_partial_and_absent_pbc_agree(tmp_path):
    partial = tmp_path / "partial.con"
    absent = tmp_path / "absent.con"
    closed = tmp_path / "closed.con"
    readcon.write_con(str(partial), [_frame([False, True, False])])
    readcon.write_con(str(absent), [_frame(None)])
    readcon.write_con(str(closed), [_frame([False, False, False])])

    assert _python_flags(partial) == [False, True, False]
    assert _python_flags(absent) == [True, True, True]
    assert _python_flags(closed) == [False, False, False]
    assert readcon.read_con(str(absent))[0].metadata.get("pbc") is None

    source = tmp_path / "pbc_probe.cpp"
    source.write_text(PROBE)
    binary = tmp_path / "pbc_probe"
    subprocess.run(
        [
            "g++",
            "-std=c++17",
            f"-I{INCLUDE}",
            str(source),
            str(ARCHIVE),
            "-lmpi",
            "-lpthread",
            "-ldl",
            "-lm",
            "-o",
            str(binary),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    printed = subprocess.run(
        [str(binary), str(partial), str(absent), str(closed)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    assert printed == ["0 1 0", "1 1 1", "0 0 0"]
