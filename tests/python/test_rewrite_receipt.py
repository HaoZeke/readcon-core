"""The receipt rewriter takes path prefixes from the environment."""

import importlib.util
from pathlib import Path

import pytest

SOURCE = Path(__file__).resolve().parents[2] / "scripts" / "rewrite_receipt.py"


def _module():
    spec = importlib.util.spec_from_file_location("rewrite_receipt", SOURCE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_rewrite_receipt_names_no_host_prefix():
    text = SOURCE.read_text(encoding="utf-8")
    host = "/home/" + "ubuntu"
    cargo = "/usr/" + "local/cargo"
    assert host not in text
    assert cargo not in text


def test_rewrite_receipt_uses_the_environment_prefix(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("H2H_PATH_PREFIXES", "/opt/cargo/registry=cargo-registry,/opt/cargo=cargo")
    module = _module()
    text, used = module.rewrite(
        "compile /opt/cargo/registry/src/example.rs in /work/repo/src/lib.rs",
        "/work/repo",
        module.prefix_pairs(),
    )
    assert "/opt/cargo" not in text
    assert "cargo-registry/src/example.rs" in text
    assert "./src/lib.rs" in text
    assert used
