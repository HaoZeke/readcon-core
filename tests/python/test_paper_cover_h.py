"""Outputs for the paper scripts that were still under 98 percent."""

from __future__ import annotations

import copy
import importlib.util
import json
import runpy
import subprocess
import sys
import tarfile
from decimal import Decimal
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"
CPC = ROOT / "paper" / "cpc"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverh_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def test_case_macros_skip_a_capped_route_and_reject_a_shifted_band(tmp_path, monkeypatch):
    macros = load("gen_case_macros.py")
    record = json.loads((CPC / "data/failure/restart_formats.json").read_text())
    cap = str(record["parameters"]["step_cap"])
    real_rows = macros.restart_rows
    import gen_cloud_tables

    baseline = copy.deepcopy(real_rows("data/failure/restart_formats.csv"))
    original_runs = gen_cloud_tables.kernel_runs
    monkeypatch.setattr(
        gen_cloud_tables, "kernel_runs", lambda routes: original_runs(list(baseline.values()))
    )
    out = CPC / "coverh-case.tex"
    monkeypatch.setattr(macros, "OUT", out)

    def capped(rel):
        found = real_rows(rel)
        row = found["extended XYZ (ASE)"]
        row["converged"] = "False"
        row["steps"] = cap
        return found

    monkeypatch.setattr(macros, "restart_rows", capped)
    macros.main()
    try:
        text = out.read_text(encoding="utf-8")
    finally:
        out.unlink(missing_ok=True)
    assert "rfModesXyz" in text
    assert "rfStepsXyz" not in text

    def shifted(rel):
        found = real_rows(rel)
        found["CON 3.1, shortest round trip"]["d_barrier_eV"] = "1"
        return found

    monkeypatch.setattr(macros, "restart_rows", shifted)
    with pytest.raises(SystemExit, match="bit for bit"):
        macros.main()


def test_openers_entry_rewrites_the_record():
    path = CPC / "data" / "failure" / "con_openers.json"
    saved = path.read_bytes()
    try:
        monkey_argv = ["con_openers.py"]
        sys.argv = monkey_argv
        runpy.run_path(str(SCRIPTS / "con_openers.py"), run_name="__main__")
        written = json.loads(path.read_text(encoding="utf-8"))
        assert written["readers"]
    finally:
        path.write_bytes(saved)
        sys.argv = ["pytest"]


def test_failure_table_marks_a_reader_error_and_a_writer_error(tmp_path):
    tables = load("gen_failure_tables.py")

    def rows(name):
        assert name == "matrix.csv"
        return [
            {
                "format": "CON",
                "writer": "readcon",
                "reader": "ase",
                "R1": "reader error",
                "R2": "kept",
                "R3": "kept",
                "R4": "bit-exact",
                "R5": "none",
                "pbc": "kept",
                "error": "RuntimeError: boom",
                "error_at": "demo.py:3",
                "dpos_ulp": "",
                "dpos_coord": "",
            },
            {
                "format": "XYZ",
                "writer": "ase",
                "reader": "readcon",
                "R1": "writer error",
                "R2": "kept",
                "R3": "kept",
                "R4": "bit-exact",
                "R5": "none",
                "pbc": "kept",
                "error": "stopped",
                "error_at": "",
                "dpos_ulp": "",
                "dpos_coord": "",
            },
        ]

    tables.rd = rows
    tables.OUT = tmp_path
    tables.survey()
    text = (tmp_path / "survey_table.tex").read_text(encoding="utf-8")
    assert r"\textsuperscript{a}" in text
    assert r"\textsuperscript{b}" in text
    assert "RuntimeError: boom" in text
    assert "demo.py:3" in text
    assert "stopped" in text


