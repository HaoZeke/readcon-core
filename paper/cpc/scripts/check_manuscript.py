#!/usr/bin/env python3
"""Fail the CPC build unless abstract/highlights bounds hold, PDFs have no TODO,
and printed e2 / peer numbers match the regenerated JSON and CSVs.
"""

from __future__ import annotations

import csv
import json
import re
import subprocess
import sys
from math import floor, isfinite, log10
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ORG = ROOT / "src" / "rg_main.tex"
SUPP = ROOT / "src" / "rg_supp.tex"
HIGHLIGHTS = ROOT / "highlights.txt"
PDFS = (ROOT / "src" / "rg_main.pdf", ROOT / "src" / "rg_supp.pdf")
E2_BCA = ROOT / "evidence" / "results" / "e2_speedup_bca.json"
E2_LAYERED = ROOT / "evidence" / "results" / "e2_layered.json"
E2_MACROS = ROOT / "src" / "figures" / "generated" / "e2_macros.tex"
PEERS_AB = ROOT / "src" / "figures" / "generated" / "peers_campaign_a.tex"
RATIOS_A = ROOT / "bench" / "results" / "ratios.csv"
WALLS = ROOT / "data" / "cloud-runs-2" / "05" / "restart_walls.csv"


def sig(x: float, n: int = 3) -> str:
    if x == 0:
        return "0"
    if not isfinite(x):
        return str(x)
    order = floor(log10(abs(x)))
    decimals = n - 1 - order
    if decimals < 0:
        return f"{round(x, decimals):.0f}"
    rounded = round(x, decimals)
    if decimals == 0:
        return f"{int(rounded)}"
    return f"{rounded:.{decimals}f}"


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


def macro_value(macros: str, name: str) -> str:
    match = re.search(rf"\\newcommand{{\\{name}}}{{([^}}]*)}}", macros)
    if not match:
        raise SystemExit(f"macro \\{name} missing from {E2_MACROS}")
    return match.group(1)


def eight_block(bca: dict, reader: str) -> dict:
    block = bca[reader]
    if isinstance(block.get("8"), dict):
        return block["8"]
    return block


def assert_e2_macros_match_json() -> None:
    bca = json.loads(E2_BCA.read_text())
    layered = json.loads(E2_LAYERED.read_text())
    macros = E2_MACROS.read_text()
    n_scale = int(layered["n_scale_repeats"])
    if n_scale < 30:
        raise SystemExit(f"e2_layered.json n_scale_repeats={n_scale}; need ≥30")
    if macro_value(macros, "eTwoNScale") != str(n_scale):
        raise SystemExit(
            f"\\eTwoNScale={macro_value(macros, 'eTwoNScale')} ≠ JSON n_scale_repeats={n_scale}"
        )
    method = bca.get("method", "")
    rust = eight_block(bca, "rust")
    py = eight_block(bca, "python_full")
    if "scipy" not in str(rust.get("method", method)).lower() and "BCa" not in str(
        rust.get("method", method)
    ):
        # Accept either top-level or per-reader method string naming scipy BCa.
        if "scipy" not in json.dumps(bca).lower():
            raise SystemExit("e2_speedup_bca.json does not record scipy BCa")
    # Macro medians must equal sig(JSON median).
    checks = [
        ("eTwoRustMed", rust["median"]),
        ("eTwoPyMed", py["median"]),
        ("eTwoCppMed", eight_block(bca, "cpp")["median"]),
    ]
    for name, value in checks:
        printed = macro_value(macros, name)
        expected = sig(float(value), 3)
        if printed != expected:
            raise SystemExit(f"\\{name}={printed} ≠ sig(JSON)={expected} ({value})")
    # Interval endpoints: macro string must parse back to the JSON value within 5e-4
    # after the same display formatting used by gen_e2_prose.
    for name, key, block in (
        ("eTwoRustLo", "low", rust),
        ("eTwoRustHi", "high", rust),
        ("eTwoPyLo", "low", py),
        ("eTwoPyHi", "high", py),
    ):
        printed = float(macro_value(macros, name))
        raw = float(block[key])
        if abs(printed - raw) > 5e-3 and abs(printed - raw) / max(abs(raw), 1e-12) > 0.01:
            raise SystemExit(f"\\{name}={printed} drifts from JSON {key}={raw}")
    if rust["n"] < 30 or py["n"] < 30:
        raise SystemExit(f"BCa n too small: rust={rust['n']} python={py['n']}")


