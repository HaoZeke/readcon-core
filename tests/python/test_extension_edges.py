"""Observable edges of the extension that the CON fixtures do not take."""

import os

import pytest

import readcon


def test_feature_flags_match_this_build():
    assert readcon.has_mpi_support() is False
    assert readcon.has_parallel_support() is True
    assert readcon.symbol_to_atomic_number("Cu") == 29
    assert readcon.symbol_to_atomic_number("nope") == 0
    assert readcon.atomic_number_to_symbol(29) == "Cu"
    assert readcon.atomic_number_to_symbol(0) == "X"


def test_chemfiles_entry_points_report_the_missing_feature():
    frame = readcon.read_first_frame("resources/test/tiny_cuh2.con")
    calls = [
        lambda: readcon.read_chemfiles("resources/test/water_min.xyz", step=2, skip=1),
        lambda: readcon.read_chemfiles_first("resources/test/water_min.xyz"),
        lambda: readcon.read_chemfiles_nth("resources/test/water_min.xyz", 0),
        lambda: readcon.read_chemfiles_nsteps("resources/test/water_min.xyz"),
        lambda: readcon.read_chemfiles_memory("3\n\nO 0 0 0\nH 0 0 1\nH 0 1 0\n", "XYZ", guess_bonds=True),
        lambda: readcon.select_on_frame(frame, "name H"),
        lambda: readcon.select_atom_indices(frame, "name H"),
        lambda: readcon.evaluate_selection_on_frames("name H", [frame]),
        lambda: readcon.select_atom_positions_on_frames("name H", [frame]),
    ]
    if readcon.has_chemfiles_support():
        first = readcon.read_chemfiles_first("resources/test/water_min.xyz")
        assert first.atoms[0].symbol in ("O", "H")
        assert readcon.read_chemfiles_nsteps("resources/test/water_min.xyz") >= 1
        selected = readcon.select_on_frame(frame, "name H")
        assert selected is not None
        indices = readcon.select_atom_indices(frame, "name Cu")
        assert indices
    else:
        for call in calls:
            with pytest.raises(RuntimeError, match="chemfiles"):
                call()


def test_two_workers_release_a_multi_frame_list():
    frames = readcon.read_con("resources/test/tiny_multi_cuh2.con", n_threads=2)
    assert len(frames) == 2
    assert frames[0].atoms[0].symbol == "Cu"
    assert frames[1].atoms[0].symbol
    positions = readcon.read_positions("resources/test/tiny_multi_cuh2.con", n_threads=2)
    assert positions.shape[0] == 2
    assert positions.shape[2] == 3


def test_extxyz_without_a_lattice_is_rejected_and_ase_reads_con():
    with pytest.raises(ValueError, match="lattice"):
        readcon.read_extxyz("resources/test/water_min.xyz")
    atoms = readcon.read_con_as_ase("resources/test/tiny_cuh2.con")
    assert len(atoms) == 1
    assert len(atoms[0]) == 4
    assert atoms[0].get_chemical_symbols()[0] == "Cu"


def test_convert_and_index_errors_name_the_cause():
    if readcon.has_chemfiles_support():
        report = readcon.convert_to_con(
            "resources/test/water_min.xyz", "resources/test/out-water.con"
        )
        assert report["n_frames"] >= 1
        os.remove("resources/test/out-water.con")
    else:
        with pytest.raises(OSError):
            readcon.convert_to_con("resources/test/water_min.xyz", "out-unused.con")
    with pytest.raises(OSError, match="out of range"):
        readcon.read_nth_frame("resources/test/tiny_cuh2.con", 5)
    assert readcon.count_frames("resources/test/tiny_cuh2.con") == 1