def test_version_rows_resolve_a_relative_library_and_a_refused_rewrite(tmp_path, monkeypatch):
    safety = load("version_safety.py")
    monkeypatch.setenv("READCON_LIB", "libreadcon_core.so")
    resolved = safety.lib_path()
    assert resolved.is_absolute()
    assert resolved.name == "libreadcon_core.so"

    import readcon
    from error_class import classify

    class Frame:
        pass

    monkeypatch.setattr(readcon, "read_con", lambda path: [Frame()])

    def refuse(frames):
        raise ValueError("no rewrite")

    monkeypatch.setattr(readcon, "write_con_string", refuse)
    row = safety.python_row(readcon, classify, tmp_path / "frame.con", {})
    assert row["python"] == "accept"
    assert row["python_rewrite"].startswith("refused:")


def test_velocity_rejects_a_disagreed_unit_and_a_kinetic_mismatch(monkeypatch):
    velocity = load("velocity_case.py")
    import hcn_case

    monkeypatch.setattr(hcn_case, "binary", lambda: Path("reader"))
    monkeypatch.setattr(
        velocity.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(
            args,
            0,
            stdout=(
                '{"unit_conversion_factor_fs": 1, "spelling": "s", '
                '"canonical": "c", "unit_conversion_factor_fs_squared": 1}\n'
            ),
        ),
    )
    with pytest.raises(SystemExit, match="natural unit disagrees"):
        velocity.natural_fs()

    import ase

    monkeypatch.setattr(ase.Atoms, "get_kinetic_energy", lambda self: -1.0)
    with pytest.raises(SystemExit, match="kinetic energy"):
        velocity.slab_10001(10.0, 8.6e-5)


def test_velocity_entry_rewrites_the_table():
    tex = CPC / "src" / "figures" / "generated" / "velocity_case.tex"
    saved = tex.read_bytes()
    try:
        sys.argv = ["velocity_case.py", "--tables"]
        runpy.run_path(str(SCRIPTS / "velocity_case.py"), run_name="__main__")
        assert "velNaturalFs" in tex.read_text(encoding="utf-8")
    finally:
        tex.write_bytes(saved)
        sys.argv = ["pytest"]


def test_mpi_stops_on_a_short_log_and_says_when_eight_ranks_are_faster(tmp_path, monkeypatch, capsys):
    mpi = load("mpi_from_log.py")
    short = tmp_path / "short.log"
    short.write_text(
        "repeats 6\nrepeat 0 mode shared launch ranks 1 frames 1 local 1 ns 1000 sum 1 exit 0\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(mpi, "LOG", short)
    monkeypatch.setattr(mpi, "GEN", tmp_path / "gen")
    assert mpi.main() == 1
    assert "1 repeats" in capsys.readouterr().err

    def line(mode, ranks, ns):
        return (
            f"repeat 0 mode {mode} launch ranks {ranks} frames 1 local 1 ns {ns} sum 1 exit 0\n"
        )

    body = "repeats 6\n"
    for mode in ("shared", "scatter"):
        for ranks, ns in ((1, 5000), (2, 4000), (4, 3000), (8, 1000)):
            body += line(mode, ranks, ns) * 6
    full = tmp_path / "fast.log"
    full.write_text(body, encoding="utf-8")
    monkeypatch.setattr(mpi, "LOG", full)
    assert mpi.main() == 0
    text = (tmp_path / "gen" / "mpi_macros.tex").read_text(encoding="utf-8")
    assert "faster than four" in text
    assert (tmp_path / "gen" / "mpi_host.pdf").is_file()


def test_mpi_entry_rewrites_the_table():
    generated = CPC / "src" / "figures" / "generated"
    names = ("mpi_macros.tex", "mpi_host_table.tex", "mpi_host.pdf")
    saved = {generated / name: (generated / name).read_bytes() for name in names if (generated / name).is_file()}
    try:
        sys.argv = ["mpi_from_log.py"]
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "mpi_from_log.py"), run_name="__main__")
        assert caught.value.code == 0
    finally:
        for path, data in saved.items():
            path.write_bytes(data)
        sys.argv = ["pytest"]


