"""Sides the r2.30 table left at zero because no call built the value."""

import os
import subprocess
import sys
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


def test_iterator_skip_stops_at_the_end(tmp_path: Path):
    path = tmp_path / "one.con"
    readcon.write_con(str(path), [readcon.ConFrame.from_ase(_atoms())], compression="none")
    iterator = readcon.iter_con(str(path))
    assert iterator.skip(4) == 1
    assert iterator.skip(1) == 0


def test_metadata_accepts_an_integer_past_i64():
    frame = readcon.ConFrame.from_ase(_atoms())
    frame.metadata = {"count": 2**63}
    assert frame.metadata["count"] == 2**63


def test_comm_rejects_canonical_output(tmp_path: Path):
    frame = readcon.ConFrame.from_ase(_atoms())
    dest = tmp_path / "canon.con"
    with pytest.raises(ValueError, match="uncompressed"):
        readcon.write_con(str(dest), [frame], compression="none", canonical=True, comm=object())


class _OtherConstraint:
    index = [0]


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
        [
            FixAtoms(indices=[0]),
            FixCartesian([1], mask=[True, False, True]),
            _OtherConstraint(),
        ]
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


_F32 = """\
user
{"con_spec_version":2,"sections":["velocities","forces"],"storage_dtypes":{"positions":"float32","velocities":"float32","forces":"float32","masses":"float32"},"energy":-1.5}
10.0 10.0 10.0
90.0 90.0 90.0
0 0
1 0
1
1
63.546
Cu
Coordinates of Component 1
   0.250000    0.500000    0.750000 0    0
Cu
Velocities of Component 1
   0.100000    0.000000    0.000000 0    0
Cu
Forces of Component 1
   0.200000    0.000000    0.000000 0    0
"""

_ZERO = """\
user
{"con_spec_version":2}
4.0 5.0 6.0
90.0 90.0 90.0
0 0
0 0
0
"""


def _with_storage(text: str) -> str:
    needle = '{"con_spec_version":2,'
    insert = (
        '{"con_spec_version":2,"storage_dtypes":{"positions":"float32",'
        '"velocities":"float32","forces":"float32","masses":"float32"},'
    )
    return text.replace(needle, insert, 1)


def test_float32_storage_reads_back(tmp_path: Path):
    source = Path(__file__).resolve().parents[2] / "resources" / "test" / "tiny_cuh2_vel_forces.con"
    path = tmp_path / "f32.con"
    path.write_text(_with_storage(source.read_text()))
    frames = readcon.read_con(str(path))
    assert len(frames) == 1
    frame = frames[0]
    assert abs(float(frame.xyz[0, 0]) - 0.6394) < 1e-4
    assert frame.vel is not None
    assert abs(float(frame.vel[0, 0]) - 0.001234) < 1e-5
    assert frame.frc is not None
    assert abs(float(frame.frc[0, 0]) - 0.123456) < 1e-5
    eager, _phases = readcon.read_con_phases(str(path), build="eager")
    assert len(eager) == 1
    assert abs(float(eager[0].xyz[0, 0]) - 0.6394) < 1e-4


def test_mixed_atom_counts_are_not_rectangular(tmp_path: Path):
    two = tmp_path / "two.con"
    one = tmp_path / "one.con"
    readcon.write_con(str(two), [readcon.ConFrame.from_ase(_atoms())])
    single = ase.Atoms("Cu", positions=[[0.0, 0.0, 0.0]], cell=[4.0, 5.0, 6.0], pbc=True)
    readcon.write_con(str(one), [readcon.ConFrame.from_ase(single)])
    mixed = tmp_path / "mixed.con"
    mixed.write_text(two.read_text() + one.read_text())
    eager, _phases = readcon.read_con_phases(str(mixed), build="eager")
    assert len(eager) == 2
    assert len(eager[0]) != len(eager[1])


def test_null_metadata_energy_and_frame_index(tmp_path: Path):
    source = Path(__file__).resolve().parents[2] / "resources" / "test" / "tiny_cuh2_vel_forces.con"
    path = tmp_path / "null.con"
    path.write_text(source.read_text().replace('"energy":-42.5', '"energy":null,"frame_index":null'))
    frame = readcon.read_con(str(path))[0]
    assert frame.energy is None
    assert frame.frame_index is None
    frame.metadata["time"] = None
    assert frame.time is None


def test_partial_magmom_is_not_written(tmp_path: Path):
    frame = readcon.ConFrame.from_ase(_atoms())
    frame[0].mx = 1.0
    path = tmp_path / "mx.con"
    readcon.write_con(str(path), [frame])
    back = readcon.read_con(str(path))[0]
    assert back[0].mx is None
    frame[0].my = 2.0
    readcon.write_con(str(path), [frame])
    back = readcon.read_con(str(path))[0]
    assert back[0].mz is None


def test_more_workers_than_frames(tmp_path: Path):
    path = _two_frames(tmp_path)
    frames = readcon.read_con(str(path), n_threads=4)
    assert len(frames) == 2
    frames.touch_checksum()
    del frames


def test_empty_and_zero_atom_files_do_not_build_a_slab(tmp_path: Path):
    empty = tmp_path / "empty.con"
    empty.write_text("")
    eager, _phases = readcon.read_con_phases(str(empty), build="eager")
    assert len(eager) == 0
    lazy = readcon.read_con(str(empty))
    assert len(lazy) == 0
    zero = tmp_path / "zero.con"
    zero.write_text(_ZERO)
    eager, _phases = readcon.read_con_phases(str(zero), build="eager")
    assert len(eager) == 1
    assert len(eager[0]) == 0


