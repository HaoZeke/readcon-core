"""Bench and example output must not record a machine name."""

from __future__ import annotations

import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "benches"))

from cpu_model import (  # noqa: E402
    _field,
    cpu_model,
    cpu_record,
    platform_label,
    strip_host_token,
)

OUTPUT_DIRS = (ROOT / "benches" / "results", ROOT / "examples")
SOURCE_GLOBS = (
    (ROOT / "benches", "*.py"),
    (ROOT / "examples", "*.rs"),
    (ROOT / "scripts", "bench_*.py"),
)
FORBIDDEN_CALLS = (
    "gethostname",
    'Command::new("hostname")',
    "Command::new('hostname')",
)


def _read_hostname_file() -> str:
    try:
        return Path("/etc/hostname").read_text(encoding="utf-8")
    except OSError:
        return ""


def _machine_names() -> set[str]:
    names = set()
    for raw in (platform.node(), _read_hostname_file()):
        node = raw.strip()
        if node and node.lower() != "localhost":
            names.add(node)
    return names


def _text_files(path: Path):
    if not path.exists():
        return
    if path.is_file():
        yield path
        return
    for child in path.rglob("*"):
        if child.is_file() and ".git" not in child.parts:
            yield child


def test_cpu_record_keeps_the_model_name_and_the_ids(monkeypatch):
    assert (
        cpu_record("Intel(R) Xeon(R) Platinum 8480+", "6", "143", "8")
        == "Intel Xeon Platinum 8480+ family 6 model 143 stepping 8"
    )
    assert cpu_record("", "6", "", "") == "family 6"
    assert cpu_record("EPYC", "", "", "") == "EPYC"
    assert _field("no-colon\nmodel : 7\n", "model") == "7"
    assert _field("model name : Xeon\n", "model") == ""
    def read(self, *args, **kwargs):
        assert str(self) == "/proc/cpuinfo"
        return (
            "model name : Intel Xeon Platinum 8480+\n"
            "cpu family : 6\nmodel : 143\nstepping : 8\n"
        )

    monkeypatch.setattr(Path, "read_text", read)
    assert cpu_model() == "Intel Xeon Platinum 8480+ family 6 model 143 stepping 8"

    def missing(self, *args, **kwargs):
        assert str(self) == "/proc/cpuinfo"
        raise OSError("absent")

    monkeypatch.setattr(Path, "read_text", missing)
    assert cpu_model() == "unknown"

    def blank(self, *args, **kwargs):
        assert str(self) == "/proc/cpuinfo"
        return "processor : 0\n"

    monkeypatch.setattr(Path, "read_text", blank)
    assert cpu_model() == "unknown"


def test_platform_label_uses_unknown_when_the_names_are_blank(monkeypatch):
    monkeypatch.setattr(platform, "system", lambda: "  ")
    monkeypatch.setattr(platform, "machine", lambda: "")
    monkeypatch.setattr(platform, "platform", lambda: "Linux")
    monkeypatch.setattr(platform, "node", lambda: "workstation")
    label = platform_label()
    assert label.startswith("unknown-unknown")
    assert "workstation" not in label.split("-")


def test_localhost_is_not_recorded(monkeypatch):
    monkeypatch.setattr(platform, "node", lambda: "localhost")

    def read(self, *args, **kwargs):
        assert self == Path("/etc/hostname")
        return "localhost\n"

    monkeypatch.setattr(Path, "read_text", read)
    assert _machine_names() == set()


def test_hostname_and_binary_files(monkeypatch, tmp_path):
    def read(self, *args, **kwargs):
        assert self == Path("/etc/hostname")
        raise OSError("absent")

    monkeypatch.setattr(Path, "read_text", read)
    assert _read_hostname_file() == ""
    monkeypatch.undo()
    folder = tmp_path / "out"
    folder.mkdir()
    plain = folder / "note.txt"
    plain.write_text("plain", encoding="utf-8")
    binary = folder / "raw.bin"
    binary.write_bytes(b"\xff")
    hidden = tmp_path / ".git" / "config"
    hidden.parent.mkdir()
    hidden.write_text("x", encoding="utf-8")
    assert list(_text_files(plain)) == [plain]
    assert list(_text_files(tmp_path / "missing")) == []
    assert plain in list(_text_files(tmp_path))
    assert hidden not in list(_text_files(tmp_path))
    seen = []
    for path in _text_files(folder):
        try:
            seen.append(path.read_text(encoding="utf-8"))
        except UnicodeDecodeError:
            continue
    assert seen == ["plain"]


def test_strip_host_token_removes_only_a_whole_component():
    label = "Linux-7.0.13-arch1-1-rg-x86_64-with-glibc2.43"
    assert strip_host_token(label, "rg") == "Linux-7.0.13-arch1-1-x86_64-with-glibc2.43"
    assert strip_host_token(label, "r") == label
    assert strip_host_token(label, "") == label


def test_platform_label_omits_the_kernel_release_and_the_hostname():
    label = platform_label()
    assert platform.release() not in label
    for name in _machine_names():
        assert name not in label.split("-")


def test_python_and_rust_cpu_model_agree():
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "--offline",
            "--lib",
            "hostlabel::tests::cpu_model_value",
            "--",
            "--exact",
            "--nocapture",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    rust = None
    for line in (proc.stdout + "\n" + proc.stderr).splitlines():
        marker = "CPU_MODEL="
        if marker in line:
            rust = line.split(marker, 1)[1].strip()
    assert rust is not None
    assert cpu_model() == rust


def test_bench_and_example_output_has_no_hostname():
    names = _machine_names()
    junk = ROOT / "examples" / "_binary_skip.bin"
    junk.write_bytes(b"\xff")
    try:
        _scan_outputs(names)
    finally:
        junk.unlink()


def _scan_outputs(names):
    for directory in OUTPUT_DIRS:
        for path in _text_files(directory):
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            for name in names:
                assert name not in text, f"{path} contains {name}"


def test_writers_do_not_call_hostname():
    for directory, pattern in SOURCE_GLOBS:
        for path in directory.glob(pattern):
            text = path.read_text(encoding="utf-8")
            for call in FORBIDDEN_CALLS:
                assert call not in text, f"{path} calls {call}"
            if path.name != "cpu_model.py":
                assert "platform.node" not in text, f"{path} reads platform.node"
