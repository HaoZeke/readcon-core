"""Observable Python paths: bad inputs, exhausted iterators, and conversions."""

import os

import numpy as np
import pytest
import readcon
from ase import Atoms
from ase.constraints import FixAtoms, FixCartesian

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def resource(name):
    return os.path.join(RESOURCES, name)


def one_atom(**kwargs):
    fields = dict(symbol="Cu", x=0.0, y=0.0, z=0.0, atom_id=1, mass=63.5)
    fields.update(kwargs)
    return readcon.Atom(**fields)


def test_none_metadata_and_a_wide_integer_round_trip():
    frame = readcon.ConFrame(
        cell=[2.0, 3.0, 4.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[one_atom()],
    )
    frame.metadata = None
    assert dict(frame.metadata) == {}
    frame.metadata = {"count": 2**63, "note": None, "pair": (1, 2)}
    stored = dict(frame.metadata)
    assert stored["count"] == 2**63
    assert stored["note"] is None
    assert list(stored["pair"]) == [1, 2]
    with pytest.raises(ValueError, match="dict"):
        frame.metadata = [1, 2]
    with pytest.raises(ValueError, match="finite"):
        frame.metadata = {"x": float("nan")}


def test_a_negative_atom_index_and_an_exhausted_iterator():
    frame = readcon.read_first_frame(resource("tiny_cuh2.con"))
    assert frame[-1].symbol == frame[len(frame) - 1].symbol
    with pytest.raises(IndexError, match="atom index"):
        frame[-(len(frame) + 1)]
    atoms = list(frame)
    assert atoms[0].symbol == "Cu"
    mark = iter(frame)
    seen = [atom.symbol for atom in mark]
    assert seen[0] == "Cu"
    assert len(seen) == len(frame)
    assert next(mark, None) is None


def test_bad_paths_wrong_types_and_a_malformed_frame():
    with pytest.raises(OSError):
        readcon.read_con("/no/such/readcon-frame.con")
    with pytest.raises(OSError):
        readcon.read_first_frame("/no/such/readcon-frame.con")
    with pytest.raises((OSError, ValueError)):
        readcon.read_con_string("this is not a con file\n")
    with pytest.raises((TypeError, ValueError)):
        readcon.ConFrame(cell="nope", angles=[90.0, 90.0, 90.0], atoms=[])
    with pytest.raises((TypeError, ValueError)):
        readcon.Atom(symbol=1, x=0.0, y=0.0, z=0.0, atom_id=0, mass=1.0)


def test_frame_list_iterator_stops_and_a_string_round_trip(tmp_path):
    path = resource("tiny_multi_cuh2.con")
    frames = readcon.read_con(path)
    mark = iter(frames)
    collected = list(mark)
    assert len(collected) == len(frames)
    assert next(mark, None) is None
    with pytest.raises(IndexError, match="out of range"):
        frames[-(len(frames) + 1)]
    text = readcon.write_con_string(frames, precision=8)
    assert "Cu" in text
    again = readcon.read_con_string(text)
    assert len(again) == len(frames)
    assert again[0].atoms[0].symbol == frames[0].atoms[0].symbol
    with pytest.raises(OSError, match="touch_checksum"):
        again.touch_checksum()
    dest = tmp_path / "missing" / "out.con"
    with pytest.raises(OSError):
        readcon.write_con(str(dest), frames)
    with pytest.raises(ValueError, match="comm="):
        readcon.write_con(
            str(tmp_path / "mpi.con.gz"), frames, comm=object(), canonical=True
        )


def test_coords_reject_a_disagreement_and_a_partial_magmom(tmp_path):
    positions = np.zeros((1, 2, 3), dtype=np.float64)
    masses = np.ones((1, 2), dtype=np.float64)
    cell = np.array([[4.0, 4.0, 4.0, 90.0, 90.0, 90.0]], dtype=np.float64)
    out = tmp_path / "coords.con"
    written = readcon.write_coords(str(out), positions, masses, cell, symbol="Cu")
    assert written["write_s"] >= 0.0
    assert out.stat().st_size > 0
    bad_mass = np.ones((1, 1), dtype=np.float64)
    with pytest.raises(ValueError, match="disagree"):
        readcon.write_coords(str(tmp_path / "bad.con"), positions, bad_mass, cell)
    atom = one_atom()
    atom.mx = 0.1
    atom.my = 0.2
    frame = readcon.ConFrame(
        cell=[2.0, 2.0, 2.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[atom],
    )
    text = readcon.write_con_string([frame])
    assert "Magmoms" not in text
    atom.mz = 0.3
    frame.atoms[0].mz = 0.3
    text = readcon.write_con_string([frame])
    assert "Magmoms" in text
    assert "0.1" in text


def test_projection_bonds_and_potential_are_visible():
    frame = readcon.read_first_frame(resource("tiny_cuh2.con"))
    projection = frame.index_projection()
    assert projection["n_atoms"] == len(frame)
    assert "pbc" in projection
    frame.metadata = {"potential": {"type": "EMT"}}
    assert frame.potential_type == "EMT"
    frame.metadata = {"potential": None}
    assert frame.potential_type is None
    frame.metadata = {"potential": {"type": None}}
    assert frame.potential_type is None
    frame.metadata = {"bonds": [{"i": 0, "j": 1, "order": 2}, [2, 3]]}
    bonds = frame.bonds
    assert bonds[0] == (0, 1, 2)
    assert bonds[1] == (2, 3)


def test_binary32_conbin_is_rejected_and_f64_round_trips(tmp_path):
    source = resource("tiny_cuh2.con")
    out = tmp_path / "frames.conbin"
    n_frames, n_atoms = readcon.write_conbin(source, str(out), binary32=False)
    assert n_frames >= 1
    assert n_atoms >= 1
    positions, velocities, atoms = readcon.read_conbin(str(out))
    assert positions.shape[0] == n_frames
    assert positions.shape[1] == n_atoms * 3
    assert velocities.shape == positions.shape
    assert atoms == n_atoms
    narrow = tmp_path / "frames32.conbin"
    readcon.write_conbin(source, str(narrow), binary32=True)
    with pytest.raises(ValueError, match="binary32"):
        readcon.read_conbin(str(narrow))
    with pytest.raises(OSError):
        readcon.read_conbin(str(tmp_path / "missing.conbin"))


def test_ase_constraints_and_repeated_tags_convert_both_ways():
    atoms = Atoms(
        "CuH",
        positions=[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        cell=[5.0, 5.0, 5.0],
        pbc=True,
    )
    atoms.set_constraint(
        [FixAtoms(indices=[0, 9]), FixCartesian(a=[1], mask=[True, False, True])]
    )
    atoms.set_tags([3, 3])
    atoms.set_array("atom_id", np.array([-1, 4], dtype=np.int64))
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame.atoms[0].symbol == "Cu"
    bare = readcon.read_first_frame(resource("tiny_cuh2.con"))
    assert bare.potential_type is None
    bare.metadata = {"time": None, "frame_index": None, "timestep": None}
    assert bare.time is None
    assert bare.frame_index is None
    assert bare.timestep is None
    bare.metadata = {"time": 1.25, "frame_index": 3, "timestep": 0.5}
    assert bare.time == 1.25
    assert bare.frame_index == 3
    assert bare.timestep == 0.5
    assert list(frame.atoms[0].fixed) == [True, True, True]
    assert list(frame.atoms[1].fixed) == [True, False, True]
    assert frame.atoms[1].atom_id == 4
    back = frame.to_ase()
    assert list(back.get_chemical_symbols())[0] == "Cu"
    assert len(back) == 2


def test_iterators_stop_and_views_follow_the_stored_columns(tmp_path):
    path = resource("tiny_multi_cuh2.con")
    mark = readcon.iter_con(path)
    first = next(mark)
    assert first.atoms[0].symbol == "Cu"
    assert mark.forward() is True
    assert next(mark, None) is None
    assert mark.forward() is False
    built = readcon.ConFrame(
        cell=[2.0, 2.0, 2.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[one_atom()],
    )
    assert built.vel is None
    assert built.frc is None
    forces = readcon.read_first_frame(resource("tiny_cuh2_forces.con"))
    assert forces.frc is not None
    assert forces.frc.shape[1] == 3
    assert abs(float(forces.frc[0, 0]) - 0.123456) < 1e-5
    text = (
        "2\n"
        'Lattice="5.0 0.0 0.0 0.0 5.0 0.0 0.0 0.0 5.0" Properties=species:S:1:pos:R:3\n'
        "H 0.0 0.0 0.0\n"
        "H 0.0 0.0 0.7\n"
    )
    xyz = tmp_path / "two.extxyz"
    xyz.write_text(text)
    loaded = readcon.read_extxyz(str(xyz))
    positions, masses, cell = loaded["positions"], loaded["masses"], loaded["cell"]
    assert positions.shape == (1, 2, 3)
    assert masses.shape[1] == 2
    assert cell.shape == (1, 6)
    pdb = tmp_path / "one.pdb"
    pdb.write_text(
        "CRYST1    5.000    5.000    5.000  90.00  90.00  90.00 P 1\n"
        "ATOM      1  H   HOH     1       0.000   0.000   0.000  1.00  0.00           H\n"
        "END\n"
    )
    pdb_pos, pdb_mass, pdb_cell = readcon.read_pdb(str(pdb))
    assert pdb_pos.shape[2] == 3
    assert pdb_mass.shape[0] == pdb_pos.shape[0]
    assert pdb_cell.shape[1] == 6
    with pytest.raises(ValueError):
        readcon.read_extxyz(str(tmp_path / "missing.extxyz"))
    with pytest.raises(ValueError):
        readcon.read_pdb(str(tmp_path / "missing.pdb"))
    projection = forces.index_projection_json()
    assert "n_atoms" in projection
    one = readcon.read_con(path, 1)
    assert len(one) >= 2
    assert one.release_frames() >= 0.0
    two = readcon.read_con(path, 2)
    assert len(two) >= 2
    assert two[0].atoms[0].symbol == "Cu"
    assert two.release_frames() >= 0.0