def test_numpy_missing_keeps_the_parsed_frame(tmp_path: Path):
    path = _two_frames(tmp_path)
    script = tmp_path / "nonumpy.py"
    script.write_text(
        "import sys\n"
        "sys.modules['numpy'] = None\n"
        "import readcon\n"
        f"frames = readcon.read_con({str(path)!r})\n"
        f"eager, _phases = readcon.read_con_phases({str(path)!r}, build='eager')\n"
        "print('N', len(frames), len(eager))\n"
        "try:\n"
        "    frames.touch_checksum()\n"
        "    print('CHECKSUM ok')\n"
        "except Exception as exc:\n"
        "    print('CHECKSUM', type(exc).__name__)\n"
        "print('OK')\n"
    )
    env = os.environ.copy()
    proc = subprocess.run(
        [sys.executable, str(script)],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr
    assert "N 2" in proc.stdout
    assert "CHECKSUM" in proc.stdout
    assert "OK" in proc.stdout


def test_projection_pbc_and_ase_edges():
    atoms = _atoms()
    frame = readcon.ConFrame.from_ase(atoms)
    projection = frame.index_projection()
    assert projection["pbc"] == (True, True, True)
    frame[0].fx = 0.1
    frame[0].fy = 0.0
    frame[0].fz = 0.0
    frame[0].fixed = [True, False, False]
    copied = frame.to_ase()
    assert copied.calc is not None
    assert "energy" not in copied.info
    assert copied.constraints


def test_cell_overflow_and_degenerate_row():
    huge = ase.Atoms(
        "Cu",
        positions=[[0.0, 0.0, 0.0]],
        cell=[[1e200, 0.0, 0.0], [0.0, 1e200, 0.0], [0.0, 0.0, 1e200]],
        pbc=True,
    )
    with pytest.raises(ValueError, match="finite"):
        readcon.ConFrame.from_ase(huge)
    flat = ase.Atoms(
        "Cu",
        positions=[[0.0, 0.0, 0.0]],
        cell=[[0.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 6.0]],
        pbc=False,
    )
    frame = readcon.ConFrame.from_ase(flat)
    assert len(frame) == 1
    assert frame[0].symbol == "Cu"


class _List:
    def __init__(self, values):
        self.values = values

    def tolist(self):
        return self.values


class _Duck:
    def __init__(self, index, mask=None, tags=None, velocities=None, forces=None, cell=None):
        self._index = index
        self._mask = mask
        self._tags = tags
        self._velocities = velocities
        self._forces = forces
        self._cell = cell if cell is not None else _Cell()

    def get_chemical_symbols(self):
        return ["Cu"]

    def get_positions(self):
        return _List([[0.0, 0.0, 0.0]])

    def get_cell(self):
        return self._cell

    @property
    def pbc(self):
        return _List([True, True, True])

    @property
    def constraints(self):
        if self._mask is None:
            return [_FixAtoms(self._index)]
        return [_FixCartesian(self._index, self._mask)]

    def get_tags(self):
        if self._tags is None:
            raise AttributeError("tags")
        return self._tags

    def get_velocities(self):
        return self._velocities

    def get_forces(self):
        return self._forces


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


class _ZeroLastCell:
    def tolist(self):
        return [[4.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 0.0]]

    def cellpar(self):
        return _List([4.0, 5.0, 0.0, 90.0, 90.0, 90.0])


class _NanCellpar(_Cell):
    def cellpar(self):
        return _List([4.0, 5.0, 6.0, float("nan"), 90.0, 90.0])


class _FixAtoms:
    def __init__(self, index):
        self.index = index


class _FixCartesian:
    def __init__(self, index, mask):
        self.index = index
        self.mask = mask


_FixAtoms.__name__ = "FixAtoms"
_FixCartesian.__name__ = "FixCartesian"


def test_duck_typed_constraints_and_optional_arrays():
    plain = readcon.ConFrame.from_ase(_Duck(0))
    assert plain[0].symbol == "Cu"
    assert plain[0].is_fixed
    scalar = readcon.ConFrame.from_ase(_Duck(_List(0)))
    assert scalar[0].is_fixed
    far = readcon.ConFrame.from_ase(_Duck(_List([99])))
    assert far[0].is_fixed is False
    with pytest.raises(ValueError, match="length 3"):
        readcon.ConFrame.from_ase(_Duck(_List([0]), mask=_List([True])))
    with pytest.raises((ValueError, TypeError)):
        readcon.ConFrame.from_ase(_Duck(_List([0]), mask=True))
    tagged = readcon.ConFrame.from_ase(_Duck(0, tags=_List([1, 2])))
    assert tagged[0].atom_id == 0
    quiet = readcon.ConFrame.from_ase(_Duck(0, velocities=None, forces=None))
    assert quiet[0].symbol == "Cu"
    listed = readcon.ConFrame.from_ase(_Duck([99], mask=_List([True, False, False])))
    assert listed[0].is_fixed is False
    with pytest.raises(ValueError):
        readcon.ConFrame.from_ase(_Duck(_List("no")))
    finite = readcon.ConFrame.from_ase(_Duck(0, cell=_ShortCell()))
    assert finite[0].symbol == "Cu"
    flat = readcon.ConFrame.from_ase(_Duck(0, cell=_ZeroRowCell()))
    assert flat[0].symbol == "Cu"
    last = readcon.ConFrame.from_ase(_Duck(0, cell=_ZeroLastCell()))
    assert last[0].symbol == "Cu"
    odd = readcon.ConFrame.from_ase(_Duck(0, cell=_NanCellpar()))
    assert odd[0].symbol == "Cu"