def test_host_stamp_tolerates_a_missing_cpuinfo_and_rewrites_one_tree(tmp_path, monkeypatch):
    import pathlib

    real = pathlib.Path.read_text

    def denied(self, *args, **kwargs):
        if self.name == "cpuinfo":
            raise OSError("denied")
        return real(self, *args, **kwargs)

    monkeypatch.setattr(pathlib.Path, "read_text", denied)
    host = load("stamp_host.py")
    assert host.RAW == ""
    monkeypatch.undo()

    folder = tmp_path / "tree"
    folder.mkdir()
    marked = "Intel" + "(R)" + " Xeon" + "(R)" + " Processor"
    (folder / "note.txt").write_text(f"cpu {marked}\n", encoding="utf-8")
    sys.argv = ["stamp_host.py", str(folder)]
    try:
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "stamp_host.py"), run_name="__main__")
        assert caught.value.code == 0
    finally:
        sys.argv = ["pytest"]
    rewritten = (folder / "note.txt").read_text(encoding="utf-8")
    assert marked not in rewritten
    assert rewritten.startswith("cpu ")


def test_release_identity_rejects_a_foreign_record_a_private_commit_and_a_bare_manifest(tmp_path, monkeypatch):
    release = load("gen_release.py")
    record = CPC / "coverh-provenance.json"
    record.write_text(
        json.dumps({"build": {"library_digest": "a" * 64}}) + "\n", encoding="utf-8"
    )
    monkeypatch.setattr(release, "BUILD_RECORD", record)
    monkeypatch.setattr(release, "named_commit", lambda: "b" * 40)
    monkeypatch.setattr(release.provenance, "library_digest", lambda: "c" * 64)
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: "c" * 64)
    monkeypatch.setattr(release.provenance, "library_version", lambda: "1.2.3")
    monkeypatch.setattr(release, "ancestor", lambda older, newer: older != release.BASE)
    monkeypatch.setattr(release, "published", lambda: ("9.9.9", "d" * 40))

    def git(*args):
        if args[:2] == ("cat-file", "-e"):
            return ""
        if args[:2] == ("rev-parse", "HEAD"):
            return "b" * 40
        if args[0] == "show" and args[1].endswith("Cargo.toml"):
            return 'version = "1.2.3"\n'
        if args[0] == "show" and args[1].endswith("CHANGELOG.md"):
            return "## v1.2.3 title\n"
        if args[0] == "describe":
            return "v0.1.0"
        if args[0] == "rev-parse" and "refs/tags" in args[1]:
            return "e" * 40
        return None

    monkeypatch.setattr(release, "git", git)
    with pytest.raises(SystemExit, match="names"):
        release.identity()

    monkeypatch.setattr(release.provenance, "library_digest", lambda: "a" * 64)
    monkeypatch.setattr(release.provenance, "library_digest_at", lambda commit: "a" * 64)
    monkeypatch.setattr(release, "ancestor", lambda older, newer: False)
    with pytest.raises(SystemExit, match="public main"):
        release.identity()

    monkeypatch.setattr(release, "named_commit", lambda: "f" * 40)

    def cargo(*args):
        if args[:2] == ("rev-parse", "HEAD"):
            return "b" * 40
        if args[0] == "show" and args[1].endswith("Cargo.toml"):
            return "name = \"demo\"\n"
        return git(*args)

    monkeypatch.setattr(release, "git", cargo)
    monkeypatch.setattr(release, "ancestor", lambda older, newer: True)
    with pytest.raises(SystemExit, match="no version"):
        release.identity()

    monkeypatch.setattr(release, "git", git)
    monkeypatch.setattr(release, "ancestor", lambda older, newer: older != release.BASE)
    try:
        found = release.identity()
        assert found["tag_state"].startswith("No tag")
        assert found["version"] == "1.2.3"
    finally:
        record.unlink(missing_ok=True)


