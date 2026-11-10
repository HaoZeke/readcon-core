"""Migration convert_to_con: foreign/CON → CON via shipped API."""

from __future__ import annotations

from pathlib import Path

import pytest

import readcon

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "resources" / "test"


def test_convert_to_con_native_multi_frame(tmp_path: Path):
    out = tmp_path / "round.con"
    report = readcon.convert_to_con(str(FIXTURES / "tiny_multi_cuh2.con"), str(out))
    assert report["native_con"] is True
    assert report["n_frames"] == 2
    assert report["n_atoms_last"] == 4
    back = readcon.read_con(str(out))
    assert len(back) == 2
    assert len(back[0]) == 4
    assert back[0].atoms[0].atom_id == 0


def test_convert_to_con_xyz_chemfiles(tmp_path: Path):
    xyz = tmp_path / "water.xyz"
    xyz.write_text(
        "3\nmigrate\nO 0 0 0\nH 0.96 0 0\nH -0.24 0.93 0\n",
        encoding="utf-8",
    )
    out = tmp_path / "water.con"
    if not readcon.has_chemfiles_support():
        with pytest.raises(Exception) as ei:
            readcon.convert_to_con(str(xyz), str(out))
        assert "chemfiles" in str(ei.value).lower()
        assert not out.exists()
        return
    report = readcon.convert_to_con(str(xyz), str(out))
    assert report["native_con"] is False
    assert report["n_frames"] == 1
    assert report["n_atoms_last"] == 3
    frame = readcon.read_first_frame(str(out))
    assert len(frame) == 3
    symbols = [a.symbol for a in frame.atoms]
    assert symbols.count("O") == 1
    assert symbols.count("H") == 2
