"""Run the paper scripts' real entry points on small inputs and fixtures."""

from __future__ import annotations

import importlib.util
import json
import runpy
import subprocess
import sys
import types
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    key = "paper_" + name.replace("/", "_").replace(".py", "")
    spec = importlib.util.spec_from_file_location(key, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[key] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_mutant_helpers_and_a_small_run(tmp_path, monkeypatch):
    mutant = load("mutant_differential.py")
    assert mutant.seed_list("1-2,4") == [1, 2, 4]
    raw = "1 2 3"
    with pytest.raises(mutant.indep_reader.Rejection):
        mutant.counts_field_first(raw, 1, 4)
    number, end = mutant.json_number_unbounded("1e999 ", 0)
    assert number == float("inf")
    assert end > 0
    assert mutant.split_lines_final_cr(b"a\r") == mutant.SPLIT_LINES(b"a")
    files = mutant.seed_files()
    assert files
    produced = list(mutant.cases(files, 1, 3))
    assert len(produced) == 3
    text = produced[0][2]
    indep = mutant.read_indep(text)
    got = mutant.read_readcon(text)
    assert indep["verdict"] in {"accept", "reject", "crash"}
    assert mutant.outcome(got)[0] == got["verdict"]
    mutant.read_indep("\x00not a frame")
    mutant.read_readcon("")
    explained = mutant.explain(text, got)
    assert explained is None or "group" in explained
    monkeypatch.setattr(mutant, "OUT", tmp_path / "mutants.json")
    monkeypatch.setattr(mutant, "MACROS", tmp_path / "mutant_macros.tex")
    monkeypatch.setattr(mutant.provenance, "newer_sources", lambda path: [])
    monkeypatch.setattr(sys, "argv", ["mutant_differential.py", "--seeds", "1", "--cases", "4"])
    assert mutant.main() in (0, 1)
    assert (tmp_path / "mutants.json").is_file()


def test_test_run_builds_cells_and_listings(tmp_path, monkeypatch):
    run = load("test_run.py")
    sha = run.commit()
    assert len(sha) == 40
    assert run.version()
    assert len(run.frame_sha()) == 64
    listed = run.commands(sha)
    assert sha[:8] in listed[2]
    text = run.cell_text([30.0, 60.0, 90.0], True)
    assert '"validate":true' in text
    script = tmp_path / "example.sh"
    script.write_text(
        "#!/bin/sh\n"
        "echo \"Failed to parse frame: AtLine { line: 4, extra\" >&2\n"
        "exit 1\n",
        encoding="utf-8",
    )
    script.chmod(0o755)
    checkout = tmp_path / "checkout"
    checkout.mkdir()
    cells = run.cell_runs(checkout, [str(script)])
    assert cells["flat"]["line"] == 4
    assert cells["none_validate"]["exit"] == 1
    record = json.loads(run.RECORD.read_text())
    listing = run.listing(record)
    assert "lst:testrun" in listing
    macros = run.macros(record)
    assert "trCellLine" in macros
    monkeypatch.setattr(run, "commit", lambda: "abc")
    monkeypatch.setattr(
        run.subprocess,
        "run",
        lambda *args, **kwargs: type("P", (), {"stdout": "zzz\n", "returncode": 0})(),
    )
    with pytest.raises(SystemExit, match="not at the commit"):
        run.run(tmp_path, "")


def test_gen_release_checks_and_render(monkeypatch):
    release = load("gen_release.py")
    assert release.pull_request("Add a reader (#12)") == "12"
    with pytest.raises(SystemExit, match="no pull request"):
        release.pull_request("no number")
    assert release.ancestor(release.BASE, release.BASE)
    assert release.published()[0]
    named = release.named_commit()
    assert len(named) == 40
    monkeypatch.setattr(
        release.provenance, "library_digest_at", lambda commit: release.provenance.library_digest()
    )
    found = release.identity()
    assert found["commit"] == named
    text = release.render()
    assert "readconversion" in text
    monkeypatch.setattr(sys, "argv", ["gen_release.py", "--commit"])
    assert release.main() == 0


def test_bench_dlpack_times_a_small_frame_and_reads_a_log(tmp_path, monkeypatch):
    dlpack = load("bench_dlpack.py")
    monkeypatch.setattr(dlpack, "SIZES", (4,))
    monkeypatch.setattr(dlpack, "REPEATS", 2)
    monkeypatch.setattr(dlpack, "WARMUP", 0)
    monkeypatch.setattr(dlpack, "LOG", tmp_path / "dlpack.log")
    monkeypatch.setattr(dlpack, "OUT", tmp_path)
    monkeypatch.setattr(dlpack, "PAPER", tmp_path)
    (tmp_path / "logs").mkdir()
    monkeypatch.setattr(sys, "argv", ["bench_dlpack.py"])
    assert dlpack.main() == 0
    log = (tmp_path / "dlpack.log").read_text(encoding="utf-8")
    assert "summary numpy copy" in log
    monkeypatch.setattr(dlpack, "LOG", tmp_path / "missing.log")
    with pytest.raises(SystemExit, match="missing"):
        dlpack.from_log()
    short = tmp_path / "short.log"
    short.write_text("rev_parse abc\n", encoding="utf-8")
    monkeypatch.setattr(dlpack, "LOG", short)
    with pytest.raises(SystemExit, match="no numpy"):
        dlpack.from_log()

    class View:
        def data_ptr(self):
            return 7

    assert dlpack.pointer(View()) == 7
    assert dlpack.pointer(object()) == -1

    def cpuinfo(self, *args, **kwargs):
        if str(self).endswith("cpuinfo"):
            return "model name : AMD EPYC 7R13\n"
        return Path.read_text(self, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", cpuinfo)
    assert "EPYC" in dlpack.cpu_model()


def test_number_rule_guards_fire_on_a_disagreement(monkeypatch):
    front = load("number_rules/front.py")
    assert front.si_factor('&["ev"]') > 0
    assert front.miller(front.RESTART_PY, "3, 3, 3") == 100
    assert front.log_field("logs/reader-exchange.log", "frames") == 100000
    with pytest.raises(SystemExit, match="no not-a-field"):
        front.log_field("logs/reader-exchange.log", "not-a-field")
    real_param = front.param
    monkeypatch.setattr(front, "param", lambda name: -1 if name == "fmax_eV_A" else real_param(name))
    with pytest.raises(SystemExit, match="FMAX"):
        front.agree_restart()
    monkeypatch.setattr(front, "param", real_param)
    headline = load("number_rules/headline.py")
    monkeypatch.setattr(headline, "text", lambda rel: "summary nothing\n")
    with pytest.raises(SystemExit, match="no 32768"):
        headline.dlpack_ratio()
    saved = dict(headline.PREFACTORS)
    headline.PREFACTORS["other"] = {"prefactor_Hz": 1.0}
    with pytest.raises(SystemExit, match="three significant figures"):
        headline.agree_prefactors()
    headline.PREFACTORS.clear()
    headline.PREFACTORS.update(saved)
    supp = load("number_rules/supp.py")
    real_text = supp.text
    monkeypatch.setattr(supp, "text", lambda rel: "" if rel == supp.DRIFT_PY else real_text(rel))
    with pytest.raises(SystemExit, match="6.0"):
        supp.agree_supp()
    versions = load("number_rules/versions.py")
    with pytest.raises(SystemExit, match="not one of the readers"):
        versions.drift_reader("9.9.9")
    with pytest.raises(SystemExit, match="not in the record"):
        versions.opener("no-such-library", "ase")
    supp = load("number_rules/supp.py")
    assert supp.survey_ids()


def test_provenance_paths_and_a_failed_command(tmp_path):
    provenance = load("provenance.py")
    assert provenance.portable(ROOT / "Cargo.toml") == "Cargo.toml"
    outside = tmp_path / "note.txt"
    outside.write_text("x", encoding="utf-8")
    assert provenance.portable(outside) == "note.txt"
    assert provenance._command("false") is None
    assert provenance.normalize_cpu("  ") == "unknown"
    assert provenance.cpu_record("", "6", "", "") == "family 6"
    line = provenance.hardware_tex(
        {"cpu_model": "a_b", "uname_m": "x86_64", "nproc": 8}
    )
    assert "a\\_b" in line
    binary = tmp_path / "empty-bin"
    binary.write_bytes(b"not an elf")
    built = provenance.binary_build(binary)
    assert built["binary_sha256"]


def _git(checkout: Path, *args: str) -> str:
    done = subprocess.run(
        ["git", "-C", str(checkout), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return done.stdout.strip()


def test_test_run_checks_the_checkout_and_writes_the_listing(tmp_path, monkeypatch):
    run = load("test_run.py")
    checkout = tmp_path / "checkout"
    checkout.mkdir()
    _git(checkout, "init")
    frame = checkout / run.INPUT
    frame.parent.mkdir(parents=True)
    frame.write_bytes(run.FRAME.read_bytes())
    cargo = checkout / "Cargo.toml"
    cargo.write_text('version = "0.0.1"\n', encoding="utf-8")
    subprocess.run(
        [
            "git",
            "-C",
            str(checkout),
            "-c",
            "user.email=paper@example.com",
            "-c",
            "user.name=Paper",
            "add",
            "-A",
        ],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        [
            "git",
            "-C",
            str(checkout),
            "-c",
            "user.email=paper@example.com",
            "-c",
            "user.name=Paper",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "frame",
        ],
        check=True,
        capture_output=True,
    )
    head = _git(checkout, "rev-parse", "HEAD")
    monkeypatch.setattr(run, "commit", lambda: head)
    monkeypatch.setattr(run, "version", lambda: "9.9.9")
    with pytest.raises(SystemExit, match="Cargo.toml says"):
        run.run(checkout, "")
    monkeypatch.setattr(run, "version", lambda: "0.0.1")
    frame.write_bytes(b"not the frame")
    with pytest.raises(SystemExit, match="not the frame"):
        run.run(checkout, "")
    frame.write_bytes(run.FRAME.read_bytes())
    real_subprocess = subprocess.run

    def fake_subprocess(args, **kwargs):
        cmd = list(args)
        if cmd[0] == "git":
            return real_subprocess(args, **kwargs)
        if cmd[0] == "cargo" and "run" in cmd:
            if str(cmd[-1]).endswith("survey_frame.con"):
                return subprocess.CompletedProcess(cmd, 0, stdout="Cell\n", stderr="")
            return subprocess.CompletedProcess(
                cmd,
                1,
                stdout="",
                stderr="Failed to parse frame: AtLine { line: 4, extra }\n",
            )
        return subprocess.CompletedProcess(cmd, 0, stdout="rustc 1.88.0 (abc)\n", stderr="")

    monkeypatch.setattr(run.subprocess, "run", fake_subprocess)
    record = run.run(checkout, "")
    assert record["tree_commit"] == head
    assert record["cells"]["flat"]["line"] == 4
    worked = json.loads(run.WORKED.read_text())

    def cells(line: int | None) -> dict:
        built = {}
        for name in ("flat", "none"):
            accepted = worked[name]["accepted"]
            for key in (name, f"{name}_validate"):
                built[key] = {
                    "angles": list(worked[name]["angles"]),
                    "exit": 0 if accepted else 1,
                    "line": line,
                }
        return built

    good = {
        "cells": cells(4),
        "commands": ["cargo run"],
        "stdout": "Cell\n",
        "readcon_version": "0.0.1",
        "rustc": "rustc 1.88.0 (abc)",
    }
    assert "trCellLine" in run.macros(good)
    shifted = json.loads(json.dumps(good))
    shifted["cells"]["flat"]["angles"] = [1.0, 2.0, 3.0]
    with pytest.raises(SystemExit, match="other flat angles"):
        run.macros(shifted)
    flipped = json.loads(json.dumps(good))
    flipped["cells"]["none"]["exit"] = 1 - flipped["cells"]["none"]["exit"]
    with pytest.raises(SystemExit, match="disagree on the none cell"):
        run.macros(flipped)
    missing = json.loads(json.dumps(good))
    missing["cells"]["none"]["line"] = None
    with pytest.raises(SystemExit, match="not rejected at one line"):
        run.macros(missing)

    stored = json.loads(run.RECORD.read_text())
    record_path = tmp_path / "test_run.json"
    record_path.write_text(json.dumps(stored), encoding="utf-8")
    monkeypatch.setattr(run, "RECORD", record_path)
    monkeypatch.setattr(run, "OUT", tmp_path / "test_run.tex")
    monkeypatch.setattr(run, "MACROS", tmp_path / "test_run_macros.tex")
    monkeypatch.setattr(run, "commit", lambda: "deadbeef")
    monkeypatch.setattr(sys, "argv", ["test_run.py", "--tables"])
    with pytest.raises(SystemExit, match="another commit"):
        run.main()
    monkeypatch.setattr(run, "commit", lambda: stored["tree_commit"])
    monkeypatch.setattr(run, "frame_sha", lambda: stored["input_sha256"])
    assert run.main() is None
    assert (tmp_path / "test_run.tex").is_file()
    monkeypatch.setattr(sys, "argv", ["test_run.py"])
    with pytest.raises(SystemExit, match="--checkout is required"):
        run.main()
    monkeypatch.setattr(run, "run", lambda checkout, toolchain: stored)
    monkeypatch.setattr(sys, "argv", ["test_run.py", "--checkout", str(checkout)])
    assert run.main() is None
    monkeypatch.setattr(sys, "argv", ["test_run.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "test_run.py"), run_name="__main__")


def test_gen_release_stops_and_checks(tmp_path, monkeypatch):
    release = load("gen_release.py")
    original_published = release.published
    monkeypatch.setattr(release, "git", lambda *args: None)
    with pytest.raises(SystemExit, match="no release tag"):
        release.published()
    monkeypatch.setattr(release, "COMMIT_TEX", tmp_path / "commit.tex")
    with pytest.raises(SystemExit, match="not once"):
        release.named_commit()
    (tmp_path / "commit.tex").write_text("no macro\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="not once"):
        release.named_commit()

    commit = "a" * 40
    version = "1.2.3"
    monkeypatch.setattr(release, "named_commit", lambda: commit)
    monkeypatch.setattr(release.provenance, "library_digest", lambda: "ab" * 32)
    monkeypatch.setattr(release.provenance, "library_version", lambda: version)

    def identity_git(cargo: str, changelog: str, describe_commit: str | None, tagged: str | None):
        def git(*args):
            if args == ("rev-parse", "HEAD"):
                return commit
            if args[0] == "cat-file":
                return ""
            if args[0] == "show" and args[-1].endswith("Cargo.toml"):
                return cargo
            if args[0] == "show":
                return changelog
            if args[0] == "describe" and args[-1] == release.BASE:
                return "v0.0.1"
            if args[0] == "describe":
                return describe_commit
            if args[0] == "rev-parse" and "v0.0.1" in args[-1]:
                return "e" * 40
            if args[0] == "rev-parse" and f"v{version}" in args[-1]:
                return tagged
            if args[0] == "rev-parse" and describe_commit:
                return "f" * 40
            return ""

        return git

    monkeypatch.setattr(release, "BUILD_RECORD", tmp_path / "no-record.json")
    monkeypatch.setattr(
        release, "ancestor", lambda older, newer: newer == "origin/main"
    )
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: "ab" * 32)
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "## v1.2.3 \n", "v1.2.3", commit),
    )
    found = release.identity()
    assert found["tagged"] is True
    monkeypatch.setattr(release, "git", lambda *args: None if args[0] == "cat-file" else "")
    with pytest.raises(SystemExit, match="does not hold"):
        release.identity()
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "## v1.2.3 \n", "v1.2.3", None),
    )
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: None)
    with pytest.raises(SystemExit, match="no library files"):
        release.identity()
    monkeypatch.setattr(release, "ancestor", lambda older, newer: False)
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: "cd" * 32)
    with pytest.raises(SystemExit, match="this tree holds"):
        release.identity()
    monkeypatch.setattr(
        release, "ancestor", lambda older, newer: newer == "origin/main"
    )
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: "ab" * 32)
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "0.0.0"\n', "## v1.2.3 \n", "v1.2.3", None),
    )
    with pytest.raises(SystemExit, match="Cargo.toml is"):
        release.identity()
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "no section\n", "v1.2.3", None),
    )
    with pytest.raises(SystemExit, match="no section"):
        release.identity()
    monkeypatch.setattr(
        release,
        "ancestor",
        lambda older, newer: newer == "origin/main"
        or (older == release.BASE and newer == commit),
    )
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "## v1.2.3 \n", "v1.2.3", None),
    )
    with pytest.raises(SystemExit, match="is not an ancestor"):
        release.identity()
    monkeypatch.setattr(
        release, "ancestor", lambda older, newer: newer == "origin/main"
    )
    monkeypatch.setattr(release, "published", lambda: (version, "e" * 40))
    with pytest.raises(SystemExit, match="needs a new version"):
        release.identity()
    monkeypatch.setattr(release, "published", original_published)
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "## v1.2.3 \n", "v1.2.3", "d" * 40),
    )
    with pytest.raises(SystemExit, match="names"):
        release.identity()
    monkeypatch.setattr(
        release,
        "git",
        identity_git('version = "1.2.3"\n', "## v1.2.3 \n", None, None),
    )
    with pytest.raises(SystemExit, match="no release tag ancestor"):
        release.identity()

    out = tmp_path / "release.tex"
    monkeypatch.setattr(release, "OUT", out)
    monkeypatch.setattr(release, "render", lambda: "macro\n")
    monkeypatch.setattr(release, "named_commit", lambda: commit)
    monkeypatch.setattr(sys, "argv", ["gen_release.py", "--check"])
    assert release.main() == 1
    out.write_text("macro\n", encoding="utf-8")
    assert release.main() == 0
    monkeypatch.setattr(sys, "argv", ["gen_release.py"])
    assert release.main() == 0
    monkeypatch.setattr(sys, "argv", ["gen_release.py", "--commit"])
    with pytest.raises(SystemExit) as caught:
        runpy.run_path(str(SCRIPTS / "gen_release.py"), run_name="__main__")
    assert caught.value.code == 0


