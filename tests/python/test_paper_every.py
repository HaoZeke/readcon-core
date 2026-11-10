"""Call every paper script entry point that can run on the records in this tree."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    key = "every_" + name.replace("/", "_").replace(".py", "")
    spec = importlib.util.spec_from_file_location(key, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[key] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_host_from_log_writes_the_public_table_and_rejects_a_gap(tmp_path):
    host = load("host_from_log.py")
    assert host.main() == 0
    assert (host.GEN / "host_table.tex").is_file()
    with pytest.raises(SystemExit, match="has no"):
        host.field("rev_parse only\n", "cpu_model")
    with pytest.raises(SystemExit, match="does not hold"):
        host.version_at("deadbeef")
    with pytest.raises(SystemExit, match="no version"):
        host.cxx_macro("no digits here")
    assert host.cxx_macro("g++ (Ubuntu) 13.3.0") == "g++ 13.3.0"
    assert host.exact_tag("d5e77f997438557c72c3b3e360d446310f85d6b7") == "v0.18.0"
    assert host.exact_tag("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") is None
    text = host.table(
        {"readcon_tip": {"error": "missing & sample"}},
        ("readcon_tip", "readcon_base"),
        "caption",
        "tab:x",
    )
    assert "not run" in text
    assert "missing" in text
    assert host.tex_escape("a&b%c$d#e_f{g}h~i^j\\k")
    assert host.num("1.23456", 4)
    tagged = host.LOG.read_text(encoding="utf-8").replace(
        "baseline_commit ac4f437989dea5aad9f75ff045af09ccf7127eba",
        "baseline_commit 12ca800bc2212c565941e04094cba6b4d99a62e0",
    )
    log = tmp_path / "host.log"
    log.write_text(tagged, encoding="utf-8")
    host.LOG = log
    host.GEN = tmp_path / "gen"
    assert host.main() == 0

    def cargo_without_version(*args, **kwargs):
        import subprocess as sp

        return sp.CompletedProcess(args, 0, stdout='name = "x"\n', stderr="")

    original = host.subprocess.run
    host.subprocess.run = cargo_without_version
    host.exact_tag = lambda commit: None
    try:
        with pytest.raises(SystemExit, match="no version"):
            host.version_at("d5e77f997438557c72c3b3e360d446310f85d6b7")
    finally:
        host.subprocess.run = original


def test_bench_readers_helpers_and_a_fast_main(tmp_path, monkeypatch):
    bench = load("bench_readers.py")
    assert bench.hwm_kib() > 0
    assert bench.filesystem(ROOT)
    assert bench._os_name()
    assert any(line.startswith("cpu_family ") for line in bench._hardware_lines())
    assert bench.cpu_model()
    assert bench.thread_count("readcon_full_t8") == 8
    assert bench.thread_count("readcon_full_t1") == 1
    assert bench.eon_build_argv()[0].endswith("g++")
    sample = bench.reference(2, 2)
    assert sample.shape == (2, 2, 3)
    med, iqr = bench.type7([1.0, 2.0, 3.0, 4.0])
    assert med == 2.5
    assert iqr > 0
    assert "sha256=" in bench.binary_lines()[0]
    original_rebuild = bench.rebuild_eon_allframes
    monkeypatch.setattr(bench, "rebuild_eon_allframes", lambda: None)
    monkeypatch.setattr(bench, "N_FRAMES", 2)
    monkeypatch.setattr(bench, "LOG", tmp_path / "bench.log")
    monkeypatch.setattr(bench, "WORK", tmp_path / "work")

    def ok(name, path):
        version = "0.18.0" if name.startswith(("readcon_full", "readcon_pos")) else "0.17.3"
        if name == "numpy":
            return {
                "error": "short",
                "seconds": None,
                "mib": None,
                "matched": 0,
                "got": 0,
                "requested": 2,
                "exit": 1,
            }
        return {
            "seconds": 0.01,
            "mib": 1.0,
            "matched": 2,
            "got": 2,
            "requested": 2,
            "exit": 0,
            "error": "",
            "module_version": version,
            "module_file": "readcon",
            "module_sha256": "ab",
        }

    original_one = bench.one
    monkeypatch.setattr(bench, "one", ok)
    monkeypatch.setattr(bench, "VARIANT", "public")
    assert bench.main() == 0
    monkeypatch.setattr(bench, "VARIANT", "tip")

    def wrong(name, path):
        doc = ok(name, path)
        doc["module_version"] = "9.9.9"
        return doc

    monkeypatch.setattr(bench, "one", wrong)
    monkeypatch.setattr(bench, "VARIANT", "public")
    assert bench.main() == 0
    bench.one = original_one
    text = (tmp_path / "bench.log").read_text(encoding="utf-8")
    assert "summary arm=numpy error=short" in text
    assert "exit_code 0" in text

    def compiled(*args, **kwargs):
        return None

    bench.rebuild_eon_allframes = original_rebuild
    monkeypatch.setattr(bench.subprocess, "run", compiled)
    bench.rebuild_eon_allframes()
    missing = tmp_path / "no-binary"
    monkeypatch.setattr(bench, "EON_BIN", missing)
    assert "sha256=missing" in " ".join(bench.binary_lines())
    real_read = Path.read_text

    def read_text(self, encoding="utf-8", errors=None):
        if self.name == "mounts":
            return "short\n"
        if self.name == "os-release":
            return "NAME=test\n"
        if self.name == "cpuinfo":
            return "processor : 0\n"
        if self.name == "status":
            return "Name: test\n"
        return real_read(self, encoding=encoding)

    monkeypatch.setattr(Path, "read_text", read_text)
    with pytest.raises(SystemExit, match="no filesystem"):
        bench.filesystem(tmp_path)
    with pytest.raises(SystemExit, match="PRETTY_NAME"):
        bench._os_name()
    assert bench.cpu_model() == "unknown"

    def read_family(self, encoding="utf-8", errors=None):
        if self.name == "cpuinfo":
            return "cpu family : 6\n"
        return real_read(self, encoding=encoding)

    monkeypatch.setattr(Path, "read_text", read_family)
    assert bench.cpu_model() == "family 6"

    class Status:
        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def __iter__(self):
            return iter(["Name: test\n"])

    monkeypatch.setattr("builtins.open", lambda *args, **kwargs: Status())
    assert bench.hwm_kib() == 0

    class Proc:
        def __init__(self, ok):
            self.returncode = 0 if ok else 1
            self.stdout = (
                '{"seconds": 0.2, "hwm_before": 10, "hwm_after": 1034, '
                '"matched": 2, "got": 2, "requested": 2, "error": ""}\n'
                if ok
                else "not json\n"
            )
            self.stderr = ""

    def run_arm(argv, input=None, text=None, capture_output=None, env=None, **kwargs):
        arm = argv[3] if len(argv) > 3 else ""
        return Proc(arm != "numpy")

    monkeypatch.setattr(bench.subprocess, "run", run_arm)
    for arm in ("eon", "eon_py", "readcon_eon_api", "numpy"):
        doc = bench.one(arm, tmp_path / "frames.con")
        assert "exit" in doc


