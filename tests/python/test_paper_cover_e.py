"""Real outputs for the five lowest paper scripts."""

from __future__ import annotations

import gzip
import importlib.util
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"
CPC = ROOT / "paper" / "cpc"
TINY = ROOT / "resources" / "test" / "tiny_cuh2.con"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("covere_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def _release_library() -> Path:
    """The release cdylib. Build it when the tree has not produced one yet."""
    import os
    import subprocess

    override = os.environ.get("READCON_LIB")
    if override:
        lib = Path(override)
        if lib.is_file():
            return lib
    candidates = [
        ROOT / "target" / "release" / "libreadcon_core.so",
        ROOT / "target" / "release" / "libreadcon_core.dylib",
        ROOT / "target" / "release" / "readcon_core.dll",
    ]
    for lib in candidates:
        if lib.is_file():
            return lib
    subprocess.run(
        ["cargo", "build", "--release", "--locked", "--lib"],
        cwd=ROOT,
        check=True,
    )
    for lib in candidates:
        if lib.is_file():
            return lib
    raise AssertionError("cargo build --release --lib did not write the release library")


def test_c_library_reads_a_real_file_and_rejects_a_missing_one(tmp_path, monkeypatch):
    paths = load("corpus_paths.py")
    lib = _release_library()
    plain = tmp_path / "tiny.con"
    plain.write_bytes(TINY.read_bytes())
    mapped = tmp_path / "tiny.mapped.con"
    mapped.write_bytes(plain.read_bytes())
    gz = tmp_path / "tiny.con.gz"
    gz.write_bytes(gzip.compress(plain.read_bytes(), mtime=0))
    text = tmp_path / "tiny.txt"
    text.write_bytes(plain.read_bytes())
    job = {
        "id": "0",
        "text": str(text),
        "plain": str(plain),
        "mapped": str(mapped),
        "gzip": str(gz),
    }
    got = paths.clib_child("clib", lib, [job])["0"]
    assert got["clib/file/1"].startswith("accept ")
    assert got["clib/count/serial"].startswith("accept ")
    missing = tmp_path / "absent.con"
    bad = dict(job, plain=str(missing), mapped=str(missing), gzip=str(missing))
    rejected = paths.clib_child("clib", lib, [bad])["0"]
    assert rejected["clib/file/1"].startswith("reject")

    class Boom(BaseException):
        pass

    import types

    fake = types.ModuleType("readcon")

    def boom(*args, **kwargs):
        raise Boom("stop")

    fake.read_con_string = boom
    fake.read_con = boom
    fake.iter_con = boom
    fake.count_frames = boom
    monkeypatch.setitem(sys.modules, "readcon", fake)
    utf = dict(job, utf8=True, text=str(text))
    with pytest.raises(Boom):
        paths.wheel_child([utf])

    one = {
        "corpus": "corpus31",
        "path": plain,
        "expected": "accept",
        "frames": None,
        "error": None,
        "line": None,
    }
    job_row = {"id": "0", "utf8": False, "compressed": None, "added": 0, "padding": "front"}
    got_map = {}
    for build in paths.BUILDS:
        got_map[build] = {"0": {key: "accept 2" for key in paths.read_keys(build)}}
    summary = paths.results([job_row], [one], got_map)
    assert summary["totals"]["skipped"] >= 1
    disagree = dict(job_row, utf8=True)
    split = {}
    for build in paths.BUILDS:
        row = {}
        for key in paths.read_keys(build):
            row[key] = "accept 2" if "string" not in key else "accept 9"
        split[build] = {"0": row}
    split_summary = paths.results([disagree], [one], split)
    assert split_summary["totals"]["files_differ"] == ["tiny.con"]
    record = {
        "results": summary,
        "provenance": {
            "build": {"library_digest": "abc"},
            "c_libraries": {"clib": {"library_digest": "def"}},
        },
    }
    compared = paths.comparable(record)
    assert compared["digests"] == ["abc", "def"]


def test_crosscheck_counts_the_real_corpus_and_one_row(tmp_path, monkeypatch):
    cross = load("corpus_crosscheck.py")
    spec = cross.spec_record()
    assert spec["clauses"] > 0
    assert spec["manifest_reader_minor"] == 1 or isinstance(spec["manifest_reader_minor"], int)
    features = cross.corpus31_features()
    assert features["files"] > 0
    frame = {
        "positions": [[0.0, 0.0, 0.0]],
        "masks": [[False, False, True]],
        "ids": [7],
        "velocities": None,
        "major": 3,
        "minor": 1,
        "declared_time": "fs",
        "time_unit": "fs",
        "cell_lengths": [10.0, 10.0, 10.0],
        "cell_angles": [90.0, 90.0, 90.0],
        "lattice_vectors": None,
        "pbc": [True, True, True],
        "symbols": ["H"],
        "masses": [1.008],
    }
    viewed = cross.fields(frame)
    assert "version" not in viewed
    assert viewed["ids"] == [7]
    sample = next((CPC / "data" / "corpus31").glob("*.con"))
    entry = {
        "path": sample,
        "group": "corpus31 accept",
        "basis": "explicit",
        "expected": "accept",
        "frames": 1,
        "error": None,
        "line": None,
        "pbc": None,
        "fixed": None,
        "time_unit_override": False,
        "rewrite": False,
        "clause": "CON-line.1",
    }
    monkeypatch.setattr(cross, "corpus", lambda: [entry])
    monkeypatch.setattr(sys, "argv", ["corpus_crosscheck.py", "--out", str(tmp_path)])
    cross.main()
    record = json.loads((tmp_path / "corpus_crosscheck.json").read_text(encoding="utf-8"))
    assert record["groups"]["all"]["files"] == 1
    assert (tmp_path / "corpus_crosscheck.tex").is_file()


def test_cloud_tables_reject_a_bad_kernel_rerun(tmp_path, monkeypatch):
    cloud = load("gen_cloud_tables.py")
    assert cloud.ratio("3", "2") == "1.50"
    assert cloud.slug("CON 3.1") == "CON_3_1"
    monkeypatch.setattr(cloud, "KERNELS", tmp_path)
    with pytest.raises(SystemExit, match="nehalem"):
        cloud.kernel_runs([{"route": "kept", "error": "", "steps": "1"}])
    (tmp_path / "nehalem.json").write_text(
        json.dumps({"routes": {"kept": {"steps": 9, "file_dpos_A": 0, "d_barrier_eV": 0, "max_disp_A": 0}}}),
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="steps"):
        cloud.kernel_runs(
            [{"route": "kept", "error": "", "steps": "1", "file_dpos_A": "0", "d_barrier_eV": "0", "max_disp_A": "0"}]
        )
    notes: dict[str, str] = {}
    row = {
        "route": "broken",
        "error": "absent.con",
        "steps": "1",
        "converged": "True",
        "free_modes_mid": "1",
        "file_dpos_A": "0",
        "d_barrier_eV": "0",
        "max_disp_A": "0",
        "force_calls": "1",
    }
    _columns, lines = cloud.restart_rows([row], notes, 1.0, False, {}, {}, None)
    assert any("no file" in line for line in lines)
    stopped = dict(
        row,
        error="",
        converged="False",
        steps="4",
        barrier_eV="1",
        force_calls="1",
        kept="mask id",
        file_version="none",
    )
    runs = {
        "r": {
            "routes": {
                "broken": {
                    "file_dpos_A": 0.0,
                    "d_barrier_eV": 0.0,
                    "max_disp_A": 0.0,
                }
            }
        }
    }
    _columns, text = cloud.restart_rows([stopped], {}, 1.0, False, {}, runs, 4)
    assert any("not converged" in line for line in text)
    monkeypatch.setattr(cloud, "WALLS", tmp_path / "walls")
    assert cloud.restart_walls() == ({}, 0)


def test_flat_source_copies_a_bibliography(tmp_path):
    em = load("make_em_flat.py")
    stage = tmp_path / "stage"
    stage.mkdir()
    (stage / "rg_main.tex").write_text("\\" + "bibliography{rg_main}\n", encoding="utf-8")
    (stage / "rg_main.bbl").write_text("\\bibitem{a}\n", encoding="utf-8")
    (stage / "note.txt").write_text("plain\n", encoding="utf-8")
    spec = tmp_path / "spec.org"
    spec.write_text("spec\n", encoding="utf-8")
    real = em.arxiv.SPEC_ORG
    em.arxiv.SPEC_ORG = spec
    try:
        names = em.write_flat(stage, tmp_path / "flat", tmp_path / "snap", {"rewrites": [], "notes": []})
    finally:
        em.arxiv.SPEC_ORG = real
    assert "rg_main.bbl" in names
    assert (tmp_path / "flat" / "rg_main.bbl").read_text(encoding="utf-8").startswith("\\bibitem")


def test_failure_matrix_reads_a_version_and_a_shift():
    matrix = load("failure_matrix.py")
    import numpy as np

    missing = matrix.version_carried("CON 3.1", Path("no-such.con"))
    assert missing == "no file"
    bad = Path("paper/cpc/data/sizes.csv")
    # a non-json second line is not a version
    assert matrix.version_carried("CON 3.1", CPC / "data" / "sizes.csv") == "none"
    fx = {"pos": np.array([[0.0, 0.0, 0.0]])}
    shifted = matrix.shift(fx, np.array([[1.0, 0.0]]))
    assert shifted["dpos"] != shifted["dpos"]
    same = matrix.shift(fx, np.array([[0.0, 0.0, 0.25]]))
    assert same["dpos"] == pytest.approx(0.25)
    cif = CPC / "data" / "hcn.con"
    # version_carried for CIF reads the first line only
    marker = tmp_path_file(CPC)
    assert matrix.version_carried("CIF", cif) in {"none", cif.read_text(encoding="utf-8").splitlines()[0]}
    assert matrix.time_unit_fs(None) is None
    graded = matrix.grade(
        {
            "mask": __import__("numpy").array([[True, False, False]]),
            "ids": __import__("numpy").array([1]),
            "symbols": ["H"],
            "pos": __import__("numpy").zeros((1, 3)),
            "vel": __import__("numpy").zeros((1, 3)),
            "cell": __import__("numpy").eye(3),
            "pbc": [True, True, True],
            "masses": __import__("numpy").array([1.0]),
        },
        {
            "mask": __import__("numpy").array([[True, False, False]]),
            "labels": ["H1"],
            "ids": __import__("numpy").array([1]),
            "pos": __import__("numpy").zeros((1, 3)),
            "cell": __import__("numpy").eye(3),
            "pbc": [True, True, True],
        },
    )
    assert graded["R2"] == "label"
    try:
        raise RuntimeError("boom")
    except RuntimeError as exc:
        exc.__traceback__ = type(exc.__traceback__)(
            None, None, None
        ) if False else exc.__traceback__
    located = matrix.where(RuntimeError("x"))
    assert located == ""


def test_corpus_paths_measure_and_a_disagreeing_check(tmp_path, monkeypatch):
    paths = load("corpus_paths.py")
    lib = tmp_path / "lib.so"
    lib.write_bytes(b"so")
    monkeypatch.setattr(paths, "CLIB", lib)
    monkeypatch.setattr(paths, "CLIB_PARALLEL", lib)
    monkeypatch.setattr(paths, "map_bytes", lambda: 64)
    monkeypatch.setattr(paths.provenance, "newer_sources", lambda path: ["src/ffi.rs"])
    with pytest.raises(SystemExit, match="older"):
        paths.measure()
    monkeypatch.setattr(paths.provenance, "newer_sources", lambda path: [])
    monkeypatch.setattr(
        paths.provenance,
        "readcon_build",
        lambda: {"library_digest": "a", "newer_sources": ["src/ffi.rs"]},
    )
    with pytest.raises(SystemExit, match="readcon module"):
        paths.measure()
    monkeypatch.setattr(
        paths.provenance,
        "readcon_build",
        lambda: {"library_digest": "a", "newer_sources": []},
    )
    monkeypatch.setattr(paths.provenance, "binary_build", lambda path: {"library_digest": "b"})
    monkeypatch.setattr(paths.provenance, "host", lambda: {"cpu": "guest"})
    monkeypatch.setattr(paths.provenance, "portable", lambda path: "manifest.toml")
    monkeypatch.setattr(paths, "corpus", lambda: [])
    monkeypatch.setattr(paths, "prepare", lambda files, work: [])
    monkeypatch.setattr(paths, "run_child", lambda build, jobs: {})
    record = paths.measure()
    assert record["results"]["totals"]["files"] == 0
    monkeypatch.setattr(paths, "measure", lambda: record)
    monkeypatch.setattr(paths, "MACROS", tmp_path / "m.tex")
    monkeypatch.setattr(paths, "TABLE", tmp_path / "t.tex")
    out = tmp_path / "out.json"
    out.write_text(
        json.dumps(
            {
                "results": {"totals": {"files": 9}},
                "provenance": record["provenance"],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(sys, "argv", ["corpus_paths.py", "--check", "--out", str(out)])
    assert paths.main() == 1
    out.write_text(json.dumps(record), encoding="utf-8")
    assert paths.main() == 1
    (tmp_path / "m.tex").write_text(paths.macros(record["results"]), encoding="utf-8")
    (tmp_path / "t.tex").write_text(paths.table(record["results"]), encoding="utf-8")
    assert paths.main() == 0


def test_cloud_tables_reject_a_residue_and_a_cap(monkeypatch, tmp_path):
    cloud = load("gen_cloud_tables.py")
    monkeypatch.setattr(cloud, "KERNELS", tmp_path)
    (tmp_path / "nehalem.json").write_text(
        json.dumps(
            {
                "routes": {
                    "kept": {
                        "steps": 1,
                        "file_dpos_A": 1.0,
                        "d_barrier_eV": 0.0,
                        "max_disp_A": 0.0,
                    }
                }
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="nehalem"):
        cloud.kernel_runs(
            [
                {
                    "route": "kept",
                    "error": "",
                    "steps": "1",
                    "file_dpos_A": "0",
                    "d_barrier_eV": "0",
                    "max_disp_A": "0",
                }
            ]
        )
    row = {
        "route": "broken",
        "error": "",
        "steps": "3",
        "converged": "False",
        "free_modes_mid": "1",
        "file_dpos_A": "0",
        "d_barrier_eV": "0",
        "max_disp_A": "0",
        "kept": "mask",
        "file_version": "none",
    }
    runs = {
        "r": {
            "routes": {
                "broken": {"file_dpos_A": 0.0, "d_barrier_eV": 0.0, "max_disp_A": 0.0}
            }
        }
    }
    with pytest.raises(SystemExit, match="not converged"):
        cloud.restart_rows([row], {}, 1.0, False, {}, runs, None)


def tmp_path_file(root: Path) -> Path:
    return root
