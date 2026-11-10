"""Scorer tests. They use fixture JSON, not a live llvm-cov profile."""

import importlib.util
import json
from pathlib import Path

import pytest

_PATH = Path(__file__).resolve().parents[2] / "scripts" / "physical_coverage.py"
_SPEC = importlib.util.spec_from_file_location("physical_coverage", _PATH)
assert _SPEC and _SPEC.loader
_MOD = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MOD)
ScoreError = _MOD.ScoreError
check_display = _MOD.check_display
display_count = _MOD.display_count
line_hits = _MOD.line_hits
load_inventory = _MOD.load_inventory
score_inventory = _MOD.score_inventory
union_lines = _MOD.union_lines


def _export(name, segments):
    return {"data": [{"files": [{"filename": name, "segments": segments}]}]}


def test_display_uses_a_suffix_past_the_table_when_the_count_is_huge():
    assert display_count(10**18) == "1.00E"
    assert display_count(10**21) == "1000E"


def test_a_segment_that_spans_lines_keeps_the_higher_count():
    export = _export(
        "src/ffi.rs",
        [
            [1, 1, 2, True, True, False],
            [4, 1, 9, True, True, False],
            [4, 2, 1, True, True, False],
        ],
    )
    hits = line_hits(export, "src/ffi.rs")
    assert hits[1] == 2
    assert hits[3] == 2
    assert hits[4] == 9
    second = _export(
        "src/ffi.rs",
        [[1, 1, 1, True, True, False], [2, 1, 0, False, False, False]],
    )
    both = {"data": export["data"] + second["data"]}
    assert line_hits(both, "src/ffi.rs")[1] == 2


def test_display_matches_the_observed_rounding():
    assert display_count(0) == "0"
    assert display_count(999) == "999"
    assert display_count(784346) == "784k"
    assert display_count(1566582) == "1.56M"
    assert display_count(10_500) == "10.5k"
    check_display(784346, "784k")
    with pytest.raises(ScoreError, match="negative"):
        display_count(-1)
    with pytest.raises(ScoreError, match="display"):
        check_display(10, "10k")


def test_union_keeps_one_line_and_the_highest_count():
    # A counted segment covers through the next segment's line.
    # Line 13 is zero in both objects, so it stays a miss.
    # Line 10 has counts 0 and 5; the line is covered.
    first = _export(
        "src/ffi.rs",
        [
            [10, 1, 0, True, True, False],
            [10, 4, 5, True, True, False],
            [11, 1, 0, False, False, False],
            [13, 1, 0, True, True, False],
            [14, 1, 0, False, False, False],
        ],
    )
    second = _export(
        "src/ffi.rs",
        [
            [10, 1, 0, True, True, False],
            [12, 1, 3, True, True, False],
            [12, 8, 3, False, False, False],
            [13, 1, 0, True, True, False],
            [14, 1, 0, False, False, False],
        ],
    )
    scored = union_lines([first, second], ["src/ffi.rs"])
    assert scored["src/ffi.rs"]["miss"] == [13, 14]
    assert scored["src/ffi.rs"]["hit"] == scored["src/ffi.rs"]["total"] - 2
    hits = line_hits(first, "src/ffi.rs")
    assert hits[10] == 5
    assert hits[11] == 5
    assert hits[13] == 0
    assert hits[14] == 0


def test_multiline_overlap_gap_and_wrong_source():
    spanned = _export(
        "src/ffi.rs",
        [
            [1, 1, 4, True, True, False],
            [3, 1, 9, True, True, False],
            [3, 8, 9, False, False, False],
            [8, 1, 0, True, True, True],
            [10, 1, 0, False, False, False],
        ],
    )
    hits = line_hits(spanned, "src/ffi.rs")
    assert hits[1] == 4
    assert hits[2] == 4
    assert hits[3] == 9
    assert 8 not in hits
    assert 9 not in hits
    assert 10 not in hits
    wrong = _export("src/ffi.rs.old", [[1, 1, 1, True, True, False]])
    with pytest.raises(ScoreError, match="missing source"):
        line_hits(wrong, "src/ffi.rs")


def test_rejects_a_missing_source_object_or_unknown_count(tmp_path: Path):
    with pytest.raises(ScoreError, match="no objects"):
        union_lines([], ["src/ffi.rs"])
    with pytest.raises(ScoreError, match="no sources"):
        union_lines([_export("src/ffi.rs", [])], [])
    with pytest.raises(ScoreError, match="missing source"):
        line_hits(_export("src/other.rs", []), "src/ffi.rs")
    with pytest.raises(ScoreError, match="no data"):
        line_hits({"data": []}, "src/ffi.rs")
    with pytest.raises(ScoreError, match="not a list"):
        line_hits({"data": [{"files": "no"}]}, "src/ffi.rs")
    with pytest.raises(ScoreError, match="short segment"):
        line_hits(_export("src/ffi.rs", [[1]]), "src/ffi.rs")
    with pytest.raises(ScoreError, match="unknown count"):
        line_hits(_export("src/ffi.rs", [[1, 1, True, True]]), "src/ffi.rs")
    with pytest.raises(ScoreError, match="negative"):
        line_hits(_export("src/ffi.rs", [[1, 1, -3, True]]), "src/ffi.rs")
    with pytest.raises(ScoreError, match="bad line"):
        line_hits(_export("src/ffi.rs", [["x", 1, 1, True]]), "src/ffi.rs")
    with pytest.raises(ScoreError, match="segments missing"):
        line_hits(
            {"data": [{"files": [{"filename": "src/ffi.rs"}]}]},
            "src/ffi.rs",
        )

    inventory = tmp_path / "inventory.json"
    inventory.write_text(json.dumps({"objects": [], "sources": ["src/ffi.rs"]}), encoding="utf-8")
    with pytest.raises(ScoreError, match="no objects"):
        load_inventory(inventory)
    inventory.write_text(json.dumps({"objects": [{"id": "a"}], "sources": []}), encoding="utf-8")
    with pytest.raises(ScoreError, match="no sources"):
        load_inventory(inventory)
    inventory.write_text(
        json.dumps({"objects": [{"id": "a"}], "sources": ["src/ffi.rs"]}),
        encoding="utf-8",
    )
    with pytest.raises(ScoreError, match="missing id"):
        load_inventory(inventory)
    export = tmp_path / "missing.json"
    inventory.write_text(
        json.dumps(
            {
                "objects": [{"id": "a", "digest": "abc", "export": str(export)}],
                "sources": ["src/ffi.rs"],
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(ScoreError, match="missing export"):
        load_inventory(inventory)
    export.write_text(
        json.dumps(_export("/work/src/ffi.rs", [[4, 1, 2, True]])),
        encoding="utf-8",
    )
    scored = score_inventory(inventory)
    assert scored["src/ffi.rs"]["hit"] == 1
    assert scored["src/ffi.rs"]["total"] == 1
