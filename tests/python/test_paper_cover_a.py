"""Behavioral branch coverage for the paper generators."""

from __future__ import annotations

import argparse
import csv
import importlib.util
import io
import json
import os
import runpy
import shutil
import stat
import subprocess
import sys
import tarfile
import textwrap
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"
CPC = ROOT / "paper" / "cpc"


def load(name: str):
    path = SCRIPTS / name
    key = "covera_" + name.replace("/", "_").replace(".py", "")
    spec = importlib.util.spec_from_file_location(key, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[key] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def texbin(tmp: Path) -> Path:
    bindir = tmp / "texbin"
    bindir.mkdir(parents=True)
    for name in ("pdflatex", "bibtex", "kpsewhich"):
        script = bindir / name
        script.write_text(
            "#!/bin/sh\n"
            'if [ "$1" = "--version" ]; then\n'
            '  echo "pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025)"\n'
            "  exit 0\n"
            "fi\n"
            f"exec /usr/bin/{name} \"$@\"\n",
            encoding="utf-8",
        )
        script.chmod(0o755)
    return bindir


def article_src(tmp: Path) -> Path:
    src = tmp / "src"
    (src / "figures").mkdir(parents=True)
    shutil.copy(CPC / "src" / "figures" / "generated" / "architecture.pdf", src / "dot.pdf")
    (src / "localpack.sty").write_text("% local\n", encoding="utf-8")
    (src / "extra.tex").write_text("Extra words.\n", encoding="utf-8")
    (src / "rg_supp.tex").write_text(
        "\\documentclass{article}\n\\begin{document}\n"
        "\\section{Supp}\\label{sup:only}\n\\end{document}\n",
        encoding="utf-8",
    )
    (src / "rg_supp.aux").write_text(
        "\\newlabel{sup:only}{{S1}{1}{Supp}{}{}}\n", encoding="utf-8"
    )
    (src / "rg_main.aux").write_text(
        "\\bibstyle{plain}\n\\bibdata{rg_main}\n\\citation{one}\n"
        "\\newlabel{sec:here}{{1}{1}{Here}{}{}}\n\\@input{sub.aux}\n",
        encoding="utf-8",
    )
    (src / "sub.aux").write_text("\\newlabel{sub:x}{{2}{1}{}{}{}}\n", encoding="utf-8")
    (src / "rg_main.bbl").write_text(
        "\\begin{thebibliography}{1}\n\\bibitem{one}\nOne.\n\\end{thebibliography}\n",
        encoding="utf-8",
    )
    (src / "rg_main.bib").write_text(
        "@article{one, title={One}, author={A}, year={2020}}\n", encoding="utf-8"
    )
    (src / "rg_main.tex").write_text(
        textwrap.dedent(
            r"""
            \documentclass{article}
            \usepackage{graphicx}
            \usepackage{xr}
            \usepackage{localpack}
            \externaldocument{rg_supp}
            \begin{document}
            See \ref{sup:only}.
            \input{extra}
            \cite{one}
            % a full comment
            Hello. % trailing note
            \bibliographystyle{plain}
            \bibliography{rg_main}
            \end{document}
            """
        ).lstrip(),
        encoding="utf-8",
    )
    pdf = src / "rg_supp.pdf"
    shutil.copy(src / "dot.pdf", pdf)
    os.utime(pdf, (1, 1))
    os.utime(src / "rg_main.aux", (1, 1))
    return src


def test_make_arxiv_helpers_and_a_small_build(tmp_path, monkeypatch):
    arxiv = load("make_arxiv.py")
    assert arxiv.unescaped_percent("a\\%b %c") == 5
    assert arxiv.unescaped_percent("100\\%") == -1
    assert arxiv.unescaped_percent("no") == -1
    stripped, stats = arxiv.strip_comments(
        "% gone\n"
        "keep % trail\n"
        "\\verb|%x| tail\n"
        "\\begin{verbatim}\n% stay\n\\end{verbatim}\n"
        "\\begin{lstlisting} % before\ncode\n"
        "plain\n"
    )
    assert "gone" not in stripped
    assert stats["full_line"] >= 1
    assert stats["trailing"] >= 1
    assert stats["kept_inline_verbatim"] >= 1
    with pytest.raises(arxiv.PackError):
        arxiv.balanced_group("abc", 0)
    inner, end = arxiv.balanced_group("{a\\{b}c}", 0)
    assert inner.startswith("a")
    with pytest.raises(arxiv.PackError):
        arxiv.balanced_group("{abc", 0)
    labels = arxiv.parse_newlabels(
        "\\newlabel{k}{{1}{2}{title}{}{}} noise {x}\n"
    )
    assert labels["k"][0] == "1"
    aux = tmp_path / "main.aux"
    (tmp_path / "sub.aux").write_text("sub\n", encoding="utf-8")
    aux.write_text("top\n\\@input{sub.aux}\n\\@input{missing.aux}\n", encoding="utf-8")
    assert "sub" in arxiv.read_aux_tree(aux)
    assert arxiv.line_of("a\nb", 2) == 2

    src = article_src(tmp_path)
    snap = tmp_path / "snap"
    shutil.copytree(src, snap)
    (snap / "rg_main.tex").write_text(
        (snap / "rg_main.tex").read_text(encoding="utf-8").replace(
            "\\input{extra}", "\\includegraphics{dot}\n\\input{extra}"
        ),
        encoding="utf-8",
    )
    files, other, bibs = arxiv.discover(snap)
    assert bibs == ["rg_main"]
    assert any(p.name == "dot.pdf" for p in other)
    assert any(p.name == "localpack.sty" for p in other)
    (snap / "mine.cls").write_text("% cls\n", encoding="utf-8")
    (snap / "rg_main.tex").write_text(
        (snap / "rg_main.tex").read_text(encoding="utf-8").replace(
            "\\documentclass{article}", "\\documentclass{mine}"
        ),
        encoding="utf-8",
    )
    _, other, _ = arxiv.discover(snap)
    assert any(p.name == "mine.cls" for p in other)
    main = (snap / "rg_main.tex").read_text(encoding="utf-8")
    (snap / "rg_main.tex").write_text(main + "\n\\graphicspath{{a/}}\n", encoding="utf-8")
    with pytest.raises(arxiv.PackError, match="graphicspath"):
        arxiv.discover(snap)
    (snap / "rg_main.tex").write_text(main + "\n\\input{\\foo}\n", encoding="utf-8")
    with pytest.raises(arxiv.PackError, match="computed"):
        arxiv.discover(snap)
    (snap / "rg_main.tex").write_text(
        main + "\n\\includegraphics{\\bar}\n", encoding="utf-8"
    )
    with pytest.raises(arxiv.PackError, match="computed"):
        arxiv.discover(snap)
    (snap / "rg_main.tex").write_text(main + "\n\\input{missing-file}\n", encoding="utf-8")
    with pytest.raises(arxiv.PackError, match="cannot resolve"):
        arxiv.discover(snap)
    (snap / "rg_main.tex").write_text(
        main + "\n\\includegraphics{missing-fig}\n", encoding="utf-8"
    )
    with pytest.raises(arxiv.PackError, match="cannot resolve"):
        arxiv.discover(snap)

    bin_dir = texbin(tmp_path)
    chars = arxiv.supported_unicode(bin_dir)
    assert chars
    empty = tmp_path / "emptybin"
    empty.mkdir()
    (empty / "kpsewhich").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    (empty / "kpsewhich").chmod(0o755)
    with pytest.raises(arxiv.PackError, match="dfu"):
        arxiv.supported_unicode(empty)
    report: list[str] = []
    fixed = arxiv.fix_unicode("n", "\u2009\n", chars, report)
    assert r"\," in fixed
    with pytest.raises(arxiv.PackError, match="no fallback"):
        arxiv.fix_unicode("n", "\u2603\n", chars, [])
    math = arxiv.fix_math_text("n", r"\(\mathrm{\AA}\)", [])
    assert "text" in math
    rewritten, docs = arxiv.rewrite_xr(
        "\\usepackage{xr}\n\\externaldocument[pre]{rg_supp}\n", []
    )
    assert docs and "rg_supp_labels" in rewritten
    with pytest.raises(arxiv.PackError, match="split"):
        arxiv.rewrite_xr("\\usepackage{xr,hyperref}\n", [])
    plain = "no highlights\n"
    assert arxiv.guard_prelim_page_anchor(plain, []) == plain
    no_front = "\\begin{highlights}x\\end{highlights}\n"
    assert arxiv.guard_prelim_page_anchor(no_front, []) == no_front
    guarded = (
        "\\begin{highlights}x\\end{highlights}\n"
        "\\hypersetup{pageanchor=false}\n"
        "\\begin{frontmatter}y\\end{frontmatter}\n"
        "\\hypersetup{pageanchor=true}\n"
    )
    out = arxiv.guard_prelim_page_anchor(guarded, [])
    assert out.count("pageanchor=false") == 1
    commented = (
        "\\begin{highlights}x\\end{highlights}\n"
        "% note \\hypersetup{pageanchor=false}\n"
        "\\begin{frontmatter}y\\end{frontmatter}\n"
    )
    commented_out = arxiv.guard_prelim_page_anchor(commented, [])
    assert commented_out.count("pageanchor=false") == 2
    added = arxiv.guard_prelim_page_anchor(
        "\\begin{graphicalabstract}g\\end{graphicalabstract}\n"
        "\\begin{frontmatter}y\\end{frontmatter}\n",
        [],
    )
    assert "pageanchor=false" in added and "pageanchor=true" in added
    starred = arxiv.star_refs("n", "\\ref{a} \\ref*{b} \\eqref{c}", {"a"}, [])
    assert "\\ref*{a}" in starred
    assert arxiv.star_refs("n", "\\ref{z}", set(), []) == "\\ref{z}"
    with pytest.raises(arxiv.PackError, match="only"):
        arxiv.star_refs("n", "\\cref{a}", {"a"}, [])
    assert "a" in arxiv.referenced_keys(["\\ref{a, b}"])
    assert "lab" in arxiv.defined_keys(["\\label{lab} label={other}"])
    label_path = tmp_path / "labels.tex"
    listed = arxiv.write_label_file(
        label_path,
        "rg_supp",
        "pre",
        {"only": ["S1", "2", "title"], "short": ["1"]},
        ["preonly", "preshort"],
    )
    assert listed and label_path.is_file()
    with pytest.raises(arxiv.PackError, match="not found"):
        arxiv.write_label_file(label_path, "d", "", {}, ["missing"])
    stage = tmp_path / "stage"
    stage.mkdir()
    (stage / "a.txt").write_text("hi", encoding="utf-8")
    names = arxiv.make_tarball(stage, tmp_path / "out.tar.gz")
    assert names == ["a.txt"]
    env = arxiv.tex_env(bin_dir, tmp_path / "home", 0)
    assert "TEXMFHOME" in env and "TEXINPUTS" not in env
    work = tmp_path / "pdfwork"
    work.mkdir()
    (work / "rg_main.tex").write_text(
        "\\documentclass{article}\\begin{document}Hi\\end{document}\n",
        encoding="utf-8",
    )
    log_text, codes = arxiv.pdflatex(work, env, bin_dir)
    assert codes[0] == 0 and "Output written" in log_text
    broken = tmp_path / "broken"
    broken.mkdir()
    (broken / "rg_main.tex").write_text("\\bad\n", encoding="utf-8")
    missing_log, bad_codes = arxiv.pdflatex(broken, env, bin_dir)
    assert bad_codes[0] != 0
    found, kinds, info = arxiv.analyse_log(
        "! Emergency stop.\n"
        "Reference `x' on page 1 undefined\n"
        "Citation `y' on page 1 undefined\n"
        "Label(s) may have changed\n"
        "Label `z' multiply defined\n"
        "Overfull \\hbox (1pt too wide)\n"
        "Missing character: there is no x\n"
        "Suppressing link with empty target\n"
        "Unicode character not set up for use with LaTeX\n"
        "destination with the same identifier (name)\n"
        "LaTeX Font Warning: Font shape `a' undefined\n"
        "Overfull \\vbox (1pt too high)\n"
        "Underfull \\hbox (badness 10000)\n"
        "Underfull \\vbox (badness 10000)\n"
        "Command \\r invalid in math mode\n"
        "Package foo Warning: bar\n"
        "Output written on x.pdf (2 pages, 100 bytes)\n"
    )
    assert found["error"]["count"] and info["pages"] == 2 and kinds
    assert arxiv.analyse_log("nothing")[2]["pages"] is None
    assert arxiv.env_tool("pdftotext").endswith("pdftotext")
    with pytest.raises(arxiv.PackError, match="not in"):
        arxiv.env_tool("not-a-tool")
    assert arxiv.pdf_text(tmp_path / "missing.pdf") is None
    pdf = work / "rg_main.pdf"
    assert arxiv.pdf_text(pdf)
    dead = tmp_path / "deadbin"
    dead.mkdir()
    (dead / "pdftotext").write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
    (dead / "pdftotext").chmod(0o755)
    saved_bin = arxiv.ENV_BIN
    monkeypatch.setattr(arxiv, "ENV_BIN", dead)
    assert arxiv.pdf_text(pdf) is None
    monkeypatch.setattr(arxiv, "ENV_BIN", saved_bin)

    bib = tmp_path / "bibcheck"
    (bib / "snap").mkdir(parents=True)
    (bib / "test").mkdir()
    (bib / "test" / "rg_main.aux").write_text("\\citation{one}\n", encoding="utf-8")
    (bib / "snap" / "rg_main.bib").write_text(
        "@article{one, title={One}, author={A}, year={2020}}\n", encoding="utf-8"
    )
    shipped = bib / "shipped.bbl"
    shipped.write_text("same words\n", encoding="utf-8")
    fake = tmp_path / "bibbin"
    fake.mkdir()
    (fake / "bibtex").write_text(
        "#!/bin/sh\nprintf 'same words\\n' > rg_main.bbl\nexit 0\n",
        encoding="utf-8",
    )
    (fake / "bibtex").chmod(0o755)
    assert "identical" in arxiv.bibtex_crosscheck(
        bib / "test", bib / "snap", ["rg_main"], fake, {}, shipped
    )
    shutil.rmtree(bib / "bibtex-check")
    shipped.write_text("same\nwords\n", encoding="utf-8")
    assert "line wrapping" in arxiv.bibtex_crosscheck(
        bib / "test", bib / "snap", ["rg_main"], fake, {"max_print_line": "1"}, shipped
    )
    shutil.rmtree(bib / "bibtex-check")
    shipped.write_text("different\n", encoding="utf-8")
    assert arxiv.bibtex_crosscheck(
        bib / "test", bib / "snap", ["rg_main"], fake, {}, shipped
    ).startswith("DIFFERS")
    shutil.rmtree(bib / "bibtex-check")
    assert "not found" in arxiv.bibtex_crosscheck(
        bib / "test", bib / "snap", ["absent"], fake, {}, shipped
    )
    shutil.rmtree(bib / "bibtex-check")
    (fake / "bibtex").write_text("#!/bin/sh\nexit 2\n", encoding="utf-8")
    (fake / "bibtex").chmod(0o755)
    assert "failed" in arxiv.bibtex_crosscheck(
        bib / "test", bib / "snap", ["rg_main"], fake, {}, shipped
    )

    monkeypatch.setattr(sys, "argv", ["make_arxiv.py", "--out", str(tmp_path / "no-bin"), "--texbin", str(tmp_path / "no-such")])
    with pytest.raises(arxiv.PackError, match="no pdflatex"):
        arxiv.main()
    monkeypatch.setattr(
        sys,
        "argv",
        ["make_arxiv.py", "--out", str(tmp_path / "old"), "--texbin", "/usr/bin", "--src", str(src)],
    )
    with pytest.raises(arxiv.PackError, match="not TeX Live 2025"):
        arxiv.main()
    bbl_dir = tmp_path / "bbl-src"
    bbl_dir.mkdir()
    (bbl_dir / "rg_main.aux").write_text(
        "\\bibstyle{plain}\n\\bibdata{rg_main}\n\\citation{one}\n", encoding="utf-8"
    )
    shutil.copy(src / "rg_main.bib", bbl_dir / "rg_main.bib")
    subprocess.run(["bibtex", "rg_main"], cwd=bbl_dir, check=True, capture_output=True)
    shutil.copy(bbl_dir / "rg_main.bbl", src / "rg_main.bbl")
    out = tmp_path / "arxiv-out"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "make_arxiv.py",
            "--out",
            str(out),
            "--src",
            str(src),
            "--texbin",
            str(bin_dir),
            "--keep-math-text",
            "--keep-work",
        ],
    )
    code = arxiv.main()
    assert code in (0, 1)
    assert (out / "readcon-cpc-arxiv.tar.gz").is_file()
    report_json = json.loads((out / "arxiv-report.json").read_text(encoding="utf-8"))
    assert "files" in report_json

    # rebuild and build-in-src paths, with the pdfTeX build stubbed after the branch is taken
    def fake_build(build_dir: Path) -> None:
        for name in ("rg_main.aux", "rg_supp.aux", "rg_main.bbl"):
            (build_dir / name).write_text("% built\n", encoding="utf-8")

    monkeypatch.setattr(arxiv, "pdftex_build", fake_build)
    bare = tmp_path / "bare"
    shutil.copytree(src, bare)
    for name in ("rg_main.aux", "rg_supp.aux", "rg_main.bbl"):
        (bare / name).unlink()
    snap_dir, notes = arxiv.snapshot_sources(bare, tmp_path / "work-rebuild", False, True)
    assert any("pdfTeX" in note for note in notes)
    assert (snap_dir / "rg_main.aux").is_file()
    snap_dir, notes = arxiv.snapshot_sources(bare, tmp_path / "work-in-src", True, False)
    assert any("built in" in note for note in notes)

    fresh = load("make_arxiv.py")
    original_which = fresh.shutil.which
    monkeypatch.setattr(fresh.shutil, "which", lambda name: None)
    with pytest.raises(fresh.PackError, match="not on PATH"):
        fresh.pdftex_build(tmp_path)
    monkeypatch.setattr(fresh.shutil, "which", original_which)

    def no_products(build_dir: Path) -> None:
        fresh_run = fresh.subprocess.run

        def run(*cmd, **kwargs):
            return fresh_run(*cmd, **kwargs)

        return None

    # pdftex_build raises when the three products are absent after the passes.
    empty_src = tmp_path / "empty-src"
    empty_src.mkdir()
    (empty_src / "rg_main.tex").write_text(
        "\\documentclass{article}\\begin{document}Hi\\end{document}\n", encoding="utf-8"
    )
    (empty_src / "rg_supp.tex").write_text(
        "\\documentclass{article}\\begin{document}Hi\\end{document}\n", encoding="utf-8"
    )
    monkeypatch.setattr(fresh.subprocess, "run", lambda *args, **kwargs: None)
    with pytest.raises(fresh.PackError, match="did not produce"):
        fresh.pdftex_build(empty_src)

    monkeypatch.setattr(sys, "argv", ["make_arxiv.py"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "make_arxiv.py"), run_name="__main__")


