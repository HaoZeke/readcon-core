"""Calls the Python surface the default suite leaves cold."""

import os

import pytest
import readcon

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def resource(name):
    return os.path.join(RESOURCES, name)


def one_atom(**kwargs):
    fields = dict(symbol="H", x=0.0, y=0.0, z=0.0, atom_id=0, mass=1.0)
    fields.update(kwargs)
    return readcon.Atom(**fields)


def test_coordinate_arrays_round_trip(tmp_path):
    source = resource("tiny_multi_cuh2.con")
    coords = readcon.read_coords(source)
    assert coords["n_frames"] == 2
    assert coords["positions"].shape[0] == 2
    dest = tmp_path / "round.con"
    written = readcon.write_coords(
        str(dest),
        coords["positions"],
        coords["masses"],
        coords["cell"],
        symbol="Cu",
    )
    assert written["write_s"] >= 0.0
    again = readcon.read_coords(str(dest))
    assert again["n_atoms"] == coords["n_atoms"]
    with pytest.raises(TypeError, match="not contiguous"):
        readcon.write_coords(
            str(tmp_path / "bad.con"),
            coords["positions"],
            coords["masses"][:, :1],
            coords["cell"],
        )


def test_frame_properties_and_metadata():
    atom = one_atom(
        vx=1.0,
        vy=0.0,
        vz=0.0,
        fx=0.0,
        fy=1.0,
        fz=0.0,
        energy=-1.5,
        dx=0.1,
        dy=0.0,
        dz=0.0,
        sx=0.2,
        sy=0.0,
        sz=0.0,
        fixed=[True, True, True],
    )
    assert atom.has_energy
    assert atom.has_velocity
    frame = readcon.ConFrame(
        cell=[10.0, 10.0, 10.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[atom],
        metadata={
            "bonds": [[0, 0], {"i": 0, "j": 0, "order": 1}, "skip"],
            "potential": {"type": "EMT"},
            "energy": 1.25,
            "frame_index": 3,
            "time": 0.5,
            "timestep": 0.1,
            "neb_bead": 2,
            "neb_band": 4,
        },
    )
    assert frame.has_bonds
    assert frame.bonds
    assert frame.potential_type == "EMT"
    assert frame.energy == pytest.approx(1.25)
    assert frame.frame_index == 3
    assert frame.time == pytest.approx(0.5)
    assert frame.timestep == pytest.approx(0.1)
    assert frame.neb_bead == 2
    assert frame.neb_band == 4
    assert frame.vel is not None
    assert frame.frc is not None
    assert frame.energies_array() is not None
    assert frame.atom_index_by_id(0) == 0
    assert frame.atom_index_by_id(9) is None
    assert frame.index_projection()["n_atoms"] == 1
    frame.metadata = {"potential": None, "bonds": None}
    assert frame.potential_type is None
    assert frame.bonds == []
    assert frame.has_bonds is False
    frame.metadata = {"potential": "EMT", "bonds": {"i": 0}}
    assert frame.potential_type is None
    assert frame.bonds == []
    bare = readcon.ConFrame(
        cell=[1.0, 1.0, 1.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[one_atom()],
    )
    assert bare.vel is None
    assert bare.frc is None
    assert bare.energies_array() is None
    assert bare.has_bonds is False
    assert not bare.atoms[0].has_energy


def test_readers_writers_and_feature_flags(tmp_path):
    path = resource("tiny_cuh2.con")
    frames, phases = readcon.read_con_phases(path, build="lazy")
    assert phases["n_frames"] >= 1
    eager, eager_phases = readcon.read_con_phases(path, 2, build="eager")
    assert len(eager) == len(frames)
    assert eager_phases["n_frames"] == phases["n_frames"]
    with pytest.raises(ValueError, match="unknown build"):
        readcon.read_con_phases(path, build="sideways")
    listed = readcon.read_all_frames(path, 2)
    assert len(listed) >= 1
    assert listed.phases["n_frames"] >= 1
    digest, seconds = listed.touch_checksum()
    assert isinstance(digest, int)
    assert seconds >= 0.0
    assert listed.release_frames() >= 0.0
    single = readcon.read_con(path)
    digest, seconds = single.touch_checksum()
    assert isinstance(digest, int)
    assert seconds >= 0.0
    assert single.release_frames() >= 0.0
    with pytest.raises(
        NotImplementedError, match="comm= needs readcon built with the mpi feature"
    ):
        readcon.read_con(path, comm=object())
    with pytest.raises(
        NotImplementedError, match="comm= needs readcon built with the mpi feature"
    ):
        readcon.write_con(str(tmp_path / "mpi.con"), frames, comm=object())
    dest = tmp_path / "plain.con"
    readcon.write_con(
        str(dest), frames, precision=6, compression="none", canonical=True
    )
    assert dest.stat().st_size > 0
    gzip = tmp_path / "plain.con.gz"
    readcon.write_con(str(gzip), frames, compression="gzip")
    assert gzip.read_bytes()[:2] == b"\x1f\x8b"
    with pytest.raises(OSError, match="unknown compression: bzip2"):
        readcon.write_con(str(tmp_path / "nope.con"), frames, compression="bzip2")
    text = readcon.write_con_string(frames, precision=8, canonical=True)
    assert "Cu" in text
    first = readcon.read_first_frame(path)
    nth = readcon.read_nth_frame(path, 0)
    assert len(first) == len(nth)
    positions = readcon.read_positions(path, 2)
    assert positions.shape[-1] == 3
    probe = readcon.scan_probe(path, 2, True)
    assert probe[0] >= 1
    assert readcon.has_parallel_support() is True
    assert readcon.has_mpi_support() is False
    with pytest.raises(OSError, match="CON-bin"):
        readcon.read_conbin(path)
    if readcon.has_chemfiles_support():
        xyz = os.path.join(os.path.dirname(path), "water_min.xyz")
        loaded = readcon.read_chemfiles_first(xyz)
        assert loaded.atoms[0].symbol in ("O", "H")
        assert readcon.read_chemfiles_nsteps(xyz) >= 1
        chosen = readcon.select_atom_indices(first, "name Cu")
        assert list(chosen)
        picked = first.select_atoms("name Cu")
        assert list(picked)
    else:
        chemfiles = "chemfiles support is not enabled"
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.read_chemfiles(path)
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.read_chemfiles_first(path)
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.read_chemfiles_nth(path, 0)
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.read_chemfiles_nsteps(path)
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.read_chemfiles_memory("H 0 0 0\n", "XYZ")
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.select_on_frame(first, "name H")
        with pytest.raises(RuntimeError, match=chemfiles):
            readcon.select_atom_indices(first, "name H")
        with pytest.raises(RuntimeError, match=chemfiles):
            first.select("name H")
        with pytest.raises(RuntimeError, match=chemfiles):
            first.select_atoms("name H")
    with pytest.raises(ValueError, match="no atoms"):
        readcon.read_pdb(path)
    uneven = tmp_path / "uneven.con"
    wide = readcon.ConFrame(
        cell=[4.0, 4.0, 4.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[one_atom(), one_atom(symbol="O", atom_id=1, x=1.0)],
    )
    narrow = readcon.ConFrame(
        cell=[4.0, 4.0, 4.0],
        angles=[90.0, 90.0, 90.0],
        atoms=[one_atom()],
    )
    readcon.write_con(str(uneven), [wide, narrow])
    assert uneven.stat().st_size > 0
    with pytest.raises(ValueError, match="same atom count"):
        readcon.read_coords(str(uneven))


def test_sections_extxyz_pdb_conbin_and_ase(tmp_path):
    vel = readcon.read_con(resource("tiny_cuh2_vel_forces.con"))
    frame = vel[0]
    assert frame.vel is not None
    assert frame.frc is not None
    multi = readcon.read_con(resource("tiny_multi_cuh2.con"), 8)
    assert multi.release_frames() >= 0.0
    text = open(resource("tiny_multi_cuh2.con"), encoding="utf-8").read()
    from_text = readcon.read_con_string(text, 2)
    assert len(from_text) >= 2
    assert from_text.release_frames() >= 0.0
    ase_frames = readcon.read_con_as_ase(resource("tiny_cuh2.con"))
    assert ase_frames
    from ase import Atoms

    finite = Atoms("H", positions=[[0.0, 0.0, 0.0]], cell=[4.0, 4.0, 4.0], pbc=True)
    assert readcon.ConFrame.from_ase(finite).cell[0] > 0
    zero = Atoms("H", positions=[[0.0, 0.0, 0.0]], cell=[0.0, 0.0, 0.0], pbc=False)
    zero_frame = readcon.ConFrame.from_ase(zero)
    assert zero_frame.cell[0] == pytest.approx(0.0)
    bad = Atoms("H", positions=[[0.0, 0.0, 0.0]])
    bad.set_cell([[float("nan"), 0, 0], [0, 1, 0], [0, 0, 1]])
    with pytest.raises(ValueError, match="finite numbers"):
        readcon.ConFrame.from_ase(bad)
    ext = tmp_path / "one.extxyz"
    ext.write_text(
        "2\n"
        'Lattice="10 0 0 0 12 0 0 0 14" Properties=species:S:1:pos:R:3:masses:R:1 pbc="T T T"\n'
        "Cu 1.0 2.0 3.0 63.546\n"
        "Cu 4.0 5.0 6.0 63.546\n",
        encoding="utf-8",
    )
    loaded = readcon.read_extxyz(str(ext))
    positions, masses, cell = loaded["positions"], loaded["masses"], loaded["cell"]
    assert positions.shape == (1, 2, 3)
    assert masses.shape == (1, 2)
    assert cell.shape == (1, 6)
    pdb = tmp_path / "one.pdb"
    pdb.write_text(
        "CRYST1   10.000   12.000   14.000  90.00  90.00  90.00 P 1\n"
        "MODEL     1\n"
        "ATOM      1   Cu MOL     1       0.000   1.000   2.000  1.00  0.00          CU\n"
        "ATOM      2   Cu MOL     1       3.000   4.000   5.000  1.00  0.00          CU\n"
        "ENDMDL\n",
        encoding="utf-8",
    )
    positions, masses, cell = readcon.read_pdb(str(pdb))
    assert positions.shape[-1] == 3
    out = tmp_path / "tiny.conbin"
    n_frames, n_atoms = readcon.write_conbin(
        resource("tiny_cuh2.con"), str(out), binary32=False
    )
    assert n_frames >= 1 and n_atoms >= 1
    pos, velocity, atoms = readcon.read_conbin(str(out))
    assert pos.shape[0] == n_frames
    assert velocity.shape[0] == n_frames
    assert atoms == n_atoms
    narrow = tmp_path / "f32.conbin"
    readcon.write_conbin(resource("tiny_cuh2.con"), str(narrow), binary32=True)
    with pytest.raises(ValueError, match="binary32"):
        readcon.read_conbin(str(narrow))


def test_iterator_and_index_edges():
    text = open(resource("tiny_multi_cuh2.con"), encoding="utf-8").read()
    listed_from_text = readcon.read_con_string(text, 2)
    assert len(listed_from_text) >= 2
    iterator = readcon.iter_con(resource("tiny_multi_cuh2.con"), stream=True)
    assert iterator.forward() is True
    assert iterator.skip(1) == 1
    assert iterator.nth(0) is None
    mapped = readcon.iter_con(resource("tiny_cuh2.con"), mmap=True)
    assert mapped.forward() is True
    with pytest.raises(ValueError, match="mmap and stream cannot both be true"):
        readcon.iter_con(resource("tiny_cuh2.con"), mmap=True, stream=True)
    frames = list(readcon.read_con(resource("tiny_cuh2.con")))
    assert frames
    listed = readcon.read_con(resource("tiny_multi_cuh2.con"))
    assert listed[0] is not None
    assert listed[-1] is not None
    with pytest.raises(IndexError, match="out of range"):
        listed[99]
    assert len(listed[:]) >= 1
