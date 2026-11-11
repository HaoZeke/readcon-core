# Copyright (c) readcon-core developers
"""The extended XYZ writer round-trips the committed fixtures."""

from pathlib import Path

import numpy as np
import pytest

import readcon

FIXTURES = Path(__file__).resolve().parents[2] / "bench" / "h2h" / "fixtures"


def _same(left, right, tol=1e-9):
    assert np.asarray(left, dtype=np.float64) == pytest.approx(
        np.asarray(right, dtype=np.float64), abs=tol
    )


@pytest.mark.parametrize("name", ["small-1.extxyz", "small-n.extxyz"])
def test_write_rereads_with_readcon(tmp_path, name):
    original = readcon.read_extxyz(str(FIXTURES / name))
    out = tmp_path / name
    readcon.write_extxyz(str(out), original)
    again = readcon.read_extxyz(str(out))
    assert again["species"] == original["species"]
    _same(again["positions"], original["positions"])
    _same(again["masses"], original["masses"])
    _same(again["cell"], original["cell"], tol=1e-6)
    _same(again["arrays"]["forces"], original["arrays"]["forces"])


def test_write_rereads_with_ase_and_extxyz(tmp_path):
    pytest.importorskip("ase")
    pytest.importorskip("extxyz")
    from ase.io import read as ase_read

    import extxyz

    original = readcon.read_extxyz(str(FIXTURES / "small-1.extxyz"))
    out = tmp_path / "small-1.extxyz"
    readcon.write_extxyz(str(out), original)
    for fmt in ("extxyz", "cextxyz"):
        image = ase_read(str(out), index=0, format=fmt)
        assert image.get_chemical_symbols() == original["species"][0]
        _same(image.positions, original["positions"][0])
        _same(image.get_forces(), original["arrays"]["forces"][0])
        _same(image.cell.lengths(), original["cell"][0, :3], tol=1e-6)
        _same(image.cell.angles(), original["cell"][0, 3:], tol=1e-6)
    frame = extxyz.read_dicts(str(out))
    frame = frame[0] if isinstance(frame, list) else frame
    assert list(frame.arrays["species"]) == original["species"][0]
    _same(frame.arrays["pos"], original["positions"][0])
    _same(frame.arrays["forces"], original["arrays"]["forces"][0])
    _same(frame.arrays["masses"], original["masses"][0])