def test_trace_reports_a_wrong_identifier_and_an_unused_rule():
    trace = load("trace_numbers.py")
    from numbers_lib import Rule

    class Printed:
        document = "rg_main.tex"
        line = 4
        text = "1.2.3"
        macro = ""
        context = "version"
        value = Decimal("1.2")
        quantum = Decimal("0.1")

        @property
        def normal(self):
            return trace.normal(self.text)

    unused = Rule("never-printed", "0", "record.json", "count")
    wrong = Rule("1.2.3", "9.9.9", "record.json", "version")
    trace.scan = lambda: ([Printed()], [("rg_main.tex", 4, "1.2.3", "version", "version")])
    trace.load_rules = lambda: [unused, wrong]
    _rows, problems = trace.trace()
    assert any("9.9.9" in item for item in problems)
    assert any("matches no printed number" in item for item in problems)


def test_trace_entry_checks_the_ledger():
    sys.argv = ["trace_numbers.py", "--no-write"]
    try:
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "trace_numbers.py"), run_name="__main__")
        assert caught.value.code in (0, 1)
    finally:
        sys.argv = ["pytest"]


def test_release_check_names_this_trees_version_files(monkeypatch):
    manuscript = load("check_manuscript.py")
    _version, commit = manuscript.release()
    real = subprocess.run

    def run(cmd, **kwargs):
        if isinstance(cmd, (list, tuple)) and "rev-parse" in cmd and "HEAD" in cmd:
            return subprocess.CompletedProcess(cmd, 0, stdout=commit + "\n", stderr="")
        return real(cmd, **kwargs)

    monkeypatch.setattr(manuscript.subprocess, "run", run)
    record = CPC / "data" / "coverh-release.json"
    record.write_text(
        json.dumps(
            {
                "library_digest": "a" * 64,
                "readcon_version": "9.9.9",
                "readcon_module": "readcon",
                "readcon_module_sha256": "b" * 64,
            }
        )
        + "\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(manuscript, "records", lambda: [record])
    try:
        with pytest.raises(manuscript.Failed, match="9.9.9") as caught:
            manuscript.check_release(
                {
                    "readcon_version": "0",
                    "newer_sources": [],
                    "readcon_module_sha256": "c" * 64,
                }
            )
    finally:
        record.unlink()
    message = str(caught.value)
    assert "VERSION" in message
    assert "library" in message


def test_pdf_check_skips_a_url_that_continues_on_the_next_line(tmp_path, monkeypatch):
    manuscript = load("check_manuscript.py")
    pdf = tmp_path / "rg_main.pdf"
    pdf.write_bytes(b"%PDF")
    monkeypatch.setattr(manuscript, "PDFS", [pdf])

    def output(cmd, **kwargs):
        if "-raw" in cmd:
            return "see https://example.com/abc\nhttps://continued\n"
        return "clean page\n"

    monkeypatch.setattr(manuscript.subprocess, "check_output", output)
    text = manuscript.check_pdfs()
    assert text == (
        "PDFs carry no TODO, TBD, FIXME or XXX and no split identifier"
    )


def test_flat_pack_copies_a_figure_and_rejects_a_missing_citation(tmp_path, monkeypatch):
    combined = load("make_arxiv_combined.py")
    src = tmp_path / "src"
    (src / "figures" / "generated").mkdir(parents=True)
    (src / "rg_main.tex").write_text(
        "\\documentclass{article}\n\\includegraphics{figures/generated/dot.pdf}\n"
        "\\bibliography{rg_main}\n\\end{document}\n",
        encoding="utf-8",
    )
    (src / "rg_supp.tex").write_text("\\begin{document}supp\\end{document}\n", encoding="utf-8")
    (src / "rg_main.bib").write_text("@article{one,title={One}}\n", encoding="utf-8")
    (src / "figures" / "generated" / "dot.pdf").write_bytes(b"%PDF")
    monkeypatch.setattr(combined.arxiv, "supported_unicode", lambda texbin: set())
    monkeypatch.setattr(combined, "combine", lambda main, supp: main)
    stage = combined.stage_sources(src, tmp_path / "work", tmp_path, {"rewrites": [], "unicode": []})
    assert (stage / "dot.pdf").is_file()

    texbin = tmp_path / "texbin"
    texbin.mkdir()

    def tool(name, body):
        path = texbin / name
        path.write_text(body, encoding="utf-8")
        path.chmod(0o755)

    tool(
        "pdflatex",
        "#!/bin/sh\nprintf '%s\\n' '\\citation{wanted}' > main.aux\nexit 0\n",
    )
    tool(
        "bibtex",
        "#!/bin/sh\nprintf '%s\\n' '\\bibitem{other}' > main.bbl\nexit 0\n",
    )
    pack = stage.parent
    (pack / "rg_main.bib").write_text("@article{one,title={One}}\n", encoding="utf-8")
    (stage / "main.tex").write_text("\\bibliography{rg_main}\n", encoding="utf-8")
    with pytest.raises(combined.arxiv.PackError, match="lacks cited keys"):
        combined.write_bbl(stage, pack, texbin, {}, {"unicode": []})

    sys.argv = ["make_arxiv_combined.py", "--out", str(tmp_path / "out"), "--texbin", str(tmp_path / "missing")]
    try:
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "make_arxiv_combined.py"), run_name="__main__")
        assert caught.value.code == 2
    finally:
        sys.argv = ["pytest"]


