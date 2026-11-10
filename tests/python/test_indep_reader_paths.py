"""Exercise the independent reader, including the rejection branches."""

from __future__ import annotations

import gzip
import importlib.util
import math
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load():
    spec = importlib.util.spec_from_file_location(
        "indep_reader_paths", SCRIPTS / "indep_reader.py"
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


R = load()
MINIMAL = (ROOT / "resources/conformance/valid/v2_minimal.con").read_text(encoding="utf-8")


def rejects(fn, *args):
    with pytest.raises(R.Rejection):
        fn(*args)


def test_bytes_tokens_and_numbers():
    plain, kind = R.unwrap(b"abc")
    assert (plain, kind) == (b"abc", "none")
    packed = gzip.compress(b"frame")
    assert R.unwrap(packed) == (b"frame", "gzip")
    rejects(R.unwrap, R.GZIP_MAGIC + b"nope")
    zstd = pytest.importorskip("zstandard")
    blob = zstd.ZstdCompressor().compress(b"frame")
    assert R.unwrap(blob)[1] == "zstd"
    rejects(R.unwrap, R.ZSTD_MAGIC + b"xxxx")

    assert R.split_lines(b"a\r\nb") == ["a", "b"]
    assert R.split_lines(b"a\n") == ["a"]
    assert "\udcff" in R.split_lines(b"\xff")[0]
    assert R.blank(" \t") and not R.blank("a")
    assert R.white(" \u3000") and not R.white("a")
    assert R.fields(" a \tb ") == ["a", "b"]
    assert R.words("Of") == ["of"]
    assert R.label_ok("Coordinates of Component 1", "Coordinates", 1)
    assert not R.label_ok("nope", "Coordinates", 1)
    assert R.shown(2**200).startswith("<integer")
    assert R.shown("ok") == "'ok'"

    assert R.number("1.5", 1, "x") == 1.5
    rejects(R.number, "1e9999", 1, "x")
    rejects(R.number, "NaN", 1, "x")
    rejects(R.number, "nope", 1, "x")
    assert R.unsigned("007") == 7
    assert R.unsigned("1" * 21) is None
    assert R.unsigned(str(R.MAX_U64 + 1)) is None
    assert R.flag_of("3", 1, 2) == 3
    rejects(R.flag_of, "x", 1, 2)
    rejects(R.flag_of, "2", 1, 1)
    rejects(R.flag_of, "9", 1, 2)
    assert R.mask_of(1, 1) == (True, True, True)
    assert R.mask_of(5, 2) == (True, False, True)
    assert R.atom_id("4", 1) == 4
    rejects(R.atom_id, "x", 1)
    assert R.counts_of("1 2", 1, 2) == [1, 2]
    rejects(R.counts_of, "x", 1, 1)
    rejects(R.counts_of, "1", 1, 2)
    assert R.floats_of("1 2", 1, 2, "box") == [1.0, 2.0]
    rejects(R.floats_of, "1", 1, 2, "box")


def test_units_and_json():
    assert R.unit_dimension("angstrom*m/nm")[0] != 0
    assert R.unit_dimension("(eV)")
    for expr in (
        "",
        "angstrom)",
        "!",
        "furlong",
        "angstrom^.",
        "angstrom^^2",
        "(angstrom",
    ):
        with pytest.raises(ValueError):
            R.unit_dimension(expr)
    assert R.canonical_unit("  EV ")

    with pytest.raises(R.JsonError):
        R._json_string("no", 0)
    with pytest.raises(R.JsonError):
        R._json_string('"unterminated', 0)
    assert R._json_string(r'"a\n\u0041"', 0)[0] == "a\nA"
    with pytest.raises(R.JsonError):
        R._json_string('"\n"', 0)
    with pytest.raises(R.JsonError):
        R._json_string(r'"\q"', 0)
    with pytest.raises(R.JsonError):
        R._json_string(r'"\uDC00"', 0)
    with pytest.raises(R.JsonError):
        R._json_string(r'"\uD800"', 0)
    with pytest.raises(R.JsonError):
        R._json_string(r'"\uD800\u0041"', 0)
    assert R._json_string(r'"\uD800\uDC00"', 0)[0] == "\U00010000"
    with pytest.raises(R.JsonError):
        R._json_number("x", 0)
    with pytest.raises(R.JsonError):
        R._json_number("1e9999", 0)
    assert R._json_number("1" * 40, 0)[0] == int("1" * 40)
    with pytest.raises(R.JsonError):
        R._json_colon("  x", 0)
    assert R.parse_json(" true ") is True
    assert R.parse_json("false") is False
    assert R.parse_json("null") is None
    assert R.parse_json("[1, 2]") == [1, 2]
    assert R.parse_json('{"a": []}') == {"a": []}
    with pytest.raises(R.JsonError):
        R.parse_json("")
    with pytest.raises(R.JsonError):
        R.parse_json("1 2")
    with pytest.raises(R.JsonError):
        R.parse_json("{" * (R.MAX_DEPTH + 1) + "}" * (R.MAX_DEPTH + 1))
    with pytest.raises(R.JsonError):
        R.parse_json("[1 2]")

    assert R.is_number(1.0) and not R.is_number(True)
    assert R.is_index(0) and not R.is_index(-1) and not R.is_index(1.0)
    assert R.finite(1.0) and not R.finite(math.inf) and not R.finite("1")
    assert R.finite(1)
    assert R.finite(10**400) is False


def test_metadata_cell_and_frames():
    rejects(R.check_units, {}, 3, 2)
    assert R.check_units({}, 2, 2) == {}
    rejects(R.check_units, {"units": []}, 2, 2)
    rejects(R.check_units, {"units": {}}, 3, 2)
    rejects(R.check_units, {"units": {"length": 1, "energy": "eV"}}, 3, 2)
    rejects(R.check_units, {"units": {"length": "furlong", "energy": "eV"}}, 3, 2)
    rejects(R.check_units, {"units": {"length": "eV", "energy": "eV"}}, 3, 2)
    units = R.check_units({"units": {"length": "angstrom", "energy": "eV"}}, 3, 2)
    assert units["length"] == "angstrom"

    rejects(R.check_bonds, "no", 2)
    rejects(R.check_bonds, ["no"], 2)
    rejects(R.check_bonds, [[1]], 2)
    rejects(R.check_bonds, [{"i": 1}], 2)
    R.check_bonds([[0, 1], {"i": 0, "j": 1, "order": 1}], 2)

    rejects(R.check_arity, [], 2)
    rejects(R.check_arity, {"velocities": -1}, 2)
    rejects(R.check_arity, {"velocities": 0}, 2)
    rejects(R.check_arity, {"velocities": 1}, 2)
    R.check_arity({"velocities": 3, "note": 2}, 2)

    rejects(R.check_reserved_keys, {"con_spec_minor": -1}, 2)
    rejects(R.check_reserved_keys, {"pbc": [True, True]}, 2)
    rejects(R.check_reserved_keys, {"lattice_vectors": [[1, 2, 3]]}, 2)
    rejects(R.check_reserved_keys, {"energy": "no"}, 2)
    rejects(R.check_reserved_keys, {"frame_index": -1}, 2)
    rejects(R.check_reserved_keys, {"generator": 1}, 2)
    rejects(R.check_reserved_keys, {"potential": []}, 2)
    rejects(R.check_reserved_keys, {"converged": 1}, 2)
    R.check_reserved_keys(
        {
            "con_spec_minor": 1,
            "pbc": [True, False, True],
            "energy": 1.0,
            "frame_index": 0,
            "generator": "t",
            "potential": {},
            "converged": False,
        },
        2,
    )

    rejects(R.spec_major, "2", 2)
    rejects(R.spec_major, 2.5, 2)
    rejects(R.spec_major, 1, 2)
    rejects(R.spec_major, 9, 2)
    assert R.spec_major(2, 2) == 2

    assert R.metadata("  not json", 2, False) is None
    rejects(R.metadata, "\ud800", 2, False)
    rejects(R.metadata, "{\ud800}", 2, False)
    rejects(R.metadata, "{", 2, False)
    rejects(R.metadata, "{}", 2, False)
    rejects(R.metadata, '{"con_spec_version": 2, "validate": 1}', 2, False)
    rejects(R.metadata, '{"con_spec_version": 2, "sections": [1]}', 2, False)
    rejects(R.metadata, '{"con_spec_version": 2, "validate": true}', 2, False)
    meta = R.metadata(
        '{"con_spec_version": 3, "units": {"length": "angstrom", "energy": "eV"},'
        ' "sections": [], "validate": true, "bonds": [[0, 1]]}',
        2,
        False,
    )
    assert meta["validate"] is True
    assert R.cell_matrix([1, 1, 1], [90, 90, 90]) is not None
    R.cell_matrix([1, 1, 1], [0, 0, 0])
    R.cell_matrix([1, 1, 1], [90, 90, 0])

    lines = R.Lines(["a", " \t", ""])
    assert lines.text(0) == "a"
    bad = R.Lines(["\ud800"])
    rejects(bad.text, 0)
    assert lines.tail(1)

    frames = R.read_text(MINIMAL)
    assert len(frames) == 1
    corpus = list((ROOT / "resources/conformance").rglob("*.con"))
    corpus += list((ROOT / "resources/test").glob("*.con"))
    rejected = 0
    for path in corpus:
        for validate in (False, True):
            try:
                R.read_path(path, validate)
            except R.Rejection:
                rejected += 1
    assert rejected
    assert R.main([str(corpus[0])]) == 0
    invalid = next(p for p in corpus if "invalid" in p.as_posix())
    assert R.main(["--validate", str(invalid)]) == 0
    packed = gzip.compress(MINIMAL.encode())
    assert R.read_bytes(packed)
    assert R.read_lines(R.split_lines(MINIMAL.encode()))

    assert R.unit_dimension("angstrom^-1")
    deep = "1"
    for _ in range(R.MAX_DEPTH + 2):
        deep = '{"a": ' + deep + "}"
    with pytest.raises(R.JsonError):
        R.parse_json(deep)
    rejects(R.check_reserved_keys, {"section_arity": []}, 2)
    rejects(R.row, "1 2", 1, 3, 2, False, "atom")
    rejects(R.row, "1", 1, 3, 2, True, "atom")

    def con(meta: str, rest: list[str]) -> str:
        return "\n".join(["t", meta, "10 10 10", "90 90 90", "", "", *rest]) + "\n"

    body = ["1", "1", "1", "Cu", "Coordinates of Component 1", "0 0 0 0 1"]
    rejects(R.read_text, "\ufeff" + MINIMAL)
    rejects(
        R.read_text,
        con(
            '{"con_spec_version":3,"con_spec_minor":1,"units":{"length":"angstrom","energy":"eV"},"sections":["velocities"]}',
            body,
        ),
    )
    rejects(R.read_text, con('{"con_spec_version":2,"validate":true}', ["1", "1", "-1", "Cu", "Coordinates of Component 1", "0 0 0 0 1"]), )
    rejects(R.read_text, con('{"con_spec_version":2,"validate":true}', ["1", "1", "1", "Zz", "Coordinates of Component 1", "0 0 0 0 1"]))
    rejects(R.read_text, con('{"con_spec_version":2}', ["1", "1", "1", "Cu", "Coordinates of Component 9", "0 0 0 0 1"]))
    rejects(R.read_text, con('{"con_spec_version":2}', ["1", "1", "1", "Cu"]))
    zero = con('{"con_spec_version":3,"units":{"length":"angstrom","energy":"eV"},"sections":["forces"]}', ["0", "", ""])
    rejects(R.read_text, zero)
    rejects(R.read_text, con('{"con_spec_version":2}', ["0", "nope", ""]))
    R.read_text(con('{"con_spec_version":2}', ["0", "", ""]))
    vel = body + ["", "Cu", "Velocities of Component 1", "0 0 0 0 1"]
    got = R.read_text(con('{"con_spec_version":2,"units":{"time":"fs"}}', vel))
    assert got[0]["legacy_velocities"] or got[0]["time_override"]
    extra = body + ["", "Cu", "Extra of Component 1", "0 0 1"]
    R.read_text(
        con(
            '{"con_spec_version":3,"con_spec_minor":2,"units":{"length":"angstrom","energy":"eV","time":"fs"},'
            '"sections":["extra"],"section_arity":{"extra":1}}',
            extra,
        )
    )
    spreads = body + ["", "Cu", "Spreads of Component 1", "-1 0 0 0 1"]
    rejects(
        R.read_text,
        con(
            '{"con_spec_version":3,"units":{"length":"angstrom","energy":"eV","time":"fs"},"sections":["spreads"],"validate":true}',
            spreads,
        ),
    )


def _head(meta: str, lengths: str, angles: str, rest: list[str]) -> str:
    return "\n".join(["t", meta, lengths, angles, "", "", *rest]) + "\n"


def test_section_and_header_rejections():
    base = {
        "major": 2,
        "validate": True,
        "counts": [1],
        "types": [{"symbol": "Cu"}],
        "flags": [0],
        "ids": [1],
        "sections": {},
    }
    rejects(R.read_block, R.Lines([]), 0, "forces", 3, True, base)
    rejects(R.read_block, R.Lines(["nope"]), 0, "forces", 3, True, base)
    rejects(R.read_block, R.Lines(["", "H"]), 0, "forces", 3, True, base)
    rejects(R.read_block, R.Lines(["", "Cu"]), 0, "forces", 3, True, base)
    rejects(
        R.read_block,
        R.Lines(["", "Cu", "Nope of Component 1"]),
        0,
        "forces",
        3,
        True,
        base,
    )
    loose = dict(base, validate=False)
    rejects(
        R.read_block,
        R.Lines(["", "Cu", "Nope of Component 1"]),
        0,
        "forces",
        3,
        True,
        loose,
    )
    rejects(
        R.read_block,
        R.Lines(["", "Cu", "Forces of Component 1"]),
        0,
        "forces",
        3,
        True,
        base,
    )
    rejects(
        R.read_block,
        R.Lines(["", "Cu", "Forces of Component 1", "0 0 0 7 9"]),
        0,
        "forces",
        3,
        True,
        base,
    )
    rejects(
        R.read_block,
        R.Lines(["", "Cu", "Extra of Component 1", "0"]),
        0,
        "extra",
        1,
        False,
        loose,
    )
    lines = R.Lines(["", "Cu", "Velocities of Component 1", "0 0 0"])
    assert R.legacy_block(lines, 0, [1]) is False
    body = ["1", "1", "0", "Cu", "Coordinates of Component 1", "0 0 0 0 1"]
    rejects(R.read_text, _head('{"con_spec_version":2,"validate":true}', "0 10 10", "90 90 90", body))
    rejects(R.read_text, _head('{"con_spec_version":2,"validate":true}', "10 10 10", "0 90 90", body))
    rejects(R.read_text, _head('{"con_spec_version":2}', "10 10 10", "0 0 0", ["1", "1", "1", "Cu", "Coordinates of Component 1", "0 0 0 0 1"]))
    rejects(R.read_text, _head('{"con_spec_version":2,"validate":true}', "10 10 10", "90 90 90", ["1", "1", "0", "Cu", "Coordinates of Component 1", "0 0 0 0 1"]))
    rejects(R.read_text, _head('{"con_spec_version":2}', "10 10 10", "90 90 90", ["1", "1", "1"]))
    rejects(R.read_text, _head('{"con_spec_version":2}', "10 10 10", "90 90 90", ["1", "1", "1", "Cu", "Coordinates of Component 1"]))
    rejects(R.read_text, "only\n")
    legacy = ["1", "1", "1", "Cu", "Coordinates of Component 1", "0 0 0 0 1", "", "Cu", "Velocities of Component 1", "0 0 0 0 1"]
    rejects(
        R.read_text,
        _head(
            '{"con_spec_version":3,"con_spec_minor":1,"units":{"length":"angstrom","energy":"eV"}}',
            "10 10 10",
            "90 90 90",
            legacy,
        ),
    )
    for gamma in (1e-320, 1e-310, 1e-16):
        R.cell_matrix([1, 1, 1], [0, 90, gamma])


def test_reader_main(monkeypatch):
    import runpy

    path = ROOT / "resources/conformance/valid/v2_minimal.con"
    monkeypatch.setattr(sys, "argv", ["indep_reader.py", str(path)])
    with pytest.raises(SystemExit) as caught:
        runpy.run_path(str(SCRIPTS / "indep_reader.py"), run_name="__main__")
    assert caught.value.code == 0