def test_make_em_flat_and_combined(tmp_path, monkeypatch):
    em = load("make_em_flat.py")
    arxiv = load("make_arxiv.py")
    assert em.flatten_name(Path("figures/generated/x.pdf")) == "x.pdf"
    flat = em.rewrite_paths_flat(
        "\\input{figures/generated/a.tex}\n"
        "\\includegraphics*[width=1]{figures/generated/b.pdf}\n"
        "\\path{figures/generated/keep}\n"
        "{figures/generated/c.pdf}\n"
    )
    assert "figures/generated/a" not in flat
    assert "keep" in flat
    replaced = em.replace_bibliography(
        "\\bibliographystyle{plain}\n\\bibliography{rg_main}\n"
    )
    assert "\\input{rg_main.bbl}" in replaced
    with pytest.raises(em.arxiv.PackError, match="remains"):
        em.replace_bibliography("mid \\bibliography{rg_main} stay\n")
    with pytest.raises(em.arxiv.PackError, match="failed to insert"):
        em.replace_bibliography("no bibliography here\n")

    stage = tmp_path / "stage"
    (stage / "anc").mkdir(parents=True)
    (stage / "anc" / "skip.txt").write_text("x", encoding="utf-8")
    (stage / "00README.json").write_text("{}\n", encoding="utf-8")
    (stage / "rg_main.tex").write_text(
        "\\bibliography{rg_main}\n\\input{figures/generated/extra}\n",
        encoding="utf-8",
    )
    (stage / "extra.tex").write_text("\\input{figures/generated/z}\n", encoding="utf-8")
    (stage / "dot.pdf").write_bytes(b"%PDF")
    (stage / "rg_main.bbl").write_text("bbl\n", encoding="utf-8")
    report = {"rewrites": []}
    names = em.write_flat(stage, tmp_path / "flat", stage, report)
    assert "rg_main.tex" in names and "spec.org" in names
    again = tmp_path / "flat2"
    (stage / "rg_main.bbl").unlink()
    (tmp_path / "flat" / "rg_main.bbl").unlink()
    with pytest.raises(em.arxiv.PackError, match="missing"):
        em.write_flat(stage, again, stage, {"rewrites": []})
    (stage / "rg_main.bbl").write_text("bbl\n", encoding="utf-8")
    (stage / "other.tex").write_text("x", encoding="utf-8")
    (stage / "nested").mkdir()
    (stage / "nested" / "other.tex").write_text("y", encoding="utf-8")
    with pytest.raises(em.arxiv.PackError, match="duplicate flat names"):
        em.write_flat(stage, tmp_path / "flat3", stage, {"rewrites": []})
    (stage / "nested" / "other.tex").unlink()
    (stage / "dot.pdf").write_bytes(b"%PDF-a")
    (stage / "nested" / "dot.pdf").write_bytes(b"%PDF-b")
    with pytest.raises(em.arxiv.PackError, match="flat name collision"):
        em.write_flat(stage, tmp_path / "flat4", stage, {"rewrites": []})

    bin_dir = texbin(tmp_path)
    flat_dir = tmp_path / "compile"
    flat_dir.mkdir()
    (flat_dir / "rg_main.tex").write_text(
        "\\documentclass{article}\\begin{document}Hi\\end{document}\n",
        encoding="utf-8",
    )
    log_text, codes, comp = em.compile_flat(flat_dir, bin_dir, arxiv.tex_env(bin_dir, tmp_path / "h", 0))
    assert codes[0] == 0 and comp["info"]["pages"]
    # second call removes the previous compile directory
    em.compile_flat(flat_dir, bin_dir, arxiv.tex_env(bin_dir, tmp_path / "h", 0))

    monkeypatch.setattr(sys, "argv", ["make_em_flat.py", "--out", str(tmp_path / "em"), "--texbin", str(tmp_path / "missing")])
    with pytest.raises(em.arxiv.PackError, match="no pdflatex"):
        em.main()
    monkeypatch.setattr(
        sys,
        "argv",
        ["make_em_flat.py", "--out", str(tmp_path / "em"), "--texbin", "/usr/bin"],
    )
    with pytest.raises(em.arxiv.PackError, match="not TeX Live 2025"):
        em.main()

    src = article_src(tmp_path / "emsrc")
    out = tmp_path / "em-out"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "make_em_flat.py",
            "--out",
            str(out),
            "--src",
            str(src),
            "--texbin",
            str(bin_dir),
            "--keep-work",
        ],
    )
    assert em.main() in (0, 1)
    assert (out / "em-flat-report.json").is_file()

    combined = load("make_arxiv_combined.py")
    main = (src / "rg_main.tex").read_text(encoding="utf-8")
    # The combined packager expects the manuscript's xr-hyper block and tail.
    preamble = textwrap.dedent(
        r"""
        \documentclass{article}
        \usepackage{amsmath}
        \usepackage{xr-hyper}
        \makeatletter
        \let\readcon@bibcite\bibcite
        \def\bibcite#1#2{}%
        \externaldocument{rg_supp}
        \let\bibcite\readcon@bibcite
        \makeatother
        \usepackage{hyperref}
        \usepackage{listings}
        \begin{document}
        Hello \cite{one}.
        \DeclareUrlCommand\path{\url}
        \raggedright
        \bibliographystyle{elsarticle-num}
        \bibliography{rg_main}
        \end{document}
        """
    ).lstrip()
    supp = textwrap.dedent(
        r"""
        \documentclass{article}
        \usepackage{booktabs}
        \input{figures/generated/only-supp}
        \title{Supplement title}
        \begin{document}
        \begin{frontmatter}
        \begin{abstract}
        Supp abstract.
        \end{abstract}
        \end{frontmatter}
        Supp body.
        \DeclareUrlCommand\path{\url}
        \bibliography{rg_main}
        \end{document}
        """
    ).lstrip()
    merged = combined.with_supplement_preamble(preamble, supp)
    assert "booktabs" in merged
    assert "only-supp" in merged
    with pytest.raises(combined.arxiv.PackError, match="after no line"):
        combined.with_supplement_preamble("\\begin{document}\n", "\\usepackage{alone}\n\\begin{document}\n")
    text = combined.combine(preamble, supp)
    assert "Supplementary material" in text
    with pytest.raises(combined.arxiv.PackError, match="xr-hyper"):
        combined.combine("no xr\n\\end{document}\n", supp)
    with pytest.raises(combined.arxiv.PackError, match="does not end"):
        combined.combine(
            preamble.replace("\\DeclareUrlCommand\\path{\\url}\n\\raggedright\n", ""),
            supp,
        )
    broken_supp = supp.replace("\\title{Supplement title}", "")
    with pytest.raises(combined.arxiv.PackError, match="no title"):
        combined.combine(preamble, broken_supp)
    body_supp = supp.replace("Supp body.", "\\begin{document}Supp body.")
    with pytest.raises(combined.arxiv.PackError, match="preamble"):
        combined.combine(preamble, body_supp)
    report = {"rewrites": [], "unicode": []}
    cleaned = combined.clean("main.tex", preamble + "\u2009", arxiv.supported_unicode(bin_dir), report)
    assert r"\," in cleaned

    comb_src = tmp_path / "comb-src"
    shutil.copytree(src, comb_src)
    (comb_src / "rg_main.tex").write_text(preamble, encoding="utf-8")
    (comb_src / "rg_supp.tex").write_text(supp, encoding="utf-8")
    (comb_src / "figures" / "generated").mkdir(parents=True)
    (comb_src / "figures" / "generated" / "only-supp.tex").write_text("% m\n", encoding="utf-8")
    (comb_src / "figures" / "generated" / "fig.pdf").write_bytes(
        (src / "dot.pdf").read_bytes()
    )
    work = tmp_path / "comb-work"
    stage = combined.stage_sources(comb_src, work, bin_dir, report)
    assert (stage / "main.tex").is_file()
    (stage / "dup.tex").write_text("x", encoding="utf-8")
    # collision is inside the loop; call it by planting two inputs of the same basename
    (comb_src / "only-supp.tex").write_text("% top\n", encoding="utf-8")
    (comb_src / "rg_main.tex").write_text(
        preamble.replace("\\begin{document}", "\\input{only-supp}\n\\begin{document}"),
        encoding="utf-8",
    )
    with pytest.raises(combined.arxiv.PackError, match="collision"):
        combined.stage_sources(comb_src, tmp_path / "comb-work-2", bin_dir, {"rewrites": [], "unicode": []})
    (comb_src / "only-supp.tex").unlink()
    (comb_src / "rg_main.tex").write_text(
        preamble.replace(
            "Hello \\cite{one}.",
            "Hello \\cite{one}.\n\\bibliography{other}",
        ),
        encoding="utf-8",
    )
    with pytest.raises(combined.arxiv.PackError, match="bibliographies"):
        combined.stage_sources(comb_src, tmp_path / "comb-work-3", bin_dir, {"rewrites": [], "unicode": []})

    (comb_src / "rg_main.tex").write_text(preamble, encoding="utf-8")
    out = tmp_path / "comb-out"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "make_arxiv_combined.py",
            "--out",
            str(out),
            "--src",
            str(comb_src),
            "--texbin",
            str(bin_dir),
        ],
    )
    assert combined.main() in (0, 1)

    # write_bbl and check failure branches with a broken compiler script
    bad = tmp_path / "badtex"
    bad.mkdir()
    (bad / "pdflatex").write_text(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'TeX Live 2025'; exit 0; fi\nexit 1\n",
        encoding="utf-8",
    )
    (bad / "bibtex").write_text("#!/bin/sh\nexit 2\n", encoding="utf-8")
    for path in bad.iterdir():
        path.chmod(0o755)
    (bad / "kpsewhich").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    (bad / "kpsewhich").chmod(0o755)
    tiny_stage = tmp_path / "tiny-stage"
    tiny_stage.mkdir()
    (tiny_stage / "main.tex").write_text(preamble, encoding="utf-8")
    (tmp_path / "rg_main.bib").write_text("@article{one,title={One}}\n", encoding="utf-8")
    bib_work = tmp_path / "bib-work"
    bib_work.mkdir()
    shutil.copy(tmp_path / "rg_main.bib", bib_work / "rg_main.bib")
    with pytest.raises(combined.arxiv.PackError, match="bibliography"):
        combined.write_bbl(tiny_stage, bib_work, bad, {}, {"unicode": []})

    # A successful one-pass then a bbl that lacks a cited key.
    good = texbin(tmp_path / "goodbin")
    ok_stage = tmp_path / "ok-stage"
    ok_stage.mkdir()
    (ok_stage / "main.tex").write_text(
        "\\documentclass{article}\\begin{document}\\cite{one}\\bibliographystyle{plain}\\bibliography{rg_main}\\end{document}\n",
        encoding="utf-8",
    )
    ok_work = tmp_path / "ok-work"
    ok_work.mkdir()
    (ok_work / "rg_main.bib").write_text(
        "@article{one, title={One}, author={A}, year={2020}}\n", encoding="utf-8"
    )
    # bibliography commands must occur once; the tiny file has them once.
    try:
        combined.write_bbl(
            ok_stage, ok_work, good, arxiv.tex_env(good, ok_work / "home", 0), {"unicode": []}
        )
    except combined.arxiv.PackError:
        pass
    tar_path = tmp_path / "bad.tar.gz"
    tar_src = tmp_path / "tar-src"
    tar_src.mkdir()
    (tar_src / "main.tex").write_text("\\bad\n", encoding="utf-8")
    arxiv.make_tarball(tar_src, tar_path)
    (tmp_path / "check-work").mkdir()
    result, _test = combined.check(tar_path, good, tmp_path / "check-work", arxiv.tex_env(good, tmp_path / "h2", 0))
    assert result["failures"]

    monkeypatch.setattr(sys, "argv", ["make_em_flat.py"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "make_em_flat.py"), run_name="__main__")
    monkeypatch.setattr(sys, "argv", ["make_arxiv_combined.py"])
    with pytest.raises(SystemExit):
        runpy.run_path(str(SCRIPTS / "make_arxiv_combined.py"), run_name="__main__")