def test_mutant_disagreements_and_the_check(tmp_path, monkeypatch):
    mutant = load("mutant_differential.py")
    number, end = mutant.json_number_unbounded("0 ", 0)
    assert number == 0
    assert end > 0
    files = mutant.seed_files()
    produced = list(mutant.cases(files, 2, 30))
    assert len(produced) == 30
    monkeypatch.setattr(
        mutant.indep_reader,
        "read_text",
        lambda text: (_ for _ in ()).throw(RuntimeError("boom")),
    )
    crashed = mutant.read_indep("frame")
    assert crashed["verdict"] == "crash"
    monkeypatch.setattr(
        mutant.readcon, "read_con_string", lambda text: (_ for _ in ()).throw(KeyboardInterrupt())
    )
    with pytest.raises(KeyboardInterrupt):
        mutant.read_readcon("frame")

    class Odd(BaseException):
        pass

    monkeypatch.setattr(
        mutant.readcon, "read_con_string", lambda text: (_ for _ in ()).throw(Odd("odd"))
    )
    odd = mutant.read_readcon("frame")
    assert odd["verdict"] == "crash"
    monkeypatch.setattr(
        mutant,
        "read_indep",
        lambda text: {"verdict": "crash", "class": "Z", "line": None, "message": "m"},
    )
    assert mutant.explain("abc", {"verdict": "accept", "frames": 1}) is None
    monkeypatch.setattr(mutant, "seed_files", lambda: [("a.con", "x")])
    monkeypatch.setattr(
        mutant,
        "cases",
        lambda files, seed, count: iter([(0, "a.con", "x")]),
    )
    monkeypatch.setattr(
        mutant,
        "read_readcon",
        lambda text: {"verdict": "reject", "class": "Nope", "line": 1, "message": "m"},
    )
    monkeypatch.setattr(mutant, "explain", lambda text, got: None)
    record = mutant.run([1], 1)
    assert record["totals"]["unexplained"] == 1
    assert mutant.comparable(record)["build"]["readcon_version"]
    out = tmp_path / "mutants.json"
    saved = json.dumps(record)
    out.write_text(saved, encoding="utf-8")
    macros = tmp_path / "mutant_macros.tex"
    monkeypatch.setattr(mutant, "OUT", out)
    monkeypatch.setattr(mutant, "MACROS", macros)
    monkeypatch.setattr(mutant.provenance, "newer_sources", lambda path: [])
    monkeypatch.setattr(sys, "argv", ["mutant_differential.py", "--tables", "--out", str(out)])
    assert mutant.main() == 0
    assert macros.is_file()
    written = macros.read_text(encoding="utf-8")
    payload = json.loads(saved)
    payload["totals"]["cases"] = -1
    out.write_text(json.dumps(payload), encoding="utf-8")
    monkeypatch.setattr(
        sys,
        "argv",
        ["mutant_differential.py", "--check", "--seeds", "1", "--cases", "1", "--out", str(out)],
    )
    assert mutant.main() == 1
    out.write_text(saved, encoding="utf-8")
    macros.write_text("nope\n", encoding="utf-8")
    assert mutant.main() == 1
    macros.write_text(written, encoding="utf-8")
    assert mutant.main() == 0
    monkeypatch.setattr(mutant.provenance, "newer_sources", lambda path: ["src/ffi.rs"])
    with pytest.raises(SystemExit, match="older than"):
        mutant.main()
    monkeypatch.setattr(sys, "argv", ["mutant_differential.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "mutant_differential.py"), run_name="__main__")


def test_bench_dlpack_log_and_optional_consumers(tmp_path, monkeypatch):
    dlpack = load("bench_dlpack.py")
    monkeypatch.setattr(dlpack, "SIZES", (4,))
    monkeypatch.setattr(dlpack, "REPEATS", 2)
    monkeypatch.setattr(dlpack, "WARMUP", 1)
    monkeypatch.setattr(dlpack, "ROOT", tmp_path)
    monkeypatch.setattr(dlpack, "LOG", tmp_path / "dlpack.log")
    monkeypatch.setattr(dlpack, "OUT", tmp_path)
    monkeypatch.setattr(dlpack, "PAPER", tmp_path)
    log = "\n".join(
        [
            "rev_parse abcdef",
            "sample numpy from_dlpack n=4 rep=0 seconds=1.0e-6",
            "sample numpy copy n=4 rep=0 seconds=2.0e-6",
            "summary numpy copy n=4 n_rep=1 median=2.0e-6 q1=1.0e-6 q3=3.0e-6",
            "summary numpy from_dlpack n=4 n_rep=1 median=1.0e-6 q1=1.0e-6 q3=1.0e-6",
        ]
    )
    (tmp_path / "dlpack.log").write_text(log + "\n", encoding="utf-8")
    assert dlpack.from_log() == 0
    monkeypatch.setattr(dlpack, "SIZES", (4, 8))
    with pytest.raises(SystemExit, match="no numpy summary"):
        dlpack.from_log()
    monkeypatch.setattr(dlpack, "SIZES", (4,))
    (tmp_path / "dlpack.log").write_text(
        "rev_parse abcdef\n"
        "summary numpy copy n=4 n_rep=1 median=2.0e-6 q1=1.0e-6 q3=3.0e-6\n"
        "summary numpy from_dlpack n=4 n_rep=1 median=1.0e-6 q1=1.0e-6 q3=1.0e-6\n",
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="no numpy samples"):
        dlpack.from_log()

    class Arr:
        __array_interface__ = {"data": (11, False)}

    assert dlpack.pointer(Arr()) == 11
    samples = dlpack.time_call(lambda: None, 1, 1)
    assert len(samples) == 1

    import builtins

    real_import = builtins.__import__

    def block_jax(name, globals=None, locals=None, fromlist=(), level=0):
        if name == "jax" or name.startswith("jax."):
            raise ImportError("no jax")
        return real_import(name, globals, locals, fromlist, level)

    monkeypatch.setattr(builtins, "__import__", block_jax)
    notes = dlpack.load_consumers()[1]
    assert any("jax import failed" in note for note in notes)
    monkeypatch.setattr(builtins, "__import__", real_import)
    torch = types.ModuleType("torch")
    torch.__version__ = "0"
    torch.from_dlpack = lambda array: array
    cupy = types.ModuleType("cupy")
    cupy.__version__ = "0"
    cupy.from_dlpack = lambda array: array
    monkeypatch.setitem(sys.modules, "torch", torch)
    monkeypatch.setitem(sys.modules, "cupy", cupy)
    loaded = dlpack.load_consumers()[0]
    assert "torch" in loaded and "cupy" in loaded

    import readcon

    original_read = readcon.read_con
    monkeypatch.setattr(dlpack, "ROOT", ROOT)
    monkeypatch.setattr(readcon, "read_con", lambda path: [])
    monkeypatch.setattr(dlpack, "LOG", tmp_path / "fail.log")
    monkeypatch.setattr(sys, "argv", ["bench_dlpack.py"])
    assert dlpack.main() == 1
    monkeypatch.setattr(readcon, "read_con", original_read)
    monkeypatch.setattr(dlpack, "SIZES", (4,))
    monkeypatch.setattr(dlpack, "REPEATS", 1)
    monkeypatch.setattr(dlpack, "WARMUP", 0)
    monkeypatch.setattr(dlpack, "LOG", tmp_path / "live.log")
    monkeypatch.setattr(dlpack, "OUT", tmp_path)
    monkeypatch.setattr(dlpack, "PAPER", tmp_path)

    def fail_late(array):
        raise RuntimeError("nope")

    monkeypatch.setattr(
        dlpack,
        "load_consumers",
        lambda: ({"numpy": dlpack.np.from_dlpack, "torch": fail_late}, ["torch 0"]),
    )
    monkeypatch.setattr(dlpack, "plot", lambda exchange, copies: None)
    monkeypatch.setattr(sys, "argv", ["bench_dlpack.py"])
    assert dlpack.main() == 0
    assert "consumer_fail torch" in (tmp_path / "live.log").read_text(encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["bench_dlpack.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "bench_dlpack.py"), run_name="__main__")


def test_number_rules_cover_the_other_guards(monkeypatch):
    front = load("number_rules/front.py")
    real_grep = front.grep
    real_param = front.param
    real_text = front.text
    real_si = front.si_factor

    def grep_endpoint(rel, pat):
        if "ENDPOINT_FMAX = " in pat:
            return "nope"
        return real_grep(rel, pat)

    monkeypatch.setattr(front, "grep", grep_endpoint)
    with pytest.raises(SystemExit, match="endpoint tolerance"):
        front.agree_restart()

    def param_endpoint(name):
        if name == "endpoint_fmax_eV_A":
            return -1
        return real_param(name)

    monkeypatch.setattr(front, "grep", real_grep)
    monkeypatch.setattr(front, "param", param_endpoint)
    with pytest.raises(SystemExit, match="endpoint_fmax_eV_A"):
        front.agree_restart()

    def grep_ase(rel, pat):
        if "ASE_DEFAULT_FMAX" in pat:
            return "99"
        return real_grep(rel, pat)

    monkeypatch.setattr(front, "param", real_param)
    monkeypatch.setattr(front, "grep", grep_ase)
    with pytest.raises(SystemExit, match="ASE_DEFAULT_FMAX"):
        front.agree_restart()
    monkeypatch.setattr(front, "grep", real_grep)
    monkeypatch.setattr(front, "text", lambda rel: "")
    with pytest.raises(SystemExit, match="ENDPOINT_FMAX"):
        front.agree_restart()
    monkeypatch.setattr(front, "text", real_text)

    def param_spring(name):
        if name == "spring_eV_A2":
            return -1
        return real_param(name)

    monkeypatch.setattr(front, "param", param_spring)
    with pytest.raises(SystemExit, match="spring_eV_A2"):
        front.agree_restart()

    def param_sites(name):
        if name == "sites":
            return [[0, 0, 0], [0, 1, 0]]
        return real_param(name)

    monkeypatch.setattr(front, "param", param_sites)
    with pytest.raises(SystemExit, match="neighbouring hollows"):
        front.agree_restart()
    monkeypatch.setattr(front, "param", real_param)

    def si_angstrom(aliases):
        if "angstrom" in aliases:
            return 1.0
        return real_si(aliases)

    monkeypatch.setattr(front, "si_factor", si_angstrom)
    with pytest.raises(SystemExit, match="scipy.constants"):
        front.agree_units()

    def si_amu(aliases):
        if aliases.startswith('&["u"'):
            return 1.0
        return real_si(aliases)

    monkeypatch.setattr(front, "si_factor", si_amu)
    with pytest.raises(SystemExit, match="CODATA 2018"):
        front.agree_units()
    monkeypatch.setattr(front, "si_factor", real_si)
    monkeypatch.setattr(front, "ABOVE_2_53", 3)
    monkeypatch.setattr(front, "BIG_IDS", [1])
    with pytest.raises(SystemExit, match="2\\^53"):
        front.agree_ids()
    monkeypatch.setattr(front, "variance_contours", lambda: [0.5])
    with pytest.raises(SystemExit, match="variance contours"):
        front.agree_contours()
    monkeypatch.setattr(front, "variance_contours", lambda: [0.05, 0.95])
    monkeypatch.setattr(front, "grep", lambda rel, pat: "19")
    with pytest.raises(SystemExit, match="DEFAULT_READ_SIZE"):
        front.agree_contours()
    monkeypatch.setattr(front, "text", lambda rel: "print('x')\n")
    with pytest.raises(SystemExit, match="percentiles"):
        front.bootstrap_percent()

    supp = load("number_rules/supp.py")
    real_jget = supp.jget

    def jget_vacuum(rel, key):
        if key == "parameters.vacuum_A":
            return -1
        return real_jget(rel, key)

    monkeypatch.setattr(supp, "jget", jget_vacuum)
    with pytest.raises(SystemExit, match="vacuum"):
        supp.agree_supp()
    monkeypatch.setattr(supp, "jget", real_jget)
    real_grep = supp.grep
    monkeypatch.setattr(
        supp,
        "grep",
        lambda rel, pat: "0" if rel == supp.ENVIRONMENT else real_grep(rel, pat),
    )
    with pytest.raises(SystemExit, match="is not the"):
        supp.agree_supp()


def test_provenance_digest_host_and_main(tmp_path, monkeypatch):
    provenance = load("provenance.py")
    digest = provenance.library_digest_at("HEAD")
    assert digest is None or len(digest) == 64
    shared = provenance.readcon_module()
    assert shared is not None
    compiler = provenance._rustc_of(shared)
    assert compiler
    hardware = provenance.hardware()
    assert hardware["uname_m"]
    assert provenance.cpu_record("Xeon", "", "", "") == "Xeon"
    original = Path.read_text

    def missing(self, *args, **kwargs):
        if self.name in {"cpuinfo", "meminfo"}:
            raise OSError("absent")
        return original(self, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", missing)
    host = provenance.host()
    assert host["cpu_model"]
    monkeypatch.setattr(sys, "argv", ["provenance.py", "--digest"])
    runpy.run_path(str(SCRIPTS / "provenance.py"), run_name="__main__")
    monkeypatch.setattr(sys, "argv", ["provenance.py"])
    runpy.run_path(str(SCRIPTS / "provenance.py"), run_name="__main__")
