"""Errors a caller can observe: a bad atom, a bad write, a bad index, a bad file."""

import os

import pytest

import readcon


RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def _resource(name):
    return os.path.join(RESOURCES, name)


def test_a_non_atom_entry_is_rejected():
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    frames[0].atoms.append("nope")
    with pytest.raises(TypeError, match="Atom"):
        frames[0].coords_array()


def test_a_non_frame_write_is_rejected():
    with pytest.raises(TypeError, match="ConFrame"):
        readcon.write_con_string([1, 2, 3])


def test_a_trajectory_index_past_the_end_is_rejected(tmp_path):
    path = _resource("tiny_cuh2.con")
    traj = readcon.ConTrajectory(path, write_index=False)
    with pytest.raises(IndexError, match="out of range"):
        traj[99]


def test_mmap_and_stream_together_are_rejected():
    with pytest.raises(ValueError, match="mmap and stream"):
        readcon.iter_con(_resource("tiny_cuh2.con"), mmap=True, stream=True)


def test_a_garbage_file_fails_the_parse_and_the_skip(tmp_path):
    path = tmp_path / "garbage.con"
    path.write_text("this is not a con frame\n")
    text = readcon.iter_con(str(path))
    with pytest.raises(OSError, match="parse error"):
        next(text)
    streamed = readcon.iter_con(str(path), stream=True)
    with pytest.raises(OSError, match="skip error|parse error"):
        streamed.forward()
