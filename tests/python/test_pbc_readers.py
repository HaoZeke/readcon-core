"""Periodicity on a file that is not periodic in every direction.

A missing ``pbc`` key is periodic in all three directions
(``docs/source/spec.rst:698``, CON-cell.3 at
``paper/cpc/src/figures/generated/spec_normative.tex:184``).
A present key that is not three booleans is an error (CON-cell.4 at
``spec_normative.tex:186``, ``src/parser.rs:1108``).
This test writes one partially periodic file, one file with no ``pbc``
key, and one file open in every direction, then compares the Python
reader with a C++ program that opens those files. It also checks that
both readers reject a key that is not three booleans.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest
import readcon

ROOT = Path(__file__).resolve().parents[2]
INCLUDE = ROOT / "include"
ARCHIVE = ROOT / "target" / "release" / "libreadcon_core.a"

PROBE = r"""
#include "readcon-core.hpp"
#include <cstdio>
#include <string>
int main(int argc, char **argv) {
    if (argc < 2)
        return 2;
    if (std::string(argv[1]) == "json") {
        if (argc != 3)
            return 2;
        try {
            auto p = readcon::pbc_from_metadata_json(argv[2]);
            std::printf("%d %d %d\n", (int)p[0], (int)p[1], (int)p[2]);
            return 0;
        } catch (const std::exception &err) {
            std::fprintf(stderr, "%s\n", err.what());
            return 1;
        }
    }
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

    broken = tmp_path / "broken.con"
    text = partial.read_text(encoding="utf-8")
    broken.write_text(text.replace("[false,true,false]", "[1,2,3]"), encoding="utf-8")
    with pytest.raises(Exception, match="pbc must be a length-3 boolean array"):
        readcon.read_con(str(broken))
    for payload in (
        '{"pbc":[1, 2, 3]}',
        '{"pbc":{"x": true}}',
        '{"pbc":[true, false]}',
    ):
        rejected = subprocess.run(
            [str(binary), "json", payload],
            capture_output=True,
            text=True,
        )
        assert rejected.returncode == 1
        assert "length-3 boolean array" in rejected.stderr
