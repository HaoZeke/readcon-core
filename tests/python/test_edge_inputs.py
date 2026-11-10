"""Malformed CON input and empty iterators through the Python API."""

import os

import pytest

import readcon

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def test_a_missing_file_and_a_truncated_buffer_fail(tmp_path):
    with pytest.raises(OSError):
        readcon.read_con(str(tmp_path / "missing.con"))
    source = os.path.join(RESOURCES, "tiny_cuh2.con")
    text = open(source, encoding="utf-8").read()
    dest = tmp_path / "cut.con"
    dest.write_text(text[:40], encoding="utf-8")
    with pytest.raises(Exception) as caught:
        readcon.read_con(str(dest))
    assert str(caught.value)
    frames = list(readcon.iter_con(source))
    assert frames
    assert len(frames[0].atoms) > 0
    empty = tmp_path / "empty.con"
    empty.write_text("", encoding="utf-8")
    assert list(readcon.iter_con(str(empty))) == []
    with pytest.raises(Exception) as truncated:
        list(readcon.iter_con(str(dest)))
    assert str(truncated.value)


def test_bad_metadata_and_an_index_past_the_end():
    atom = readcon.Atom(symbol="H", x=0.0, y=0.0, z=0.0, atom_id=0, mass=1.0)
    with pytest.raises(Exception) as not_a_dict:
        readcon.ConFrame(
            cell=[1.0, 1.0, 1.0],
            angles=[90.0, 90.0, 90.0],
            atoms=[atom],
            metadata=["nope"],
        )
    assert "dict" in str(not_a_dict.value)
    with pytest.raises(Exception) as bad_key:
        readcon.ConFrame(
            cell=[1.0, 1.0, 1.0],
            angles=[90.0, 90.0, 90.0],
            atoms=[atom],
            metadata={1: "x"},
        )
    assert "string" in str(bad_key.value)
    with pytest.raises(Exception) as non_finite:
        readcon.ConFrame(
            cell=[1.0, 1.0, 1.0],
            angles=[90.0, 90.0, 90.0],
            atoms=[atom],
            metadata={"energy": float("nan")},
        )
    assert "finite" in str(non_finite.value)
    frame = readcon.ConFrame(
        cell=[1.0, 1.0, 1.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[atom],
        metadata={"note": "ok", "flag": True, "count": 2},
    )
    assert frame[0].symbol == "H"
    assert frame[-1].symbol == "H"
    with pytest.raises(IndexError) as past:
        frame[3]
    assert "out of range" in str(past.value)
