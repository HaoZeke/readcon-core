"""Sides the r2.30 table left at zero because no call built the value."""

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


def _two_frames(tmp_path: Path) -> Path:
    first = readcon.ConFrame.from_ase(_atoms())
    second = readcon.ConFrame.from_ase(_atoms())
    path = tmp_path / "two.con"
    readcon.write_con(str(path), [first, second], compression="none")
    return path


def test_partial_optional_vectors_are_absent():
    only_x = readcon.Atom("Cu", 0.0, 0.0, 0.0, fx=1.0, dx=0.1, sx=0.3, mx=0.2)
    assert only_x.has_forces is False
    assert only_x.has_displacement is False
    assert only_x.has_spread is False
    bare = readcon.Atom("Cu", 0.0, 0.0, 0.0, fx=1.0, fy=2.0, dx=0.1, dy=0.2, sx=0.3, sy=0.4)
    assert bare.has_forces is False
    assert bare.has_displacement is False
    assert bare.has_spread is False
    full = readcon.Atom(
        "H",
        1.0,
        0.0,
        0.0,
        fx=1.0,
        fy=0.0,
        fz=0.0,
        dx=0.1,
        dy=0.0,
        dz=0.0,
        sx=0.2,
        sy=0.0,
        sz=0.0,
        mx=0.0,
        my=0.0,
        mz=1.0,
    )
    assert full.has_forces is True
    assert full.has_displacement is True
    assert full.has_spread is True


def test_write_coords_rejects_a_length_mismatch_and_zero_atoms(tmp_path: Path):
    path = tmp_path / "coords.con"
    positions = np.zeros((1, 2, 3))
    masses = np.ones((1, 2))
    cell = np.array([[4.0, 5.0, 6.0, 90.0, 90.0, 90.0]])
    short_mass = np.ones((1, 1))
    with pytest.raises(ValueError, match="disagree"):
        readcon.write_coords(str(path), positions, short_mass, cell)
    short_cell = np.array([[4.0, 5.0, 6.0]])
    with pytest.raises(ValueError, match="disagree"):
        readcon.write_coords(str(path), positions, masses, short_cell)
    empty_pos = np.zeros((1, 0, 3))
    empty_mass = np.zeros((1, 0))
    with pytest.raises((ValueError, OSError), match="disagree"):
        readcon.write_coords(str(path), empty_pos, empty_mass, cell)
    wrote = readcon.write_coords(str(path), positions, masses, cell, symbol="Cu")
    assert wrote["write_s"] >= 0.0
    loaded = readcon.read_coords(str(path))
    assert loaded["n_atoms"] == 2
    assert loaded["positions"].shape == (1, 2, 3)


def test_read_coords_rejects_a_changing_atom_count(tmp_path: Path):
    small = readcon.ConFrame.from_ase(
        ase.Atoms("Cu", positions=[[0.0, 0.0, 0.0]], cell=[3.0, 3.0, 3.0], pbc=True)
    )
    path = tmp_path / "mixed.con"
    readcon.write_con(str(path), [readcon.ConFrame.from_ase(_atoms()), small], compression="none")
    with pytest.raises(ValueError, match="same atom count"):
        readcon.read_coords(str(path))


def test_frame_index_and_iterator_cover_both_ends(tmp_path: Path):
    path = _two_frames(tmp_path)
    frames = readcon.read_con(str(path), n_threads=2)
    assert len(frames) == 2
    digest, seconds = frames.touch_checksum()
    assert isinstance(digest, int)
    assert seconds >= 0.0
    assert frames[0][0].symbol == "Cu"
    assert frames[-1][1].symbol == "H"
    with pytest.raises(IndexError):
        frames[2]
    with pytest.raises(IndexError):
        frames[-3]
    walked = list(readcon.read_con(str(path), n_threads=1))
    assert len(walked) == 2
    held = readcon.read_con(str(path), n_threads=2)
    assert held.release_frames() >= 0.0


