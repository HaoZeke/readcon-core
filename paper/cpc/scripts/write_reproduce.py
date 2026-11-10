#!/usr/bin/env python3
"""Write REPRODUCE.md from the manuscript inputs and the format commands.

    python3 paper/cpc/scripts/write_reproduce.py
"""

from __future__ import annotations

import re
from pathlib import Path

CPC = Path(__file__).resolve().parents[1]
SRC = CPC / "src"
OUT = CPC / "REPRODUCE.md"

# argv of scripts/regen_paper.py FORMAT, MEASURE, and the orphan outputs.
COMMANDS = {
    "cargo_version_tex.py": "python3 scripts/cargo_version_tex.py",
    "gen_release.py": "python3 scripts/gen_release.py",
    "src_anchors.py": "python3 scripts/src_anchors.py",
    "gen_failure_tables.py": "python3 scripts/gen_failure_tables.py",
    "survey_listing.py": "python3 scripts/survey_listing.py --tables",
    "spec_corpus_recount.py": "python3 scripts/spec_corpus_recount.py --tables",
    "gen_cloud_tables.py": "python3 scripts/gen_cloud_tables.py",
    "e5_prefactor.py": "python3 evidence/e5_prefactor.py --tables",
    "velocity_case.py": "python3 scripts/velocity_case.py --tables",
    "gen_case_macros.py": "python3 scripts/gen_case_macros.py",
    "gen_tables.py": "python3 scripts/gen_tables.py",
    "r0174_tables.py": "python3 scripts/r0174_tables.py",
    "h2h_tables.py": "python3 scripts/h2h_tables.py",
    "binary_formats_tables.py": "python3 scripts/binary_formats_tables.py",
    "ordering_tests.py": "python3 scripts/ordering_tests.py",
    "corpus31_results.py": "python3 scripts/corpus31_results.py --tables",
    "corpus_paths.py": "python3 scripts/corpus_paths.py --tables",
    "every_path_count.py": "python3 scripts/every_path_count.py --tables",
    "corpus_crosscheck.py": "python3 scripts/corpus_crosscheck.py --tables",
    "clause_coverage.py": "python3 scripts/clause_coverage.py",
    "mutant_differential.py": "python3 scripts/mutant_differential.py --tables",
    "file_sizes.py": "python3 scripts/file_sizes.py --table",
    "worked_example.py": "python3 scripts/worked_example.py --tables",
    "test_run.py": "python3 scripts/test_run.py --tables",
    "sinum.py": "python3 scripts/sinum.py",
    "gen_figures.py": "python3 scripts/gen_figures.py",
    "plot_perf.py": "python3 scripts/plot_perf.py",
    "gen_spec_supp.py": "python3 scripts/gen_spec_supp.py --as-is",
    "plot_figures.py": "python3 scripts/plot_figures.py",
    "plot_restart_formats.py": "python3 scripts/plot_restart_formats.py",
    "plot_structures.py": "python3 scripts/plot_structures.py",
    "plot_supp.py": "python3 scripts/plot_supp.py",
    "host_from_log.py": "python3 scripts/host_from_log.py",
    "db_from_log.py": "python3 scripts/db_from_log.py",
    "mpi_from_log.py": "python3 scripts/mpi_from_log.py",
    "bench_dlpack.py": "python3 scripts/bench_dlpack.py --from-log",
    "regen_figures.py": "python3 scripts/regen_figures.py",
    "gen_version_map.py": "python3 scripts/gen_version_map.py",
    "plot_union_extra.py": "python3 scripts/plot_union_extra.py",
}

ORPHANS = [
    (
        "logs/NUMBERS.tsv",
        "python3 scripts/numbers_from_log.py logs/reader-exchange.log logs/NUMBERS.tsv",
    ),
    (
        "records.tar.xz and records.sha256",
        "python3 scripts/pack_records.py --write",
    ),
    (
        "src/rg_main.pdf and src/rg_supp.pdf",
        "bash scripts/build_tex.sh",
    ),
    (
        "variants/rg_main-b.pdf and variants/rg_supp-b.pdf",
        "bash scripts/build_tex.sh  # then the public-release build named in variants/NUMBERS.txt",
    ),
    (
        "data/cloud-runs-2/05/bands/*.npz",
        "python3 data/cloud-runs-2/cloud-run/run_05.py",
    ),
    (
        "the tables and figures together",
        "python3 scripts/regen_paper.py",
    ),
    (
        "the same targets through Snakemake",
        "snakemake -c1 all",
    ),
]


def manuscript_inputs() -> list[Path]:
    blob = []
    for name in ("rg_main.tex", "rg_supp.tex"):
        blob.append((SRC / name).read_text())
    text = "\n".join(blob)
    text = "\n".join(re.sub(r"(?<!\\)%.*", "", line) for line in text.splitlines())
    names = re.findall(r"\\input\{([^}]+)\}", text)
    names += re.findall(r"\\includegraphics(?:\[[^\]]*\])?\{([^}]+)\}", text)
    found = []
    seen = set()
    for item in names:
        path = item if item.endswith((".tex", ".pdf")) else item + ".tex"
        full = SRC / path
        if full in seen:
            continue
        seen.add(full)
        found.append(full)
    listing = SRC / "figures/generated/survey_listing.tex"
    if listing.is_file():
        for item in re.findall(r"\\lstinputlisting(?:\[[^\]]*\])?\{([^}]+)\}", listing.read_text()):
            full = SRC / item
            if full not in seen:
                seen.add(full)
                found.append(full)
    return found


def script_of(path: Path) -> tuple[str, str]:
    if not path.is_file() or path.suffix != ".tex":
        if path.suffix == ".pdf":
            return "plot scripts via regen_paper.py", COMMANDS["regen_figures.py"]
        if path.suffix == ".con":
            return "survey_listing.py", COMMANDS["survey_listing.py"]
        return "source file", "tracked in git"
    head = path.read_text(errors="replace").splitlines()[:1]
    line = head[0] if head else ""
    match = re.search(r"([A-Za-z0-9_]+\.py)", line)
    if not match:
        return "source file", "tracked in git"
    name = match.group(1)
    return name, COMMANDS.get(name, f"python3 scripts/{name}")


def main() -> None:
    lines = [
        "# Reproduction",
        "",
        "Commands run from `paper/cpc`.",
        "Each manuscript input names the script that writes it.",
        "An output is written by that command, not by hand.",
        "",
        "## Manuscript inputs",
        "",
        "| Input | Script | Command |",
        "|---|---|---|",
    ]
    for path in manuscript_inputs():
        rel = path.relative_to(CPC).as_posix()
        script, command = script_of(path)
        lines.append(f"| `{rel}` | `{script}` | `{command}` |")
    lines += [
        "",
        "## Outputs that had no command",
        "",
        "| Output | Command |",
        "|---|---|",
    ]
    for output, command in ORPHANS:
        lines.append(f"| `{output}` | `{command}` |")
    lines += [
        "",
        "## Timing hostname",
        "",
        "A timing receipt keeps a hostname key.",
        "The value is the literal bench.",
        "The manuscript check accepts that value and no other.",
        "A missing key is a failure.",
        "A value other than bench is a failure.",
        "Any other host, hostname, or nodename value is a failure.",
        "",
    ]
    OUT.write_text("\n".join(lines))
    print(f"wrote {OUT} ({len(manuscript_inputs())} inputs)")


if __name__ == "__main__":
    main()
