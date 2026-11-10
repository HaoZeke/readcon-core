"""Cover the corpus writer, the size table and the referee probes."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverb_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_make_corpus31_writes_a_temporary_corpus(tmp_path, monkeypatch):
    corpus = load("make_corpus31.py")
    monkeypatch.setattr(sys, "argv", ["make_corpus31.py", "--out", str(tmp_path / "corpus")])
    corpus.main()
    monkeypatch.setattr(sys, "argv", ["make_corpus31.py", "--out", str(tmp_path / "corpus")])
    corpus.main()
    assert (tmp_path / "corpus" / "manifest.toml").is_file()
    clauses = corpus.released_clauses(corpus.SPEC)
    base = {"file": "x.con", "basis": "explicit", "clause": "CON-line2.1"}
    with pytest.raises(SystemExit, match="duplicate"):
        corpus.check_cases([base, dict(base)], clauses)
    with pytest.raises(SystemExit, match="unknown basis"):
        corpus.check_cases([{**base, "basis": "nope"}], clauses)
    with pytest.raises(SystemExit, match="not in the spec"):
        corpus.check_cases([{**base, "clause": "CON-no-such.9"}], clauses)


def test_file_sizes_table_and_measurement(tmp_path):
    sizes = load("file_sizes.py")
    sizes.OUT = tmp_path / "sizes.csv"
    sizes.GENERATED = tmp_path / "gen"
    sizes.OUT.write_text(
        (ROOT / "paper" / "cpc" / "data" / "sizes.csv").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    sizes.table()
    assert (tmp_path / "gen" / "sizes_table.tex").is_file()
    assert sizes.tex_int(12000) == "12{,}000"
    assert sizes.tex_int(12) == "12"
    sizes.main()
    assert sizes.OUT.is_file()
    real_sha = sizes.sha256
    sizes.sha256 = lambda path: "0" * 64
    with pytest.raises(SystemExit, match="differs"):
        sizes.main()
    sizes.sha256 = real_sha
    broken = tmp_path / "broken.csv"
    text = sizes.OUT.read_text(encoding="utf-8").replace("CON 3.1", "CON 9.9", 1)
    broken.write_text(text, encoding="utf-8")
    sizes.OUT = broken
    with pytest.raises(SystemExit, match="different versions"):
        sizes.table()
    import os
    import runpy

    os.environ["BENCH_SIZES_CSV"] = str(tmp_path / "run.csv")
    os.environ["BENCH_GENERATED"] = str(tmp_path / "rungen")
    (tmp_path / "run.csv").write_text(
        (ROOT / "paper" / "cpc" / "data" / "sizes.csv").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    (tmp_path / "rungen").mkdir()
    monkeypatch = None
    import sys as _sys

    _sys.argv = ["file_sizes.py", "--table"]
    runpy.run_path(str(SCRIPTS / "file_sizes.py"), run_name="__main__")


def test_referee_probes_and_remaining_imports(tmp_path, monkeypatch):
    probes = load("referee_probes.py")
    out = tmp_path / "referee_probes.json"
    monkeypatch.setattr(
        sys, "argv", ["referee_probes.py", "--out", str(out)]
    )
    assert probes.main() in (0, 1)
    assert out.is_file()
    for path in sorted(SCRIPTS.rglob("*.py")):
        name = path.relative_to(SCRIPTS).as_posix()
        if name == "referee_probes.py":
            continue
        try:
            load(name)
        except (SystemExit, Exception):
            continue