def _arxiv_tree(root: Path, main: str, bbl: str, aux: str = "") -> None:
    root.mkdir(exist_ok=True)
    (root / "rg_main.tex").write_text(main, encoding="utf-8")
    (root / "rg_supp.tex").write_text("supp\n", encoding="utf-8")
    (root / "rg_main.aux").write_text(aux, encoding="utf-8")
    (root / "rg_supp.aux").write_text("\n", encoding="utf-8")
    (root / "rg_main.bbl").write_text(bbl, encoding="utf-8")
    (root / "rg_main.bib").write_text("@article{a,title={A}}\n", encoding="utf-8")


def _pdflatex(bin_dir: Path) -> None:
    bin_dir.mkdir()
    tool = bin_dir / "pdflatex"
    tool.write_text("#!/bin/sh\necho 'pdfTeX (TeX Live 2025)'\n", encoding="utf-8")
    tool.chmod(0o755)


def test_arxiv_parser_resolves_a_graphic_and_a_repeated_input(tmp_path):
    arxiv = load("make_arxiv.py")
    labels = arxiv.parse_newlabels(r"\newlabel{k}{plain}")
    assert labels["k"] == []
    (tmp_path / "fig.pdf").write_bytes(b"%PDF")
    assert arxiv.resolve_graphic(tmp_path, "fig.pdf") == tmp_path / "fig.pdf"
    snap = tmp_path / "snap"
    snap.mkdir()
    (snap / "rg_main.tex").write_text("\\input{b}\\input{b}\n", encoding="utf-8")
    (snap / "b.tex").write_text("\\input{rg_main}\n", encoding="utf-8")
    tex, _other, _bibs = arxiv.discover(snap)
    assert [path.name for path in tex] == ["rg_main.tex", "b.tex"]


