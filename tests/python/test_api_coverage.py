"""Exercise Python entry points the other tests leave idle."""

import os

import numpy as np
import pytest
import readcon

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")
PDB = """\
CRYST1   10.000   12.000   14.000  90.00  90.00  90.00 P 1
ATOM      1   Cu MOL     1       0.000   1.000   2.000  1.00  0.00          CU
ATOM      2   Cu MOL     1       3.000   4.000   5.000  1.00  0.00          CU
END
"""
EXTXYZ = """\
2
Lattice="10 0 0 0 12 0 0 0 14" Properties=species:S:1:pos:R:3:masses:R:1 pbc="T T T"
Cu 0 1 2 63.5
Cu 3 4 5 63.5
"""


def _resource(name):
    return os.path.join(RESOURCES, name)


def _one():
    return readcon.read_con_string(open(_resource("tiny_cuh2_vel_forces.con")).read())[
        0
    ]


def test_coords_round_trip(tmp_path):
    path = _resource("tiny_multi_cuh2.con")
    got = readcon.read_coords(path)
    assert got["n_frames"] == 2
    assert got["positions"].shape[2] == 3
    out = tmp_path / "coords.con"
    written = readcon.write_coords(
        str(out), got["positions"], got["masses"], got["cell"], symbol="Cu"
    )
    assert written["write_s"] >= 0.0
    assert out.stat().st_size > 0
    with pytest.raises(ValueError):
        readcon.write_coords(str(out), got["positions"], got["masses"][:1], got["cell"])


def test_pdb_and_extxyz(tmp_path):
    pdb = tmp_path / "cu.pdb"
    xyz = tmp_path / "cu.extxyz"
    pdb.write_text(PDB)
    xyz.write_text(EXTXYZ)
    positions, masses, cell = readcon.read_pdb(str(pdb))
    assert positions.shape == (1, 2, 3)
    assert masses.shape[1] == 2
    assert cell.shape == (1, 6)
    loaded = readcon.read_extxyz(str(xyz))
    positions, masses, cell = loaded["positions"], loaded["masses"], loaded["cell"]
    assert positions.shape == (1, 2, 3)
    assert cell.shape == (1, 6)
    with pytest.raises(Exception):
        readcon.read_pdb(str(xyz))
    with pytest.raises(Exception):
        readcon.read_extxyz(str(pdb))


def test_frame_columns_and_metadata():
    frame = _one()
    assert frame.vel is not None
    assert frame.frc is not None
    assert frame.velocities_array() is not None
    assert frame.forces_array() is not None
    assert frame.energies_array() is None or frame.energies_array().shape[0] == len(
        frame
    )
    assert frame.bonds == []
    frame.metadata["potential"] = {"type": "EMT"}
    frame.metadata["bonds"] = [[0, 1], [1, 2, 1]]
    assert frame.potential_type == "EMT"
    bonds = frame.bonds
    assert bonds[0][0] == 0
    text = readcon.write_con_string([frame])
    again = readcon.read_con_string(text)[0]
    assert again.vel is not None
    assert again.frc is not None


def test_read_con_as_ase_and_feature_flags():
    atoms = readcon.read_con_as_ase(_resource("tiny_cuh2.con"))
    assert len(atoms) == 1
    assert atoms[0].get_chemical_symbols()[0] == "Cu"
    # The flag matches the build. Default features leave parallel off.
    assert readcon.has_parallel_support() in (True, False)
    assert readcon.has_mpi_support() is False
    if readcon.has_chemfiles_support():
        loaded = readcon.read_chemfiles(_resource("water_min.xyz"))
        assert len(loaded) == 1
    else:
        with pytest.raises(RuntimeError):
            readcon.read_chemfiles(_resource("water_min.xyz"))


