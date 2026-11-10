"""Public Python calls for arms the merged report left open."""

from pathlib import Path

import pytest

import readcon
from ase import Atoms

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "resources" / "test"


def test_bond_dicts_and_metadata_json_round_trip(tmp_path):
    frame = readcon.read_con(str(FIXTURE / "tiny_cuh2.con"))[0]
    frame.metadata["bonds"] = [
        {"i": 0, "j": 1, "order": 2},
        {"i": 1, "j": 2},
        [0, 1],
    ]
    bonds = frame.bonds
    assert bonds[0] == (0, 1, 2)
    assert bonds[1] == (1, 2)
    assert bonds[2][0] == 0
    frame.set_metadata_json('{"energy": 1.5, "con_spec_version": 2, "sections": []}')
    assert frame.metadata["energy"] == 1.5
    assert "sections" not in frame.metadata
    out = tmp_path / "one.con"
    frame.write_con(str(out))
    assert out.is_file()
    missing = tmp_path / "absent" / "one.con"
    with pytest.raises(OSError):
        frame.write_con(str(missing))


def test_from_ase_rejects_a_nonfinite_cell():
    atoms = Atoms("H", positions=[[0, 0, 0]], cell=[[1, 0, 0], [0, 1, 0], [0, 0, 1]], pbc=True)
    atoms.set_cell([[float("nan"), 0, 0], [0, 1, 0], [0, 0, 1]], scale_atoms=False)
    with pytest.raises(ValueError, match="finite"):
        readcon.ConFrame.from_ase(atoms)


def test_chemfiles_format_and_a_trajectory_index(tmp_path):
    if not readcon.has_chemfiles_support():
        pytest.fail("the coverage extension is built with chemfiles")
    path = FIXTURE / "water_min.xyz"
    loaded = readcon.read_chemfiles(str(path), format="XYZ", guess_bonds=True)
    assert len(loaded) == 1
    assert len(loaded[0]) == 3
    text = (FIXTURE / "tiny_cuh2.con").read_text()
    out = tmp_path / "tiny.con"
    out.write_text(text)
    traj = readcon.ConTrajectory(str(out))
    assert len(traj) >= 1
    with pytest.raises(IndexError):
        traj[99]
    stream = readcon.iter_con(str(out), stream=True)
    first = next(stream)
    assert len(first) >= 1


def test_a_communicator_without_mpi_is_rejected():
    if readcon.has_mpi_support():
        return
    with pytest.raises(NotImplementedError, match="mpi"):
        readcon.read_con(str(FIXTURE / "tiny_cuh2.con"), comm=object())
