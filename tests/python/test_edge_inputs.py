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