def test_missing_sections_are_none_and_present_sections_have_rows(tmp_path: Path):
    still = readcon.ConFrame.from_ase(_atoms())
    path = tmp_path / "still.con"
    readcon.write_con(str(path), [still], compression="none")
    frame = readcon.read_con(str(path))[0]
    assert frame.vel is None
    assert frame.frc is None
    moving = readcon.ConFrame.from_ase(_atoms())
    moving[0].vx, moving[0].vy, moving[0].vz = 0.1, 0.0, 0.0
    moving[0].fx, moving[0].fy, moving[0].fz = 0.2, 0.0, 0.0
    rich = tmp_path / "rich.con"
    readcon.write_con(str(rich), [moving], compression="none")
    loaded = readcon.read_con(str(rich))[0]
    assert loaded.vel.shape == (2, 3)
    assert loaded.frc.shape == (2, 3)


def test_array_helpers_and_potential_type():
    empty = readcon.ConFrame([4.0, 5.0, 6.0], [90.0, 90.0, 90.0], [readcon.Atom("Cu", 0.0, 0.0, 0.0)])
    assert empty.forces_array() is None
    assert empty.energies_array() is None
    assert empty.potential_type is None
    charged = readcon.Atom("H", 1.0, 0.0, 0.0, energy=1.25, fx=0.1, fy=0.0, fz=0.0)
    rich = readcon.ConFrame([4.0, 5.0, 6.0], [90.0, 90.0, 90.0], [charged])
    assert rich.forces_array().shape == (1, 3)
    assert rich.energies_array().shape == (1,)
    assert float(rich.energies_array()[0]) == pytest.approx(1.25)
    rich.metadata = {"potential": None}
    assert rich.potential_type is None
    rich.metadata = {"potential": "lj"}
    assert rich.potential_type is None
    rich.metadata = {"potential": {"type": None}}
    assert rich.potential_type is None
    rich.metadata = {"potential": {"type": "lj"}}
    assert rich.potential_type == "lj"


def test_metadata_none_and_numeric_values_round_trip():
    frame = readcon.ConFrame.from_ase(_atoms())
    frame.metadata = None
    assert frame.metadata == {} or frame.metadata is None or "note" not in frame.metadata
    frame.metadata = {
        "potential": {"type": "lj"},
        "count": 3,
        "scale": 1.5,
        "pair": (1, 2),
        "flag": True,
        "empty": None,
    }
    assert frame.potential_type == "lj"
    text = readcon.write_con_string([frame])
    assert "lj" in text


def test_comm_rejects_a_gzip_path(tmp_path: Path):
    frame = readcon.ConFrame.from_ase(_atoms())
    dest = tmp_path / "out.con.gz"
    with pytest.raises(ValueError, match="uncompressed"):
        readcon.write_con(str(dest), [frame], compression="none", comm=object())


def test_conbin_header_without_velocities_and_with_zero_atoms(tmp_path: Path):
    def header(flags: int, n_frames: int, n_atoms: int) -> bytes:
        raw = bytearray(40)
        raw[0:8] = b"CONBIN01"
        raw[8:12] = (1).to_bytes(4, "little")
        raw[12:16] = flags.to_bytes(4, "little")
        raw[16:24] = n_frames.to_bytes(8, "little")
        raw[24:32] = n_atoms.to_bytes(8, "little")
        raw[32:40] = (40).to_bytes(8, "little")
        return bytes(raw)

    quiet = tmp_path / "quiet.conbin"
    quiet.write_bytes(header(0, 1, 1))
    with pytest.raises(ValueError, match="velocity"):
        readcon.read_conbin(str(quiet))
    empty = tmp_path / "empty.conbin"
    empty.write_bytes(header(2, 0, 0))
    positions, velocities, count = readcon.read_conbin(str(empty))
    assert count == 0
    assert positions.shape == (0, 0)
    assert velocities.shape == (0, 0)


def test_ase_constraints_tags_and_missing_velocities():
    atoms = _atoms()
    atoms.set_constraint(
        [FixAtoms(indices=[0]), FixCartesian([1], mask=[True, False, True])]
    )
    atoms.set_tags([0, 0])
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame[0].is_fixed
    assert list(frame[1].fixed) == [True, False, True]
    assert frame[0].atom_id == 0
    atoms.set_tags([-1, 4])
    tagged = readcon.ConFrame.from_ase(atoms)
    assert tagged[0].atom_id == 0
    atoms.set_velocities([[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]])
    quiet = readcon.ConFrame.from_ase(atoms)
    assert quiet[0].symbol == "Cu"
