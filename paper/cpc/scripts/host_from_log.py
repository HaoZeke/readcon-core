#!/usr/bin/env python3
"""Write the host table and the ratio macros from reader-exchange.log."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOG = ROOT / "logs" / "reader-exchange.log"
GEN = ROOT / "src" / "figures" / "generated"

LABELS = {
    "readcon_full_t1": r"readcon \texttt{read\_con}, 1 thread",
    "readcon_full_t8": r"readcon \texttt{read\_con}, 8 threads",
    "readcon_pos_t1": r"readcon \texttt{read\_positions}, 1 thread",
    "readcon_pos_t8": r"readcon \texttt{read\_positions}, 8 threads",
    "readcon_v0173": r"readcon-core \texttt{read\_con}, 1 thread",
    "readcon_base": r"baseline \texttt{read\_con}, 1 thread",
    "ase_eon": r"ASE \texttt{get\_positions}, 1 thread",
    "eon_py": r"eOn \texttt{loadcons}, 1 thread",
    "readcon_eon_api": r"\texttt{read\_all\_frames} then \texttt{con2matter}, 1 thread",
    "readcon_db": r"readcon-db ingest, 1 thread",
    "readcon_first": r"\texttt{read\_first\_frame}, 1 frame",
    "eon": r"eOn \texttt{con2matter}, 1 frame",
    "numpy": r"NumPy \texttt{loadtxt}, 1 thread",
    "chemfiles": r"chemfiles positions, 1 thread",
    "ovito": r"OVITO positions, 1 thread",
    "ase_extxyz": r"ASE extended XYZ \texttt{get\_positions}, 1 thread",
    "pymatgen": r"pymatgen \texttt{cart\_coords}, 1 thread",
    "h5py": r"h5py dataset, 1 thread",
    "mdanalysis": r"MDAnalysis positions, 1 thread",
    "conbin": r"CON-bin \texttt{read\_conbin}, 1 thread",
}

CON_ORDER = (
    "readcon_full_t1",
    "readcon_full_t8",
    "readcon_pos_t1",
    "readcon_pos_t8",
    "readcon_v0173",
    "readcon_base",
    "ase_eon",
    "eon_py",
    "readcon_eon_api",
    "readcon_db",
    "readcon_first",
    "eon",
)
CONTEXT_ORDER = (
    "numpy",
    "chemfiles",
    "ovito",
    "ase_extxyz",
    "pymatgen",
    "h5py",
    "mdanalysis",
    "conbin",
)


def rows(text: str) -> dict[str, dict[str, str]]:
    found = {}
    for line in text.splitlines():
        if not line.startswith("summary "):
            continue
        fields = dict(part.split("=", 1) for part in line.split() if "=" in part)
        found[fields["arm"]] = fields
    return found


def num(value: str, digits: int) -> str:
    return f"{float(value):.{digits}g}"


def tex_escape(text: str) -> str:
    out = []
    for ch in text:
        if ch in "&%$#_{}":
            out.append("\\" + ch)
        elif ch == "\\":
            out.append("\\textbackslash{}")
        elif ch == "~":
            out.append("\\textasciitilde{}")
        elif ch == "^":
            out.append("\\textasciicircum{}")
        else:
            out.append(ch)
    return "".join(out)


def table(
    found: dict[str, dict[str, str]], order: tuple[str, ...], caption: str, label: str
) -> str:
    lines = [
        "% Written by paper/cpc/scripts/host_from_log.py from logs/reader-exchange.log.",
        "\\begin{table}[tbp]",
        "\\centering",
        "\\footnotesize",
        f"\\caption{{{caption}}}",
        f"\\label{{{label}}}",
        "\\resizebox{\\linewidth}{!}{%",
        "\\begin{tabular}{@{}lrrrr@{}}",
        "\\toprule",
        "Reader & Median (s) & IQR (s) & MiB & Matched \\\\",
        "\\midrule",
    ]
    for name in order:
        row = found.get(name)
        if row is None:
            lines.append(
                f"{LABELS[name]} & \\multicolumn{{4}}{{l}}{{not run}} \\\\"
            )
            continue
        if row.get("median_s") is None:
            err = tex_escape(row.get("error") or "no sample")
            lines.append(
                f"{LABELS[name]} & \\multicolumn{{4}}{{l}}{{{err}}} \\\\"
            )
            continue
        matched = f"{row['matched']}/{row['requested']}"
        lines.append(
            f"{LABELS[name]} & {num(row['median_s'], 4)} & {num(row['iqr_s'], 3)} "
            f"& {num(row['mib_median'], 4)} & {matched} \\\\"
        )
    lines.extend(["\\bottomrule", "\\end{tabular}}", "\\end{table}"])
    return "\n".join(lines)


def field(text: str, key: str) -> str:
    prefix = key + " "
    for line in text.splitlines():
        if line.startswith(prefix):
            return line[len(prefix):].strip()
    raise SystemExit(f"reader-exchange.log has no {key} line")


def exact_tag(commit: str) -> str | None:
    repo = ROOT.parents[1]
    tag = subprocess.run(
        ["git", "-C", str(repo), "describe", "--tags", "--exact-match", commit],
        capture_output=True,
        text=True,
        check=False,
    )
    if tag.returncode == 0:
        return tag.stdout.strip()
    return None


def version_at(commit: str) -> str:
    """The version tag that names ``commit``, or the version in its Cargo.toml."""
    repo = ROOT.parents[1]
    tag = exact_tag(commit)
    if tag:
        return tag.removeprefix("v")
    cargo = subprocess.run(
        ["git", "-C", str(repo), "show", f"{commit}:Cargo.toml"],
        capture_output=True,
        text=True,
        check=False,
    )
    if cargo.returncode != 0:
        raise SystemExit(f"reader-exchange.log names {commit}, which this repository does not hold")
    for line in cargo.stdout.splitlines():
        if line.startswith('version = "'):
            return line.split('"', 2)[1]
    raise SystemExit(f"{commit}:Cargo.toml has no version")


def cxx_macro(raw: str) -> str:
    """``g++ 13.3.0`` from the version line recorded in the log."""
    import re

    match = re.search(r"(\d+\.\d+\.\d+)", raw)
    if not match:
        raise SystemExit(f"reader-exchange.log cxx line has no version: {raw}")
    return f"g++ {match.group(1)}"


def arm_s(found: dict[str, dict[str, str]], name: str) -> float:
    return float(found[name]["median_s"])


def arm_mib(found: dict[str, dict[str, str]], name: str) -> float:
    return float(found[name]["mib_median"])


def main() -> int:
    text = LOG.read_text(encoding="utf-8")
    found = rows(text)
    measured = field(text, "rev_parse")
    release_commit = field(text, "release_commit")
    release_version = version_at(release_commit)
    measured_version = version_at(measured)
    baseline = field(text, "baseline_commit")
    baseline_version = version_at(baseline)
    def arm_label(commit: str, version: str) -> str:
        tag = exact_tag(commit)
        if tag:
            return f"{version}, tag {tag}"
        return f"{version}, untagged"

    LABELS["readcon_full_t1"] = (
        f"{measured_version} \\texttt{{read\\_con}}, 1 thread"
    )
    LABELS["readcon_full_t8"] = (
        f"{measured_version} \\texttt{{read\\_con}}, 8 threads"
    )
    LABELS["readcon_pos_t1"] = (
        f"{measured_version} \\texttt{{read\\_positions}}, 1 thread"
    )
    LABELS["readcon_pos_t8"] = (
        f"{measured_version} \\texttt{{read\\_positions}}, 8 threads"
    )
    LABELS["readcon_v0173"] = (
        f"{arm_label(release_commit, release_version)} \\texttt{{read\\_con}}, 1 thread"
    )
    LABELS["readcon_base"] = (
        f"{arm_label(baseline, baseline_version)} \\texttt{{read\\_con}}, 1 thread, baseline"
    )
    full = arm_s(found, "readcon_full_t1")
    full8 = arm_s(found, "readcon_full_t8")
    pos1 = arm_s(found, "readcon_pos_t1")
    pos8 = arm_s(found, "readcon_pos_t8")
    base = arm_s(found, "readcon_v0173")
    h5 = arm_s(found, "h5py")
    conbin = arm_s(found, "conbin")
    ase = arm_s(found, "ase_eon")
    eon = arm_s(found, "eon_py")
    db = arm_s(found, "readcon_db")
    ghz = float(field(text, "cpu_mhz")) / 1000
    GEN.mkdir(parents=True, exist_ok=True)
    macros = "\n".join(
        [
            "% Written by paper/cpc/scripts/host_from_log.py from logs/reader-exchange.log.",
            f"\\newcommand{{\\rcHostCommit}}{{\\seqsplit{{{measured}}}}}",
            f"\\newcommand{{\\rcReleaseCommit}}{{\\seqsplit{{{release_commit}}}}}",
            f"\\newcommand{{\\rcReleaseVersion}}{{{release_version}}}",
            f"\\newcommand{{\\rcMeasuredVersion}}{{{measured_version}}}",
            f"\\newcommand{{\\rcBaselineCommit}}{{\\seqsplit{{{baseline}}}}}",
            f"\\newcommand{{\\rcBaselineVersion}}{{{baseline_version}}}",
            f"\\newcommand{{\\rcFullOneMedian}}{{{full:.4g}}}",
            f"\\newcommand{{\\rcFullEightMedian}}{{{full8:.4g}}}",
            f"\\newcommand{{\\rcPosOneMedian}}{{{pos1:.4g}}}",
            f"\\newcommand{{\\rcPosEightMedian}}{{{pos8:.4g}}}",
            f"\\newcommand{{\\rcTipMedian}}{{{full:.4g}}}",
            f"\\newcommand{{\\rcReleaseMedian}}{{{base:.4g}}}",
            f"\\newcommand{{\\rcTipOverRelease}}{{{base / full:.3g}}}",
            f"\\newcommand{{\\rcFullThreads}}{{{full / full8:.3g}}}",
            f"\\newcommand{{\\rcHdfMedian}}{{{h5:.4g}}}",
            f"\\newcommand{{\\rcConbinMedian}}{{{conbin:.4g}}}",
            f"\\newcommand{{\\rcTipMib}}{{{arm_mib(found, 'readcon_full_t1'):.4g}}}",
            f"\\newcommand{{\\rcAseMedian}}{{{ase:.4g}}}",
            f"\\newcommand{{\\rcTipOverAse}}{{{ase / full:.3g}}}",
            f"\\newcommand{{\\rcPosOverAse}}{{{ase / pos1:.3g}}}",
            f"\\newcommand{{\\rcEonMedian}}{{{eon:.4g}}}",
            f"\\newcommand{{\\rcTipOverEon}}{{{eon / full:.3g}}}",
            f"\\newcommand{{\\rcPosOverEon}}{{{eon / pos1:.3g}}}",
            f"\\newcommand{{\\rcApiMedian}}{{{arm_s(found, 'readcon_eon_api'):.4g}}}",
            f"\\newcommand{{\\rcDbMedian}}{{{db:.4g}}}",
            f"\\newcommand{{\\rcAseMib}}{{{arm_mib(found, 'ase_eon'):.4g}}}",
            f"\\newcommand{{\\rcChemMib}}{{{arm_mib(found, 'chemfiles'):.4g}}}",
            f"\\newcommand{{\\rcOvitoMib}}{{{arm_mib(found, 'ovito'):.4g}}}",
            f"\\newcommand{{\\rcFirstMedian}}{{{arm_s(found, 'readcon_first'):.4g}}}",
            f"\\newcommand{{\\rcMatterMedian}}{{{arm_s(found, 'eon'):.4g}}}",
            f"\\newcommand{{\\rcHostFamily}}{{{field(text, 'cpu_family')}}}",
            f"\\newcommand{{\\rcHostModel}}{{{field(text, 'cpu_model_id')}}}",
            f"\\newcommand{{\\rcHostCpu}}{{{field(text, 'cpu_model')}}}",
            f"\\newcommand{{\\rcHostGhz}}{{{ghz:.1f}}}",
            f"\\newcommand{{\\rcHostCores}}{{{field(text, 'nproc')}}}",
            f"\\newcommand{{\\rcHostMem}}{{{int(round(int(field(text, 'mem_kib')) / 1024 / 1024))}}}",
            f"\\newcommand{{\\rcHostKernel}}{{{field(text, 'kernel')}}}",
            f"\\newcommand{{\\rcHostOs}}{{{field(text, 'os')}}}",
            f"\\newcommand{{\\rcHostRustc}}{{{field(text, 'rustc')}}}",
            f"\\newcommand{{\\rcHostCxx}}{{{cxx_macro(field(text, 'cxx'))}}}",
            f"\\newcommand{{\\rcHostFs}}{{{field(text, 'filesystem')}}}",
            f"\\newcommand{{\\rcHostCache}}{{{field(text, 'page_cache')}}}",
            "",
        ]
    )
    (GEN / "host_macros.tex").write_text(macros, encoding="utf-8")
    (GEN / "host_table.tex").write_text(
        table(
            found,
            CON_ORDER,
            "Coordinate reads of 100000 frames, six fresh processes, type 7 median. "
            "Each row names the call and the thread count. "
            "\\texttt{read\\_con} builds a frame per record. "
            "\\texttt{read\\_positions} returns one coordinate array. "
            "ASE \\texttt{get\\_positions} and eOn \\texttt{loadcons} build every frame on 1 thread. "
            "The C++ row is \\texttt{read\\_all\\_frames} and then \\texttt{con2matter} on every frame. "
            "readcon-db is an ingest of the file followed by a read of that corpus. "
            "The last two rows read one frame. "
            "A one-frame row is not a ratio against a full read, and the ingest is not a ratio against a read.",
            "tab:host",
        )
        + "\n",
        encoding="utf-8",
    )
    (GEN / "format_context.tex").write_text(
        table(
            found,
            CONTEXT_ORDER,
            "Context for choosing a format, each call at 1 thread. These readers open XYZ, HDF5, or the mapped binary coordinate file. readcon does not read XYZ or HDF5.",
            "tab:format-context",
        )
        + "\n",
        encoding="utf-8",
    )
    print(GEN / "host_macros.tex")
    return 0


if __name__ == "__main__":
    sys.exit(main())