def test_arxiv_pack_reports_a_bad_bibliography_and_a_failed_compile(tmp_path, monkeypatch):
    arxiv = load("make_arxiv.py")
    texbin = tmp_path / "texbin"
    _pdflatex(texbin)
    monkeypatch.setattr(arxiv, "supported_unicode", lambda bin_dir: set())
    monkeypatch.setattr(arxiv, "env_tool", lambda name: name)
    spec = tmp_path / "spec.org"
    spec.write_text("spec\n", encoding="utf-8")
    saved_spec = arxiv.SPEC_ORG
    arxiv.SPEC_ORG = spec
    bbl = "\\begin{thebibliography}{1}\n\\bibitem{a} A\n\\end{thebibliography}\n"

    def invoke(main: str, **kwargs):
        src = tmp_path / "src"
        if src.exists():
            for path in src.rglob("*"):
                if path.is_file():
                    path.unlink()
        else:
            src.mkdir()
        _arxiv_tree(src, main, kwargs.get("bbl", bbl), kwargs.get("aux", ""))
        if kwargs.get("old_pdf"):
            pdf = src / "rg_supp.pdf"
            pdf.write_bytes(b"%PDF")
            pdf.touch()
            newer = src / "rg_supp.tex"
            newer.write_text("supp\n", encoding="utf-8")
            import os
            os.utime(pdf, (1, 1))
        monkeypatch.setattr(
            sys,
            "argv",
            ["make_arxiv.py", "--out", str(tmp_path / "out"), "--src", str(src), "--texbin", str(texbin)],
        )

    try:
        invoke("\\externaldocument{other}\n\\bibliography{rg_main}\n")
        with pytest.raises(arxiv.PackError, match="externaldocument"):
            arxiv.main()
        invoke("\\bibliography{rg_main}\n", bbl="no bibliography here\n")
        with pytest.raises(arxiv.PackError, match="thebibliography"):
            arxiv.main()
        invoke("\\bibliography{rg_main}\n", aux="\\citation{missing}\n")
        with pytest.raises(arxiv.PackError, match="lacks cited keys"):
            arxiv.main()
        arxiv.SPEC_ORG = tmp_path / "absent.org"
        invoke("\\bibliography{rg_main}\n", aux="\\citation{a}\n")
        with pytest.raises(arxiv.PackError, match="is missing"):
            arxiv.main()
        arxiv.SPEC_ORG = spec
        invoke(
            "\\bibliography{rg_main}\n\\input{extra}\n",
            aux="\\citation{a}\n",
        )
        extra = tmp_path / "src" / "extra.tex"
        extra.write_text("\\documentclass{article}\n", encoding="utf-8")
        with pytest.raises(arxiv.PackError, match="documentclass"):
            arxiv.main()

        def compiled(workdir, env, bin_dir):
            (workdir / "rg_main.aux").write_text("\\citation{a}\n", encoding="utf-8")
            (workdir / "rg_main.log").write_text("log\n", encoding="utf-8")
            (workdir / "rg_main.pdf").write_bytes(b"%PDF")
            return "! Undefined control sequence.\n", [1]

        monkeypatch.setattr(arxiv, "pdflatex", compiled)
        monkeypatch.setattr(
            arxiv,
            "pdf_text",
            lambda pdf: "see ??" if "arxiv-test" in str(pdf) else "clean",
        )
        monkeypatch.setattr(arxiv, "bibtex_crosscheck", lambda *args: "DIFFERS from bibtex")
        saved_filter = getattr(tarfile, "data_filter", None)
        if saved_filter is not None:
            delattr(tarfile, "data_filter")
        try:
            invoke(
                "\\mathrm{x\\AA}\n\\bibliography{other}\n",
                aux="\\citation{a}\n",
                old_pdf=True,
            )
            assert arxiv.main() == 1
        finally:
            if saved_filter is not None:
                tarfile.data_filter = saved_filter
        report = json.loads((tmp_path / "out" / "arxiv-report.json").read_text(encoding="utf-8"))
        assert any("bibliography databases" in note for note in report["notes"])
        assert any("older than" in note for note in report["notes"])
        assert "DIFFERS" in report["bbl_check"]
        assert any("??" in item for item in report["failures"])
        assert any("unstripped" in item for item in report["failures"])
        assert any("comment stripping" in item for item in report["failures"])
        assert any(item.startswith("bbl differs") for item in report["failures"])
    finally:
        arxiv.SPEC_ORG = saved_spec


def test_arxiv_entry_reports_a_pack_error(tmp_path):
    sys.argv = ["make_arxiv.py", "--out", str(tmp_path / "out"), "--texbin", str(tmp_path / "missing")]
    try:
        with pytest.raises(SystemExit) as caught:
            runpy.run_path(str(SCRIPTS / "make_arxiv.py"), run_name="__main__")
        assert caught.value.code == 2
    finally:
        sys.argv = ["pytest"]
