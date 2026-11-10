"""Optional frame fields, bad indexes, and the write compression choices."""

from pathlib import Path

import ase
import pytest

readcon = pytest.importorskip("readcon")


def test_missing_optional_arrays_and_a_bad_index():
    atoms = ase.Atoms("Cu", positions=[[0.0, 0.0, 0.0]], cell=[4.0, 4.0, 4.0], pbc=True)
    frame = readcon.ConFrame.from_ase(atoms)
    assert not frame.has_velocities
    assert frame[0].symbol == "Cu"
    assert frame[-1].symbol == "Cu"
    with pytest.raises(IndexError):
        frame[3]
    with pytest.raises(IndexError):
        frame[-2]


def test_from_ase_without_info_keys():
    atoms = ase.Atoms("H", positions=[[1.0, 0.0, 0.0]], cell=[3.0, 3.0, 3.0], pbc=True)
    atoms.info.clear()
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].symbol == "H"


def test_write_con_none_gzip_and_a_gz_path(tmp_path: Path):
    atoms = ase.Atoms(
        "CuH",
        positions=[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        cell=[5.0, 5.0, 5.0],
        pbc=True,
    )
    atoms.set_velocities([[0.1, 0.0, 0.0], [0.0, 0.2, 0.0]])
    frame = readcon.ConFrame.from_ase(atoms)
    plain = tmp_path / "plain.con"
    gzip_path = tmp_path / "forced.con"
    suffix = tmp_path / "by-suffix.con.gz"
    readcon.write_con(str(plain), [frame], compression="none")
    readcon.write_con(str(gzip_path), [frame], compression="gzip")
    readcon.write_con(str(suffix), [frame])
    assert plain.is_file()
    assert gzip_path.is_file()
    assert suffix.is_file()
    loaded = readcon.read_con(str(plain), n_threads=2)
    assert loaded[0].has_velocities
    with pytest.raises(OSError):
        readcon.write_con(str(tmp_path / "bad.con"), [frame], compression="zstd")
