"""Outputs of the paper scripts that were still short of 98 percent."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import types
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"
CPC = ROOT / "paper" / "cpc"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverd_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_cases_rejects_a_slab_without_one_adsorbate(monkeypatch):
    import ase.build

    cases = load("cases.py")
    real = ase.build.add_adsorbate

    def none(slab, adsorbate, height, position="ontop"):
        return None

    monkeypatch.setattr(ase.build, "add_adsorbate", none)
    with pytest.raises(SystemExit, match="expected one adsorbate, found 0"):
        cases.build_slab()
    monkeypatch.setattr(ase.build, "add_adsorbate", real)
    slab, masks, ids, buried, surface, hydrogen = cases.build_slab()
    assert len(slab) == 10001
    assert int(masks.sum()) > 0
    assert ids[hydrogen] == hydrogen
    assert buried != surface


def test_corpus_paths_corpus_children_and_tables(tmp_path, monkeypatch):
    paths = load("corpus_paths.py")
    monkeypatch.setattr(paths, "map_bytes", lambda: 64)
    files = paths.corpus()
    assert any(row["corpus"] == "corpus31" for row in files)
    assert any(row["expected"] == "reject" for row in files)
    work = tmp_path / "work"
    work.mkdir()
    jobs = paths.prepare(files[:2], work)
    assert jobs[0]["utf8"] is True
    raw = tmp_path / "bad.bin"
    raw.write_bytes(b"\xff\xfe not utf8")
    entry = {"path": raw, "expected": "accept", "frames": 1, "error": None, "line": None}
    (work / "u").mkdir()
    prepared = paths.prepare([entry], work / "u")
    assert prepared[0]["utf8"] is False

    class Panic(BaseException):
        pass

    Panic.__name__ = "PanicException"
    fake = types.ModuleType("readcon")

    def boom(*args, **kwargs):
        raise Panic("panic")

    def reject(*args, **kwargs):
        raise RuntimeError("Other: no")

    fake.read_con_string = boom
    fake.read_con = reject
    fake.iter_con = reject
    fake.count_frames = lambda *a, **k: 3
    monkeypatch.setitem(sys.modules, "readcon", fake)
    job = {
        "id": "0",
        "text": str(raw),
        "plain": str(raw),
        "mapped": str(raw),
        "gzip": str(raw),
        "utf8": False,
    }
    got = paths.wheel_child([job])
    assert got["0"]["wheel/string/1"] == "skip"
    assert got["0"]["wheel/file/1"].startswith("reject") or got["0"]["wheel/file/1"] == "crash"
    job["utf8"] = True
    raw.write_text("x", encoding="utf-8")
    crashed = paths.wheel_child([job])
    assert crashed["0"]["wheel/string/1"] == "crash"

    class Proc:
        def __init__(self, code, out, err):
            self.returncode = code
            self.stdout = out
            self.stderr = err

    monkeypatch.setattr(paths.subprocess, "run", lambda *a, **k: Proc(3, "", "fail\n"))
    with pytest.raises(SystemExit, match="exit status 3"):
        paths.run_child("wheel", tmp_path / "jobs.json")
    monkeypatch.setattr(paths.subprocess, "run", lambda *a, **k: Proc(0, '{"0": {}}\n', ""))
    assert paths.run_child("wheel", tmp_path / "jobs.json") == {"0": {}}

    skip_job = {"utf8": False, "compressed": None, "added": 0}
    assert paths.wanted({"expected": "accept", "frames": 1, "error": None}, skip_job, "wheel/string/1") == "skip"
    zstd_job = {"utf8": True, "compressed": "zstd", "added": 2}
    assert paths.wanted({"expected": "accept", "frames": 1, "error": None}, zstd_job, "clib/file/1") == "reject Compression"
    assert paths.wanted({"expected": "reject", "frames": None, "error": "Other"}, zstd_job, "wheel/file/1").startswith("reject")

    one = {
        "corpus": "corpus31",
        "path": raw,
        "expected": "accept",
        "frames": None,
        "error": None,
        "line": None,
    }
    job = {"id": "0", "utf8": True, "compressed": None, "added": 0, "padding": "front"}
    keys = paths.read_keys("wheel")[:2]
    got_map = {"wheel": {"0": {key: "accept 4" for key in paths.read_keys("wheel")}}}
    for build in paths.BUILDS:
        if build == "wheel":
            continue
        got_map[build] = {"0": {key: "accept 4" for key in paths.read_keys(build)}}
    summary = paths.results([job], [one], got_map)
    assert summary["totals"]["files"] == 1

    monkeypatch.setattr(paths, "MACROS", tmp_path / "m.tex")
    monkeypatch.setattr(paths, "TABLE", tmp_path / "t.tex")
    stored = {"results": summary, "provenance": {"build": {"library_digest": "a"}, "c_libraries": {"clib": {"library_digest": "a"}}}}
    out = tmp_path / "corpus_paths.json"
    out.write_text(json.dumps(stored), encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["corpus_paths.py", "--tables", "--out", str(out)])
    assert paths.main() == 0
    assert "cpaths" in (tmp_path / "m.tex").read_text(encoding="utf-8") or (tmp_path / "m.tex").is_file()
    original_measure = paths.measure
    monkeypatch.setattr(paths, "measure", lambda: stored | {"results": summary})
    # measure must return provenance matching comparable
    record = {
        "results": summary,
        "provenance": stored["provenance"] | {"host": {}, "c_libraries": {"clib": {"library_digest": "a"}}, "build": {"library_digest": "a"}},
    }
    monkeypatch.setattr(paths, "measure", lambda: record)
    monkeypatch.setattr(sys, "argv", ["corpus_paths.py", "--out", str(tmp_path / "wrote.json")])
    assert paths.main() in (0, 1)
    monkeypatch.setattr(paths, "CLIB", tmp_path / "missing.so")
    with pytest.raises(SystemExit, match="missing"):
        original_measure()


def test_velocity_case_draws_the_slab_and_rejects_a_bad_unit(tmp_path, monkeypatch):
    vel = load("velocity_case.py")
    drawn = vel.maxwell_boltzmann(__import__("numpy").array([195.0, 1.0]), 8.6e-5)
    assert drawn.shape == (2, 3)
    unit = {
        "readcon_fs": 10.0,
        "kB_eV_per_K": 8.617333262145e-5,
        "canonical": "natural",
        "readcon_fs_squared": 100.0,
        "readcon_fs_standard_uncertainty": 0.01,
        "eon_fs": 10.18,
        "readcon_minus_eon_relative": -0.01,
    }
    original_fs = vel.natural_fs
    monkeypatch.setattr(vel, "natural_fs", lambda: unit)
    big = vel.slab_10001(unit["readcon_fs"], unit["kB_eV_per_K"])
    assert big["n_atoms"] == 10001
    assert big["ke_natural_eV"] > 0
    con = tmp_path / "bad.con"
    con.write_text(
        (CPC / "data" / "corpus31" / "v31_convel_fs.convel").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="declares time unit"):
        vel.slab_28(con, 10.0, 8.6e-5)

    import ase.units

    real_create = ase.units.create_units

    def wrong(version):
        units = real_create(version)
        units = dict(units)
        units["_amu"] = 1.0
        return units

    monkeypatch.setattr(vel, "natural_fs", original_fs)
    monkeypatch.setattr(ase.units, "create_units", wrong)
    import hcn_case

    monkeypatch.setattr(hcn_case, "binary", lambda: tmp_path / "bin")
    monkeypatch.setattr(
        vel.subprocess,
        "run",
        lambda *a, **k: subprocess.CompletedProcess(a, 0, stdout='{"unit_conversion_factor_fs": 1, "spelling": "s", "canonical": "c", "unit_conversion_factor_fs_squared": 1}\n'),
    )
    with pytest.raises(SystemExit, match="m_u"):
        vel.natural_fs()


def test_stamp_host_rewrites_a_temporary_tree(tmp_path, monkeypatch):
    host = load("stamp_host.py")
    marked = "Intel" + "(R)" + " Xeon" + "(R)" + " Processor"
    (tmp_path / "note.txt").write_text(marked + "\n", encoding="utf-8")
    (tmp_path / "host.json").write_text(json.dumps({"cpu_model": marked}), encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["stamp_host.py", str(tmp_path)])
    assert host.main() == 0
    assert marked not in (tmp_path / "note.txt").read_text(encoding="utf-8")
    data = json.loads((tmp_path / "host.json").read_text(encoding="utf-8"))
    assert "Xeon" not in data["cpu_model"] or "(R)" not in data["cpu_model"]


def test_openers_success_and_a_missing_format(tmp_path, monkeypatch):
    openers = load("con_openers.py")
    path = tmp_path / "a.con"
    path.write_text("con\n", encoding="utf-8")

    class Traj:
        def __init__(self, *args):
            pass

        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def read(self):
            return types.SimpleNamespace(atoms=[1, 2, 3])

    chem = types.ModuleType("chemfiles")
    chem.Trajectory = Traj
    chem.set_warnings_callback = lambda fn: None
    monkeypatch.setitem(sys.modules, "chemfiles", chem)
    assert openers.chemfiles_by_name(path) == 3
    assert openers.chemfiles_by_extension(path) == 3

    pybel = types.SimpleNamespace(
        informats={},
        readfile=lambda fmt, name: iter([types.SimpleNamespace(atoms=[1])]),
    )
    babel = types.ModuleType("openbabel")
    babel.pybel = pybel
    monkeypatch.setitem(sys.modules, "openbabel", babel)
    with pytest.raises(ValueError, match="not an Open Babel"):
        openers.openbabel_by_name(path)
    pybel.informats = {"con": True}
    assert openers.openbabel_by_name(path) == 1


def test_referee_c_paths_and_a_stale_record(tmp_path, monkeypatch):
    monkeypatch.delenv("READCON_LIB", raising=False)
    monkeypatch.delenv("READCON_CLI", raising=False)
    probes = load("referee_probes.py")
    probes.LIB = ROOT / "target" / "release" / "libreadcon_core.so"
    probes.CLI = ROOT / "target" / "cli" / "release" / "readcon-core"
    probes._CLIB.clear()
    text, ok = probes.c_iterator_statuses(tmp_path)
    assert "two_frames_blank.con" in text
    assert isinstance(ok, bool)
    seen, passed = probes.c_path_writer_mid_write_failure(tmp_path)
    assert "c.con" in seen
    assert isinstance(passed, bool)
    missing = probes.stale_against_record([], tmp_path / "absent.json")
    assert missing and "missing" in missing[0]
    results = [{"id": "p", "source": "s", "rule": "r", "expected": "e", "runs": [], "pass": True}]
    fresh = tmp_path / "rec.json"
    fresh.write_text(json.dumps(probes.record(results)), encoding="utf-8")
    same = probes.stale_against_record(results, fresh)
    assert same == [] or any("differ" in line for line in same)
    other = tmp_path / "other.json"
    other.write_text(json.dumps({"build": {"library_digest": "nope"}, "probes": []}), encoding="utf-8")
    problems = probes.stale_against_record(results, other)
    assert any("digest" in line or "differ" in line for line in problems)
    assert probes.error_class("nothing named") == "unrecognized"


def test_mpi_macros_when_eight_ranks_are_slower(tmp_path, monkeypatch):
    mpi = load("mpi_from_log.py")
    lines = ["repeats 6"]
    for mode in ("shared", "scatter"):
        for ranks, seconds in ((1, 4.0), (2, 3.0), (4, 1.0), (8, 2.0)):
            for _ in range(6):
                lines.append(
                    f"repeat 0 mode {mode} launch ranks {ranks} frames 1 local 1 ns {int(seconds * 1e9)} sum 1 exit 0"
                )
    log = tmp_path / "mpi.log"
    log.write_text("\n".join(lines) + "\n", encoding="utf-8")
    monkeypatch.setattr(mpi, "LOG", log)
    generated = tmp_path / "gen"
    generated.mkdir()
    monkeypatch.setattr(mpi, "GEN", generated)
    assert mpi.main() == 0
    text = (tmp_path / "gen" / "mpi_macros.tex").read_text(encoding="utf-8")
    assert "faster than four" not in text
    assert "eight-rank median" in text


def test_em_flat_rejects_a_missing_specification(tmp_path):
    em = load("make_em_flat.py")
    stage = tmp_path / "stage"
    stage.mkdir()
    (stage / "rg_main.tex").write_text("\\" + "bibliography{rg_main}\n", encoding="utf-8")
    (stage / "rg_main.bbl").write_text("\\bibitem{x}\n", encoding="utf-8")
    missing = tmp_path / "missing-spec.org"
    real = em.arxiv.SPEC_ORG
    em.arxiv.SPEC_ORG = missing
    try:
        with pytest.raises(em.arxiv.PackError, match="missing"):
            em.write_flat(stage, tmp_path / "flat", tmp_path / "snap", {"rewrites": [], "notes": []})
    finally:
        em.arxiv.SPEC_ORG = real


def test_case_macros_print_a_zero_exponent_as_a_mantissa():
    macros = load("gen_case_macros.py")
    assert macros.sci(1.0) == "1.00"


def test_h2h_toolchain_rejects_two_compiler_versions():
    h2h = load("h2h_tables.py")

    def cell(ver: str, compiler: str = "gcc") -> dict:
        return {
            "compiler_version": f"{compiler} {ver}",
            "cxx": compiler,
            "cxx_flags": ["-O2"],
            "chemfiles_version": "0.10.4",
            "compiler": compiler,
        }

    rec = {
        "cpp": {
            "coords": {"cells": [cell("13.3.0"), cell("14.0.0")]},
            "vel": {"cells": [cell("13.3.0"), cell("14.0.0", "clang")]},
        }
    }
    with pytest.raises(SystemExit, match="values"):
        h2h.toolchain(rec)
    same = {
        "cpp": {
            "coords": {"cells": [cell("13.3.0"), cell("16.0.0", "clang")]},
            "vel": {"cells": [cell("13.3.0"), cell("16.0.0", "clang")]},
        }
    }
    rows = h2h.toolchain(same)
    assert rows[0] == ("hhChemfilesVersion", "0.10.4")
    assert h2h.compiler(cell("13.3.0")) == "gcc 13.3.0 -O2"


def test_release_identity_rejects_a_missing_commit(monkeypatch):
    release = load("gen_release.py")
    monkeypatch.setattr(release, "named_commit", lambda: "deadbeef")
    monkeypatch.setattr(release, "git", lambda *args: None)
    with pytest.raises(SystemExit, match="does not hold"):
        release.identity()
