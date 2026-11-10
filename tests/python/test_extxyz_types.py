"""read_extxyz returns atomic numbers with the symbol lookup used by read_con."""

import os

import numpy as np
import pytest

import readcon

SINGLE = """\
2
Lattice="10 0 0 0 12 0 0 0 14" Properties=species:S:1:pos:R:3:masses:R:1 pbc="T T T"
Cu 1.0 2.0 3.0 63.546
Cu 4.0 5.0 6.0 63.546
"""

MIXED = """\
2
Lattice="8 0 0 0 8 0 0 0 8" Properties=species:S:1:pos:R:3
Cu 1.0 0.0 0.0
H 0.0 2.0 0.0
2
Lattice="9 0 0 0 9 0 0 0 9" Properties=species:S:1:pos:R:3
O 0.0 0.0 3.0
Cu 4.0 0.0 0.0
"""

MALFORMED = """\
1
Lattice="10 0 0 0 10 0 0 0 10" Properties=species:S:1:pos:R:3
Nope 0.0 0.0 0.0
"""


def _write(tmp_path, name, text):
    path = os.path.join(tmp_path, name)
    with open(path, "w", encoding="utf-8") as handle:
        written = handle.write(text)
    assert written == len(text)
    return path


def test_single_species_types_match_the_file(tmp_path):
    path = _write(tmp_path, "single.xyz", SINGLE)
    positions, masses, cell, types = readcon.read_extxyz(path)
    assert positions.shape == (1, 2, 3)
    assert masses.shape == (1, 2)
    assert cell.shape == (1, 6)
    assert types.shape == (1, 2)
    assert types.dtype == np.uint64
    expected = np.array([[29, 29]], dtype=np.uint64)
    assert np.array_equal(types, expected)
    assert types[0, 0] == 29
    assert positions[0, 0, 0] == pytest.approx(1.0)
    assert cell[0, 0] == pytest.approx(10.0)
    assert cell[0, 3] == pytest.approx(90.0)


def test_mixed_species_frames_match_the_file(tmp_path):
    path = _write(tmp_path, "mixed.xyz", MIXED)
    positions, masses, cell, types = readcon.read_extxyz(path)
    assert positions.shape == (2, 2, 3)
    assert masses.shape == (2, 2)
    assert cell.shape == (2, 6)
    assert types.dtype == np.uint64
    assert types.tolist() == [[29, 1], [8, 29]]
    assert positions[1, 1, 0] == pytest.approx(4.0)
    assert cell[1, 0] == pytest.approx(9.0)


def test_malformed_species_is_an_error(tmp_path):
    path = _write(tmp_path, "bad.xyz", MALFORMED)
    with pytest.raises(ValueError, match="species"):
        readcon.read_extxyz(path)


NO_SPECIES = """\
1
Lattice="10 0 0 0 10 0 0 0 10" Properties=pos:R:3
1.0 2.0 3.0
"""


def test_missing_species_column_is_an_error(tmp_path):
    path = _write(tmp_path, "nospecies.xyz", NO_SPECIES)
    with pytest.raises(ValueError, match="species"):
        readcon.read_extxyz(path)


def test_non_utf8_species_is_an_error(tmp_path):
    path = os.path.join(tmp_path, "raw.xyz")
    body = (
        b'1\nLattice="10 0 0 0 10 0 0 0 10" Properties=species:S:1:pos:R:3\n'
        b"\xff 0.0 0.0 0.0\n"
    )
    with open(path, "wb") as handle:
        written = handle.write(body)
    assert written == len(body)
    with pytest.raises(ValueError, match="species"):
        readcon.read_extxyz(path)