def test_chemfiles_and_selection_report_the_build():
    path = _resource("water_min.xyz")
    xyz = "1\nwater\nO 0.0 0.0 0.0\n"
    frame = readcon.read_con(_resource("tiny_cuh2.con"))[0]
    frames = readcon.read_con(_resource("tiny_multi_cuh2.con"))
    if readcon.has_chemfiles_support():
        loaded = readcon.read_chemfiles(path)
        assert len(loaded) == 1
        assert {atom.symbol for atom in loaded[0].atoms} == {"O", "H"}
        first = readcon.read_chemfiles_first(path)
        assert len(first) == 3
        nth = readcon.read_chemfiles_nth(path, 0)
        assert len(nth) == 3
        assert readcon.read_chemfiles_nsteps(path) == 1
        memory = readcon.read_chemfiles_memory(xyz, "XYZ")
        assert len(memory) == 1
        assert memory[0].atoms[0].symbol == "O"
        selected = readcon.select_on_frame(frame, "name Cu")
        assert selected["selection"] == "name Cu"
        assert selected["context_size"] == 1
        assert len(selected["primary_indices"]) >= 1
        assert frame.select("name Cu")["selection"] == "name Cu"
        indices = readcon.select_atom_indices(frame, "name Cu")
        assert indices == selected["primary_indices"]
        multi = readcon.evaluate_selection_on_frames("name Cu", frames)
        assert multi["selection"] == "name Cu"
        assert len(multi["frames"]) == len(frames)
        positions = readcon.select_atom_positions_on_frames("name Cu", frames)
        assert positions["selection"] == "name Cu"
        assert len(positions["frames"]) == len(frames)
        return
    for call in (
        lambda: readcon.read_chemfiles(path),
        lambda: readcon.read_chemfiles_first(path),
        lambda: readcon.read_chemfiles_nth(path, 0),
        lambda: readcon.read_chemfiles_nsteps(path),
        lambda: readcon.read_chemfiles_memory(xyz, "XYZ"),
    ):
        with pytest.raises(RuntimeError, match="(?i)chemfiles"):
            call()
    for call in (
        lambda: readcon.select_on_frame(frame, "name Cu"),
        lambda: frame.select("name Cu"),
        lambda: readcon.select_atom_indices(frame, "name Cu"),
        lambda: readcon.evaluate_selection_on_frames("name Cu", frames),
        lambda: readcon.select_atom_positions_on_frames("name Cu", frames),
    ):
        with pytest.raises(Exception, match="(?i)chemfiles|feature"):
            call()


def test_write_con_compression_and_mpi_stub(tmp_path):
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    plain = tmp_path / "out.con"
    packed = tmp_path / "out.con.gz"
    readcon.write_con(
        str(plain), frames, precision=8, compression="none", canonical=True
    )
    readcon.write_con(str(packed), frames, compression="gzip")
    assert plain.stat().st_size > 0
    assert packed.stat().st_size > 0
    with pytest.raises(OSError):
        readcon.write_con(str(plain), frames, compression="bzip2")
    comm = object()
    with pytest.raises(NotImplementedError):
        readcon.read_con(str(plain), comm=comm)
    with pytest.raises(NotImplementedError):
        readcon.write_con(str(plain), frames, compression="none", comm=comm)
    with pytest.raises(ValueError):
        readcon.write_con(str(plain), frames, canonical=True, comm=comm)
    with pytest.raises(ValueError, match="uncompressed"):
        readcon.write_con(str(packed), frames, compression="gzip", comm=comm)


class _Cell:
    def __init__(self, rows):
        self.rows = rows

    def tolist(self):
        return self.rows

    def cellpar(self):
        return _Arr([10.0, 10.0, 10.0, 90.0, 90.0, 90.0])


class _Arr:
    def __init__(self, value):
        self.value = value

    def tolist(self):
        return self.value

    def __bool__(self):
        return self.value is not None


class _Constraint:
    def __init__(self, name, index, mask=None):
        self.name = name
        self.index = index
        self.mask = mask

    @property
    def __class__(self):
        return self

    def __name__(self):
        return self.name


class _StubAtoms:
    """Stand-in for ASE Atoms so each conversion branch runs."""

    def __init__(self, **kw):
        self.kw = kw

    def get_chemical_symbols(self):
        return self.kw.get("symbols", ["Cu", "H"])

    def get_positions(self):
        return _Arr(self.kw.get("positions", [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]))

    def get_cell(self):
        return self.kw.get(
            "cell", _Cell([[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]])
        )

    @property
    def constraints(self):
        return self.kw.get("constraints", [])

    @property
    def pbc(self):
        return _Arr(self.kw.get("pbc", [True, True, True]))

    def get_masses(self):
        return _Arr(self.kw.get("masses", [63.5, 1.0]))

    def get_array(self, name):
        if name in self.kw:
            return _Arr(self.kw[name])
        raise KeyError(name)

    def get_tags(self):
        if self.kw.get("tags_fail"):
            raise RuntimeError("tags are absent")
        return _Arr(self.kw.get("tags", [0, 0]))

    def get_velocities(self):
        if "velocities" not in self.kw:
            return None
        return _Arr(self.kw["velocities"])

    def get_forces(self):
        if "forces" not in self.kw:
            return None
        return _Arr(self.kw["forces"])


