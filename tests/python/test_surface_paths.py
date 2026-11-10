"""Bonds, forces, and CON-bin reads and writes."""

from pathlib import Path

import ase
import pytest
from ase.calculators.singlepoint import SinglePointCalculator

readcon = pytest.importorskip("readcon")


def test_forces_bonds_and_conbin(tmp_path: Path):
    atoms = ase.Atoms(
        "CuH",
        positions=[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        cell=[5.0, 5.0, 5.0],
        pbc=True,
    )
    atoms.set_velocities([[0.1, 0.0, 0.0], [0.0, 0.2, 0.0]])
    atoms.calc = SinglePointCalculator(
        atoms,
        energy=1.5,
        forces=[[0.2, 0.0, 0.0], [0.0, 0.1, 0.0]],
    )
    frame = readcon.ConFrame.from_ase(atoms)
    assert frame.has_forces
    assert frame.frc.shape == (2, 3)
    assert frame.vel.shape == (2, 3)
    frame.metadata = {"bonds": [[0, 1], {"i": 0, "j": 1, "order": 1}]}
    bonds = frame.bonds
    assert frame.has_bonds
    assert bonds[0] == (0, 1)
    assert bonds[1] == (0, 1, 1)
    frame.metadata = {"bonds": [{"i": 0}]}
    with pytest.raises(ValueError):
        frame.bonds
    frame.metadata = {"bonds": [[0, 1], {"i": 0, "j": 1, "order": 1}]}
    plain = tmp_path / "pair.con"
    readcon.write_con(str(plain), [frame], compression="none")
    text = readcon.write_con_string([frame], precision=6, canonical=True)
    assert "Cu" in text
    binary = tmp_path / "pair.conbin"
    n_frames, n_atoms = readcon.write_conbin(str(plain), str(binary))
    assert n_frames == 1
    assert n_atoms == 2
    positions, velocities, count = readcon.read_conbin(str(binary), n_threads=2)
    assert count == 2
    assert positions.shape[0] == 1
    assert velocities.shape[1] == 6
    binary32 = tmp_path / "pair32.conbin"
    readcon.write_conbin(str(plain), str(binary32), binary32=True)
    with pytest.raises(ValueError):
        readcon.read_conbin(str(binary32))
    still = readcon.ConFrame.from_ase(
        ase.Atoms("H", positions=[[0.0, 0.0, 0.0]], cell=[3.0, 3.0, 3.0], pbc=True)
    )
    quiet = tmp_path / "still.con"
    readcon.write_con(str(quiet), [still])
    still_bin = tmp_path / "still.conbin"
    readcon.write_conbin(str(quiet), str(still_bin))
    readcon.read_conbin(str(still_bin))
    with pytest.raises(OSError):
        readcon.read_conbin(str(tmp_path / "missing.conbin"))
    with pytest.raises(OSError):
        readcon.write_conbin(str(tmp_path / "missing.con"), str(tmp_path / "out.conbin"))
    assert readcon.has_mpi_support() in (True, False)
    assert readcon.count_frames(str(plain)) == 1
