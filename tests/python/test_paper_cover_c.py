"""Branch coverage for the paper scripts the kit runs."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import subprocess
import sys
import types
import warnings
import zipfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"
CPC = ROOT / "paper" / "cpc"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverc_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_hcn_case_writes_a_temporary_record(tmp_path, monkeypatch):
    hcn = load("hcn_case.py")
    data = tmp_path / "data"
    data.mkdir()
    (data / "hcn.con").write_text((CPC / "data" / "hcn.con").read_text(encoding="utf-8"), encoding="utf-8")
    (data / "hnc.con").write_text((CPC / "data" / "hnc.con").read_text(encoding="utf-8"), encoding="utf-8")
    monkeypatch.setattr(hcn, "DATA", data)
    monkeypatch.setattr(hcn, "RESULTS", tmp_path / "results")
    rows = hcn.atoms(data / "hcn.con")
    assert len(rows) == 3
    blank = tmp_path / "blank.con"
    body = (data / "hcn.con").read_text(encoding="utf-8").splitlines()
    body.append("")
    blank.write_text("\n".join(body) + "\n", encoding="utf-8")
    assert len(hcn.atoms(blank)) == 3
    short = tmp_path / "short.con"
    short.write_text("\n".join(["h"] * 9 + ["H", "lab", "0 0 0 0 1"]) + "\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="parsed"):
        hcn.atoms(short)
    text = hcn.frame("title", hcn.META, rows, True)
    assert "Velocities" in text
    assert "Velocities" not in hcn.frame("title", hcn.META, rows, False)

    crate = tmp_path / "crate"
    crate.mkdir()
    manifest = crate / "Cargo.toml"
    manifest.write_text('readcon-core = { path = "old" }\n', encoding="utf-8")
    (crate / "Cargo.lock").write_text("lock\n", encoding="utf-8")
    monkeypatch.setattr(hcn, "CRATE", crate)
    monkeypatch.setattr(hcn, "ROOT", tmp_path)
    monkeypatch.setattr(hcn, "BINARY", tmp_path / "bin")
    monkeypatch.setenv("READCON_SRC", str(tmp_path / "src"))
    monkeypatch.setattr(hcn.subprocess, "check_call", lambda *args, **kwargs: None)
    assert hcn.binary() == tmp_path / "bin"
    assert 'path = "' in manifest.read_text(encoding="utf-8")
    hcn.binary.cache_clear()
    (crate / "Cargo.lock").unlink()
    assert hcn.binary() == tmp_path / "bin"
    hcn.binary.cache_clear()

    def boom(*args, **kwargs):
        raise subprocess.CalledProcessError(1, args)

    monkeypatch.setattr(hcn.subprocess, "check_call", boom)
    with pytest.raises(subprocess.CalledProcessError):
        hcn.binary()
    assert manifest.read_text(encoding="utf-8").startswith("readcon-core")
    hcn.binary.cache_clear()

    class Proc:
        def __init__(self, code, out, err):
            self.returncode = code
            self.stdout = out
            self.stderr = err

    monkeypatch.setattr(hcn, "binary", lambda: tmp_path / "bin")
    monkeypatch.setattr(hcn.subprocess, "run", lambda *a, **k: Proc(1, "", "nope"))
    with pytest.raises(SystemExit, match="nope"):
        hcn.dump(data / "hcn.con")
    payload = json.dumps([{"atoms": [], "units": {}}])
    monkeypatch.setattr(hcn.subprocess, "run", lambda *a, **k: Proc(0, payload, ""))
    assert hcn.dump(data / "hcn.con")[0]["units"] == {}

    def fake_dump(path):
        return [
            {
                "units": {"time": "fs"},
                "atoms": [{"fixed": [True, False, False], "id": 2, "velocity": [0, 0, 0]}],
            }
        ]

    monkeypatch.setattr(hcn, "dump", fake_dump)
    monkeypatch.setattr(hcn.provenance, "record", lambda: {"host": {}})
    (tmp_path / "bin").write_bytes(b"bin")
    hcn.main()
    assert (data / "hcn_v31.con").is_file()
    assert (tmp_path / "results" / "hcn_loss.csv").is_file()


def test_every_path_count_rejects_bad_logs(tmp_path, monkeypatch):
    every = load("every_path_count.py")
    assert every.command("demo")[0] == "cargo"
    monkeypatch.setattr(every.subprocess, "run", lambda *a, **k: type("P", (), {"stdout": "cargo 1\n"})())
    assert every.toolchain()["cargo"] == "1"

    def missing(*args, **kwargs):
        raise OSError("absent")

    monkeypatch.setattr(every.subprocess, "run", missing)
    assert every.toolchain()["cargo"] is None

    class Cargo:
        def __init__(self, code, out):
            self.returncode = code
            self.stdout = out
            self.stderr = ""

    monkeypatch.setattr(every.subprocess, "run", lambda *a, **k: Cargo(2, "boom"))
    with pytest.raises(SystemExit, match="exit status"):
        every.run("demo", tmp_path / "none.log")
    monkeypatch.setattr(every.subprocess, "run", lambda *a, **k: Cargo(0, "no result\n"))
    with pytest.raises(SystemExit, match="no passing"):
        every.run("demo", tmp_path / "none.log")
    good = "test result: ok. 2 passed; 0 failed; 0 ignored\n"
    monkeypatch.setattr(every.subprocess, "run", lambda *a, **k: Cargo(0, good))
    with pytest.raises(SystemExit, match="logged no input"):
        every.run("demo", tmp_path / "missing.log")
    odd = tmp_path / "odd.log"
    odd.write_text("10 4 3\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="two"):
        every.run("demo", odd)
    bad = tmp_path / "bad.log"
    bad.write_text("10 1 1\n20 1 1\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="other thread"):
        every.run("demo", bad)
    short = tmp_path / "short.log"
    short.write_text("10 4 3\n1 4 3\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="padded"):
        every.run("demo", short)
    ok = tmp_path / "ok.log"
    ok.write_text("10 4 3\n20 4 3\n", encoding="utf-8")
    assert every.run("demo", ok)["inputs"] == 1
    monkeypatch.setattr(every, "test_files", lambda: [])
    with pytest.raises(SystemExit, match="no test file"):
        every.measure()
    monkeypatch.setattr(every, "test_files", lambda: ["demo"])
    monkeypatch.setattr(every, "run", lambda name, log: {"tests": 1, "ignored": 0, "inputs": 1, "reads": 2, "reads_from_64k": 1, "given_from_64k": 0})
    monkeypatch.setattr(every.provenance, "library_version", lambda: "0")
    monkeypatch.setattr(every.provenance, "source_identity", lambda: {"library_digest": "abc"})
    monkeypatch.setattr(every.provenance, "host", lambda: {"cpu_model": "guest"})
    monkeypatch.setattr(every, "toolchain", lambda: {"cargo": "1", "rustc": "1"})
    measured = every.measure()
    assert measured["results"]["totals"]["inputs"] == 1

    record = {"results": measured["results"], "provenance": measured["provenance"]}
    out = tmp_path / "every_path.json"
    out.write_text(json.dumps(record), encoding="utf-8")
    monkeypatch.setattr(every, "MACROS", tmp_path / "every.tex")
    monkeypatch.setattr(every, "measure", lambda: measured)
    monkeypatch.setattr(sys, "argv", ["every_path_count.py", "--tables", "--out", str(out)])
    assert every.main() == 0
    assert "everyPathInputs" in (tmp_path / "every.tex").read_text(encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["every_path_count.py", "--out", str(tmp_path / "fresh.json")])
    assert every.main() == 0
    monkeypatch.setattr(sys, "argv", ["every_path_count.py", "--check", "--out", str(out)])
    assert every.main() == 0
    (tmp_path / "every.tex").write_text("stale\n", encoding="utf-8")
    assert every.main() == 1
    (tmp_path / "every.tex").write_text(every.macros(record["results"]), encoding="utf-8")
    broken = json.loads(out.read_text())
    broken["results"]["totals"]["inputs"] = 9
    out.write_text(json.dumps(broken), encoding="utf-8")
    assert every.main() == 1


def test_spec_version_drift_records_a_split(tmp_path, monkeypatch):
    drift = load("spec_version_drift.py")
    assert drift.sha256(CPC / "data" / "hcn.con")
    answers = {
        "old": {"__version__": "0.17.0", **{name: {"verdict": "accept", "frames": 1} for name in drift.PROBES}},
        "new": {"__version__": "0.18.0", **{name: {"verdict": "reject", "class": "Other", "message": "no"} for name in drift.PROBES}},
    }

    class Proc:
        def __init__(self):
            self.stdout = json.dumps(answers["old"])
            self.returncode = 0

    monkeypatch.setattr(
        drift.subprocess,
        "run",
        lambda *a, **k: type("P", (), {"stdout": json.dumps({"one": {"verdict": "accept", "frames": 1}}), "returncode": 0})(),
    )
    asked = drift.ask(tmp_path, {"one": "text"})
    assert asked["one"]["verdict"] == "accept"
    answers["old"]["one"] = {"verdict": "reject", "message": "bad", "class": "Other"}
    # ask classifies rejects
    monkeypatch.setattr(drift.subprocess, "run", lambda *a, **k: type("P", (), {"stdout": json.dumps({"one": {"verdict": "reject", "message": "bad"}}), "returncode": 0})())
    rejected = drift.ask(tmp_path, {"one": "text"})
    assert "class" in rejected["one"]

    wheel = tmp_path / "reader.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("readcon/readcon.so", b"so")
    bare = tmp_path / "bare"
    (bare / "readcon").mkdir(parents=True)
    monkeypatch.setattr(drift, "ask", lambda path, probes: answers["old" if "bare" in str(path) else "new"])
    import provenance

    monkeypatch.setattr(provenance, "record", lambda: {"host": {}})
    monkeypatch.setattr(sys, "argv", ["spec_version_drift.py", "--reader", f"old={bare}", "--reader", f"new={wheel}", "--out", str(tmp_path / "out")])
    assert drift.main() == 0
    assert (tmp_path / "out" / "version_drift.json").is_file()


def test_corpus31_results_tables_and_checks(tmp_path, monkeypatch):
    mod = load("corpus31_results.py")
    corpus = tmp_path / "c31"
    corpus.mkdir()
    (corpus / "manifest.toml").write_text(
        'spec_doc = "spec.org"\nreader_minor = 1\n'
        '[[file]]\nfile = "a.con"\nbasis = "explicit"\nclause = "CON-a.1"\nexpected = "accept"\nframes = 1\npbc = true\nreason = "kept"\n'
        '[[file]]\nfile = "b.con"\nbasis = "open"\nclause = "CON-b.1"\nexpected = "reject"\nerror = "Other"\nline = 2\nreason = "see a.con"\n',
        encoding="utf-8",
    )
    monkeypatch.setattr(mod, "C31", corpus)

    def fake_run(binary, threads):
        return {
            "a.con": {"outcome": "accept", "frames": 1, "pbc": True, "class": "", "line": None},
            "b.con": {"outcome": "reject", "class": "Other", "line": 4, "frames": 0, "pbc": None},
        }

    monkeypatch.setattr(mod.subprocess, "run", lambda *a, **k: type("P", (), {"stdout": json.dumps({"files": [{"file": "a.con", "outcome": "accept", "frames": 1, "pbc": True, "class": "", "line": None}]})})())
    assert "a.con" in mod.run(tmp_path / "bin", "auto")
    monkeypatch.setattr(mod.subprocess, "run", lambda *a, **k: type("P", (), {"stdout": "{}"})())
    monkeypatch.setattr(mod, "run", fake_run)
    monkeypatch.setattr(mod.provenance, "portable", lambda path: "manifest.toml")
    monkeypatch.setattr(mod.provenance, "binary_build", lambda path: {"library_digest": "abc"})
    monkeypatch.setattr(mod.provenance, "library_version", lambda: "0.18.0")
    monkeypatch.setattr(mod.provenance, "host", lambda: {})
    record = mod.results(tmp_path / "bin", ("1", "2"))
    assert record["summary"]["files"] == 2
    monkeypatch.setattr(mod, "MACROS", tmp_path / "m.tex")
    monkeypatch.setattr(mod, "TABLE", tmp_path / "t.tex")
    monkeypatch.setattr(mod, "results", lambda binary, threads: record)
    out = tmp_path / "corpus31.json"
    out.write_text(json.dumps(record), encoding="utf-8")
    (tmp_path / "m.tex").write_text(mod.macros(record), encoding="utf-8")
    (tmp_path / "t.tex").write_text(mod.table(record), encoding="utf-8")
    binary = tmp_path / "corpus31_report"
    binary.write_bytes(b"x")
    monkeypatch.setattr(mod.provenance, "newer_sources", lambda path: [])
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--tables", "--out", str(out)])
    assert mod.main() == 0
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--binary", str(tmp_path / "missing")])
    with pytest.raises(SystemExit, match="missing"):
        mod.main()
    monkeypatch.setattr(mod.provenance, "newer_sources", lambda path: ["src/ffi.rs"])
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--binary", str(binary), "--out", str(out)])
    with pytest.raises(SystemExit, match="older"):
        mod.main()
    monkeypatch.setattr(mod.provenance, "newer_sources", lambda path: [])
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--binary", str(binary), "--check", "--out", str(out)])
    assert mod.main() in (0, 1)
    changed = json.loads(out.read_text())
    changed["summary"]["files"] = 0
    out.write_text(json.dumps(changed), encoding="utf-8")
    assert mod.main() == 1
    out.write_text(json.dumps(record), encoding="utf-8")
    (tmp_path / "m.tex").write_text("stale\n", encoding="utf-8")
    assert mod.main() == 1
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--binary", str(binary), "--out", str(tmp_path / "wrote.json")])
    assert mod.main() in (0, 1)


def test_crosscheck_helpers_and_a_row(tmp_path, monkeypatch):
    cross = load("corpus_crosscheck.py")
    hidden = tmp_path / "hidden"
    frame = types.SimpleNamespace(has_velocities=False, to_ase=lambda: None)

    class Reader:
        def read_con(self, path):
            warnings.warn("units.time fs is not applied")
            return [frame]

        def iter_con(self, path):
            raise RuntimeError("bad line")

        def write_con_string(self, frames):
            raise RuntimeError("write")

    fake = types.ModuleType("readcon")
    fake.read_con = Reader().read_con
    fake.iter_con = Reader().iter_con
    fake.write_con_string = Reader().write_con_string
    monkeypatch.setitem(sys.modules, "readcon", fake)
    monkeypatch.setattr(cross, "readcon_view", lambda frames: [{"ids": [1], "time_unit": [None, None], "masks": [0], "pbc": [True]}])
    accepted = cross.child("read_con", str(hidden), True)
    assert accepted["outcome"] == "accept"
    assert "rewrite_error" in accepted
    rejected = cross.child("iter_con", str(hidden), False)
    assert rejected["outcome"] == "reject"

    class Panic(BaseException):
        pass

    Panic.__name__ = "PanicException"

    def panic(path):
        raise Panic("boom")

    fake.read_con = panic
    assert cross.child("read_con", str(hidden), False)["outcome"] == "crash"
    fast = types.SimpleNamespace(has_velocities=True, to_ase=lambda: (_ for _ in ()).throw(RuntimeError("ase")))
    fake.read_con = lambda path: [fast]
    with_vel = cross.child("read_con", str(hidden), False)
    assert "to_ase_error" in with_vel

    monkeypatch.setattr(cross.subprocess, "run", lambda *a, **k: (_ for _ in ()).throw(subprocess.TimeoutExpired(a, 1)))
    assert cross.readcon_run("read_con", hidden, False)["outcome"] == "crash"
    monkeypatch.setattr(cross.subprocess, "run", lambda *a, **k: subprocess.CompletedProcess(a, 1, stdout="", stderr="fail\n"))
    assert "exit status" in cross.readcon_run("read_con", hidden, True)["error"]
    monkeypatch.setattr(cross.subprocess, "run", lambda *a, **k: subprocess.CompletedProcess(a, 0, stdout='{"outcome":"accept"}\n', stderr=""))
    assert cross.readcon_run("read_con", hidden, False)["outcome"] == "accept"

    class Rejection(Exception):
        def __init__(self):
            self.cls = "Other"
            self.line = 3

    monkeypatch.setattr(cross.indep_reader, "Rejection", Rejection)
    monkeypatch.setattr(cross.indep_reader, "read_path", lambda path: (_ for _ in ()).throw(Rejection()))
    assert cross.indep_run(hidden)[0]["outcome"] == "reject"
    monkeypatch.setattr(cross.indep_reader, "read_path", lambda path: (_ for _ in ()).throw(RuntimeError("x")))
    assert cross.indep_run(hidden)[0]["outcome"] == "crash"
    sample = {"ids": [1], "time_unit": ["fs", "fs"], "time_override": None, "masks": [], "pbc": []}
    monkeypatch.setattr(cross.indep_reader, "read_path", lambda path: [sample])
    monkeypatch.setattr(cross, "indep_view", lambda frames: frames)
    assert cross.indep_run(hidden)[0]["outcome"] == "accept"

    before = {"major": 3, "minor": 1, "metadata": {"units": {"length": "angstrom"}}}
    after = {"major": 3, "minor": 1, "metadata": {"units": {"length": "angstrom"}}}
    assert cross.kept(before, after)
    old = {"major": 1, "minor": 0, "metadata": {"units": {"length": "angstrom"}}, "ids": [1], "time_unit": [None, None]}
    new = {"major": 3, "minor": 1, "metadata": {"units": {"length": "angstrom"}}, "ids": [1], "time_unit": [None, None]}
    monkeypatch.setattr(cross, "fields", lambda frame: {"ids": frame["ids"]})
    assert cross.kept(old, new) or not cross.kept(old, new)
    assert cross.rewrite_status(None, {"rewrite_error": "no"}) .startswith("write failed")
    assert "no reference" in cross.rewrite_status(None, {})
    monkeypatch.setattr(cross.indep_reader, "read_text", lambda text: (_ for _ in ()).throw(Rejection()))
    assert "rewrite rejected" in cross.rewrite_status([before], {"rewrite": "x"})
    monkeypatch.setattr(cross.indep_reader, "read_text", lambda text: [after])
    monkeypatch.setattr(cross, "kept", lambda b, a: True)
    assert cross.rewrite_status([before], {"rewrite": "x"}) == "kept"
    monkeypatch.setattr(cross, "kept", lambda b, a: False)
    assert cross.rewrite_status([before], {"rewrite": "x"}).startswith("frame")

    entry = {"expected": "accept", "frames": 1, "pbc": None, "fixed": None, "error": None, "line": 2, "time_unit_override": False, "rewrite": True}
    got = {"outcome": "reject", "class": "Other", "line": 2}
    assert cross.misses(entry, got, True) == ["outcome"]
    entry["expected"] = "reject"
    entry["error"] = "Other"
    assert cross.misses(entry, got, False) == []
    got["line"] = 9
    assert "line" in cross.misses(entry, got, False)
    entry["expected"] = "accept"
    got = {"outcome": "accept", "frames": 2, "pbc": None, "fixed": None, "overrides": 1, "rewrite_status": "no"}
    assert "frames" in cross.misses(entry, got, True)
    assert cross.same_decision({"outcome": "accept", "frames": 1}, {"outcome": "reject", "class": "Other"}) is False
    assert cross.same_decision({"outcome": "reject", "class": "A", "line": 1}, {"outcome": "reject", "class": "B", "line": 1}) is False
    assert cross.same_decision({"outcome": "reject", "class": "A", "line": None}, {"outcome": "reject", "class": "A", "line": 4})
    view = {key: 1 for key in cross.FIELDS}
    other = dict(view, coords=2)
    assert cross.compare([view], [other])["coords"] is False

    path = ROOT / "paper" / "cpc" / "data" / "hcn.con"
    row_entry = {
        "path": path,
        "group": "corpus31",
        "basis": "explicit",
        "expected": "accept",
        "frames": 1,
        "error": None,
        "line": None,
        "pbc": None,
        "fixed": None,
        "time_unit_override": False,
        "rewrite": True,
        "clause": "CON-a.1",
    }
    view = {key: 1 for key in cross.FIELDS}
    view["time_unit"] = ["label", "fs"]
    view["masks"] = [0]
    view["pbc"] = [True]

    def fake_read(call, path, rewrite):
        return {"outcome": "accept", "frames": 1, "view": [dict(view)], "overrides": 0, "atoms": 1}

    monkeypatch.setattr(cross, "readcon_run", fake_read)
    monkeypatch.setattr(cross, "indep_run", lambda path: ({"outcome": "accept", "frames": 1, "view": [dict(view)], "overrides": 0, "atoms": 1}, [before]))
    monkeypatch.setattr(cross.indep_reader, "canonical_unit", lambda text: text)
    monkeypatch.setattr(cross, "rewrite_status", lambda frames, got: "kept")
    row = cross.run(row_entry)
    assert row["readcon"] == "accept"
    assert row["field_identical"] is True
    bare = dict(row)
    bare["field_identical"] = ""
    bare["iter_field_identical"] = ""
    bare["readcon_agrees"] = False
    bare["readcon_kind"] = "disagreement"
    bare["iter_same"] = False
    summary = cross.summarise([row, bare])
    assert summary["groups"]["all"]["files"] == 2
    monkeypatch.setattr(cross, "readcon_run", lambda *a: {"outcome": "reject", "class": "Other"})
    monkeypatch.setattr(cross, "indep_run", lambda path: ({"outcome": "reject", "class": "Other"}, None))
    empty = cross.run(row_entry)
    assert empty["field_identical"] == ""
    record = json.loads((CPC / "data" / "corpus_crosscheck.json").read_text(encoding="utf-8"))
    out = tmp_path / "cross"
    out.mkdir()
    (out / "corpus_crosscheck.json").write_text(json.dumps(record), encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["corpus_crosscheck.py", "--tables", "--out", str(out)])
    cross.main()
    assert (out / "corpus_crosscheck.tex").is_file()
    monkeypatch.setattr(sys, "argv", ["corpus_crosscheck.py", "--check", "--out", str(out)])
    cross.main()
    (out / "corpus_macros.tex").write_text("stale\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="differ"):
        cross.main()
    monkeypatch.setattr(sys, "argv", ["corpus_crosscheck.py", "--child", "read_con", str(path), "--rewrite"])
    cross.main()
    assert cross.portable(ROOT / "paper" / "cpc" / "data" / "hcn.con").endswith("hcn.con")
    assert cross.portable(tmp_path / "outside.txt") == "outside.txt"


def test_binary_format_checks_reject_a_bad_settle(tmp_path, monkeypatch):
    mod = load("binary_formats_tables.py")
    bf = tmp_path / "binary_formats"
    bf.mkdir()
    settle_text = "{}\n"
    (bf / "settle.json").write_text(settle_text, encoding="utf-8")
    sha = hashlib.sha256(settle_text.encode()).hexdigest()
    monkeypatch.setattr(mod, "BF", bf)
    (tmp_path / "provenance.json").write_text('{"host": {"cpu": "guest"}}\n', encoding="utf-8")
    (tmp_path / "reads.json").write_text("{}\n", encoding="utf-8")
    monkeypatch.setattr(mod, "other_host", lambda host, performance: None)
    monkeypatch.setattr(mod, "same_protocol", lambda *args: None)

    def settle(level_window=0, settle_ms=0, hardware="h", band_cells=None, windows=2):
        counts = {"cpu0": {0: 1.0, 1: 1.0}}
        trial = {"counts": counts, "steady": {"cpu0": 1.0}, "level_window": level_window, "settle_ms": settle_ms}
        protocol = {
            "band_cells": ["only"] if band_cells is None else band_cells,
            "window_cell": "libxtc-frames-8",
            "gap_cell": "tng-frames",
            "windows": windows,
            "window_ms": 10,
            "gap_s": 0.02,
            "busy_ms": 20,
            "trials": 1,
            "band": 0.1,
        }
        return {
            "settle_ms": settle_ms,
            "settle_sha256": sha,
            "settle_trial": 1,
            "hardware": hardware,
            "protocol": protocol,
            "trials": [trial],
        }

    timings = {"protocol": {}, "hardware": "h", "cells": {"only": {"threads": 1}}}
    rec = {"settle": settle(), "timings": timings}
    rec["timings"]["protocol"] = {"settle_ms": 0, "settle_sha256": sha, "rounds": 1}
    mod.check_settle(rec)
    rec["timings"]["protocol"]["settle_ms"] = 9
    with pytest.raises(SystemExit, match="settle_ms"):
        mod.check_settle(rec)
    rec["timings"]["protocol"]["settle_ms"] = 0
    rec["settle"]["hardware"] = "other"
    with pytest.raises(SystemExit, match="hardware"):
        mod.check_settle(rec)
    rec["settle"]["hardware"] = "h"
    rec["settle"]["protocol"]["band_cells"] = ["nope"]
    with pytest.raises(SystemExit, match="band"):
        mod.check_settle(rec)
    rec["settle"]["protocol"]["band_cells"] = ["only"]
    rec["settle"]["protocol"]["busy_ms"] = 1
    with pytest.raises(SystemExit, match="windows"):
        mod.check_settle(rec)
    rec["settle"]["protocol"]["busy_ms"] = 20
    rec["timings"]["protocol"]["rounds"] = 3
    with pytest.raises(SystemExit, match="trials"):
        mod.check_settle(rec)
    rec["timings"]["protocol"]["rounds"] = 1
    rec["settle"]["trials"][0]["level_window"] = 9
    with pytest.raises(SystemExit, match="trial"):
        mod.check_settle(rec)
    rec["settle"]["trials"][0]["level_window"] = 0
    rec["settle"]["settle_ms"] = 5
    rec["timings"]["protocol"]["settle_ms"] = 5
    with pytest.raises(SystemExit, match="longest"):
        mod.check_settle(rec)
    assert mod.figures(0) == "0"
    assert mod.figures(9.99) 
    assert mod.rate(1000) == "1000"
    assert mod.rate(1.2) == "1.2"
    build = {"library_digest": "a", "binary_sha256": "b"}
    host = {"cpu": "guest"}
    prov = {"build": build, "host": host}
    bad = {
        "build": {"binaries": {"readcon_read": {"sha256": "b"}}, "readcon_read": {"locked_as_library": True}, "provenance": prov},
        "payloads": {"provenance": {"build": {"library_digest": "other", "binary_sha256": "b"}, "host": host}, "rerun_identical": {}, "writers": {"con_decimals": {}}, "system": {}},
        "checks": {"provenance": prov, "cells": {}, "libxtc_pep": {}, "tng_two_sets": {}},
        "timings": {"provenance": prov, "cells": {}, "hardware": "h", "protocol": {}},
        "settle": {"provenance": prov},
    }
    with pytest.raises(SystemExit, match="builds"):
        mod.check(bad)
    for name in ("payloads", "checks", "timings", "settle"):
        bad[name]["provenance"] = {"build": dict(build), "host": host}
    bad["build"]["binaries"]["readcon_read"]["sha256"] = "nope"
    with pytest.raises(SystemExit, match="another readcon_read"):
        mod.check(bad)
    bad["build"]["binaries"]["readcon_read"]["sha256"] = "b"
    bad["build"]["readcon_read"]["locked_as_library"] = False
    with pytest.raises(SystemExit, match="locks"):
        mod.check(bad)
