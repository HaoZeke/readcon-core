"""Arms of the Python extension that the other tests leave closed."""

import math
import os

import pytest

import readcon

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def test_partial_optional_vectors_are_absent():
    atom = readcon.Atom(symbol="H", x=1.0, y=2.0, z=3.0, atom_id=4, mass=1.0)
    atom.vx = 0.5
    assert atom.has_velocity is False
    atom.vy = 0.25
    atom.vz = -0.1
    assert atom.has_velocity is True
    atom.fx = 1.0
    assert atom.has_forces is False
    atom.fy = 2.0
    atom.fz = 3.0
    assert atom.has_forces is True
    atom.dx = 0.1
    assert atom.has_displacement is False
    atom.dy = 0.2
    atom.dz = 0.3
    assert atom.has_displacement is True
    atom.sx = 0.01
    assert atom.has_spread is False
    atom.sy = 0.02
    atom.sz = 0.03
    assert atom.has_spread is True
    assert "H" in repr(atom)


def test_metadata_none_list_tuple_and_skipped_keys():
    atom = readcon.Atom(symbol="Cu", x=0.0, y=0.0, z=0.0, atom_id=1, mass=63.5)
    frame = readcon.ConFrame(
        cell=[2.0, 3.0, 4.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[atom],
        metadata=None,
    )
    assert frame[0].symbol == "Cu"
    frame = readcon.ConFrame(
        cell=[2.0, 3.0, 4.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[atom],
        metadata={
            "con_spec_version": 2,
            "sections": ["forces"],
            "note": "kept",
            "pair": (1, 2),
            "flags": [True, False],
            "nested": {"a": 1},
        },
    )
    meta = dict(frame.metadata)
    assert "con_spec_version" not in meta
    assert "sections" not in meta
    assert meta["note"] == "kept"
    assert list(meta["pair"]) == [1, 2]
    assert meta["flags"] == [True, False]
    assert meta["nested"]["a"] == 1
    with pytest.raises(Exception) as bad:
        readcon.ConFrame(
            cell=[2.0, 3.0, 4.0],
            angles=[90.0, 90.0, 90.0],
            atoms=[atom],
            metadata={"bad": object()},
        )
    assert "JSON" in str(bad.value)


def test_negative_frame_index_and_a_slice():
    path = os.path.join(RESOURCES, "tiny_multi_cuh2.con")
    frames = readcon.read_con(path)
    assert len(frames) >= 2
    assert frames[-1].atoms[0].symbol == frames[len(frames) - 1].atoms[0].symbol
    with pytest.raises(IndexError):
        frames[-len(frames) - 1]
    chunk = frames[0:2]
    assert len(chunk) == 2
    assert chunk[0].atoms[0].symbol == frames[0].atoms[0].symbol


def test_forces_file_exposes_force_rows_and_a_missing_velocity():
    path = os.path.join(RESOURCES, "tiny_cuh2_forces.con")
    frame = readcon.read_con(path)[0]
    assert frame.vel is None
    frc = frame.frc
    assert frc is not None
    assert frc.shape[0] == len(frame.atoms)
    assert frc.shape[1] == 3
    assert abs(float(frc[0, 0]) - 0.123456) < 1e-6
