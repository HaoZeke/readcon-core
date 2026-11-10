"""Public inputs for the python error arms."""

import os

import pytest

import readcon

RESOURCES = os.path.join(os.path.dirname(__file__), "..", "..", "resources", "test")


def _resource(name):
    return os.path.join(RESOURCES, name)


class _BadComm:
    def py2f(self):
        return 2**31 - 1


def test_read_positions_rejects_a_missing_file(tmp_path):
    missing = tmp_path / "no-such.con"
    with pytest.raises(OSError, match="No such file"):
        readcon.read_positions(str(missing))


def test_a_text_forward_on_garbage_reports_the_skip_error(tmp_path):
    path = tmp_path / "garbage.con"
    path.write_text("this is not a con frame\n")
    iterator = readcon.iter_con(str(path))
    with pytest.raises(OSError, match="skip error"):
        iterator.forward()


def test_a_text_iterator_yields_the_first_symbol_and_skips_one():
    path = _resource("tiny_cuh2.con")
    iterator = readcon.iter_con(path)
    frame = next(iterator)
    assert frame.atoms[0].symbol == "Cu"
    assert iterator.forward() is False


def test_a_second_index_returns_the_same_symbol():
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    assert frames[0].atoms[0].symbol == frames[0].atoms[0].symbol == "Cu"


def test_array_rows_match_the_atoms_that_filled_them():
    frames = readcon.read_con(_resource("tiny_cuh2.convel"))
    frame = frames[0]
    velocities = frame.vel
    assert velocities is not None
    assert len(velocities) == len(frame)
    assert len(velocities[0]) == 3
    coords = frame.coords_array()
    assert len(coords) == len(frame)
    assert len(coords[0]) == 3


def test_a_convel_frame_exposes_its_velocity_rows():
    frames = readcon.read_con(_resource("tiny_cuh2.convel"))
    rows = frames[0].vel
    assert rows is not None
    assert len(rows) == len(frames[0])
    assert len(rows[0]) == 3


def test_release_frames_then_an_index_is_rejected():
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    assert frames.release_frames() >= 0.0
    with pytest.raises(IndexError, match="out of range for 0 frames"):
        frames[0]


def test_a_bond_dict_with_a_missing_i_and_a_bad_j_is_rejected():
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    frames[0].metadata["bonds"] = [{"j": 1}]
    with pytest.raises(ValueError, match="missing i"):
        frames[0].bonds
    frames[0].metadata["bonds"] = [{"i": 0, "j": "nope"}]
    with pytest.raises(Exception):
        frames[0].bonds


def test_a_precision_writer_rejects_a_missing_parent(tmp_path):
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    missing = tmp_path / "absent" / "out.con"
    with pytest.raises(OSError, match="failed to create writer"):
        readcon.write_con(str(missing), frames, precision=8, compression="none")


def test_a_frame_with_a_string_units_value_is_rejected(tmp_path):
    frames = readcon.read_con(_resource("tiny_cuh2.con"))
    frames[0].metadata["units"] = "angstrom"
    with pytest.raises(OSError, match="units"):
        frames[0].write_con(str(tmp_path / "bad.con"))


def test_a_bad_fortran_handle_is_rejected_when_mpi_is_built(tmp_path):
    path = _resource("tiny_cuh2.con")
    frames = readcon.read_con(path)
    try:
        readcon.read_con(path, comm=_BadComm())
    except NotImplementedError as err:
        assert "mpi feature" in str(err)
        return
    except OSError as err:
        assert "MPI_Comm_dup" in str(err)
    else:
        raise AssertionError("a bad communicator was accepted")
    out = tmp_path / "out.con"
    with pytest.raises(OSError, match="MPI_Comm_dup"):
        readcon.write_con(str(out), frames, comm=_BadComm(), compression="none")