def test_from_ase_branches():
    from ase.constraints import FixAtoms, FixCartesian

    plain = readcon.ConFrame.from_ase(_StubAtoms())
    assert len(plain) == 2
    assert plain.atoms[0].atom_id == 0
    tagged = readcon.ConFrame.from_ase(_StubAtoms(tags=[3, 4]))
    assert [atom.atom_id for atom in tagged.atoms] == [3, 4]
    moving = readcon.ConFrame.from_ase(
        _StubAtoms(
            velocities=[[0.1, 0.0, 0.0], [0.0, 0.0, 0.0]],
            forces=[[0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        )
    )
    assert moving.atoms[0].vx == pytest.approx(0.1)
    assert moving.atoms[0].fy == pytest.approx(1.0)
    fixed = readcon.ConFrame.from_ase(
        _StubAtoms(
            constraints=[
                FixAtoms(indices=[0]),
                FixCartesian(0, mask=[True, False, True]),
            ]
        )
    )
    assert fixed.atoms[0].fixed == [True, False, True]
    zero = _Cell([[0.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]])
    angled = readcon.ConFrame.from_ase(_StubAtoms(cell=zero))
    assert angled.angles[0] == pytest.approx(90.0)
    bad = _Cell([[float("nan"), 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]])
    with pytest.raises(ValueError):
        readcon.ConFrame.from_ase(_StubAtoms(cell=bad))

    class Hold:
        pass

    held = readcon.ConFrame.from_ase(_StubAtoms(constraints=[Hold()]))
    assert held.atoms[0].fixed == [False, False, False]
    far = readcon.ConFrame.from_ase(
        _StubAtoms(constraints=[FixCartesian(9, mask=[True, False, True])])
    )
    assert far.atoms[0].fixed == [False, False, False]
    short_tags = readcon.ConFrame.from_ase(_StubAtoms(tags=[4]))
    assert [atom.atom_id for atom in short_tags.atoms] == [0, 1]
    no_tags = readcon.ConFrame.from_ase(_StubAtoms(tags_fail=True))
    assert [atom.atom_id for atom in no_tags.atoms] == [0, 1]
    flat = _Cell([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]])
    flat_frame = readcon.ConFrame.from_ase(_StubAtoms(cell=flat))
    assert flat_frame.angles[0] == pytest.approx(90.0)
    assert flat_frame.angles[2] == pytest.approx(90.0)
    edge_a = _Cell([[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    edged = readcon.ConFrame.from_ase(_StubAtoms(cell=edge_a))
    assert edged.angles[0] == pytest.approx(90.0)
    huge = _Cell([[1e308, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    with pytest.raises(ValueError, match="lengths must be finite"):
        readcon.ConFrame.from_ase(_StubAtoms(cell=huge))
    dup = readcon.ConFrame.from_ase(_StubAtoms(tags=[1, 1]))
    assert [atom.atom_id for atom in dup.atoms] == [0, 1]
    short_par = _Cell([[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]])
    short_par.cellpar = lambda: _Arr([1.0, 2.0, 3.0])
    shortened = readcon.ConFrame.from_ase(_StubAtoms(cell=short_par))
    assert shortened.cell[0] == pytest.approx(10.0)
    nan_par = _Cell([[10.0, 0.0, 0.0], [0.0, 10.0, 0.0], [0.0, 0.0, 10.0]])
    nan_par.cellpar = lambda: _Arr([10.0, 10.0, 10.0, float("nan"), 90.0, 90.0])
    nan_frame = readcon.ConFrame.from_ase(_StubAtoms(cell=nan_par))
    assert nan_frame.angles[0] == pytest.approx(90.0)
    middle = _Cell([[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]])
    middle_frame = readcon.ConFrame.from_ase(_StubAtoms(cell=middle))
    assert middle_frame.angles[1] == pytest.approx(90.0)
    negative = readcon.ConFrame.from_ase(_StubAtoms(tags=[-1, 4]))
    assert [atom.atom_id for atom in negative.atoms] == [0, 1]


def test_partial_fix_and_a_lone_moment_round_trip(tmp_path):
    atom = readcon.Atom(
        symbol="Cu",
        x=0.1,
        y=0.2,
        z=0.3,
        atom_id=7,
        mass=63.5,
        fx=0.4,
        fy=0.0,
        fz=-0.1,
        fixed=[True, False, True],
        mx=0.2,
    )
    frame = readcon.ConFrame(
        cell=[3.0, 4.0, 5.0], angles=[90.0, 90.0, 90.0], atoms=[atom]
    )
    frame.metadata = {"pbc": [True, True, False]}
    projection = frame.index_projection()
    assert list(projection["pbc"]) == [True, True, False]
    ase_atoms = frame.to_ase()
    assert len(ase_atoms.constraints) == 1
    assert ase_atoms.constraints[0].index == 0
    plain = tmp_path / "partial.con"
    readcon.write_con(str(plain), [frame])
    back = readcon.read_con(str(plain))[0]
    assert back.atoms[0].fixed == [True, False, True]
    assert back.atoms[0].mx is None
    missing = tmp_path / "absent" / "out.con"
    with pytest.raises(OSError):
        readcon.write_con(str(missing), [frame])
    with pytest.raises(OSError):
        readcon.write_con(
            str(tmp_path / "absent" / "out.con.gz"), [frame], compression="gzip"
        )
    positions = np.zeros((1, 1, 3))
    masses = np.zeros((1, 2))
    cell = np.array([[3.0, 4.0, 5.0, 90.0, 90.0, 90.0]])
    with pytest.raises(ValueError, match="disagree"):
        readcon.write_coords(str(tmp_path / "bad.con"), positions, masses, cell)


def test_bonds_release_and_indexes(tmp_path):
    frame = readcon.read_con_string(open(_resource("tiny_cuh2.con")).read())[0]
    frame.metadata["bonds"] = [{"i": 0, "j": 1, "order": 1}, [2, 3]]
    assert frame.has_bonds
    assert len(frame.bonds) == 2
    frame.metadata["bonds"] = [{"i": 0, "j": 1}, {"i": 1, "j": 0, "order": None}]
    plain_bonds = frame.bonds
    assert list(plain_bonds[0]) == [0, 1]
    assert list(plain_bonds[1]) == [1, 0]
    frame.atoms[0].energy = -0.2
    assert frame.energies_array() is not None
    assert frame.atom_index_by_id(0) == 0
    assert frame.atom_index_by_id(99) is None
    frames = readcon.read_con(_resource("tiny_multi_cuh2.con"), n_threads=2)
    assert isinstance(frames.phases, dict)
    frames.release_frames()
    listed = list(readcon.read_con(_resource("tiny_multi_cuh2.con"), n_threads=2))
    assert len(listed) == 2


def test_lean_selection_and_bad_indexes_fail():
    frame = _one()
    if readcon.has_chemfiles_support():
        selected = readcon.select_on_frame(frame, "name Cu")
        assert selected["context_size"] == 1
        assert (
            readcon.select_atom_indices(frame, "name Cu") == selected["primary_indices"]
        )
        multi = readcon.evaluate_selection_on_frames("name Cu", [frame])
        assert len(multi["frames"]) == 1
        positions = readcon.select_atom_positions_on_frames("name Cu", [frame])
        assert len(positions["frames"]) == 1
    else:
        with pytest.raises(Exception, match="(?i)chemfiles|feature"):
            readcon.select_on_frame(frame, "name Cu")
        with pytest.raises(Exception, match="(?i)chemfiles|feature"):
            readcon.select_atom_indices(frame, "name Cu")
        with pytest.raises(Exception, match="(?i)chemfiles|feature"):
            readcon.evaluate_selection_on_frames("name Cu", [frame])
        with pytest.raises(Exception, match="(?i)chemfiles|feature"):
            readcon.select_atom_positions_on_frames("name Cu", [frame])
    frames = readcon.read_con(_resource("tiny_multi_cuh2.con"))
    with pytest.raises(IndexError):
        frames[99]
    with pytest.raises(ValueError, match="unknown build"):
        readcon.read_con_phases(_resource("tiny_cuh2.con"), build="sideways")


def test_iterator_skip_and_past_the_end():
    path = _resource("tiny_multi_cuh2.con")
    it = readcon.iter_con(path)
    assert it.forward() is True
    assert it.skip(1) == 1
    assert it.nth(0) is None
    assert it.forward() is False
    streamed = readcon.iter_con(path, stream=True)
    assert streamed.forward() is True
    assert streamed.skip(10) == 1
    with pytest.raises(ValueError, match="mmap and stream"):
        readcon.iter_con(path, mmap=True, stream=True)
    with pytest.raises(OSError):
        readcon.read_nth_frame(path, 50)


def test_symbol_table_and_conbin(tmp_path):
    assert readcon.symbol_to_atomic_number("Fe") == 26
    assert readcon.atomic_number_to_symbol(1) == "H"
    assert readcon.symbol_to_atomic_number("nope") == 0
    src = _resource("tiny_cuh2_vel_forces.con")
    path = tmp_path / "frames.conbin"
    n_frames, n_atoms = readcon.write_conbin(src, str(path))
    assert n_frames >= 1
    positions, velocities, atoms = readcon.read_conbin(str(path), n_threads=1)
    assert positions.shape[0] == n_frames
    assert atoms == n_atoms
    assert velocities.shape == positions.shape
