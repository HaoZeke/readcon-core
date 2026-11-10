"""Failure branches of the paper scripts, on fixtures and patched inputs."""

from __future__ import annotations

import importlib.util
import json
import os
import runpy
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    key = "branch_" + name.replace("/", "_").replace(".py", "")
    spec = importlib.util.spec_from_file_location(key, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[key] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_small_helpers(tmp_path, monkeypatch):
    error = load("error_class.py")

    class Exc(Exception):
        kind = "IncompleteVelocitySection"
        line = 0

    assert error.classify(Exc())[0] == "IncompleteSection"
    assert error.classify(Exception("line 3: invalid count: x"))[:2] == (
        "InvalidCount",
        3,
    )
    assert error.classify(Exception("no such message"))[0] == "Other"

    numbers = load("numbers_lib.py")
    table = tmp_path / "rows.csv"
    table.write_text("a,b\n1,2\n3,4\n", encoding="utf-8")
    monkeypatch.setattr(numbers, "path", lambda rel: table)
    with pytest.raises(SystemExit, match="matched 0 rows"):
        numbers.row("rows.csv", a="9")
    assert numbers.cell("rows-b.csv", "b", a="1") == 2.0
    script = tmp_path / "driver.py"
    script.write_text("WIDTH = 1.5\n", encoding="utf-8")
    monkeypatch.setattr(numbers, "text", lambda rel: script.read_text())
    with pytest.raises(SystemExit, match="no match"):
        numbers.grep("driver.py", r"missing (\d+)")
    assert numbers.const("driver.py", "WIDTH") == 1.5
    span = numbers.Rule("1", 1, "s", "r", where=("doc:10-20",))
    assert span.applies("doc", 12)
    assert span.applies("doc", 3) is False

    font = load("vendored_font.py")
    monkeypatch.setattr(font, "FONTS", tmp_path / "empty")
    (tmp_path / "empty").mkdir()
    with pytest.raises(SystemExit, match="no vendored"):
        font.register()

    figures = load("gen_figures.py")
    monkeypatch.setattr(figures, "FONT_CANDIDATES", (tmp_path / "missing.otf",))
    with pytest.raises(SystemExit, match="Latin Modern"):
        figures.latin_modern()


def test_plot_figures_stop_on_a_bad_face_or_setting(tmp_path, monkeypatch):
    import matplotlib.pyplot as plt

    plot = load("plot_figures.py")
    monkeypatch.setattr(plot, "_LM_CANDIDATES", (tmp_path / "missing.otf",))
    with pytest.raises(SystemExit, match="Latin Modern"):
        plot.latin_modern_path()

    import matplotlib.font_manager as fm

    def missing(family, fallback_to_default=False):
        raise ValueError("absent")

    monkeypatch.setattr(fm, "findfont", missing)
    with pytest.raises(SystemExit, match="not installed"):
        plot.require_theme_font("No Such Face")

    def other(family, fallback_to_default=False):
        return str(tmp_path / "other.ttf")

    monkeypatch.setattr(fm, "findfont", other)
    monkeypatch.setattr(
        fm.FontProperties,
        "get_name",
        lambda self: "DejaVu Sans",
    )
    with pytest.raises(SystemExit, match="resolved to"):
        plot.require_theme_font("Atkinson Hyperlegible")

    def settings(*args, **kwargs):
        return {
            "plot_type": "scatter",
            "project_path": False,
            "surface_type": "other",
            "ira_kmax": 1,
            "show_pts": False,
            "theme": "ruhi",
        }

    monkeypatch.setattr(plot, "merge_plot_settings", settings)
    with pytest.raises(SystemExit, match="landscape"):
        plot.house_style()
    def surface(*args, **kwargs):
        return {
            "plot_type": "landscape",
            "project_path": True,
            "surface_type": "other",
            "ira_kmax": 1,
            "show_pts": True,
            "theme": "ruhi",
        }

    monkeypatch.setattr(plot, "merge_plot_settings", surface)
    with pytest.raises(SystemExit, match="surface_type"):
        plot.house_style()

    def points(*args, **kwargs):
        return {
            "plot_type": "landscape",
            "project_path": True,
            "surface_type": "grad_imq_ny",
            "ira_kmax": 14,
            "show_pts": False,
            "theme": "ruhi",
        }
    monkeypatch.setattr(plot, "merge_plot_settings", points)
    with pytest.raises(SystemExit, match="show-pts"):
        plot.house_style()

    monkeypatch.setattr(plot, "OUT", tmp_path)
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    plot._save(fig, "tiny", tmp_path)
    assert (tmp_path / "tiny.png").is_file()

    def no_surface(*args, **kwargs):
        return None

    monkeypatch.setattr(plot, "plot_landscape_surface", no_surface)

    class Ax:
        def contourf(self, *args, **kwargs):
            return "cs"

        def clabel(self, *args, **kwargs):
            return []

    band = {"r": [0.0], "p": [0.0], "grad_r": [0.0], "grad_p": [0.0], "z": [0.0]}
    with pytest.raises(SystemExit, match="no filled contour"):
        plot._draw_surface(Ax(), band, None, "viridis")

    fig, ax = plt.subplots()
    plot._companion(ax, [0.0, 1.0], [1.0, 2.0], "k", "o", "-", "band", saddle=False)
    plt.close(fig)


def test_regen_figures_records_a_failure_and_a_skip(tmp_path, monkeypatch):
    regen = load("regen_figures.py")
    monkeypatch.setattr(regen, "LOG", tmp_path / "regen.log")

    def boom():
        raise RuntimeError("plot failed")

    monkeypatch.setattr(regen.gen_figures, "main", boom)
    monkeypatch.setattr(regen.plot_structures, "main", lambda: None)
    monkeypatch.setattr(regen.plot_restart_formats, "main", lambda: None)
    monkeypatch.setattr(regen.plot_supp, "main", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "dlpack", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "toc", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "mpi_plots", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "db_plots", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "anatomy_and_errors", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "landscapes", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "timing_and_fairness", lambda: None)
    monkeypatch.setattr(regen.plot_union_extra, "main", lambda: None)
    real_is_dir = Path.is_dir

    def skip_evidence(self):
        if self.name == "neb":
            return False
        return real_is_dir(self)

    monkeypatch.setattr(Path, "is_dir", skip_evidence)
    assert regen.main() == 1
    text = (tmp_path / "regen.log").read_text(encoding="utf-8")
    assert "fail gen_figures" in text
    assert "skip restart" in text


def test_regen_paper_checks_the_archive_and_the_library(tmp_path, monkeypatch):
    paper = load("regen_paper.py")
    monkeypatch.setattr(paper, "CPC", tmp_path)
    (tmp_path / "records.tar.xz").write_bytes(b"not an archive")
    (tmp_path / "records.sha256").write_text("00 records.tar.xz\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="does not match"):
        paper.unpack_records()
    (tmp_path / "ok.py").write_text("print('ok')\n", encoding="utf-8")
    ledger: list = []
    paper.run(["ok.py"], os.environ.copy(), ledger)
    assert ledger and ledger[0]["step"] == "ok.py"

    monkeypatch.setattr(paper, "unpack_records", lambda: None)
    monkeypatch.setattr(paper, "READS", tmp_path / "reads.json")
    monkeypatch.setattr(sys, "argv", ["regen_paper.py", "--measure"])
    original = Path.is_file

    def missing_library(self):
        if self.name == "libreadcon_core.so":
            return False
        return original(self)

    monkeypatch.setattr(Path, "is_file", missing_library)
    with pytest.raises(SystemExit, match="is missing"):
        paper.main()
    monkeypatch.setattr(Path, "is_file", original)
    seen = []

    def fake_run(step, env, ledger=None):
        seen.append(step)
        if ledger is not None:
            ledger.append({"step": " ".join(step)})

    monkeypatch.setattr(paper, "run", fake_run)
    monkeypatch.setattr(sys, "argv", ["regen_paper.py"])
    paper.main()
    assert (tmp_path / "reads.json").is_file()
    assert seen[-1] == paper.VERSION_MAP


def test_clause_coverage_guards(tmp_path, monkeypatch):
    clause = load("clause_coverage.py")
    monkeypatch.setattr(clause, "rel", lambda path: path.name)
    monkeypatch.setattr(clause, "SPEC", tmp_path / "spec.org")
    (tmp_path / "spec.org").write_text("no clauses\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="no clause ids"):
        clause.spec_clauses()
    rust = tmp_path / "sample.rs"
    rust.write_text(
        "/// names CON-file.1\n#[cfg_attr(all(),\n    test)]\nfn sample() {}\n",
        encoding="utf-8",
    )
    clause.rust_tests(rust)
    clause = load("clause_coverage.py")
    original_reasons = dict(clause.REASONS)
    cid = next(iter(clause.spec_clauses()))[0]
    reasons = dict(original_reasons)
    reasons[cid] = "named and also given a reason"
    monkeypatch.setattr(clause, "REASONS", reasons)
    with pytest.raises(SystemExit, match="a reason but"):
        clause.build()
    out = tmp_path / "clauses.json"
    monkeypatch.setattr(clause, "OUT", out)
    monkeypatch.setattr(clause, "MACROS", tmp_path / "clauses.tex")
    monkeypatch.setattr(clause, "REASONS", original_reasons)
    monkeypatch.setattr(sys, "argv", ["clause_coverage.py", "--check", "--out", str(out)])
    code = clause.main()
    assert code in (0, 1)
    if code == 1:
        clause.main = clause.main
        monkeypatch.setattr(sys, "argv", ["clause_coverage.py", "--out", str(out)])
        assert clause.main() == 0
        monkeypatch.setattr(
            sys, "argv", ["clause_coverage.py", "--check", "--out", str(out)]
        )
        assert clause.main() == 0


def test_r0174_check_stops_on_each_disagreement():
    tables = load("r0174_tables.py")
    assert tables.sig(0) == "0"
    assert tables.ratio(0) == "0"
    prov = tables.load("provenance.json")
    reads = tables.load("reads.json")
    memray = tables.load("memray.json")
    grind = tables.load("cachegrind/cachegrind_results.json")
    cpp = tables.load("cpp/build.json")
    checks = tables.load("cpp/checks.json")
    broken = json.loads(json.dumps(prov))
    broken["build"]["readcon_version"] = "0"
    with pytest.raises(SystemExit, match="is not release"):
        tables.check(broken, reads, memray, grind, cpp, checks)
    other = json.loads(json.dumps(reads))
    other["provenance"]["build"]["library_digest"] = "0" * 64
    with pytest.raises(SystemExit, match="differ on"):
        tables.check(prov, other, memray, grind, cpp, checks)
    host = json.loads(json.dumps(reads))
    host["provenance"]["host"]["kernel"] = "other"
    with pytest.raises(SystemExit, match="another host"):
        tables.check(prov, host, memray, grind, cpp, checks)
    short = json.loads(json.dumps(reads))
    short["protocol"]["rounds"] = 0
    with pytest.raises(SystemExit, match="fewer than"):
        tables.check(prov, short, memray, grind, cpp, checks)
    hardware = json.loads(json.dumps(reads))
    hardware["hardware"] = "other host"
    with pytest.raises(SystemExit, match="hardware line"):
        tables.check(prov, hardware, memray, grind, cpp, checks)
    held = json.loads(json.dumps(prov))
    held_reads = json.loads(json.dumps(reads))
    held_mem = json.loads(json.dumps(memray))
    held_grind = json.loads(json.dumps(grind))
    sha = str(held_grind.get("git_sha") or "abc")
    held.setdefault("build", {})["commit"] = sha
    held_reads.setdefault("provenance", {}).setdefault("build", {})["commit"] = sha
    held_mem.setdefault("provenance", {}).setdefault("build", {})["commit"] = sha
    held_grind.setdefault("provenance", {})["source_commit"] = sha
    held_grind["git_sha"] = sha
    stale = json.loads(json.dumps(held_grind))
    stale["provenance"]["newer_sources"] = ["src/lib.rs"]
    with pytest.raises(SystemExit, match="older than"):
        tables.check(held, held_reads, held_mem, stale, cpp, checks)
    digest = json.loads(json.dumps(cpp))
    digest["library_digest"] = "f" * 64
    with pytest.raises(SystemExit, match="other library"):
        tables.check(held, held_reads, held_mem, held_grind, digest, checks)
    cells = json.loads(json.dumps(held_reads))
    cells["cells"] = {}
    with pytest.raises(SystemExit, match="holds the cells"):
        tables.check(held, cells, held_mem, held_grind, cpp, checks)


def test_worked_example_stops_when_the_release_or_the_record_disagrees(tmp_path, monkeypatch):
    worked = load("worked_example.py")

    def failed(*args, **kwargs):
        return subprocess.CompletedProcess(args, 1, stdout="", stderr="missing")

    monkeypatch.setattr(worked.subprocess, "run", failed)
    with pytest.raises(SystemExit, match="does not hold"):
        worked.release_source("a" * 40, "src/types.rs")

    def source(*args, **kwargs):
        return subprocess.CompletedProcess(args, 0, stdout="fn main() {}\n", stderr="")

    monkeypatch.setattr(worked.subprocess, "run", source)
    with pytest.raises(SystemExit, match="no cell radicand"):
        worked.release_window("b" * 40)
    with pytest.raises(SystemExit, match="does not declare"):
        worked.release_thresholds("b" * 40)
    monkeypatch.setattr(worked, "radicand", lambda angles: 0.0)
    with pytest.raises(SystemExit, match="-2\\^-52"):
        worked.compute(1e-12)
    record = json.loads(worked.RECORD.read_text())
    record["ase"]["reads"]["right"]["error"] = "unexpected"
    with pytest.raises(SystemExit, match="reads the headers"):
        worked.render(record)
    frame = tmp_path / "frame.con"
    frame.write_bytes(b"not the survey frame")
    monkeypatch.setattr(worked, "FRAME", frame)
    monkeypatch.setattr(sys, "argv", ["worked_example.py", "--tables"])
    with pytest.raises(SystemExit, match="differs from the SHA-256"):
        worked.main()


def test_version_map_notes_and_a_missing_build(tmp_path, monkeypatch):
    version_map = load("gen_version_map.py")
    build = {
        "library_and_bindings": ["src/lib.rs", "src/ffi.rs"],
        "diff_stat_source_tree_to_release_head_library_and_bindings": [],
        "source_tree": "a" * 40,
        "release_head": "b" * 40,
        "library_digest_at_release_head": "c" * 64,
        "diff_stat_source_tree_to_release_head": ["src/lib.rs | 1", "src/ffi.rs | 2"],
        "diff_stat_commit_to_release_head_library_and_bindings": [],
        "commit": "d" * 40,
        "library_digest_at_commit": "e" * 64,
    }
    assert "are those" in version_map.release_note(build)
    build["diff_stat_source_tree_to_release_head_library_and_bindings"] = ["src/lib.rs | 1"]
    build["diff_stat_commit_to_release_head_library_and_bindings"] = ["Cargo.toml | 2"]
    build["commit_to_release_head_changes_only_version_lines"] = True
    assert "differ from those" in version_map.release_note(build)
    assert version_map.from_provenance({"build": [], "host": {}}, {}) is None
    noted = version_map.from_provenance(
        {
            "build": {"source_commit": "a" * 40, "readcon_version": "0.0.0"},
            "host": {"family": "6", "model": "1", "mhz": "2000", "nproc": 4},
        },
        {},
    )
    assert noted is not None and noted.extra
    root = tmp_path / "cpc"
    data = root / "data"
    data.mkdir(parents=True)
    (data / "campaign_host.json").write_text("{}\n", encoding="utf-8")
    (data / "row.json").write_text("{}\n", encoding="utf-8")
    monkeypatch.setattr(version_map, "CPC", root)
    monkeypatch.setattr(version_map, "provenance_records", lambda: set())
    monkeypatch.setattr(version_map, "outputs", lambda: ({}, {}))
    assert version_map.carrier("data/row.json")[0] == "campaign"
    (data / "campaign_host.json").unlink()
    (data / "host.json").write_text("{}\n", encoding="utf-8")
    assert version_map.carrier("data/row.json")[0] == "host"
    map_out = ROOT / "paper" / "cpc" / "data" / "branch-map.tex"
    monkeypatch.setattr(version_map, "OUT", map_out)

    def places():
        return {"data/a.json": [("rg_main.tex", 1, "")]}, []

    monkeypatch.setattr(version_map, "record_places", places)
    monkeypatch.setattr(version_map, "carrier", lambda rel: None)
    monkeypatch.setattr(version_map, "needs_build", lambda rel: True)
    text, problems = version_map.render()
    assert problems and "names no build" in problems[0]
    held = version_map.Build(
        "0.0.0",
        "ab",
        None,
        None,
        None,
        None,
        "host",
        "",
        "note",
    )
    monkeypatch.setattr(version_map, "carrier", lambda rel: ("record", rel))
    monkeypatch.setattr(version_map, "build_of", lambda *args: held)
    monkeypatch.setattr(version_map, "shown", lambda rels: {rel: rel for rel in rels})
    text, problems = version_map.render()
    assert "note" in text
    monkeypatch.setattr(version_map, "render", lambda: ("table\n", ["problem"]))
    try:
        assert version_map.main() == 1
    finally:
        map_out.unlink(missing_ok=True)


def test_trace_and_union_and_manuscript_guards(tmp_path, monkeypatch):
    trace = load("trace_numbers.py")
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--only", "rg_main.tex:1-5", "--no-write"])
    assert trace.main() in (0, 1)
    ledger = ROOT / "paper" / "cpc" / "data" / "branch-ledger.tsv"
    monkeypatch.setattr(trace, "LEDGER", ledger)
    ledger.write_text("stale\n", encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--check"])
    try:
        assert trace.main() == 1
    finally:
        ledger.unlink(missing_ok=True)

    union = load("plot_union_extra.py")
    monkeypatch.setattr(union, "READER_LOG", tmp_path / "missing.log")
    with pytest.raises(SystemExit, match="missing"):
        union._reader_samples()
    (tmp_path / "missing.log").write_text(
        "sample arm=readcon_v0173 seconds=\n"
        "sample arm=readcon_v0173 seconds=1\n"
        "sample arm=readcon_tip seconds=1\n",
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="at least six"):
        union._reader_samples()
    monkeypatch.setattr(
        union,
        "_read",
        lambda path: [
            {"format": "a", "writer": "w", "reader": "r", "dpos": "nope"},
            {"format": "b", "writer": "w", "reader": "r", "dpos": ""},
        ],
    )
    monkeypatch.setattr(union, "OUT", tmp_path)

    def theme():
        import matplotlib.pyplot as plt

        fig, ax = plt.subplots()
        plt.close(fig)
        return {}

    # matrix needs a real theme from house style; call the colour loop only.
    union.matrix(union._theme())

    manuscript = load("check_manuscript.py")
    monkeypatch.setattr(manuscript, "ENV_BIN", tmp_path)
    with pytest.raises(manuscript.Failed, match="is not in the environment"):
        manuscript.tool("no-such-tool")
    digest = manuscript.sha256(ROOT / "Cargo.toml")
    assert len(digest) == 64
    with pytest.raises(manuscript.Failed, match="not found"):
        manuscript.check_front("no abstract here")
    generated = tmp_path / "generated"
    generated.mkdir()
    (generated / "orphan.tex").write_text("orphan\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "GENERATED", generated)
    with pytest.raises(manuscript.Failed, match="generated files"):
        manuscript.check_generated("", "")

    def failed(command, **kwargs):
        return subprocess.CompletedProcess(command, 2, stdout="out", stderr="err")

    monkeypatch.setattr(manuscript.subprocess, "run", failed)
    with pytest.raises(manuscript.Failed, match="exited 2"):
        manuscript.check_rebuild()

    def ok(command, **kwargs):
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr(manuscript.subprocess, "run", ok)
    states = iter([{"a.pdf": "1"}, {"a.pdf": "2"}])
    monkeypatch.setattr(manuscript, "built_pdfs", lambda: next(states))
    with pytest.raises(manuscript.Failed, match="different PDFs"):
        manuscript.check_rebuild()
    monkeypatch.setattr(manuscript, "built_pdfs", lambda: {"a.pdf": "1"})
    assert "byte-identical" in manuscript.check_rebuild()


def test_the_last_open_branches(tmp_path, monkeypatch):
    trace = load("trace_numbers.py")
    lines = [
        "\\begin{document}",
        "intro",
        "\\section{Conclusions}",
        "The run ends.",
        "\\section{References}",
    ]
    picked = trace.headline_lines(lines, 0)
    assert lines.index("\\section{Conclusions}") in picked
    assert trace.macro_number("not a number") is None

    import importlib

    real_import = importlib.import_module

    def missing(name, *args, **kwargs):
        if name == "number_rules.missing":
            raise ModuleNotFoundError(name="number_rules.missing")
        if name == "number_rules.broken":
            raise ModuleNotFoundError(name="other")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(trace.importlib, "import_module", missing)
    monkeypatch.setattr(trace, "RULE_MODULES", ["missing"])
    assert trace.load_rules() == []
    monkeypatch.setattr(trace, "RULE_MODULES", ["broken"])
    with pytest.raises(ModuleNotFoundError):
        trace.load_rules()

    paper = load("regen_paper.py")
    monkeypatch.setattr(paper, "CPC", tmp_path)
    monkeypatch.setattr(paper, "unpack_records", lambda: None)
    monkeypatch.setattr(paper, "READS", tmp_path / "reads.json")
    monkeypatch.setattr(paper, "MEASURE", [["ok.py"]])
    (tmp_path / "ok.py").write_text("print(1)\n", encoding="utf-8")
    monkeypatch.setattr(paper, "run", lambda step, env, ledger=None: None)
    monkeypatch.setattr(sys, "argv", ["regen_paper.py", "--measure"])
    original = Path.is_file

    def library_present(self):
        if self.name == "libreadcon_core.so":
            return True
        return original(self)

    monkeypatch.setattr(Path, "is_file", library_present)
    paper.main()

    worked = load("worked_example.py")

    def mmap_only(*args, **kwargs):
        return subprocess.CompletedProcess(
            args, 0, stdout="const MMAP_THRESHOLD: u64 = 8 * 1024;\n", stderr=""
        )

    monkeypatch.setattr(worked.subprocess, "run", mmap_only)
    with pytest.raises(SystemExit, match="does not declare"):
        worked.release_thresholds("c" * 40)
    record = json.loads(worked.RECORD.read_text())
    record["atoms"] = -1
    stored = tmp_path / "worked.json"
    stored.write_text(json.dumps(record), encoding="utf-8")
    monkeypatch.setattr(worked, "RECORD", stored)
    monkeypatch.setattr(worked, "OUT", tmp_path / "worked.tex")
    monkeypatch.setattr(sys, "argv", ["worked_example.py", "--tables"])
    with pytest.raises(SystemExit, match="differs from the frame"):
        worked.main()

    tables = load("r0174_tables.py")
    reads = tables.load("reads.json")
    prov = tables.load("provenance.json")
    memray = tables.load("memray.json")
    grind = tables.load("cachegrind/cachegrind_results.json")
    cpp = tables.load("cpp/build.json")
    checks = tables.load("cpp/checks.json")
    assert tables.ratio(0.996)
    shifted = json.loads(json.dumps(reads))
    key = tables.PROTOCOL_KEYS[0]
    shifted["protocol"][key] = "other"
    with pytest.raises(SystemExit, match="ran"):
        tables.same_protocol("sample", shifted, reads)
    hardware = json.loads(json.dumps(reads))
    hardware["hardware"] = "other host"
    with pytest.raises(SystemExit, match="hardware"):
        tables.same_protocol("sample", hardware, reads)
    named = json.loads(json.dumps(prov))
    named_reads = json.loads(json.dumps(reads))
    named_mem = json.loads(json.dumps(memray))
    named_grind = json.loads(json.dumps(grind))
    named["build"]["commit"] = "1234abcd"
    named_reads["provenance"]["build"]["commit"] = "1234abcd"
    named_mem["provenance"]["build"]["commit"] = "1234abcd"
    named_grind["git_sha"] = "ffff"
    named_grind["provenance"]["source_commit"] = "ffff"
    with pytest.raises(SystemExit, match="Cachegrind harness ran"):
        tables.check(named, named_reads, named_mem, named_grind, cpp, checks)
    artifacts = json.loads(json.dumps(reads))
    artifacts["cpp_artifacts"] = {}
    aligned = json.loads(json.dumps(prov))
    aligned_reads = json.loads(json.dumps(reads))
    aligned_mem = json.loads(json.dumps(memray))
    aligned_grind = json.loads(json.dumps(grind))
    sha = str(aligned_grind.get("git_sha") or "abc")
    aligned.setdefault("build", {})["commit"] = sha
    aligned_reads.setdefault("provenance", {}).setdefault("build", {})["commit"] = sha
    aligned_mem.setdefault("provenance", {}).setdefault("build", {})["commit"] = sha
    aligned_grind.setdefault("provenance", {})["source_commit"] = sha
    aligned_grind["git_sha"] = sha
    artifacts = json.loads(json.dumps(aligned_reads))
    artifacts["cpp_artifacts"] = {}
    with pytest.raises(SystemExit, match="other C\\+\\+"):
        tables.check(aligned, artifacts, aligned_mem, aligned_grind, cpp, checks)
    bad_checks = json.loads(json.dumps(checks))
    bad_checks[0]["disagreements"] = ["checksum"]
    with pytest.raises(SystemExit, match="disagree"):
        tables.check(aligned, aligned_reads, aligned_mem, aligned_grind, cpp, bad_checks)
    other_mem = json.loads(json.dumps(aligned_mem))
    other_mem["cells"] = []
    with pytest.raises(SystemExit, match="different Python cells"):
        tables.check(aligned, aligned_reads, other_mem, aligned_grind, cpp, checks)
    mismatched = json.loads(json.dumps(aligned_reads))
    next(iter(mismatched["cells"].values()))["mismatches"] = ["one"]
    with pytest.raises(SystemExit, match="disagrees with payloads"):
        tables.check(aligned, mismatched, aligned_mem, aligned_grind, cpp, checks)
    mem_bad = json.loads(json.dumps(aligned_mem))
    mem_bad["cells"][0]["mismatches"] = ["one"]
    with pytest.raises(SystemExit, match="memray cell"):
        tables.check(aligned, aligned_reads, mem_bad, aligned_grind, cpp, checks)
    extra = json.loads(json.dumps(aligned_grind))
    extra["scenarios"]["not_a_label"] = 1
    with pytest.raises(SystemExit, match="without a label"):
        tables.check(aligned, aligned_reads, aligned_mem, extra, cpp, checks)

    union = load("plot_union_extra.py")
    monkeypatch.setattr(union, "OUT", tmp_path)
    monkeypatch.setattr(
        union,
        "_read",
        lambda path: [
            {"format": "a", "writer": "w", "reader": "r", "dpos": "1.5"},
            {"format": "b", "writer": "w", "reader": "r", "dpos": "nope"},
        ],
    )
    union.matrix(union._theme())
    union.reader_figures()

    sys.path.insert(0, str(SCRIPTS))
    import plot_figures

    monkeypatch.setattr(plot_figures, "OUT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["plot_supp.py"])
    runpy.run_path(str(SCRIPTS / "plot_supp.py"), run_name="__main__")


def test_manuscript_front_matter_and_record_guards(tmp_path, monkeypatch):
    manuscript = load("check_manuscript.py")
    text = manuscript.MAIN.read_text()
    broken = text.replace("\\begin{abstract}", "\\begin{abstract} " + ("word " * 300), 1)
    broken = broken.replace("\\begin{highlights}", "\\begin{highlights}\n\\item only one\n", 1)
    broken = broken.replace("Program Title", "Program Name", 1)
    broken = broken.replace("(to be added by", "(added by", 1)
    broken = broken.replace("This research did not receive", "Funding was", 1)
    broken = broken.replace(manuscript.COMPETING, "Competing.", 1)
    broken = broken.replace(manuscript.AI_STATEMENT, "gone", 1)
    with pytest.raises(manuscript.Failed, match="front matter"):
        manuscript.check_front(broken)
    inside = text.replace(
        "\\begin{abstract}",
        "\\begin{abstract}\n\\noindent\\textbf{PROGRAM SUMMARY}\n",
        1,
    )
    with pytest.raises(manuscript.Failed):
        manuscript.check_front(inside)

    generated = ROOT / "paper" / "cpc" / "branch-leak"
    generated.mkdir()
    (generated / "leak.tex").write_text("see /tmp/secret\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "GENERATED", generated)
    try:
        with pytest.raises(manuscript.Failed, match="absolute path"):
            manuscript.check_leaks()
    finally:
        (generated / "leak.tex").unlink(missing_ok=True)
        generated.rmdir()

    root = tmp_path / "root"
    walls = root / "data" / "restart_walls"
    failure = root / "data" / "failure"
    walls.mkdir(parents=True)
    failure.mkdir()
    (walls / "restart_walls.json").write_text('{"wall_repeats": 2}\n', encoding="utf-8")
    (failure / "restart_formats.csv").write_text(
        "route,error,force_calls\na,Error: boom,1\nb,,2\n", encoding="utf-8"
    )
    (walls / "restart_walls.csv").write_text(
        "route,n_repeats,error,force_calls\n"
        "a,1,Error: boom,1\n"
        "b,2,,9\n"
        "c,2,,1\n",
        encoding="utf-8",
    )
    (walls / "restart_walls_repeats.csv").write_text("route\nx\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "ROOT", root)
    with pytest.raises(manuscript.Failed, match="restart walls"):
        manuscript.check_walls()
    scripts = tmp_path / "scripts"
    scripts.mkdir()
    (scripts / "restart_formats.py").write_text("x = 1\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "SCRIPTS", scripts)
    with pytest.raises(manuscript.Failed, match="FAILS_AS"):
        manuscript.fails_as()

    monkeypatch.setattr(
        manuscript,
        "check_front",
        lambda text: (_ for _ in ()).throw(RuntimeError("boom")),
    )
    monkeypatch.setattr(sys, "argv", ["check_manuscript.py"])
    assert manuscript.main() == 1


def test_spec_measure_and_the_formula_parser(tmp_path, monkeypatch):
    spec = load("gen_spec_supp.py")
    with pytest.raises(SystemExit, match="cannot read"):
        spec.tokens("???")
    with pytest.raises(SystemExit, match="expected"):
        spec.Expr("a").take(")")
    with pytest.raises(SystemExit, match="trailing"):
        spec.Expr("(1) 2").vector()
    with pytest.raises(SystemExit, match="unexpected"):
        spec.Expr("+").factor()
    with pytest.raises(SystemExit, match="no LaTeX"):
        spec.latex(("other",))
    versions = ROOT / "paper" / "cpc" / "data" / "spec" / "versions-branch.json"
    monkeypatch.setattr(spec, "VERSIONS", versions)
    try:
        spec.measure()
    finally:
        versions.unlink(missing_ok=True)
    try:
        spec.compile_test(spec.OUT)
    except SystemExit:
        pass


def test_spec_and_manuscript_edges(tmp_path, monkeypatch):
    original_run = subprocess.run
    spec = load("gen_spec_supp.py")
    mapped, dropped = spec.citation_map()
    assert isinstance(mapped, dict)
    monkeypatch.setattr(
        spec,
        "bib_dois",
        lambda path: {"only": "10.0000/missing"}
        if path == spec.SPEC_BIB
        else {"paper": "10.1000/present"},
    )
    assert spec.citation_map()[1]
    with pytest.raises(SystemExit, match="no width"):
        spec.width(("other",))
    assert spec.value(("+", ("atom", "1"), ("atom", "2")), {}) == 3
    with pytest.raises(SystemExit, match="cannot evaluate"):
        spec.value(("nope",), {})
    with pytest.raises(SystemExit, match="has no clause"):
        spec.cell_equation("no clause\n")
    with pytest.raises(SystemExit, match="no a, b and c"):
        spec.cell_equation(f"- {spec.CELL_CLAUSE} :: text without a formula\n")
    versions = ROOT / "paper" / "cpc" / "data" / "spec" / "versions-branch.json"
    versions.write_text('{"tags": []}\n', encoding="utf-8")
    monkeypatch.setattr(spec, "VERSIONS", versions)
    try:
        with pytest.raises(SystemExit, match="no tag"):
            spec.release_rows(spec.SPEC.read_text())
        original_git = spec.git
        monkeypatch.setattr(
            spec,
            "git",
            lambda *args, **kwargs: subprocess.CompletedProcess(
                args, 0, stdout="deadbeef\n", stderr=""
            ),
        )
        with pytest.raises(SystemExit, match="no release tag"):
            spec.spec_macros(spec.SPEC.read_text())
    finally:
        versions.unlink(missing_ok=True)
    original_spec = spec.SPEC
    empty = ROOT / "paper" / "cpc" / "data" / "spec" / "empty-spec.org"
    empty.write_text("* Intro\nno rounded sentence\n", encoding="utf-8")
    monkeypatch.setattr(spec, "SPEC", empty)
    try:
        with pytest.raises(SystemExit, match="does not hold"):
            spec.org_source()
    finally:
        empty.unlink(missing_ok=True)
    with pytest.raises(SystemExit, match="pandoc output still has"):
        spec.postprocess(r"\section{Hi}")
    monkeypatch.setattr(
        spec.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(
            args[0], 0, stdout="pandoc 0.1\n", stderr=""
        ),
    )
    with pytest.raises(SystemExit, match="pandoc 0.1"):
        spec.pandoc_version()

    def fake_run(cmd, cwd=None, **kwargs):
        joined = " ".join(str(part) for part in cmd)
        if "kpsewhich" in joined:
            return subprocess.CompletedProcess(cmd, 1, stdout="", stderr="")
        if cwd is not None and "pdflatex" in joined:
            generated = spec.PAPER_SRC / "figures" / "generated"
            (Path(cwd) / "test.log").write_text(
                f"({generated}/foo.tex\n"
                "! Undefined control sequence.\n"
                "Overfull \\hbox (10.5pt too wide) in paragraph at lines 3--4\n",
                encoding="utf-8",
            )
            return subprocess.CompletedProcess(cmd, 1, stdout="", stderr="")
        return original_run(cmd, cwd=cwd, **kwargs)

    monkeypatch.setattr(spec.subprocess, "run", fake_run)
    with pytest.raises(SystemExit, match="compile test failed"):
        spec.compile_test(spec.OUT)
    monkeypatch.setattr(spec, "git", original_git)
    monkeypatch.setattr(spec, "SPEC", original_spec)
    monkeypatch.setattr(spec, "VERSIONS", versions)
    versions.write_text('{"tags": []}\n', encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["gen_spec_supp.py", "--measure"])
    try:
        spec.main()
    finally:
        versions.unlink(missing_ok=True)

    manuscript = load("check_manuscript.py")
    text = manuscript.MAIN.read_text()
    highlights = ROOT / "paper" / "cpc" / "branch-highlights.txt"
    highlights.write_text("One highlight here.\nTwo highlights here.\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "HIGHLIGHTS", highlights)
    import re

    short = re.sub(
        r"\\begin\{highlights\}.*?\\end\{highlights\}",
        lambda _match: (
            "\\begin{highlights}\n\\item One highlight here.\n"
            "\\item Two highlights here.\n\\end{highlights}"
        ),
        text,
        count=1,
        flags=re.S,
    )
    try:
        with pytest.raises(manuscript.Failed, match="3 to 5"):
            manuscript.check_front(short)
        moved = text.replace(
            r"\bibliography{rg_main}",
            "\\section{After}\n\n\\bibliography{rg_main}",
            1,
        )
        with pytest.raises(manuscript.Failed, match="not the last section"):
            manuscript.check_front(moved)
    finally:
        highlights.unlink(missing_ok=True)

    docs = ROOT / "paper" / "cpc" / "branch-docs"
    docs.mkdir()
    (docs / "written.md").write_text("no commit here\n", encoding="utf-8")
    (docs / "notes.md").write_text("a note without the macro file\n", encoding="utf-8")
    monkeypatch.setattr(
        manuscript,
        "REPRODUCTION_DOCUMENTS",
        ["branch-docs/written.md", "branch-docs/notes.md"],
    )
    monkeypatch.setattr(manuscript, "WRITTEN_DOCUMENTS", ["branch-docs/written.md"])
    try:
        found = manuscript.printed_commits("a" * 40, "b" * 64)
    finally:
        (docs / "written.md").unlink()
        (docs / "notes.md").unlink()
        docs.rmdir()
    assert any("does not print the commit" in line for line in found)
    assert any("does not name src/commit.tex" in line for line in found)

    root = tmp_path / "openers"
    (root / "src").mkdir(parents=True)
    (root / "src" / "rg_supp.tex").write_text("nothing\n", encoding="utf-8")
    failure = root / "data" / "failure"
    failure.mkdir(parents=True)
    real_openers = manuscript.ROOT / "data" / "failure" / "con_openers.json"
    (failure / "con_openers.json").write_text(real_openers.read_text(), encoding="utf-8")
    monkeypatch.setattr(manuscript, "ROOT", root)
    with pytest.raises(manuscript.Failed, match="neither document"):
        manuscript.check_openers("no toolkit sentence")

    monkeypatch.setattr(manuscript, "ROOT", SCRIPTS.parent)
    generated = ROOT / "paper" / "cpc" / "branch-markers"
    generated.mkdir()
    (generated / "todo.tex").write_text("% TODO later\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "GENERATED", generated)
    try:
        with pytest.raises(manuscript.Failed, match="TODO"):
            manuscript.check_markers()
    finally:
        (generated / "todo.tex").unlink()
        generated.rmdir()

    monkeypatch.setattr(manuscript, "PDFS", (ROOT / "paper" / "cpc" / "missing.pdf",))
    with pytest.raises(manuscript.Failed, match="is missing"):
        manuscript.check_pdfs()

    reads = json.loads(manuscript.READS.read_text())
    reads["steps"] = [{"step": "demo", "ran": ["pixi", "pandoc"]}]
    monkeypatch.setattr(manuscript, "READS", tmp_path / "reads.json")
    manuscript.READS.write_text(json.dumps(reads), encoding="utf-8")
    with pytest.raises(manuscript.Failed, match="ran pixi"):
        manuscript.check_steps()

    import gen_version_map

    monkeypatch.setattr(
        manuscript, "GENERATED", SCRIPTS.parent / "src" / "figures" / "generated"
    )
    current = (manuscript.GENERATED / "version_map.tex").read_text()
    monkeypatch.setattr(gen_version_map, "render", lambda: (current, []))
    assert "version map current" in manuscript.check_version_map()


def test_manuscript_release_and_conformance(tmp_path, monkeypatch):
    manuscript = load("check_manuscript.py")
    text = manuscript.MAIN.read_text().replace("Chatbots were used", "People were used", 1)
    with pytest.raises(manuscript.Failed, match="generative-AI"):
        manuscript.check_front(text)
    assert manuscript.built_pdfs()

    version, commit = manuscript.release()
    digest = manuscript.provenance.library_digest()

    def quiet(*args, **kwargs):
        return subprocess.CompletedProcess(args[0] if args else [], 0, stdout="ok\n", stderr="")

    monkeypatch.setattr(manuscript.subprocess, "run", quiet)
    monkeypatch.setattr(manuscript, "printed_commits", lambda *args: [])
    monkeypatch.setattr(manuscript, "records", lambda: [])
    monkeypatch.setattr(manuscript, "library_at", lambda commit: digest)
    imported = {
        "readcon_version": version,
        "newer_sources": [],
        "readcon_module_sha256": "ab" * 32,
    }
    assert "records name" in manuscript.check_release(imported)
    monkeypatch.setattr(manuscript, "release", lambda: ("0.0.0", commit))
    assert "records name" in manuscript.check_release(imported)
    monkeypatch.setattr(manuscript, "release", lambda: (version, commit))
    monkeypatch.setattr(manuscript, "library_at", lambda commit: None)
    with pytest.raises(manuscript.Failed, match="does not hold"):
        manuscript.check_release({**imported, "readcon_version": "0.0.0"})
    record = ROOT / "paper" / "cpc" / "data" / "branch-record.json"
    record.write_text(
        json.dumps(
            {
                "library_digest": "cd" * 32,
                "readcon_version": "9.9.9",
                "newer_sources": ["src/lib.rs"],
                "readcon_module": "readcon",
                "readcon_module_sha256": "ef" * 32,
            }
        ),
        encoding="utf-8",
    )
    other = ROOT / "paper" / "cpc" / "data" / "branch-other.json"
    other.write_text(
        json.dumps(
            {
                "library_digest": digest,
                "readcon_version": version,
                "readcon_module": "readcon",
                "readcon_module_sha256": "11" * 32,
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(manuscript, "library_at", lambda commit: digest)
    monkeypatch.setattr(manuscript, "records", lambda: [record, other])
    try:
        with pytest.raises(manuscript.Failed, match="module"):
            manuscript.check_release(imported)
    finally:
        record.unlink(missing_ok=True)
        other.unlink(missing_ok=True)

    def copy_outputs(cmd, env=None, **kwargs):
        if env and "BENCH_GENERATED" in env:
            dest = Path(env["BENCH_GENERATED"])
            for name in manuscript.PERFORMANCE_OUTPUTS:
                (dest / name).write_bytes((manuscript.GENERATED / name).read_bytes())
            Path(env["R0174_LEDGER"]).write_bytes(manuscript.PERFORMANCE_LEDGER.read_bytes())
        return subprocess.CompletedProcess(
            cmd, 0, stdout="1 payloads rebuilt from the record\n", stderr=""
        )

    monkeypatch.setattr(manuscript.subprocess, "run", copy_outputs)
    assert "payload checksums" in manuscript.check_performance()

    root = tmp_path / "tasks"
    (root / "src").mkdir(parents=True)
    (root / "src" / "rg_supp.tex").write_text(
        "\\section{Reproduction}\nscripts/missing.py\n", encoding="utf-8"
    )
    (root / "pixi.toml").write_text(
        '[tasks]\nbad = { cmd = "scripts/missing.py" }\n', encoding="utf-8"
    )
    monkeypatch.setattr(manuscript, "ROOT", root)
    monkeypatch.setattr(manuscript, "SUPP", root / "src" / "rg_supp.tex")
    with pytest.raises(manuscript.Failed, match="pixi tasks"):
        manuscript.check_tasks()
    monkeypatch.setattr(manuscript, "ROOT", SCRIPTS.parent)
    monkeypatch.setattr(manuscript, "SUPP", SCRIPTS.parent / "src" / "rg_supp.tex")

    main = tmp_path / "main.tex"
    supp = tmp_path / "supp.tex"
    main.write_text(
        "differing cases are unexplained\na public commit\n", encoding="utf-8"
    )
    supp.write_text("a public commit\n", encoding="utf-8")
    src = tmp_path / "src"
    src.mkdir()
    (src / "measure-commands.lst").write_text("nothing\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "MAIN", main)
    monkeypatch.setattr(manuscript, "SUPP", supp)
    prose = manuscript.mutant_prose({"unexplained": 3, "differ": 1})
    assert any("unexplained" in line for line in prose)
    monkeypatch.setattr(manuscript, "SRC", src)
    cite = manuscript.citation_prose()
    assert any("public" in line for line in cite)
    assert any("lst:measure" in line for line in cite)

    monkeypatch.setattr(manuscript, "MAIN", SCRIPTS.parent / "src" / "rg_main.tex")
    monkeypatch.setattr(manuscript, "SUPP", SCRIPTS.parent / "src" / "rg_supp.tex")
    monkeypatch.setattr(manuscript, "SRC", SCRIPTS.parent / "src")
    monkeypatch.setattr(manuscript, "script_check", lambda *args: "ok")
    assert "corpus31" in manuscript.check_conformance()


def test_manuscript_meson_openers_and_corpus(tmp_path, monkeypatch):
    manuscript = load("check_manuscript.py")
    version, commit = manuscript.release()
    digest = manuscript.provenance.library_digest()
    monkeypatch.setattr(
        manuscript.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(
            [], 0, stdout="ok\n", stderr=""
        ),
    )
    monkeypatch.setattr(manuscript, "printed_commits", lambda *args: [])
    monkeypatch.setattr(manuscript, "records", lambda: [])
    monkeypatch.setattr(manuscript, "library_at", lambda commit: digest)
    original = Path.read_text

    def meson(self, *args, **kwargs):
        if self.name == "meson.build":
            return "project('readcon')\n"
        return original(self, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", meson)
    with pytest.raises(manuscript.Failed, match="meson.build"):
        manuscript.check_release(
            {
                "readcon_version": version,
                "newer_sources": [],
                "readcon_module_sha256": "ab" * 32,
            }
        )
    monkeypatch.setattr(Path, "read_text", original)

    root = tmp_path / "cpc"
    (root / "src").mkdir(parents=True)
    (root / "src" / "rg_supp.tex").write_text(
        (SCRIPTS.parent / "src" / "rg_supp.tex").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    failure = root / "data" / "failure"
    failure.mkdir(parents=True)
    openers = json.loads(
        (SCRIPTS.parent / "data" / "failure" / "con_openers.json").read_text()
    )
    reader = next(iter(openers["readers"]))
    for item in openers["readers"][reader].values():
        item["opens"] = not reader.startswith("ASE")
    (failure / "con_openers.json").write_text(json.dumps(openers), encoding="utf-8")
    monkeypatch.setattr(manuscript, "ROOT", root)
    with pytest.raises(manuscript.Failed, match="con_openers.json disagrees"):
        manuscript.check_openers((SCRIPTS.parent / "src" / "rg_main.tex").read_text())

    for rel in (
        "data/corpus31-results.json",
        "data/corpus_paths.json",
        "data/every_path.json",
        "data/spec/clause_coverage.json",
        "data/spec/mutant_differential.json",
        "data/corpus_crosscheck.json",
        "data/referee_probes.json",
    ):
        dest = root / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes((SCRIPTS.parent / rel).read_bytes())
    corpus = json.loads((root / "data" / "corpus31-results.json").read_text())
    corpus["summary"]["disagreements"] = ["a.con"]
    (root / "data" / "corpus31-results.json").write_text(
        json.dumps(corpus), encoding="utf-8"
    )
    monkeypatch.setattr(manuscript, "script_check", lambda *args: "ok")
    monkeypatch.setattr(manuscript, "citation_prose", lambda: [])
    monkeypatch.setattr(manuscript, "mutant_prose", lambda totals: [])
    with pytest.raises(manuscript.Failed, match="corpus31-results.json"):
        manuscript.check_conformance()
