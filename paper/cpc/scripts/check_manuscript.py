#!/usr/bin/env python3
"""Fail the CPC build unless abstract/highlights bounds hold and PDFs have no TODO."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ORG = ROOT / "src" / "rg_main.tex"
HIGHLIGHTS = ROOT / "highlights.txt"
PDFS = (ROOT / "src" / "rg_main.pdf", ROOT / "src" / "rg_supp.pdf")


def abstract_words(org: str) -> int:
    match = re.search(r"\\begin\{abstract\}(.*?)\\end\{abstract\}", org, re.DOTALL)
    if not match:
        raise SystemExit("abstract environment not found")
    text = match.group(1)
    text = re.sub(r"\\cite\{[^}]*\}", " ", text)
    text = re.sub(r"\\[a-zA-Z]+\{([^{}]*)\}", r" \1 ", text)
    text = re.sub(r"\\[a-zA-Z]+", " ", text)
    text = text.replace("~", " ").replace("$", " ")
    text = re.sub(r"[{}]", " ", text)
    words = [w for w in text.split() if re.search(r"[A-Za-z0-9]", w)]
    return len(words)


def assert_no_todo_in_pdfs() -> None:
    for pdf in PDFS:
        if not pdf.is_file():
            raise SystemExit(f"missing PDF for TODO check: {pdf}")
        text = subprocess.check_output(["pdftotext", str(pdf), "-"], text=True)
        if re.search(r"TODO", text):
            raise SystemExit(f"TODO found in pdftotext of {pdf.name}")


def main() -> None:
    org = ORG.read_text()
    n_words = abstract_words(org)
    if n_words > 250:
        raise SystemExit(f"abstract has {n_words} words; CPC maximum is 250")
    lines = [ln for ln in HIGHLIGHTS.read_text().splitlines() if ln.strip()]
    if not 3 <= len(lines) <= 5:
        raise SystemExit(f"highlights.txt has {len(lines)} bullets; need 3 to 5")
    for line in lines:
        if len(line) > 85:
            raise SystemExit(
                f"highlight longer than 85 characters: {line!r} ({len(line)})"
            )
    assert_no_todo_in_pdfs()
    print(f"abstract words: {n_words}; highlights: {len(lines)}; PDF TODO check clean")


if __name__ == "__main__":
    try:
        main()
    except SystemExit as exc:
        print(exc, file=sys.stderr)
        raise
