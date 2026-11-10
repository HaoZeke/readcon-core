"""Run the paper generators on the records in this tree."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    key = "mains_" + name.replace("/", "_").replace(".py", "")
    spec = importlib.util.spec_from_file_location(key, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[key] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_figure_and_table_mains(tmp_path, monkeypatch):
    figures = load("gen_figures.py")
    monkeypatch.setattr(figures, "OUT", tmp_path / "figures")
    figures.main()
    assert (tmp_path / "figures" / "architecture.pdf").is_file()

    union = load("plot_union_extra.py")
    monkeypatch.setattr(union, "OUT", tmp_path / "union")
    union.main()

    tables = load("r0174_tables.py")
    monkeypatch.setattr(tables, "OUT", tmp_path / "r0174")
    monkeypatch.setattr(tables, "LEDGER", tmp_path / "r0174.tsv")
    with pytest.raises(SystemExit, match="Cachegrind harness"):
        tables.main()

    worked = load("worked_example.py")
    record = ROOT / "paper" / "cpc" / "data" / "branch-worked.json"
    rendered = ROOT / "paper" / "cpc" / "src" / "figures" / "generated" / "branch-worked.tex"
    monkeypatch.setattr(worked, "RECORD", record)
    monkeypatch.setattr(worked, "OUT", rendered)
    monkeypatch.setattr(sys, "argv", ["worked_example.py"])
    try:
        worked.main()
        assert rendered.is_file()
    finally:
        record.unlink(missing_ok=True)
        rendered.unlink(missing_ok=True)


def test_spec_as_published(tmp_path, monkeypatch):
    spec = load("gen_spec_supp.py")
    span, tex, trials = spec.cell_equation(spec.SPEC.read_text(encoding="utf-8"))
    assert trials == 200
    assert "equation" in tex
    assert span
    generated = ROOT / "paper" / "cpc" / "src" / "figures" / "generated"
    outs = {
        "OUT": generated / "branch-spec.tex",
        "VERSIONS_OUT": generated / "branch-versions.tex",
        "MACROS_OUT": generated / "branch-macros.tex",
    }
    for name, path in outs.items():
        monkeypatch.setattr(spec, name, path)
    monkeypatch.setattr(sys, "argv", ["gen_spec_supp.py", "--as-is"])
    try:
        spec.main()
        assert outs["OUT"].is_file()
    finally:
        for path in outs.values():
            path.unlink(missing_ok=True)


def test_manuscript_checkers_pass():
    manuscript = load("check_manuscript.py")
    main = manuscript.MAIN.read_text(encoding="utf-8")
    supp = manuscript.SUPP.read_text(encoding="utf-8")
    assert "abstract" in manuscript.check_front(main)
    assert "generated" in manuscript.check_generated(main, supp)
    assert "walls" in manuscript.check_walls()
    assert "openers" in manuscript.check_openers(main)
    assert "format steps" in manuscript.check_steps()
    assert "Reproduction" in manuscript.check_tasks()
    assert manuscript.check_names()
    assert "references" in manuscript.check_references()
    summary = manuscript.script_check("gen_spec_supp.py", "specification")
    assert summary


def test_regen_success_and_a_failed_step(tmp_path, monkeypatch):
    paper = load("regen_paper.py")
    paper.unpack_records()
    monkeypatch.setattr(paper, "CPC", tmp_path)
    (tmp_path / "stop.py").write_text("import sys\nsys.exit(3)\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="exited 3"):
        paper.run(["stop.py"], {})
    (tmp_path / "ok.py").write_text("print(1)\n", encoding="utf-8")
    paper.run(["ok.py"], {})

    regen = load("regen_figures.py")
    evidence = regen.ROOT / "evidence" / "work" / "e1" / "neb"
    made = False
    if not evidence.is_dir():
        evidence.mkdir(parents=True)
        made = True
    monkeypatch.setattr(regen, "LOG", tmp_path / "regen.log")
    for name in (
        "gen_figures",
        "plot_structures",
        "plot_restart_formats",
        "plot_supp",
        "plot_union_extra",
    ):
        monkeypatch.setattr(getattr(regen, name), "main", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "dlpack", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "toc", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "mpi_plots", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "db_plots", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "anatomy_and_errors", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "landscapes", lambda: None)
    monkeypatch.setattr(regen.plot_restored, "timing_and_fairness", lambda: None)
    monkeypatch.setattr(regen.pf, "restart", lambda *args: None)
    try:
        assert regen.main() == 0
    finally:
        if made:
            evidence.rmdir()
    assert "ok restart" in (tmp_path / "regen.log").read_text(encoding="utf-8")


def test_remaining_branches(tmp_path, monkeypatch):
    import runpy

    manuscript = load("check_manuscript.py")
    assert manuscript.library_at(manuscript.release()[1])
    nested = {
        "outer": {
            "library_digest": "ab" * 32,
            "readcon_version": "0.0.1",
            "readcon_module": "readcon",
            "readcon_module_sha256": "cd" * 32,
        }
    }
    found = list(manuscript.named_builds(nested, "", True))
    assert found and found[0].where == "outer"
    assert manuscript.records()
    bare = manuscript.MAIN.read_text(encoding="utf-8").replace(
        "\\begin{highlights}", "\\begin{not-highlights}", 1
    ).replace("\\end{highlights}", "\\end{not-highlights}", 1)
    with pytest.raises(manuscript.Failed):
        manuscript.check_front(bare)
    main = tmp_path / "main.tex"
    supp = tmp_path / "supp.tex"
    main.write_text("no required sentence\n", encoding="utf-8")
    supp.write_text("no required sentence\n", encoding="utf-8")
    monkeypatch.setattr(manuscript, "MAIN", main)
    monkeypatch.setattr(manuscript, "SUPP", supp)
    problems = manuscript.mutant_prose({"unexplained": 0, "differ": -1})
    assert any("count-line" in line for line in problems)
    monkeypatch.setattr(manuscript, "MAIN", manuscript.ROOT / "src" / "rg_main.tex")
    monkeypatch.setattr(manuscript, "SUPP", manuscript.ROOT / "src" / "rg_supp.tex")

    spec = load("gen_spec_supp.py")
    monkeypatch.setattr(sys, "argv", ["gen_spec_supp.py", "--check"])
    spec.main()

    version_map = load("gen_version_map.py")
    original_cpc = version_map.CPC
    original_records = version_map.provenance_records
    original_outputs = version_map.outputs
    root = tmp_path / "cpc"
    (root / "row.json").parent.mkdir(parents=True)
    (root / "row.json").write_text(
        json.dumps({"readcon_version": "0.0.1", "tree_commit": "a" * 40}),
        encoding="utf-8",
    )
    monkeypatch.setattr(version_map, "CPC", root)
    monkeypatch.setattr(version_map, "provenance_records", lambda: set())
    monkeypatch.setattr(version_map, "outputs", lambda: ({}, {}))
    assert version_map.build_of("commit", "row.json") is not None
    host = {"family": "6", "model": "1", "mhz": "2000", "nproc": 2}
    campaign = {
        "one": {"provenance": {"build": {"readcon_version": "0.0.1"}, "host": host}},
        "two": {"provenance": {"build": {"readcon_version": "9.9.9"}, "host": host}},
    }
    (root / "camp.json").write_text(json.dumps(campaign), encoding="utf-8")
    assert version_map.build_of("campaign", "camp.json") is None

    plotted = load("plot_figures.py")
    monkeypatch.setattr(plotted, "OUT", tmp_path / "save")
    (tmp_path / "save").mkdir()
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    plotted._save(fig, "wide", None, width_cm=0.01)
    plotted._landscape_labels(ax, False)
    plt.close(fig)

    monkeypatch.setattr(sys, "argv", ["plot_figures.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "plot_figures.py"), run_name="__main__")
    monkeypatch.setattr(sys, "argv", ["regen_paper.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "regen_paper.py"), run_name="__main__")
    monkeypatch.setattr(sys, "argv", ["gen_spec_supp.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "gen_spec_supp.py"), run_name="__main__")
    map_out = ROOT / "paper" / "cpc" / "src" / "figures" / "generated" / "branch-map.tex"
    monkeypatch.setattr(version_map, "OUT", map_out)
    monkeypatch.setattr(version_map, "CPC", original_cpc)
    monkeypatch.setattr(version_map, "provenance_records", original_records)
    monkeypatch.setattr(version_map, "outputs", original_outputs)
    try:
        version_map.main()
    finally:
        map_out.unlink(missing_ok=True)
    monkeypatch.setattr(sys, "argv", ["check_manuscript.py", "--help"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "check_manuscript.py"), run_name="__main__")

    version, commit = manuscript.release()
    digest = manuscript.provenance.library_digest()
    data = ROOT / "paper" / "cpc" / "data"
    listed = data / "branch-list.json"
    empty = data / "branch-empty.json"
    measured = data / "branch-measured.json"
    listed.write_text("[1]\n", encoding="utf-8")
    empty.write_text("{}\n", encoding="utf-8")
    measured.write_text(
        json.dumps(
            {
                "library_digest": "ab" * 32,
                "readcon_version": version,
                "measures_release": [version],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(manuscript, "records", lambda: [listed, empty, measured])
    monkeypatch.setattr(manuscript, "printed_commits", lambda *args: [])
    imported = dict(manuscript.provenance.readcon_build())
    imported["newer_sources"] = ["src/lib.rs"]
    imported["readcon_version"] = version
    try:
        with pytest.raises(manuscript.Failed):
            manuscript.check_release(imported)
    finally:
        listed.unlink(missing_ok=True)
        empty.unlink(missing_ok=True)
        measured.unlink(missing_ok=True)

    import gen_version_map

    monkeypatch.setattr(gen_version_map, "render", lambda: ("different\n", []))
    with pytest.raises(manuscript.Failed, match="version map"):
        manuscript.check_version_map()

    root = tmp_path / "conform"
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
    paths = json.loads((root / "data" / "corpus_paths.json").read_text())
    paths["results"]["totals"]["differ"] = 1
    paths["results"]["totals"]["files_differ"] = ["a.con"]
    (root / "data" / "corpus_paths.json").write_text(json.dumps(paths), encoding="utf-8")
    mutants = json.loads((root / "data" / "spec" / "mutant_differential.json").read_text())
    mutants["totals"]["unexplained"] = 2
    (root / "data" / "spec" / "mutant_differential.json").write_text(
        json.dumps(mutants), encoding="utf-8"
    )
    cross = json.loads((root / "data" / "corpus_crosscheck.json").read_text())
    reader = next(iter(cross["groups"]["all"]["readers"]))
    cross["groups"]["all"]["readers"][reader]["disagreements"] = 1
    (root / "data" / "corpus_crosscheck.json").write_text(json.dumps(cross), encoding="utf-8")
    probes = json.loads((root / "data" / "referee_probes.json").read_text())
    probes["summary"]["failed_probes"] = ["probe"]
    (root / "data" / "referee_probes.json").write_text(json.dumps(probes), encoding="utf-8")
    monkeypatch.setattr(manuscript, "ROOT", root)
    monkeypatch.setattr(manuscript, "macros", lambda: {})
    monkeypatch.setattr(manuscript, "mutant_prose", lambda totals: [])
    monkeypatch.setattr(manuscript, "citation_prose", lambda: [])

    def failed_check(*args):
        raise manuscript.Failed("rerun disagreed")

    monkeypatch.setattr(manuscript, "script_check", failed_check)
    with pytest.raises(manuscript.Failed, match="corpus_paths.json"):
        manuscript.check_conformance()
    monkeypatch.setattr(manuscript, "ROOT", SCRIPTS.parent)

    pdf = ROOT / "paper" / "cpc" / "src" / "rg_main.tex"
    monkeypatch.setattr(manuscript, "PDFS", (pdf,))
    monkeypatch.setattr(manuscript, "tool", lambda name: "pdftotext")

    def pdftotext(cmd, **kwargs):
        if "-raw" in cmd:
            return (
                "see https://example.com/foo\n"
                "bar more\n"
                "\nReferences\n"
                "see https://doi.org/10.1000/abc\n"
                "https://doi.org/10.1000/def\n"
            )
        return "plain page\n"

    monkeypatch.setattr(manuscript.subprocess, "check_output", pdftotext)
    with pytest.raises(manuscript.Failed, match="identifier split"):
        manuscript.check_pdfs()

    import plot_structures
    import plot_restart_formats
    import plot_restored
    import plot_supp
    import plot_union_extra
    import gen_figures

    monkeypatch.setattr(gen_figures, "main", lambda: None)
    monkeypatch.setattr(plot_structures, "main", lambda: None)
    monkeypatch.setattr(plot_restart_formats, "main", lambda: None)
    monkeypatch.setattr(plot_supp, "main", lambda: None)
    monkeypatch.setattr(plot_union_extra, "main", lambda: None)
    monkeypatch.setattr(plot_restored, "dlpack", lambda: None)
    monkeypatch.setattr(plot_restored, "toc", lambda: None)
    monkeypatch.setattr(plot_restored, "mpi_plots", lambda: None)
    monkeypatch.setattr(plot_restored, "db_plots", lambda: None)
    monkeypatch.setattr(plot_restored, "anatomy_and_errors", lambda: None)
    monkeypatch.setattr(plot_restored, "landscapes", lambda: None)
    monkeypatch.setattr(plot_restored, "timing_and_fairness", lambda: None)
    import plot_figures as pf

    monkeypatch.setattr(pf, "restart", lambda *args: None)
    log = SCRIPTS.parent / "logs" / "regen-figures.log"
    saved = log.read_bytes() if log.is_file() else None
    monkeypatch.setattr(sys, "argv", ["regen_figures.py"])
    try:
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "regen_figures.py"), run_name="__main__")
        assert caught.value.code == 0
    finally:
        if saved is None:
            log.unlink(missing_ok=True)
        else:
            log.write_bytes(saved)


def test_exact_cover_of_the_short_scripts(tmp_path, monkeypatch):
    """The six scripts that sit just under 98 percent, counted with branches."""
    import runpy

    dlpack = load("bench_dlpack.py")
    monkeypatch.setattr(dlpack, "from_log", lambda: 0)
    monkeypatch.setattr(sys, "argv", ["bench_dlpack.py", "--from-log"])
    assert dlpack.main() == 0

    provenance = load("provenance.py")

    class Readelf:
        stdout = "string dump\n  rustc version 1.88.0 (6b00bc388)\n"

    monkeypatch.setattr(provenance.subprocess, "run", lambda *args, **kwargs: Readelf())
    assert provenance._rustc_of(ROOT / "Cargo.toml") == "1.88.0 (6b00bc388)"

    version_map = load("gen_version_map.py")
    monkeypatch.setattr(version_map, "GENERATED", tmp_path)
    (tmp_path / "branch-map.tex").write_text(
        "% from no-such-record\n\\newcommand{\\branchMacro}{1}\n",
        encoding="utf-8",
    )
    problems: list[str] = []
    version_map.macro_sources("branch-map.tex", set(), problems)
    assert any("not a record" in item for item in problems)

    traced = load("trace_numbers.py")
    ledger = tmp_path / "numbers.csv"
    monkeypatch.setattr(traced, "LEDGER", ledger)
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--no-write"])
    assert traced.main() in (0, 1)
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py"])
    traced.main()
    assert ledger.is_file()

    pdf = SCRIPTS.parent / "src" / "figures" / "generated" / "architecture.pdf"
    saved = pdf.read_bytes() if pdf.is_file() else None
    monkeypatch.setattr(sys, "argv", ["gen_figures.py"])
    try:
        runpy.run_path(str(SCRIPTS / "gen_figures.py"), run_name="__main__")
    finally:
        if saved is None:
            pdf.unlink(missing_ok=True)
        else:
            pdf.write_bytes(saved)

    monkeypatch.setattr(sys, "argv", ["worked_example.py", "--help"])
    with pytest.raises(SystemExit) as caught:
        runpy.run_path(str(SCRIPTS / "worked_example.py"), run_name="__main__")
    assert caught.value.code == 0


def test_plot_mains_write_into_a_temporary_directory(tmp_path, monkeypatch):
    import runpy

    restored = load("plot_restored.py")
    restored.OUT = tmp_path
    restored.main()
    restored.architecture()
    (tmp_path / "survey_frame.con").write_text(
        "1\nframe\nCu 0 0 0\nCu 1 0 0\n", encoding="utf-8"
    )
    restored.structures()
    assert (tmp_path / "timing.pdf").is_file()
    assert (tmp_path / "architecture.pdf").is_file()

    real_loads = restored.json.loads

    def loads(text, **kwargs):
        data = real_loads(text, **kwargs)
        if isinstance(data, dict) and "cells" in data:
            return {"cells": {}}
        if isinstance(data, dict) and any(
            isinstance(block, dict) and "mic_max_abs_A" in block
            for block in data.values()
        ):
            return {"held": {"other": []}}
        return data

    restored.json.loads = loads
    try:
        with pytest.raises(KeyError):
            restored.timing_and_fairness()
        restored.anatomy_and_errors()
    finally:
        restored.json.loads = real_loads

    structures = load("plot_structures.py")
    structures.OUT = tmp_path / "structures"
    structures.WORK = tmp_path / "work"
    structures.main()
    assert (tmp_path / "structures" / "structures.pdf").is_file()
    with pytest.raises(SystemExit):
        structures._isolate("<not-svg>", "p")

    formats = load("plot_restart_formats.py")
    saved_pdf = formats.pf.OUT / "restart_formats.pdf"
    before = saved_pdf.read_bytes() if saved_pdf.is_file() else None
    generated = SCRIPTS.parent / "src" / "figures" / "generated"
    snapshots = {
        path: path.read_bytes()
        for path in generated.glob("*.pdf")
    }
    monkeypatch.setattr(sys, "argv", ["plot_restored.py"])
    try:
        formats.main()
        runpy.run_path(str(SCRIPTS / "plot_restart_formats.py"), run_name="__main__")
        runpy.run_path(str(SCRIPTS / "plot_structures.py"), run_name="__main__")
        runpy.run_path(str(SCRIPTS / "plot_restored.py"), run_name="__main__")
    finally:
        for path, blob in snapshots.items():
            path.write_bytes(blob)
        if before is not None:
            saved_pdf.write_bytes(before)
