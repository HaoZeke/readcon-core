"""Observable results for duck-typed cells and extra workers.

The earlier checks only asked for a copper symbol, or for a frame count.
These checks name the cell and the coordinates the call returns.
"""

from __future__ import annotations

from pathlib import Path

import pytest

readcon = pytest.importorskip("readcon")


def _two_frames(tmp_path: Path) -> Path:
    import ase

    atoms = ase.Atoms("Cu", positions=[[0.25, 0.5, 0.75]], cell=[4.0, 5.0, 6.0], pbc=True)
    other = ase.Atoms("Cu", positions=[[1.25, 0.5, 0.75]], cell=[4.0, 5.0, 6.0], pbc=True)
    path = tmp_path / "two.con"
    readcon.write_con(
        str(path),
        [readcon.ConFrame.from_ase(atoms), readcon.ConFrame.from_ase(other)],
    )
    return path


class _List:
    def __init__(self, values):
        self.values = values

    def tolist(self):
        return self.values


class _Cell:
    def tolist(self):
        return [[4.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 6.0]]

    def cellpar(self):
        return _List([4.0, 5.0, 6.0, 90.0, 90.0, 90.0])


class _ShortCell(_Cell):
    def cellpar(self):
        return _List([4.0, 5.0, 6.0])


class _ZeroRowCell:
    def tolist(self):
        return [[4.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 6.0]]

    def cellpar(self):
        return _List([4.0, 0.0, 6.0, 90.0, 90.0, 90.0])


class _NanCellpar(_Cell):
    def cellpar(self):
        return _List([4.0, 5.0, 6.0, float("nan"), 90.0, 90.0])


class _FixAtoms:
    def __init__(self, index):
        self.index = index


_FixAtoms.__name__ = "FixAtoms"


class _Duck:
    def __init__(self, index, cell=None):
        self._index = index
        self._cell = cell if cell is not None else _Cell()

    def get_chemical_symbols(self):
        return ["Cu"]

    def get_positions(self):
        return _List([[0.25, 0.5, 0.75]])

    def get_cell(self):
        return self._cell

    @property
    def pbc(self):
        return _List([True, True, True])

    @property
    def constraints(self):
        return [_FixAtoms(self._index)]

    def get_tags(self):
        raise AttributeError("tags")

    def get_velocities(self):
        return None

    def get_forces(self):
        return None


def _cell(frame):
    return (
        [float(value) for value in frame.cell],
        [float(value) for value in frame.angles],
    )


def test_more_workers_than_frames_keep_coordinates(tmp_path: Path):
    path = _two_frames(tmp_path)
    serial = readcon.read_con(str(path), n_threads=1)
    wide = readcon.read_con(str(path), n_threads=8)
    assert len(wide) == 2
    for left, right in zip(serial, wide):
        assert abs(float(left.xyz[0, 0]) - float(right.xyz[0, 0])) < 1e-12
        assert abs(float(left.xyz[0, 1]) - float(right.xyz[0, 1])) < 1e-12
        assert abs(float(left.xyz[0, 2]) - float(right.xyz[0, 2])) < 1e-12
        assert list(left.cell) == list(right.cell)


def test_duck_cells_keep_the_matrix_lengths():
    short = readcon.ConFrame.from_ase(_Duck(0, cell=_ShortCell()))
    assert _cell(short) == ([4.0, 5.0, 6.0], [90.0, 90.0, 90.0])
    assert short[0].x == pytest.approx(0.25)
    assert short[0].is_fixed

    zero = readcon.ConFrame.from_ase(_Duck(0, cell=_ZeroRowCell()))
    lengths, angles = _cell(zero)
    assert lengths[0] == pytest.approx(4.0)
    assert lengths[1] == pytest.approx(0.0)
    assert lengths[2] == pytest.approx(6.0)
    assert angles == pytest.approx([90.0, 90.0, 90.0])

    odd = readcon.ConFrame.from_ase(_Duck(0, cell=_NanCellpar()))
    assert _cell(odd) == ([4.0, 5.0, 6.0], [90.0, 90.0, 90.0])

    far = readcon.ConFrame.from_ase(_Duck(99))
    assert far[0].is_fixed is False
    assert far[0].x == pytest.approx(0.25)
