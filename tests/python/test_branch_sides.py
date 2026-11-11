"""Checked inputs for the sides the coverage export still missed."""

from pathlib import Path

import ase
import numpy as np
import pytest
from ase.constraints import FixAtoms, FixCartesian

readcon = pytest.importorskip("readcon")


def _atoms():
    return ase.Atoms(
        "CuH",
        positions=[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        cell=[4.0, 5.0, 6.0],
        pbc=True,
    )


def test_nonfinite_cell_is_rejected():
    atoms = _atoms()
    atoms.set_cell([[np.nan, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    with pytest.raises(ValueError, match="finite"):
        readcon.ConFrame.from_ase(atoms)


def test_zero_edge_uses_a_right_angle():
    atoms = _atoms()
    atoms.set_cell([[0.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]])
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].symbol == "Cu"
    assert frame[1].symbol == "H"


def test_fix_atoms_and_fix_cartesian_set_masks():
    atoms = _atoms()
    atoms.set_constraint(
        [FixAtoms(indices=[0]), FixCartesian([1], mask=[True, False, True])]
    )
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].is_fixed
    assert list(frame[1].fixed) == [True, False, True]


def test_distinct_tags_become_atom_ids():
    atoms = _atoms()
    atoms.set_tags([3, 4])
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].atom_id == 3
    assert frame[1].atom_id == 4


def test_repeated_tags_do_not_become_ids():
    atoms = _atoms()
    atoms.set_tags([2, 2])
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].atom_id == 0
    assert frame[1].atom_id == 1


def test_bond_list_shapes():
    atoms = _atoms()
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame.bonds == []
    frame.metadata = {"bonds": None}
    assert frame.bonds == []
    frame.metadata = {
        "bonds": [[0, 1], [0, 1, 9], {"i": 0, "j": 1}, {"i": 1, "j": 0, "order": None}],
    }
    assert frame.bonds == [(0, 1), (0, 1), (1, 0)]
    frame.metadata = {"bonds": [{"i": 0, "j": 1, "order": 2}]}
    assert frame.bonds == [(0, 1, 2)]
    assert frame.has_bonds


def test_comm_rejects_compression_and_needs_mpi(tmp_path: Path):
    atoms = _atoms()
    frame = readcon.ConFrame.from_ase(atoms)
    dest = tmp_path / "out.con"
    with pytest.raises(ValueError, match="uncompressed"):
        readcon.write_con(str(dest), [frame], compression="gzip", comm=object())
    with pytest.raises(NotImplementedError, match="mpi"):
        readcon.write_con(str(dest), [frame], compression="none", comm=object())


def test_atom_datum_optional_fields_round_trip(tmp_path: Path):
    atom = readcon.Atom(
        "Cu",
        0.1,
        0.2,
        0.3,
        atom_id=7,
        mass=63.5,
        vx=0.1,
        energy=1.5,
        mx=0.0,
        my=0.0,
        mz=1.0,
        dx=0.01,
        dy=0.0,
        dz=0.0,
        sx=0.02,
        sy=0.0,
        sz=0.0,
    )
    assert not atom.has_velocity
    assert not atom.has_forces
    assert atom.has_displacement
    full = readcon.Atom("H", 1.0, 0.0, 0.0, vx=0.0, vy=0.0, vz=0.1, fx=0.2, fy=0.0, fz=0.0)
    assert full.has_velocity
    assert full.has_forces
    frame = readcon.ConFrame([4.0, 5.0, 6.0], [90.0, 90.0, 90.0], [atom, full])
    frame.metadata = {"potential": {"type": "lj"}, "time": 0.5, "frame_index": 3}
    assert frame.potential_type == "lj"
    assert frame.time == 0.5
    assert frame.frame_index == 3
    text = readcon.write_con_string([frame])
    assert "Cu" in text
    path = tmp_path / "datum.con"
    readcon.write_con(str(path), [frame], compression="none")
    loaded = readcon.read_con(str(path), n_threads=2)
    assert len(loaded) == 1
    assert loaded[0][0].symbol == "Cu"


def test_comm_without_mpi_is_not_implemented(tmp_path: Path):
    frame = readcon.ConFrame.from_ase(_atoms())
    path = tmp_path / "mpi-off.con"
    readcon.write_con(str(path), [frame], compression="none")
    with pytest.raises(NotImplementedError, match="mpi"):
        readcon.read_all_frames(str(path), comm=object())


def test_bad_index_and_empty_metadata():
    frame = readcon.ConFrame.from_ase(_atoms())
    with pytest.raises(IndexError):
        frame[5]
    with pytest.raises(IndexError):
        frame[-3]
    frame.metadata = {"con_spec_version": 2, "sections": [], "note": None}
    assert "note" not in frame.metadata or frame.metadata.get("note") is None