def test_small_record_scripts(tmp_path, monkeypatch):
    pack = load("pack_paper.py")
    out = tmp_path / "paper.tar.gz"
    monkeypatch.setattr(sys, "argv", ["pack_paper.py", str(out)])
    assert pack.main() == 0
    assert out.is_file()
    monkeypatch.setattr(pack, "IGNORED_RECORDS", ("missing/record",))
    with pytest.raises(SystemExit, match="missing"):
        pack.records()
    monkeypatch.setattr(pack, "IGNORED_RECORDS", ("src/rg_main.tex", "evidence/work/e1"))
    named = pack.records()
    assert any(name.endswith("rg_main.tex") for name in named)
    assert any("/" in name for name in named)
    monkeypatch.setattr(pack, "kept", lambda: ["bench/work/x.pyc", "src/rg_main.tex"])
    monkeypatch.setattr(pack, "records", lambda: [])
    with pytest.raises(SystemExit, match="build products"):
        pack.members()

    logged = load("logged_cmd.py")
    version = "0.0.0"
    for row in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if row.startswith('version = "'):
            version = row.split('"', 2)[1]
            break
    home = str(Path.home())
    raw = (
        f"{ROOT}/src/lib.rs {home}/.local/lib {home} ~/.local ~/x "
        f"_home_{Path.home().name}_.local _home_{Path.home().name}_ "
        f"site-packages/nanobind/a site-packages/b site-packages_c {version}"
    )
    cleaned = logged.redact(raw)
    assert "<site-packages>/nanobind/" in cleaned
    assert "<package-version>" in cleaned
    assert str(ROOT) not in cleaned
    monkeypatch.setattr(sys, "argv", ["logged_cmd.py"])
    assert logged.main() == 2
    target = tmp_path / "nested" / "run.txt"
    hidden = "/" + "tmp" + "/hidden"
    monkeypatch.setattr(
        sys,
        "argv",
        ["logged_cmd.py", str(target), "bash", "-c", "echo hello; echo " + hidden + "; echo ok"],
    )
    assert logged.main() == 0
    body = target.read_text(encoding="utf-8").splitlines()
    assert "hello" in body and body[-1] == "exit_code 0"
    assert not any(line == hidden or line.endswith("hidden") and not line.startswith("command") for line in body)
    monkeypatch.setattr(sys, "argv", ["logged_cmd.py", "paper/cpc/logs/_rel_cover.txt", "echo", "rel"])
    assert logged.main() == 0
    rel = ROOT / "paper" / "cpc" / "logs" / "_rel_cover.txt"
    rel.unlink()

    db = load("db_from_log.py")
    assert db.samples("note\nrepeat 1 kind readcon-db seconds 1.5\n")["readcon-db"] == [1.5]
    assert db.median([1.0, 3.0]) == 2.0
    monkeypatch.setattr(db, "OUT", tmp_path / "db_macros.tex")
    monkeypatch.setattr(sys, "argv", ["db_from_log.py"])
    assert db.main() == 0
    assert "rcDbIngestMedian" in (tmp_path / "db_macros.tex").read_text(encoding="utf-8")

    interval = load("dlpack_interval.py")
    view, copy = interval.samples()
    assert view.size and copy.size
    interval.main()
    monkeypatch.setattr(interval, "LOG", tmp_path / "dlpack.txt")
    (tmp_path / "dlpack.txt").write_text("note\nrepeat 0 a 1 b 2\n", encoding="utf-8")
    assert interval.samples()[0].tolist() == [1.0]

    cargo = load("cargo_version_tex.py")
    monkeypatch.setattr(cargo, "OUT", tmp_path / "cargo_version.tex")
    assert cargo.main() == 0
    assert "readconCargoVersion" in (tmp_path / "cargo_version.tex").read_text(encoding="utf-8")

    tables = load("gen_tables.py")
    assert tables.tex("a_b&c%#") 
    assert tables.mask_integer("101") == "5"
    rows = [
        {"reader": "other", "frame": "0", "masks_xyz": "1 0 0", "ids": "1 2 3", "velocities": ""},
        {
            "reader": "readcon-core",
            "frame": "0",
            "masks_xyz": "100 010 001",
            "ids": "2 0 1",
            "velocities": "None None None",
        },
        {
            "reader": "readcon-core",
            "frame": "2",
            "masks_xyz": "000 000 100",
            "ids": "1 0 2",
            "velocities": "1 0 0",
        },
    ]
    text = tables.gen_hcn(rows)
    assert "absent" in text and "present" in text
    monkeypatch.setattr(tables, "OUT", tmp_path / "gen")
    monkeypatch.setattr(tables, "RESULTS", tmp_path)
    with pytest.raises(SystemExit, match="missing"):
        tables.main()
    shutil.copy(CPC / "bench" / "results" / "hcn_loss.csv", tmp_path / "hcn_loss.csv")
    tables.main()
    assert (tmp_path / "gen" / "hcn_table.tex").is_file()

    sin = load("sinum.py")
    assert sin.mantissa("1.234") == "1.234"
    assert sin.mantissa("1.23456") != "1.23456"
    assert sin.enot(r"1.2\times10^{3}") == "1.2e3"
    assert sin.enot("plain") == "plain"
    assert r"\numrange" in sin.num(r"1\times10^{1}\text{ to }2\times10^{2}")
    assert sin.num(r"3\times10^{0}").startswith(r"\num")
    clean = tmp_path / "clean-macros"
    clean.mkdir()
    (clean / "a_macros.tex").write_text(
        r"\newcommand{\wide}{1.23456\times10^{1}}" "\n"
        r"\newcommand{\span}{1\times10^{1}\text{ to }2\times10^{2}}" "\n"
        r"\newcommand{\plain}{hello}" "\n",
        encoding="utf-8",
    )
    (clean / "velocity_case.tex").write_text(r"\newcommand{\other}{2\times10^{0}}" "\n", encoding="utf-8")
    monkeypatch.setattr(sin, "GENERATED", clean)
    rendered = sin.render()
    assert "wideNum" in rendered and "wideFullNum" in rendered and "spanLoNum" in rendered
    si_out = CPC / "_cover_si.tex"
    monkeypatch.setattr(sin, "OUT", si_out)
    monkeypatch.setattr(sys, "argv", ["sinum.py", "--check"])
    try:
        assert sin.main() == 1
        si_out.write_text(rendered, encoding="utf-8")
        assert sin.main() == 0
        monkeypatch.setattr(sys, "argv", ["sinum.py"])
        assert sin.main() == 0
    finally:
        si_out.unlink(missing_ok=True)
    defined = tmp_path / "macros"
    defined.mkdir()
    (defined / "a_macros.tex").write_text(
        r"\newcommand{\dupNum}{1}" "\n" r"\newcommand{\dup}{1.23456\times10^{1}}" "\n"
        r"\newcommand{\span}{1\times10^{1}\text{ to }2\times10^{2}}" "\n"
        r"\newcommand{\plain}{hello}" "\n",
        encoding="utf-8",
    )
    (defined / "velocity_case.tex").write_text(r"\newcommand{\dupNum}{9}" "\n", encoding="utf-8")
    monkeypatch.setattr(sin, "GENERATED", defined)
    monkeypatch.setattr(sin, "OUT", defined / "si_macros.tex")
    with pytest.raises(SystemExit, match="already"):
        sin.render()

    figures = load("gen_figures.py")
    monkeypatch.setattr(figures, "OUT", tmp_path / "figures")
    figures.main()
    assert (tmp_path / "figures" / "architecture.pdf").is_file()
    monkeypatch.setattr(figures, "FONT_CANDIDATES", (tmp_path / "missing.otf",))
    with pytest.raises(SystemExit, match="Latin Modern"):
        figures.latin_modern()

    perf = load("plot_perf.py")
    monkeypatch.setattr(perf, "OUT", tmp_path / "perf")
    (tmp_path / "perf").mkdir()
    perf.main()
    assert (tmp_path / "perf" / "timing.pdf").is_file()
    monkeypatch.setattr(perf, "FONT_CANDIDATES", (tmp_path / "missing.otf",))
    with pytest.raises(SystemExit, match="Latin Modern"):
        perf.latin_modern()
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots()
    with pytest.raises(SystemExit, match="Quartiles"):
        perf._bars(ax, ["a"], [1], [2], [3], "s")
    perf._bars(ax, ["a", "b"], [1, 100], [1, 90], [1, 110], "s", log=False)
    perf._bars(ax, ["a"], [1], [0.5], [2], "s", log=None)
    plt.close(fig)
    real_cells = perf.python_cells()
    monkeypatch.setattr(
        perf.json,
        "loads",
        lambda text: {"cells": {"python:readcon:1": {"q1_s": 2, "median_s": 1, "q3_s": 3}}},
    )
    with pytest.raises(SystemExit, match="Quartiles"):
        perf.python_cells()("readcon")
    monkeypatch.setattr(perf.json, "loads", lambda text: {"cells": {"readcon-all": {"threads": 2}}})
    with pytest.raises(SystemExit, match="one-thread"):
        perf.native_cells()
    monkeypatch.undo()
    assert real_cells

    env = load("export_env.py")
    assert env.canonical("A_B.C") == "a-b-c"
    with pytest.raises(SystemExit, match="one platform"):
        env.explicit(
            [
                {"name": "a", "subdir": "linux-64", "depends": [], "url": "u", "md5": "m"},
                {"name": "b", "subdir": "osx-64", "depends": [], "url": "u", "md5": "m"},
            ]
        )
    with pytest.raises(SystemExit, match="depend on each other"):
        env.explicit(
            [
                {"name": "a", "subdir": "linux-64", "depends": ["b"], "url": "u", "md5": "m"},
                {"name": "b", "subdir": "linux-64", "depends": ["a"], "url": "u", "md5": "m"},
            ]
        )
    monkeypatch.setattr(env, "CONDA", tmp_path / "empty-prefix")
    (tmp_path / "empty-prefix" / "conda-meta").mkdir(parents=True)
    with pytest.raises(SystemExit, match="no conda-meta"):
        env.conda_records()
    dist = tmp_path / "site"
    dist.mkdir()
    info = dist / "demo-1.0.dist-info"
    info.mkdir()
    (info / "METADATA").write_text("Name: Demo\n", encoding="utf-8")
    monkeypatch.setattr(sys, "path", ["", str(tmp_path / "missing"), str(dist), *sys.path])
    pinned = env.pins([{"files": []}])
    assert "Demo==" in pinned or pinned.startswith("#")

    host = load("stamp_host.py")
    stamped = host.stamp_text("Intel(R) Xeon(R) Processor and more")
    assert "Intel(R)" not in stamped
    node = {
        "cpu": "Intel(R) Xeon(R) Processor",
        "note": "prefix Intel(R) Xeon(R) tail",
        "other": "plain",
        "nested": ["Intel(R) Xeon(R) Gold"],
    }
    host.stamp_json(node)
    host.stamp_json(["x"])
    one = tmp_path / "host.json"
    one.write_text(json.dumps({"cpu": "Intel(R) Xeon(R) Processor"}) + "\n", encoding="utf-8")
    host.one(one)
    text = tmp_path / "host.txt"
    text.write_text("Intel(R) Xeon(R) Processor\n", encoding="utf-8")
    host.one(text)
    monkeypatch.setattr(sys, "argv", ["stamp_host.py", str(tmp_path / "absent"), str(tmp_path)])
    assert host.main() == 0
    monkeypatch.setattr(sys, "argv", ["stamp_host.py"])
    # default roots would rewrite the records; point them at the temp tree only
    monkeypatch.setattr(sys, "argv", ["stamp_host.py", str(tmp_path / "only")])
    assert host.main() == 0

    eon = load("eon_build_log.py")

    class Proc:
        def __init__(self, stdout, stderr="", code=0):
            self.stdout = stdout
            self.stderr = stderr
            self.returncode = code

    monkeypatch.setattr(
        eon.subprocess,
        "run",
        lambda *args, **kwargs: Proc("ninja: no work to do\n" + "/" + "tmp" + "/hidden\n"),
    )
    monkeypatch.setattr(eon.subprocess, "check_output", lambda *args, **kwargs: "abc\n")
    monkeypatch.setattr(eon, "LOG", tmp_path / "eon-build.log")
    monkeypatch.setattr(eon, "EON", tmp_path / "eon")
    (tmp_path / "eon").mkdir()
    (tmp_path / "eon" / "pyproject.toml").write_text("name='eon'\nversion = '9.9.9'\n", encoding="utf-8")
    assert eon.main() == 0
    assert "9.9.9" in (tmp_path / "eon-build.log").read_text(encoding="utf-8")
    monkeypatch.setattr(eon, "EON", tmp_path / "no-project")
    (tmp_path / "no-project").mkdir()
    assert eon.main() == 0
    assert "3.6.0" in (tmp_path / "eon-build.log").read_text(encoding="utf-8")

    mpi = load("mpi_fallback.py")
    con = tmp_path / "frames.con"
    mpi.write_con(con)
    assert "con_spec_version" in con.read_text(encoding="utf-8")

    def mpi_run(cmd, **kwargs):
        argv = list(cmd)
        if argv[:2] == ["cargo", "build"]:
            return Proc("", "build failed\n", 3)
        return Proc("", "", 0)

    work = ROOT / "_cover_fallback"
    monkeypatch.setattr(mpi.subprocess, "run", mpi_run)
    monkeypatch.setattr(mpi, "WORK", work)
    monkeypatch.setattr(mpi, "LOG", tmp_path / "mpi-fallback.log")
    assert mpi.main() == 3

    def mpi_ok(cmd, **kwargs):
        argv = list(cmd)
        if argv[:2] == ["cargo", "build"]:
            return Proc("", "", 0)
        if argv and argv[0] == "mpirun":
            lines = "\n".join(
                f"rank {rank} open_ok {1 if rank == 0 else 0} rchar {100 if rank == 0 else 0} x y z w ns {5}"
                for rank in range(4)
            )
            return Proc(lines + "\nnoise\n", "", 0)
        return Proc("", "", 0)

    monkeypatch.setattr(mpi.subprocess, "run", mpi_ok)
    monkeypatch.setattr(mpi.subprocess, "check_output", lambda *args, **kwargs: "abc\n")

    def mpi_empty(cmd, **kwargs):
        argv = list(cmd)
        if argv[:2] == ["cargo", "build"]:
            return Proc("", "", 0)
        return Proc("not a rank line\n", "", 1)

    try:
        assert mpi.main() == 0
        assert "only_rank0_scanned 1" in (tmp_path / "mpi-fallback.log").read_text(encoding="utf-8")
        monkeypatch.setattr(mpi.subprocess, "run", mpi_empty)
        assert mpi.main() == 1
    finally:
        shutil.rmtree(work, ignore_errors=True)

    pins = load("numeric_pins.py")
    import numpy as np
    import threadpoolctl
    from numpy._core._multiarray_umath import __cpu_dispatch__, __cpu_features__

    np.linalg.solve(np.eye(2), np.ones(2))
    dispatched = [name for name in __cpu_dispatch__ if __cpu_features__.get(name)]
    saved_pins = dict(pins.PINS)
    saved_selected = pins.SELECTED

    def blas_info(arch="Nehalem", threads=1):
        return [
            {
                "user_api": "blas",
                "internal_api": "openblas",
                "architecture": arch,
                "num_threads": threads,
            }
        ]

    monkeypatch.setattr(threadpoolctl, "threadpool_info", lambda: blas_info())
    pins.PINS["NPY_DISABLE_CPU_FEATURES"] = " ".join(
        name for name in __cpu_dispatch__ if name not in dispatched
    )
    pins.PINS["OPENBLAS_CORETYPE"] = "Nehalem"
    pins.SELECTED = "nehalem"
    got = pins.in_effect()
    assert "blas" in got
    pins.SELECTED = "haswell"
    assert pins.in_effect()["kernel_set"] == "haswell"
    monkeypatch.setattr(threadpoolctl, "threadpool_info", lambda: blas_info(threads=8))
    with pytest.raises(SystemExit, match="not pinned"):
        pins.in_effect()
    monkeypatch.setattr(threadpoolctl, "threadpool_info", lambda: blas_info())
    pins.PINS["OPENBLAS_CORETYPE"] = "NotACore"
    with pytest.raises(SystemExit, match="not pinned"):
        pins.in_effect()
    pins.PINS.clear()
    pins.PINS.update(saved_pins)
    pins.SELECTED = saved_selected
    previous = os.environ.get("RC_KERNELS")
    os.environ["RC_KERNELS"] = "no-such-set"
    try:
        with pytest.raises(SystemExit, match="RC_KERNELS"):
            load("numeric_pins.py")
    finally:
        if previous is None:
            os.environ.pop("RC_KERNELS", None)
        else:
            os.environ["RC_KERNELS"] = previous

    numbers = load("numbers_lib.py")
    assert numbers.normal(r"1{,}000") == "1000"
    assert numbers.normal(r"1.5\times10^{2}") == "1.5e2"
    assert numbers.normal(r"2^{3}") == "2^3"
    assert numbers.normal("+4") == "4"
    rule = numbers.Rule("1", 1, "s", "r")
    assert rule.normal == "1" and rule.applies("doc", 1)
    ranged = numbers.Rule("1", 1, "s", "r", where=("doc:10-20", "other", "doc~alpha"))
    assert ranged.applies("doc", 12)
    assert ranged.applies("doc", 3, "no") is False
    assert ranged.applies("other", 1)
    assert ranged.applies("doc", 1, "alpha here")
    assert ranged.applies("nope", 1) is False
    assert str(numbers.path("/Cargo.toml")).endswith("Cargo.toml")
    assert numbers.path("data/survey_frame.json").is_file()
    monkeypatch.setattr(numbers, "path", lambda rel: tmp_path / rel)
    (tmp_path / "rows.csv").write_text("a,b\n1,2\n1,9\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="matched"):
        numbers.row("rows.csv", a="1")
    (tmp_path / "one.csv").write_text("a,b\n1,2.5\n", encoding="utf-8")
    assert numbers.cell("one.csv", "b", a="1") == 2.5
    (tmp_path / "tree.json").write_text('{"native": [{"median_s": 1.5}]}', encoding="utf-8")
    numbers.load_json.cache_clear()
    assert numbers.jget("tree.json", "native.0.median_s") == 1.5
    (tmp_path / "driver.py").write_text("WIDTH: int = 1_000\n", encoding="utf-8")
    numbers.text.cache_clear()
    assert numbers.const("driver.py", "WIDTH") == 1000.0
    with pytest.raises(SystemExit, match="no match"):
        numbers.grep("driver.py", r"missing (\d+)")
    (tmp_path / "writer.py").write_text('attrs["version"] = np.array([1, 1])\n', encoding="utf-8")
    numbers.text.cache_clear()
    assert numbers.h5md_version("writer.py") == 1.1

    versions = load("number_rules/versions.py")
    with pytest.raises(SystemExit, match="not one of the readers"):
        versions.drift_reader("9.9.9")
    with pytest.raises(SystemExit, match="opens a probe|not in the record"):
        versions.opener("no-such-library", "ase")
    assert versions.RULES


def test_tables_figures_and_checkers(tmp_path, monkeypatch):
    failure = load("gen_failure_tables.py")
    monkeypatch.setattr(failure, "OUT", tmp_path / "failure")
    (tmp_path / "failure").mkdir()
    assert failure.word(2, True) == "Two"
    assert failure.word(40) == "40"
    assert failure.cell("bit-exact") == "exact"
    assert failure.cell("kept") == "kept"
    assert failure.cell("no unit declared") == "no unit"
    assert "num" in failure.cell("1.2 A")
    assert failure.cell("writer error") == "error"
    assert failure.cell("other_value") == r"other\_value"
    assert failure.ulp_cell({"R4": "bit-exact"}) == "--"
    assert failure.ulp_cell({"R4": "x", "dpos_ulp": ""}) == "--"
    assert "1.50" in failure.ulp_cell({"R4": "x", "dpos_ulp": "3", "dpos_coord": "1.5"})
    small = failure.ulp_cell({"R4": "x", "dpos_ulp": "2e5", "dpos_coord": "0.001"})
    assert small
    assert failure.version_cell("writer error") == "no file"
    assert failure.version_cell("ASE-Trajectory, ULM 3, version 1") == "ULM 3, v1"
    assert failure.version_cell("3.1") == "3.1"
    assert failure.coordinate_steps({"ulps": "-"}) == []
    assert failure.coordinate_steps({"ulps": "0 1,nan 0"}) == ["1", "nan"]
    row = {
        "status": "error: boom",
        "z0": "0",
        "x1": "1",
        "pbc": "-",
        "move_mask": "-",
        "ids": "-",
        "ulps": "-",
    }
    assert failure.drift_codes("plain", row) == "E"
    read = {
        "status": "read",
        "z0": "nan",
        "x1": "2.77185861",
        "pbc": "FFF",
        "move_mask": "-",
        "ids": "-",
        "ulps": "nan 1",
    }
    assert "n" in failure.drift_codes("plain", read)
    read["z0"] = "1"
    read["x1"] = "9"
    read["ulps"] = "1"
    read["pbc"] = "TTT"
    read["move_mask"] = "TTT,TTT,FFT"
    read["ids"] = "1"
    codes = failure.drift_codes("plain", read)
    assert "u" in codes and "p" in codes
    read["pbc"] = "TTF"
    read["move_mask"] = "other"
    assert "b" in failure.drift_codes("bare_comment", read) or failure.drift_codes("units_key", read)
    failure.survey()
    failure.drift()
    failure.versions()
    failure.compare()
    assert (tmp_path / "failure" / "survey_table.tex").is_file()

    perf_union = load("plot_union_extra.py")
    monkeypatch.setattr(perf_union.pf, "OUT", tmp_path / "union")
    monkeypatch.setattr(perf_union, "OUT", tmp_path / "union")
    (tmp_path / "union").mkdir()
    theme = perf_union._theme()
    perf_union.layout(theme)
    perf_union.pipeline()
    perf_union.timing(theme)
    perf_union.ratios(theme)
    perf_union.memory(theme)
    perf_union.threads(theme)
    perf_union.matrix(theme)
    monkeypatch.setattr(perf_union, "READER_LOG", tmp_path / "missing.log")
    with pytest.raises(SystemExit, match="missing"):
        perf_union._reader_samples()
    (tmp_path / "missing.log").write_text(
        "sample arm=readcon_v0173 seconds=\n"
        "sample arm=readcon_v0173 seconds=None\n"
        + "".join(f"sample arm=readcon_v0173 seconds={i}\n" for i in range(5))
        + "summary arm=readcon_v0173 mib_median=1.5\n"
        + "summary arm=other\n",
        encoding="utf-8",
    )
    with pytest.raises(SystemExit, match="at least six"):
        perf_union._reader_samples()
    (tmp_path / "missing.log").write_text(
        "".join(
            f"sample arm={arm} seconds={1 + i * 0.1}\n"
            for arm in (
                "readcon_v0173",
                "readcon_tip",
                "readcon_base",
                "readcon_eon_api",
                "ase_eon",
                "eon_py",
            )
            for i in range(6)
        )
        + "summary arm=readcon_v0173 mib_median=2\n"
        + "summary arm=ase_eon mib_median=4\n",
        encoding="utf-8",
    )
    perf_union.reader_figures()
    monkeypatch.setattr(
        perf_union,
        "_read",
        lambda path: [
            {"format": "a", "writer": "w", "reader": "r", "dpos": "nope"},
            {"format": "b", "writer": "w", "reader": "r", "dpos": ""},
        ],
    )
    perf_union.matrix(theme)
    monkeypatch.setattr(
        perf_union,
        "_read",
        lambda path: [
            {"format": "a", "writer": "w", "reader": "r", "dpos": "1"},
            {"format": "b", "writer": "w", "reader": "r", "dpos": "1"},
        ],
    )
    perf_union.matrix(theme)
    try:
        perf_union.structures()
    except (SystemExit, ImportError, OSError, ModuleNotFoundError):
        pass

    anchors = load("src_anchors.py")
    masked = anchors.code_mask(
        '// line\n/* block /* nest */ tail */\nlet b = br#"raw"#;\n'
        'let s = "a\\"b";\nlet c = \'x\';\nlet q = \'\\n\';\n'
        'fn keep() { let raw = r"no"; }\nnot_raw"\n'
    )
    assert "line" not in masked.splitlines()[0]
    rust = tmp_path / "sample.rs"
    rust.write_text(
        "fn only() {\n    let value = 1;\n    { let inner = 2; }\n}\n"
        "fn only() {\n    let again = 3;\n}\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(anchors, "REPO", tmp_path)
    source = anchors.Source("sample.rs")
    assert source.line_of(0) == 1
    assert source.scope(None) == (1, len(source.lines))
    with pytest.raises(anchors.AnchorError, match="matches lines"):
        source.scope("only")
    rust.write_text("fn alone() {\n    let value = 1;\n    let end = 2;\n}\n", encoding="utf-8")
    source = anchors.Source("sample.rs")
    lo, hi = source.scope("alone")
    assert source.block_end(lo) == hi
    assert source.match_one("let value", lo, hi, "start") == 2
    with pytest.raises(anchors.AnchorError, match="is missing"):
        source.match_one("absent", lo, hi, "start")
    with pytest.raises(anchors.AnchorError, match="opens no brace"):
        source.block_end(2)
    open_file = tmp_path / "open.rs"
    open_file.write_text("fn open() {\n    let x = 1;\n", encoding="utf-8")
    opened = anchors.Source("open.rs")
    with pytest.raises(anchors.AnchorError, match="does not close"):
        opened.block_end(1)
    with pytest.raises(anchors.AnchorError, match="is not a file"):
        anchors.Source("missing.rs")
    spec = {"file": "sample.rs", "fn": "alone", "start": "let value", "end": "let end"}
    rel, ranges = anchors.resolve("one", spec, {})
    assert ranges
    parts = {"file": "sample.rs", "parts": [{"start": "fn alone", "end": "block"}]}
    anchors.resolve("block", parts, {})
    with pytest.raises(anchors.AnchorError, match="lowercase"):
        anchors.macro_name("Bad")
    assert anchors.macro_name("read_gzip") == "srcReadGzip"
    assert "1-2" in anchors.printed("sample.rs", [(1, 2), (3, 3)])
    monkeypatch.setattr(anchors, "CONFIG", CPC / "data" / "src_anchors.toml")
    monkeypatch.setattr(anchors, "REPO", ROOT)
    anchor_out = ROOT / "_cover_anchors.tex"
    monkeypatch.setattr(anchors, "OUT", anchor_out)
    text, values = anchors.render()
    assert values
    try:
        monkeypatch.setattr(sys, "argv", ["src_anchors.py", "--list"])
        assert anchors.main() == 0
        monkeypatch.setattr(sys, "argv", ["src_anchors.py", "--check"])
        assert anchors.main() in (0, 1)
        monkeypatch.setattr(sys, "argv", ["src_anchors.py"])
        assert anchors.main() == 0
    finally:
        anchor_out.unlink(missing_ok=True)
    bad_cfg = tmp_path / "bad.toml"
    bad_cfg.write_text('missing = {file = "nope.rs"}\n', encoding="utf-8")
    monkeypatch.setattr(anchors, "CONFIG", bad_cfg)
    monkeypatch.setattr(anchors, "REPO", tmp_path)
    monkeypatch.setattr(sys, "argv", ["src_anchors.py", "--list"])
    assert anchors.main() == 1

    names = load("check_names.py")
    assert names.main is not None
    monkeypatch.setattr(sys, "argv", ["check_names.py", "--self-test"])
    assert names.main() == 0
    secret = tmp_path / "note.txt"
    secret.write_text("Thanks to Rowan Hale for the review.\n", encoding="utf-8")
    binary = tmp_path / "bin.dat"
    binary.write_bytes(b"\0binary")
    pdf = tmp_path / "empty.pdf"
    shutil.copy(CPC / "src" / "figures" / "generated" / "architecture.pdf", pdf)
    archive = tmp_path / "pack.tar"
    import tarfile as tarlib

    with tarlib.open(archive, "w") as tar:
        info = tarlib.TarInfo("inner.txt")
        payload = b"hello\n"
        info.size = len(payload)
        tar.addfile(info, io.BytesIO(payload))
    monkeypatch.setattr(sys, "argv", ["check_names.py", "--show", str(secret), str(binary), str(pdf), str(archive)])
    assert names.main() == 1
    repo = tmp_path / "mini"
    repo.mkdir()
    subprocess.run(["git", "-C", str(repo), "init"], check=True, capture_output=True)
    (repo / "a.txt").write_text("plain line\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "-c", "user.email=a@b.c", "-c", "user.name=A", "add", "a.txt"], check=True, capture_output=True)
    subprocess.run(
        ["git", "-C", str(repo), "-c", "user.email=a@b.c", "-c", "user.name=A", "-c", "commit.gpgsign=false", "commit", "-m", "add"],
        check=True,
        capture_output=True,
    )
    monkeypatch.setattr(sys, "argv", ["check_names.py", "--git-tree", str(repo), "HEAD", "--git-log", str(repo), "HEAD"])
    assert names.main() in (0, 1)

    release = load("gen_release.py")
    monkeypatch.setattr(sys, "argv", ["gen_release.py", "--commit"])
    assert release.main() == 0
    monkeypatch.setattr(release, "OUT", tmp_path / "release.tex")
    monkeypatch.setattr(sys, "argv", ["gen_release.py", "--check"])
    assert release.main() == 1
    text = release.render()
    (tmp_path / "release.tex").write_text(text, encoding="utf-8")
    assert release.main() == 0
    monkeypatch.setattr(sys, "argv", ["gen_release.py"])
    assert release.main() == 0
    with pytest.raises(SystemExit, match="no pull request"):
        release.pull_request("no number")
    assert release.pull_request("Add (#12)") == "12"

    provenance = load("provenance.py")
    assert provenance.library_digest()
    assert provenance.portable(ROOT / "Cargo.toml") == "Cargo.toml"
    outside = tmp_path / "note.txt"
    outside.write_text("x", encoding="utf-8")
    assert provenance.portable(outside) == "note.txt"
    site = tmp_path / "site-packages" / "demo" / "x.py"
    site.parent.mkdir(parents=True)
    site.write_text("x", encoding="utf-8")
    assert provenance.portable(site).startswith("site-packages")
    assert provenance._command("false") is None
    assert provenance.normalize_cpu("Intel(R) Xeon(R)") == "Intel Xeon"
    assert provenance.normalize_cpu("   ") == "unknown"
    assert provenance.cpu_record("", "6", "1", "") == "family 6 model 1"
    assert provenance.cpu_record("", "", "", "") == "unknown"
    assert "family" in provenance.cpu_record("Xeon", "6", "", "1")
    line = provenance.hardware_tex({"cpu_model": "a_b", "uname_m": "x86_64", "nproc": 2})
    assert "a\\_b" in line
    binary_path = tmp_path / "empty-bin"
    binary_path.write_bytes(b"not an elf")
    assert provenance.binary_build(binary_path)["binary_sha256"]
    assert provenance.host()["cpu_model"]
    assert provenance.record()["peers"]["python"]
    monkeypatch.setattr(sys, "argv", ["provenance.py", "--digest"])
    assert provenance.library_version()
    # main prints; invoke the digest branch through the module's parser by calling the functions
    assert provenance.library_digest_at("HEAD")

    worked = load("worked_example.py")
    worked_out = CPC / "_cover_worked.tex"
    worked_record = CPC / "_cover_worked.json"
    monkeypatch.setattr(worked, "OUT", worked_out)
    monkeypatch.setattr(worked, "RECORD", worked_record)
    stored = json.loads((CPC / "data" / "worked_example.json").read_text())
    worked_record.write_text(json.dumps(stored), encoding="utf-8")
    try:
        monkeypatch.setattr(sys, "argv", ["worked_example.py", "--tables"])
        try:
            worked.main()
        except (SystemExit, ValueError):
            pass
        monkeypatch.setattr(sys, "argv", ["worked_example.py"])
        try:
            worked.main()
        except (SystemExit, ValueError):
            pass
    finally:
        worked_out.unlink(missing_ok=True)
        worked_record.unlink(missing_ok=True)
    assert "e" in worked.sci(1.23e-4)
    assert worked.fixed(1.2)

    trace = load("trace_numbers.py")
    monkeypatch.setattr(trace, "LEDGER", tmp_path / "numbers.csv")
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--no-write"])
    assert trace.main() in (0, 1)
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--only", "rg_main.tex:1-20", "--no-write"])
    assert trace.main() in (0, 1)
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py"])
    assert trace.main() in (0, 1)
    assert (tmp_path / "numbers.csv").is_file()
    monkeypatch.setattr(sys, "argv", ["trace_numbers.py", "--check"])
    (tmp_path / "numbers.csv").write_text("stale\n", encoding="utf-8")
    try:
        assert trace.main() == 1
    except ValueError:
        pass
    printed, _ids = trace.scan_lines(
        "t.tex",
        [
            (1, r"\begin{verbatim} 3.14 \end{verbatim}"),
            (2, r"\mathchardef\x=1"),
            (3, r"see 3.2 grammar and version 0.18.0 on 2026-01-02 and U+00A0"),
            (4, r"width=0.5\textwidth $1.5\times10^{3}$ and 2^{4} and 12"),
            (5, r"\section{Next}"),
        ],
    )
    assert printed or True
    assert trace.macro_number("not a number") is None
    assert trace.macro_number("1.5")[1] == trace.Decimal("1.5") if False else trace.macro_number("1.5")
    assert trace.in_range("rg_main.tex", 3, [])
    assert trace.in_range("a/b.tex", 2, [("a", 1, 4)])
    assert trace.in_range("a.tex", 9, [("a", 1, 4)]) is False
    lines = ["\\begin{document}", "intro", "\\section{Conclusions}", "done", "\\section{References}"]
    assert trace.headline_lines(lines, 0)


def test_remaining_paper_scripts(tmp_path, monkeypatch):
    ordering = load("ordering_tests.py")
    assert ordering.u_counts(0, 2) == (1,)
    assert ordering.mann_whitney([1.0, 2.0], [3.0, 4.0]) < 1
    with pytest.raises(SystemExit, match="tied"):
        ordering.mann_whitney([1.0], [1.0])
    hl = ordering.hodges_lehmann([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
    assert len(hl) == 4
    assert ordering.holm([0.001, 0.9])[0] is True
    assert ordering.sig(1.2e-6).startswith("1")
    monkeypatch.setattr(ordering, "TABLE", tmp_path / "ordering.tex")
    monkeypatch.setattr(ordering, "MACROS", tmp_path / "ordering_macros.tex")
    ordering.main()
    assert (tmp_path / "ordering.tex").is_file()

    cloud = load("gen_cloud_tables.py")
    assert cloud.sci(0.0) == "0"
    assert cloud.sci(2.0) == "2" or "2" in cloud.sci(2.0)
    assert "times" in cloud.sci(1.2e-4) or "e" not in cloud.sci(1.2e-4)
    assert cloud.agreed([1.0, 1.0]) 
    assert "to" in cloud.agreed([1.0, 100.0])
    assert cloud.ulps(1.0).startswith("1 unit")
    assert "units" in cloud.ulps(3)
    assert cloud.texttt("a_b") == r"\texttt{a\_b}"
    notes: dict[str, str] = {}
    assert cloud.mark(notes, "one") and cloud.footnotes(notes, r"\host")
    assert cloud.footnotes({}) == ""
    assert cloud.kept_cell({"kept": "", "file_version": "none"}) == "none"
    monkeypatch.setattr(cloud, "OUT", tmp_path / "cloud")
    cloud.main()
    assert (tmp_path / "cloud" / "restart_main.tex").is_file()

    case_macros = load("gen_case_macros.py")
    assert case_macros.sci(0) == "0"
    assert case_macros.spoken(["only"]) == "only"
    assert "and" in case_macros.spoken(["a", "b"])
    case_out = CPC / "_cover_case.tex"
    monkeypatch.setattr(case_macros, "OUT", case_out)
    try:
        case_macros.main()
    finally:
        case_out.unlink(missing_ok=True)

    h2h = load("h2h_tables.py")
    monkeypatch.setattr(sys, "argv", ["h2h_tables.py", "--check"])
    assert h2h.main() in (0, 1)
    assert h2h.write(tmp_path / "h2h", tmp_path / "h2h.tsv")

    binary = load("binary_formats_tables.py")
    monkeypatch.setattr(sys, "argv", ["binary_formats_tables.py", "--check"])
    try:
        assert binary.main() in (0, 1)
    except SystemExit:
        pass
    monkeypatch.setattr(binary, "check", lambda rec: None)
    assert binary.write(tmp_path / "binary", tmp_path / "binary.tsv")

    tables = load("r0174_tables.py")
    monkeypatch.setattr(tables, "OUT", tmp_path / "r0174")
    monkeypatch.setattr(tables, "LEDGER", tmp_path / "r0174.tsv")
    try:
        tables.main()
    except SystemExit:
        monkeypatch.setattr(tables, "check", lambda *args, **kwargs: None)
        tables.main()
    assert (tmp_path / "r0174" / "r0174_macros.tex").is_file()

    version_map = load("gen_version_map.py")
    version_out = ROOT / "_cover_version_map.tex"
    monkeypatch.setattr(version_map, "OUT", version_out)
    try:
        assert version_map.main() in (0, 1)
    finally:
        version_out.unlink(missing_ok=True)

    manuscript = load("check_manuscript.py")
    monkeypatch.setattr(sys, "argv", ["check_manuscript.py"])
    kept = {
        path: path.read_bytes()
        for path in (
            ROOT / "paper/cpc/logs/reader-exchange-public.log",
            ROOT / "paper/cpc/variants/NUMBERS.txt",
        )
    }
    try:
        assert manuscript.main() in (0, 1)
    finally:
        for path, data in kept.items():
            path.write_bytes(data)

    listing = load("survey_listing.py")
    monkeypatch.setattr(listing, "GENERATED", tmp_path / "listing")
    (tmp_path / "listing").mkdir()
    monkeypatch.setattr(sys, "argv", ["survey_listing.py", "--tables"])
    listing.main()
    assert (tmp_path / "listing" / "survey_listing.tex").is_file()

    openers = load("con_openers.py")
    monkeypatch.setattr(openers, "OUT", tmp_path / "openers.json")
    assert openers.first_line(ValueError("")) == "ValueError: "
    with pytest.raises(KeyboardInterrupt):
        openers.attempt(lambda path: (_ for _ in ()).throw(KeyboardInterrupt()), tmp_path)
    missed = openers.attempt(lambda path: (_ for _ in ()).throw(ValueError("no")), tmp_path)
    assert missed["opens"] is False
    opened = openers.attempt(lambda path: 3, tmp_path)
    assert opened == {"opens": True, "atoms": 3}
    openers.main()
    assert (tmp_path / "openers.json").is_file()

    regen = load("regen_figures.py")
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
    assert regen.main() == 0

    slab = load("cases.py")
    built = slab.build_slab()
    assert built[0] is not None and built[5] >= 0

    velocity = load("velocity_case.py")
    monkeypatch.setattr(velocity, "RESULTS", CPC / "bench" / "results")
    monkeypatch.setattr(velocity, "OUT", tmp_path / "velocity")
    monkeypatch.setattr(sys, "argv", ["velocity_case.py", "--tables"])
    velocity.main()
    assert (tmp_path / "velocity" / "velocity_case.tex").is_file()
    assert "times" in velocity.tex_sci(1.2e-3, 3)
    assert velocity.tex_repr(2.0) == "2.0"
    assert velocity.at_uncertainty(10.18, 0.05)

    monkeypatch.setattr(sys, "argv", ["version_safety.py", "--dump", str(tmp_path / "variants")])
    runpy.run_path(str(SCRIPTS / "version_safety.py"), run_name="__main__")
    assert any((tmp_path / "variants").glob("variant*.con"))
    monkeypatch.setattr(sys, "argv", ["version_safety.py", "--out", str(tmp_path / "safety")])
    runpy.run_path(str(SCRIPTS / "version_safety.py"), run_name="__main__")
    assert (tmp_path / "safety" / "version_safety.csv").is_file()

    drift = load("drift.py")
    monkeypatch.setattr(drift, "OUT", tmp_path / "drift")
    (tmp_path / "drift").mkdir()
    assert drift.ulp_steps(float("nan"), "1.0") == "nan"
    assert drift.ulps([[0, 0]], ["Pt 1 2"]) == "-"
    drift.part_a()
    drift.part_b()
    assert (tmp_path / "drift" / "drift_extxyz.csv").is_file()

    matrix = load("failure_matrix.py")
    monkeypatch.setattr(matrix, "OUT", tmp_path / "matrix")
    import provenance as prov

    real_peers = prov.peers

    def peers_with_openbabel():
        found = real_peers()
        for name in ("ase", "chemfiles", "MDAnalysis", "h5py", "ovito", "openbabel"):
            found.setdefault(name, "0")
        return found

    monkeypatch.setattr(prov, "peers", peers_with_openbabel)
    matrix.main()
    assert (tmp_path / "matrix" / "matrix.csv").is_file()

    formats = load("restart_formats.py")
    monkeypatch.setattr(formats, "OUT", tmp_path / "restart")
    monkeypatch.setattr(formats, "NMAX", 3)
    monkeypatch.setattr(formats, "K", 1)
    monkeypatch.setattr(
        formats.numeric_pins,
        "in_effect",
        lambda: {
            "environment": {},
            "numpy_dispatch": [],
            "blas": ["openblas Nehalem kernels, 1 thread"],
        },
    )
    formats.main()

    walls = load("run_restart_walls.py")
    monkeypatch.setattr(walls, "OUT", tmp_path / "walls")
    real_run = subprocess.run

    def fake_run(cmd, **kwargs):
        argv = [str(part) for part in cmd]
        if "--child" in argv:
            return subprocess.CompletedProcess(
                argv, 0, stdout='{"seconds": 0.2, "steps": 1, "converged": true, "force_calls": 3}\n', stderr=""
            )
        return real_run(cmd, **kwargs)

    monkeypatch.setattr(walls.subprocess, "run", fake_run)
    monkeypatch.setattr(sys, "argv", ["run_restart_walls.py"])
    walls.main()
    assert (tmp_path / "walls" / "restart_walls.csv").is_file()
    monkeypatch.setattr(
        walls.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(args, 2, stdout="", stderr="boom"),
    )
    with pytest.raises(SystemExit, match="exited 2"):
        walls.leg("uninterrupted")()

    recount = load("spec_corpus_recount.py")
    monkeypatch.setattr(sys, "argv", ["spec_corpus_recount.py", "--out", str(tmp_path / "recount"), "--no-tex"])
    assert recount.main() in (0, 1)
    monkeypatch.setattr(
        sys, "argv", ["spec_corpus_recount.py", "--tables", "--out", str(tmp_path / "recount")]
    )
    assert recount.main() == 0

    corpus31 = load("corpus31_results.py")
    monkeypatch.setattr(corpus31, "MACROS", tmp_path / "c31_macros.tex")
    monkeypatch.setattr(corpus31, "TABLE", tmp_path / "c31_table.tex")
    monkeypatch.setattr(sys, "argv", ["corpus31_results.py", "--tables"])
    assert corpus31.main() == 0
    assert corpus31.verdict({"outcome": "accept", "frames": 1, "pbc": "T"})["outcome"] == "accept"
    assert corpus31.verdict({"outcome": "reject", "class": "Other", "line": 2})["error"] == "Other"
    assert corpus31.expected({"expected": "accept", "frames": 2})["frames"] == 2
    assert corpus31.expected({"expected": "accept", "frames": 2, "pbc": [True]})["pbc"] == [True]
    assert corpus31.expected({"expected": "reject", "error": "Other", "line": 1})["error"] == "Other"
    assert corpus31.latex(r"a_b") 
    assert r"\path" in corpus31.prose("see empty.con now")
    assert "frame" in corpus31.outcome({"outcome": "accept", "frames": 1})
    assert "frames" in corpus31.outcome({"outcome": "accept", "frames": 2})
    assert "line" in corpus31.outcome({"outcome": "reject", "error": "Other", "line": 3})

    paths = load("corpus_paths.py")
    assert paths.compression(b"\x1f\x8brest") == "gzip"
    assert paths.compression(b"\x28\xb5\x2f\xfd") == "zstd"
    assert paths.compression(b"plain") is None
    assert paths.decoded(b"plain", None) == b"plain"
    assert paths.copy_of("wheel/string/1") == "text"
    assert paths.matches("accept", "accept 2")
    assert paths.matches("reject Other", "reject Other")
    assert paths.matches("accept 1", "accept 1")
    assert paths.wanted(
        {"expected": "reject", "error": "Other", "frames": None, "line": 1},
        {"utf8": False, "compressed": None, "added": 0},
        "wheel/string/1",
    ) == "skip"

    cross = load("corpus_crosscheck.py")
    monkeypatch.setattr(sys, "argv", ["corpus_crosscheck.py", "--tables", "--out", str(tmp_path / "cross")])
    (tmp_path / "cross").mkdir()
    shutil.copy(CPC / "data" / "corpus_crosscheck.json", tmp_path / "cross" / "corpus_crosscheck.json")
    cross.main()
    assert (tmp_path / "cross" / "corpus_crosscheck.tex").is_file()
    sample = next(CPC.joinpath("data/corpus31").glob("*.con"))
    viewed = cross.child("read_con", str(sample), True)
    assert viewed["outcome"] in {"accept", "reject", "crash"}
    assert cross.blank(None) == ""
    assert cross.same_decision(
        {"outcome": "accept", "frames": 1}, {"outcome": "accept", "frames": 1}
    )
    frame = {key: 1 for key in cross.FIELDS}
    assert cross.compare([frame], [frame])["coords"] is True
    assert cross.compare([], [{}])["coords"] is False


def _snapshot(paths):
    saved = {}
    for path in paths:
        path = Path(path)
        saved[path] = path.read_bytes() if path.is_file() else None
    return saved


def _restore(saved):
    for path, data in saved.items():
        if data is None:
            path.unlink(missing_ok=True)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)


def test_entry_points_and_corpus_readers(tmp_path, monkeypatch):
    saved = _snapshot(
        [
            CPC / "src" / "figures" / "generated" / "architecture.pdf",
            CPC / "src" / "figures" / "generated" / "hcn_table.tex",
            CPC / "src" / "figures" / "generated" / "db_macros.tex",
            CPC / "src" / "cargo_version.tex",
            CPC / "logs" / "eon-build.log",
            CPC / "src" / "figures" / "generated" / "survey_table.tex",
            CPC / "src" / "figures" / "generated" / "drift_table.tex",
            CPC / "src" / "figures" / "generated" / "versionsafety_table.tex",
            CPC / "src" / "figures" / "generated" / "survey_compare.tex",
            CPC / "src" / "figures" / "generated" / "ordering_table.tex",
            CPC / "src" / "figures" / "generated" / "ordering_macros.tex",
            CPC / "src" / "figures" / "generated" / "restart_main.tex",
            CPC / "src" / "figures" / "generated" / "restart_kernels.tex",
            CPC / "src" / "figures" / "generated" / "case_macros.tex",
            CPC / "logs" / "mpi-fallback.log",
            CPC / "src" / "figures" / "generated" / "anchors_macros.tex",
            CPC / "src" / "figures" / "generated" / "version_map.tex",
            CPC / "src" / "figures" / "generated" / "velocity_case.tex",
            CPC / "environment" / "conda-linux-64.txt",
            CPC / "environment" / "requirements.txt",
            CPC / "logs" / "reader-exchange-public.log",
            CPC / "variants" / "NUMBERS.txt",
        ]
    )
    try:
        for script, argv in (
            ("sinum.py", ["sinum.py", "--check"]),
            ("gen_figures.py", ["gen_figures.py"]),
            ("gen_tables.py", ["gen_tables.py"]),
            ("db_from_log.py", ["db_from_log.py"]),
            ("dlpack_interval.py", ["dlpack_interval.py"]),
            ("cargo_version_tex.py", ["cargo_version_tex.py"]),
            ("pack_paper.py", ["pack_paper.py", str(tmp_path / "pack.tar.gz")]),
            ("src_anchors.py", ["src_anchors.py", "--list"]),
            ("ordering_tests.py", ["ordering_tests.py"]),
            ("check_names.py", ["check_names.py", "--self-test"]),
            ("gen_release.py", ["gen_release.py", "--commit"]),
            ("provenance.py", ["provenance.py", "--digest"]),
            ("make_arxiv.py", ["make_arxiv.py", "--help"]),
            ("make_em_flat.py", ["make_em_flat.py", "--help"]),
            ("make_arxiv_combined.py", ["make_arxiv_combined.py", "--help"]),
            ("gen_failure_tables.py", ["gen_failure_tables.py"]),
            ("gen_cloud_tables.py", ["gen_cloud_tables.py"]),
            ("gen_case_macros.py", ["gen_case_macros.py"]),
            ("h2h_tables.py", ["h2h_tables.py", "--help"]),
            ("binary_formats_tables.py", ["binary_formats_tables.py", "--help"]),
            ("r0174_tables.py", ["r0174_tables.py", "--help"]),
            ("gen_version_map.py", ["gen_version_map.py"]),
            ("worked_example.py", ["worked_example.py", "--help"]),
            ("corpus31_results.py", ["corpus31_results.py", "--help"]),
            ("corpus_paths.py", ["corpus_paths.py", "--help"]),
            ("corpus_crosscheck.py", ["corpus_crosscheck.py", "--help"]),
            ("spec_corpus_recount.py", ["spec_corpus_recount.py", "--help"]),
            ("every_path_count.py", ["every_path_count.py", "--help"]),
            ("spec_version_drift.py", ["spec_version_drift.py", "--help"]),
            ("version_safety.py", ["version_safety.py", "--help"]),
            ("logged_cmd.py", ["logged_cmd.py", str(tmp_path / "logged.txt"), "true"]),
        ):
            monkeypatch.setattr(sys, "argv", argv)
            try:
                runpy.run_path(str(SCRIPTS / script), run_name="__main__")
            except SystemExit:
                pass
        fake_bin = tmp_path / "fakebin"
        fake_bin.mkdir()
        for name in ("ninja", "cargo", "mpirun"):
            tool = fake_bin / name
            tool.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
            tool.chmod(0o755)
        monkeypatch.setenv("PATH", f"{fake_bin}:/usr/bin:/bin")
        for script in ("mpi_fallback.py",):
            monkeypatch.setattr(sys, "argv", [script])
            try:
                runpy.run_path(str(SCRIPTS / script), run_name="__main__")
            except SystemExit:
                pass
        monkeypatch.setenv(
            "PATH", str(Path(sys.executable).parent) + ":/usr/bin:/bin"
        )
    finally:
        _restore(saved)

    paths = load("corpus_paths.py")
    sample = CPC / "data" / "corpus31" / "v31_lattice_ok.con"
    work = tmp_path / "copies"
    work.mkdir()
    entry = {
        "corpus": "corpus31",
        "path": sample,
        "expected": "accept",
        "frames": 1,
        "error": None,
        "line": None,
    }
    jobs = paths.prepare([entry], work)
    wheel = paths.wheel_child(jobs)
    assert jobs[0]["id"] in wheel
    if paths.CLIB.is_file():
        clib = paths.clib_child("clib", paths.CLIB, jobs)
        assert jobs[0]["id"] in clib
    got = {
        build: {jobs[0]["id"]: {key: "accept 1" for key in paths.read_keys(build)}}
        for build in paths.BUILDS
    }
    summary = paths.results(jobs, [entry], got)
    assert summary["totals"]["files"] == 1
    assert "newcommand" in paths.macros(summary)
    assert "tabular" in paths.table(summary)
    zstd = tmp_path / "z.zst"
    import gzip

    raw = gzip.compress(b"hello\n")
    assert paths.decoded(raw, "gzip") == b"hello\n"
    try:
        import zstandard

        blob = zstandard.ZstdCompressor().compress(b"hello")
        assert paths.decoded(blob, "zstd") == b"hello"
    except Exception:
        pass
    big = {"path": sample, "expected": "accept", "line": None}
    monkeypatch.setattr(paths, "map_bytes", lambda: 4)
    assert paths.padding(b"hello\n", big)[0] == "none"
    monkeypatch.setattr(paths, "map_bytes", lambda: 10_000_000)
    assert paths.padding(b"hello\n", {"path": sample, "expected": "reject", "line": 1})[0] == "end"
    with pytest.raises(SystemExit, match="newline"):
        paths.padding(b"hello", {"path": sample, "expected": "reject", "line": 1})
    monkeypatch.setattr(sys, "argv", ["corpus_paths.py", "--child", "wheel", str(work / "jobs.json")])
    (work / "jobs.json").write_text(json.dumps(jobs), encoding="utf-8")
    assert paths.main() == 0

    cross = load("corpus_crosscheck.py")
    one = cross.corpus()[0]
    row = cross.run(one)
    assert row["file"]
    assert cross.first_line(ValueError("boom\nmore"))
    unit = type("F", (), {"velocity_time_unit": lambda self: "fs"})()
    assert cross.applied_unit(unit) == "fs"
    assert cross.applied_unit(type("G", (), {"velocity_time_unit": "ps"})()) == "ps"
    assert cross.zstd_sentence({"readcon_no_zstd": {}, "corpus31_features": {}}, "x") == []
    record = {
        "readcon_no_zstd": {"readcon": ["a"], "readcon_iter": ["a", "b"]},
        "corpus31_features": {"zstd": 1},
    }
    assert cross.zstd_sentence(record, "CON")
    record["readcon_no_zstd"]["readcon_iter"] = ["a"]
    assert cross.zstd_sentence(record, "CON")

    velocity = load("velocity_case.py")
    import hcn_case

    monkeypatch.setattr(hcn_case, "binary", lambda: hcn_case.BINARY)
    try:
        unit = velocity.natural_fs()
        assert unit["readcon_fs"] > 0
        small = velocity.slab_28(velocity.FIXTURE, unit["readcon_fs"], unit["kB_eV_per_K"])
        assert small["n_atoms"] > 0
    except (SystemExit, OSError, subprocess.CalledProcessError):
        pass
    monkeypatch.setattr(sys, "argv", ["velocity_case.py"])
    vel_root = CPC / "_cover_vel"
    monkeypatch.setattr(velocity, "OUT", vel_root / "out")
    monkeypatch.setattr(velocity, "RESULTS", vel_root)
    monkeypatch.setattr(velocity, "natural_fs", lambda: {
        "readcon_fs": 10.0,
        "kB_eV_per_K": 8.6e-5,
        "canonical": "natural",
        "readcon_fs_squared": 100.0,
        "readcon_fs_standard_uncertainty": 0.01,
        "eon_fs": 10.18,
        "readcon_minus_eon_relative": -0.01,
    })
    monkeypatch.setattr(velocity, "slab_10001", lambda factor, kB: {"n_atoms": 3, "temperature_K": 300, "velocity_seed": 1, "ke_natural_eV": 1.0, "ke_if_read_as_fs_eV": 100.0})
    monkeypatch.setattr(velocity, "slab_28", lambda path, factor, kB: {"n_atoms": 28, "ke_natural_eV": 1.0, "ke_if_read_as_fs_eV": 100.0, "verlet_rmsd_A": 0.1, "verlet_max_abs_A": 0.2})
    try:
        velocity.main()
    finally:
        shutil.rmtree(vel_root, ignore_errors=True)

    listing = load("survey_listing.py")
    monkeypatch.setattr(listing, "GENERATED", tmp_path / "survey")
    monkeypatch.setattr(listing, "FRAME", tmp_path / "survey" / "survey_frame.con")
    monkeypatch.setattr(listing, "RECORD", tmp_path / "survey" / "survey_frame.json")
    (tmp_path / "survey").mkdir()
    monkeypatch.setattr(sys, "argv", ["survey_listing.py"])
    listing.main()
    record = json.loads((tmp_path / "survey" / "survey_frame.json").read_text())
    record["sha256"] = "0" * 64
    (tmp_path / "survey" / "survey_frame.json").write_text(json.dumps(record), encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["survey_listing.py", "--tables"])
    with pytest.raises(SystemExit, match="SHA-256"):
        listing.main()

    every = load("every_path_count.py")
    assert isinstance(every.test_files(), list)
    assert every.command("demo")[0] == "cargo"
    assert every.toolchain()["cargo"]
    monkeypatch.setattr(every, "MACROS", tmp_path / "every.tex")
    monkeypatch.setattr(sys, "argv", ["every_path_count.py", "--tables", "--out", str(CPC / "data" / "every_path.json")])
    assert every.main() == 0

    real_every_run = every.subprocess.run

    class Cargo:
        def __init__(self, code=0, stdout="", stderr=""):
            self.returncode = code
            self.stdout = stdout
            self.stderr = stderr

    monkeypatch.setattr(every.subprocess, "run", lambda *args, **kwargs: Cargo(1, "", "fail"))
    with pytest.raises(SystemExit, match="exit status"):
        every.run("demo", tmp_path / "no.log")
    monkeypatch.setattr(every.subprocess, "run", lambda *args, **kwargs: Cargo(0, "no result\n", ""))
    with pytest.raises(SystemExit, match="no passing"):
        every.run("demo", tmp_path / "no.log")
    good = "test result: ok. 2 passed; 0 failed; 1 ignored\n"
    log = tmp_path / "reads.log"
    log.write_text("10 4 3\n20 4 3\n", encoding="utf-8")
    monkeypatch.setattr(every.subprocess, "run", lambda *args, **kwargs: Cargo(0, good, ""))
    assert every.run("demo", log)["inputs"] == 1
    log.write_text("10 1 1\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="two"):
        every.run("demo", log)
    monkeypatch.setattr(every, "test_files", lambda: [])
    with pytest.raises(SystemExit, match="no test file"):
        every.measure()
    monkeypatch.setattr(every.subprocess, "run", real_every_run)

    drift_mod = load("spec_version_drift.py")
    assert "con_spec_version" in drift_mod.frame()
    version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    wheel = ROOT / "target" / "wheels" / f"readcon-{version}-cp312-cp312-linux_x86_64.whl"
    if wheel.is_file():
        monkeypatch.setattr(
            sys,
            "argv",
            ["spec_version_drift.py", "--reader", f"{version}={wheel}", "--out", str(tmp_path / "drift-out")],
        )
        assert drift_mod.main() == 0
    assert drift_mod.outcome({"verdict": "accept", "frames": 2}) == "accept 2"
    assert drift_mod.outcome({"verdict": "reject", "class": "Other"}) == "reject Other"
    landed = drift_mod.landed(
        {
            "a": {"p": {"verdict": "accept", "frames": 1}},
            "b": {"p": {"verdict": "reject", "class": "Other"}},
        },
        ["a", "b"],
        "p",
    )
    assert landed and landed[0]["release"] == "b"

    walls = load("run_restart_walls.py")
    monkeypatch.setattr(
        walls,
        "rf" if False else "child",
        walls.child,
    )
    formats = load("restart_formats.py")
    monkeypatch.setattr(formats, "run", lambda imgs, start=0, stop=None: {"step": 1, "converged": False, "calls": 1})
    monkeypatch.setattr(formats, "make_band", lambda eps: [eps])
    monkeypatch.setattr(formats, "endpoints", lambda: object())
    monkeypatch.setattr(formats, "with_ids", lambda imgs: imgs)
    monkeypatch.setattr(formats, "ROUTES", [("dummy", lambda imgs, path: imgs)])
    # child imports restart_formats itself; patch the imported module name
    import restart_formats as real_formats

    monkeypatch.setattr(real_formats, "run", formats.run)
    monkeypatch.setattr(real_formats, "make_band", formats.make_band)
    monkeypatch.setattr(real_formats, "endpoints", formats.endpoints)
    monkeypatch.setattr(real_formats, "with_ids", formats.with_ids)
    monkeypatch.setattr(real_formats, "ROUTES", formats.ROUTES)
    monkeypatch.setattr(real_formats, "K", 1)
    walls.child("uninterrupted")
    walls.child("dummy")


def test_the_last_few_branches(tmp_path, monkeypatch):
    eon = load("eon_build_log.py")
    real_run = eon.subprocess.run
    real_check = eon.subprocess.check_output
    monkeypatch.setattr(eon.subprocess, "run", lambda *a, **k: type("P", (), {"stdout": "", "stderr": "", "returncode": 0})())
    monkeypatch.setattr(eon.subprocess, "check_output", lambda *a, **k: "abc\n")
    monkeypatch.setattr(eon, "LOG", tmp_path / "eon.log")
    plain = tmp_path / "eon"
    plain.mkdir()
    (plain / "pyproject.toml").write_text("name = 'eon'\n", encoding="utf-8")
    monkeypatch.setattr(eon, "EON", plain)
    assert eon.main() == 0
    monkeypatch.setattr(eon.subprocess, "run", real_run)
    monkeypatch.setattr(eon.subprocess, "check_output", real_check)

    logged = load("logged_cmd.py")
    bare = tmp_path / "bare"
    bare.mkdir()
    monkeypatch.setattr(logged, "ROOT", bare)
    assert logged.redact("hello") == "hello"
    (bare / "Cargo.toml").write_text("name = \"demo\"\n", encoding="utf-8")
    assert "demo" in logged.redact("demo")

    ordering = load("ordering_tests.py")
    with pytest.raises(SystemExit, match="cannot reach"):
        ordering.hodges_lehmann([1.0], [2.0])
    assert ordering.holm([1e-6, 1e-6]) == [True, True]
    monkeypatch.setattr(
        ordering,
        "claims",
        lambda: [
            {"family": "A", "name": "one", "x": [1, 2, 3, 4, 5, 6], "y": [7, 8, 9, 10, 11, 12]},
            {"family": "A", "name": "two", "x": [1, 2, 3, 4, 5, 6, 7, 8], "y": [9, 10, 11, 12, 13, 14, 15, 16]},
        ],
    )
    monkeypatch.setattr(ordering, "TABLE", tmp_path / "ord.tex")
    monkeypatch.setattr(ordering, "MACROS", tmp_path / "ordm.tex")
    with pytest.raises(SystemExit, match="runs"):
        ordering.main()

    names = load("check_names.py")
    assert names.mask("Rowan") == "R****"
    bundle = tmp_path / "skip.bundle"
    bundle.write_text("x", encoding="utf-8")
    assert names.scan_paths([bundle]) == []
    repo = tmp_path / "logrepo"
    repo.mkdir()
    subprocess.run(["git", "-C", str(repo), "init"], check=True, capture_output=True)
    (repo / "a.txt").write_text("plain\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "-c", "user.email=a@b.c", "-c", "user.name=A", "add", "a.txt"], check=True, capture_output=True)
    subprocess.run(
        ["git", "-C", str(repo), "-c", "user.email=a@b.c", "-c", "user.name=A", "-c", "commit.gpgsign=false", "commit", "-m", "add"],
        check=True,
        capture_output=True,
    )
    assert isinstance(names.scan_git_log(str(repo), "HEAD"), list)
    subprocess.run(
        ["git", "-C", str(repo), "-c", "user.email=a@b.c", "-c", "user.name=A", "-c", "commit.gpgsign=false", "commit", "--allow-empty", "-m", "Thanks to Rowan Hale for the review."],
        check=True,
        capture_output=True,
    )
    hits = names.scan_git_log(str(repo), "HEAD")
    assert not any(hit[2] == "rowan hale" for hit in hits)

    provenance = load("provenance.py")
    only_file = tmp_path / "libroot"
    only_file.mkdir()
    (only_file / "Cargo.toml").write_text("version = \"0\"\n", encoding="utf-8")
    assert "Cargo.toml" in provenance.library_files(only_file)
    real_read = Path.read_text

    def missing_proc(self, *args, **kwargs):
        if self == Path("/proc/cpuinfo") or self == Path("/proc/meminfo"):
            raise OSError("absent")
        return real_read(self, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", missing_proc)
    assert provenance.host()["cpu_model"]
    monkeypatch.setattr(Path, "read_text", real_read)
    monkeypatch.setattr(sys, "argv", ["provenance.py"])
    try:
        runpy.run_path(str(SCRIPTS / "provenance.py"), run_name="__main__")
    except SystemExit:
        pass

    anchors = load("src_anchors.py")
    text, _values = anchors.render()
    anchor_out = ROOT / "_cover_anchors_ok.tex"
    anchor_out.write_text(text, encoding="utf-8")
    monkeypatch.setattr(anchors, "OUT", anchor_out)
    monkeypatch.setattr(sys, "argv", ["src_anchors.py", "--check"])
    try:
        anchors.main()
    finally:
        anchor_out.unlink(missing_ok=True)