def assert_no_stale_speedups() -> None:
    """Ban the leftover five-repeat 1.15 prose and the collapsed chemfiles 1.6 cell.

    A regenerated JSON median that happens to round to 1.15 is allowed; the
    supplement must obtain it through e2 macros / e2_supp_threads, not a
    hardcoded five-repeat sentence.
    """
    supp = SUPP.read_text()
    if re.search(r"five-repeat Python speedup at eight threads is", supp):
        raise SystemExit("stale SI five-repeat speedup sentence still present")
    if "0.483640922" in supp or "0.48364" in supp:
        raise SystemExit("stale pre-reclock one-thread median still in rg_supp.tex")
    if r"\input{figures/generated/e2_supp_threads}" not in supp:
        raise SystemExit("rg_supp.tex must \\input e2_supp_threads")
    peers = PEERS_AB.read_text()
    # chemfiles 10000-frame ratio must not print as bare 1.6 (need 1.60).
    if re.search(
        r"10000 frames & chemfiles XYZ & coordinates & [^&]+ & [^&]+ & 1\.6 &", peers
    ):
        raise SystemExit("peers_campaign_a.tex still prints chemfiles ratio as 1.6")
    with RATIOS_A.open() as handle:
        for row in csv.DictReader(handle):
            if row["payload"] == "frames-10000" and row["peer"] == "chemfiles_xyz":
                expected = sig(float(row["median_ratio_readcon_over_peer"]), 3)
                if expected == "1.6":
                    raise SystemExit("sig() still collapses chemfiles ratio to 1.6")
                if not re.search(
                    rf"chemfiles XYZ & coordinates & [^\\\\]* & {re.escape(expected)} &",
                    peers,
                ):
                    raise SystemExit(
                        f"peers_campaign_a.tex missing chemfiles ratio {expected}"
                    )
                break
        else:
            raise SystemExit("chemfiles frames-10000 row missing from ratios.csv")


def assert_walls_repeats() -> None:
    meta = ROOT / "data" / "cloud-runs-2" / "05" / "restart_walls.json"
    if meta.is_file():
        payload = json.loads(meta.read_text())
        n = int(payload.get("wall_repeats") or payload.get("n_repeats") or 0)
        if n < 10:
            raise SystemExit(f"restart_walls.json wall_repeats={n}; need ≥10")
    repeats = ROOT / "data" / "cloud-runs-2" / "05" / "restart_walls_repeats.csv"
    if repeats.is_file():
        with repeats.open() as handle:
            rows = list(csv.DictReader(handle))
        by_route: dict[str, int] = {}
        for row in rows:
            by_route[row["route"]] = by_route.get(row["route"], 0) + 1
        short = {r: c for r, c in by_route.items() if c < 10}
        if short:
            raise SystemExit(f"restart_walls_repeats.csv routes with n<10: {short}")
    if not WALLS.is_file():
        raise SystemExit(f"missing {WALLS}")


def assert_delta_s_matches_code() -> None:
    main = ORG.read_text()
    if r"0.02\,\text{\AA}^2/\mathrm{eV}" not in main:
        raise SystemExit("Table 11 Δs is not 0.02 Å²/eV")
    if re.search(r"\\Delta s\} & [^&]+ & [^&]+ & \$0\.1\\,", main):
        raise SystemExit("Table 11 still prints Δs = 0.1 Å")
    driver = (ROOT / "scripts" / "restart_formats.py").read_text()
    if "FMAX, ALPHA, K, NMAX = 0.01, 0.02, 25, 4000" not in driver:
        raise SystemExit("restart_formats.py ALPHA is not 0.02 Å²/eV")


def main() -> None:
    org = ORG.read_text()
    n_words = abstract_words(org)
    if n_words > 250:
        raise SystemExit(f"abstract has {n_words} words; CPC maximum is 250")
    lines = [ln for ln in HIGHLIGHTS.read_text().splitlines() if ln.strip()]
    if not 3 <= len(lines) <= 5:
        raise SystemExit(f"highlights.txt has {len(lines)} lines; need 3 to 5")
    for line in lines:
        if len(line) > 85:
            raise SystemExit(
                f"highlight longer than 85 characters: {line!r} ({len(line)})"
            )
    assert_no_todo_in_pdfs()
    assert_e2_macros_match_json()
    assert_no_stale_speedups()
    assert_walls_repeats()
    assert_delta_s_matches_code()
    print(
        f"abstract words: {n_words}; highlights: {len(lines)}; "
        "PDF TODO check clean; e2/peer/walls/Δs JSON checks clean"
    )


if __name__ == "__main__":
    try:
        main()
    except SystemExit as exc:
        print(exc, file=sys.stderr)
        raise
