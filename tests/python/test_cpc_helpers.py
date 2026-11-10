"""Behavioral checks for the paper helpers the reader exchange and the host line use."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(name.replace("/", "_").replace(".py", ""), path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_logged_cmd_keeps_a_plain_line_and_drops_a_temp_path(tmp_path, monkeypatch):
    logged = load("logged_cmd.py")
    monkeypatch.setattr(sys, "argv", ["logged_cmd.py"])
    assert logged.main() == 2
    out = tmp_path / "run.txt"
    monkeypatch.setattr(
        sys,
        "argv",
        ["logged_cmd.py", str(out), "bash", "-c", "echo hello; echo /tmp/hidden"],
    )
    assert logged.main() == 0
    text = out.read_text(encoding="utf-8")
    body = [line for line in text.splitlines() if not line.startswith("command ")]
    assert "hello" in body
    assert not any("/tmp" in line or "hidden" in line for line in body)
    assert "exit_code 0" in text


def test_mpi_main_rejects_a_short_sample_and_states_the_slower_rank(tmp_path, monkeypatch):
    mpi = load("mpi_from_log.py")
    short = tmp_path / "short.log"
    short.write_text("repeats 1\nrepeat 0 run shared pad pad 1 ns 1000000000\n", encoding="utf-8")
    monkeypatch.setattr(mpi, "LOG", short)
    monkeypatch.setattr(mpi, "GEN", tmp_path)
    assert mpi.main() == 1
    lines = ["repeats 6"]
    for mode, slow in (("shared", False), ("scatter", True)):
        for ranks in (1, 2, 4, 8):
            base = 2.0 if ranks == 8 and slow else 1.0 / ranks
            for i in range(6):
                ns = int((base + i * 0.01) * 1e9)
                lines.append(f"repeat {i} run {mode} pad pad {ranks} ns {ns}")
    slow_log = tmp_path / "slow.log"
    slow_log.write_text("\n".join(lines) + "\n", encoding="utf-8")
    monkeypatch.setattr(mpi, "LOG", slow_log)
    assert mpi.main() == 0
    written = (tmp_path / "mpi_macros.tex").read_text(encoding="utf-8")
    assert "eight-rank median" in written


def test_eon_build_log_keeps_a_line_without_a_local_path(monkeypatch, tmp_path):
    eon = load("eon_build_log.py")

    class Proc:
        stdout = "ninja: no work to do\n"
        stderr = ""
        returncode = 0

    monkeypatch.setattr(eon.subprocess, "run", lambda *args, **kwargs: Proc())
    monkeypatch.setattr(eon.subprocess, "check_output", lambda *args, **kwargs: "abc\n")
    monkeypatch.setattr(eon, "LOG", tmp_path / "eon-build.log")
    monkeypatch.setattr(eon, "EON", tmp_path)
    assert eon.main() == 0
    text = (tmp_path / "eon-build.log").read_text(encoding="utf-8")
    assert "ninja: no work to do" in text
    assert "exit_code 0" in text


def test_logged_cmd_redacts_the_checkout_and_the_home():
    logged = load("logged_cmd.py")
    home = str(Path.home())
    raw = f"{ROOT}/src/lib.rs {home}/.local/lib site-packages/nanobind/x"
    cleaned = logged.redact(raw)
    assert str(ROOT) not in cleaned
    assert home not in cleaned
    assert "<site-packages>/nanobind/" in cleaned
    assert logged.redact("") == ""


def test_host_table_keeps_a_missing_arm_and_escapes_text():
    host = load("host_from_log.py")
    text = "\n".join(
        [
            "summary arm=readcon_full_t1 median_s=1.5 iqr_s=0.02 mib_median=12.5 matched=10 requested=10",
            "summary arm=ovito error=no_module",
            "cpu_model Intel Xeon Platinum 8480+ family 6 model 143 stepping 8",
        ]
    )
    found = host.rows(text)
    assert found["readcon_full_t1"]["median_s"] == "1.5"
    assert host.arm_s(found, "readcon_full_t1") == 1.5
    assert host.arm_mib(found, "readcon_full_t1") == 12.5
    assert host.num("1.23456", 4) == "1.235"
    assert host.tex_escape("a_b & c") == r"a\_b \& c"
    table = host.table(
        found,
        ("readcon_full_t1", "ovito", "pymatgen"),
        "Readers",
        "tab:readers",
    )
    assert "1.5" in table
    assert "not run" in table
    assert "no\\_module" in table
    assert host.field(text, "cpu_model").startswith("Intel Xeon Platinum")
    assert host.cxx_macro("g++ (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0") == "g++ 13.3.0"


def test_db_and_mpi_samples_use_type7():
    db = load("db_from_log.py")
    found = db.samples("repeat 1 kind readcon-db seconds 1.5\nrepeat 2 kind readcon-db seconds 2.5\n")
    assert found["readcon-db"] == [1.5, 2.5]
    assert db.median([1.0, 2.0, 3.0]) == 2.0
    mpi = load("mpi_from_log.py")
    text = (
        "repeat 0 run shared pad pad 4 ns 1000000000\n"
        "repeat 1 run shared pad pad 4 ns 3000000000\n"
    )
    samples = mpi.samples(text)
    assert samples[("shared", 4)] == [1.0, 3.0]
    median, iqr = mpi.type7([1.0, 2.0, 3.0, 4.0])
    assert median == float(np.percentile([1.0, 2.0, 3.0, 4.0], 50, method="linear"))
    assert iqr > 0
    assert mpi.fmt(1.234567) == "1.2346"


def test_bench_identity_hashes_the_extension_and_keeps_the_model(tmp_path):
    bench = load("bench_readers.py")
    median, iqr = bench.type7([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
    assert median == 3.5
    assert iqr > 0
    package = tmp_path / "readcon"
    package.mkdir()
    init = package / "__init__.py"
    init.write_text("x", encoding="utf-8")
    extension = package / "readcon.cpython-312-x86_64-linux-gnu.so"
    extension.write_bytes(b"extension-bytes")
    meta = tmp_path / "readcon-0.18.1.dist-info"
    meta.mkdir()
    (meta / "METADATA").write_text("Version: 0.18.1\n", encoding="utf-8")

    class Module:
        __file__ = str(init)
        __version__ = ""

    start = bench.WORKER.index("def module_identity")
    end = bench.WORKER.index("\nif arm in READCON_ARMS")
    import os

    namespace: dict = {"os": os}
    exec(bench.WORKER[start:end], namespace)
    version, shown, digest = namespace["module_identity"](Module())
    assert version == "0.18.1"
    assert shown.endswith(".so")
    assert digest
    assert "family" in bench.cpu_model()
    lines = bench._hardware_lines()
    assert any(line.startswith("cpu_family ") for line in lines)
    assert any(line.startswith("cxx g++ ") for line in lines)
    assert bench._os_name()
