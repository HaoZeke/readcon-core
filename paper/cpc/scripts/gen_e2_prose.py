#!/usr/bin/env python3
"""Write e2 speedup / stage macros and the SI thread paragraph from JSON.

Sources:
  paper/cpc/evidence/results/e2_layered.json
  paper/cpc/evidence/results/e2_speedup_bca.json

Outputs:
  paper/cpc/src/figures/generated/e2_macros.tex
  paper/cpc/src/figures/generated/e2_supp_threads.tex
"""
from __future__ import annotations

import json
from math import floor, isfinite, log10
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "evidence" / "results"
OUT = ROOT / "src" / "figures" / "generated"
HEADER = "% Written by paper/cpc/scripts/gen_e2_prose.py from e2_layered.json / e2_speedup_bca.json.\n"


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


def fmt_s(x: float) -> str:
    """Wall seconds as printed in the main text (up to 5 decimals, trim)."""
    return f"{x:.5f}".rstrip("0").rstrip(".") if x >= 0.1 else f"{x:.6f}".rstrip("0").rstrip(".")


def fmt_interval_end(x: float) -> str:
    """Match the manuscript's mixed precision: three decimals below 2, two above."""
    if abs(x) < 2:
        return f"{x:.3f}".rstrip("0").rstrip(".") if x >= 1 else f"{x:.3f}"
    return sig(x, 3)


def main() -> None:
    layered = json.loads((RESULTS / "e2_layered.json").read_text())
    bca = json.loads((RESULTS / "e2_speedup_bca.json").read_text())
    n_scale = int(layered["n_scale_repeats"])
    stages = layered["stages"]
    rust1 = layered["native"]["rust_1"]["median_s"]
    file_read = stages["file_read"]["median_s"]
    scan = stages["boundary_scan"]["median_s"]
    floor_sum = file_read + scan
    floor_frac = floor_sum / rust1

    def reader_block(key: str) -> dict:
        block = bca[key]
        # Prefer nested "8" if present; fall back to flat keys.
        eight = block["8"] if isinstance(block.get("8"), dict) else block
        ends = bca.get("endpoints", {}).get(key, {})
        one = ends.get("one_median_s")
        eight_s = ends.get("eight_median_s")
        if one is None:
            one = layered["scaling"][key]["1"]["median_s"]
        if eight_s is None:
            eight_s = layered["scaling"][key]["8"]["median_s"]
        return {
            "one": one,
            "eight": eight_s,
            "med": eight["median"],
            "lo": eight["low"],
            "hi": eight["high"],
            "n": eight["n"],
        }

    rust = reader_block("rust")
    cpp = reader_block("cpp")
    py = reader_block("python_full")

    macros = [HEADER]
    macros.append(rf"\newcommand{{\eTwoNScale}}{{{n_scale}}}")
    macros.append(rf"\newcommand{{\eTwoRustOne}}{{{fmt_s(rust['one'])}}}")
    macros.append(rf"\newcommand{{\eTwoRustEight}}{{{fmt_s(rust['eight'])}}}")
    macros.append(rf"\newcommand{{\eTwoRustMed}}{{{sig(rust['med'], 3)}}}")
    macros.append(rf"\newcommand{{\eTwoRustLo}}{{{fmt_interval_end(rust['lo'])}}}")
    macros.append(rf"\newcommand{{\eTwoRustHi}}{{{fmt_interval_end(rust['hi'])}}}")
    macros.append(rf"\newcommand{{\eTwoCppOne}}{{{fmt_s(cpp['one'])}}}")
    macros.append(rf"\newcommand{{\eTwoCppEight}}{{{fmt_s(cpp['eight'])}}}")
    macros.append(rf"\newcommand{{\eTwoCppMed}}{{{sig(cpp['med'], 3)}}}")
    macros.append(rf"\newcommand{{\eTwoCppLo}}{{{fmt_interval_end(cpp['lo'])}}}")
    macros.append(rf"\newcommand{{\eTwoCppHi}}{{{fmt_interval_end(cpp['hi'])}}}")
    macros.append(rf"\newcommand{{\eTwoPyOne}}{{{fmt_s(py['one'])}}}")
    macros.append(rf"\newcommand{{\eTwoPyEight}}{{{fmt_s(py['eight'])}}}")
    macros.append(rf"\newcommand{{\eTwoPyMed}}{{{sig(py['med'], 3)}}}")
    macros.append(rf"\newcommand{{\eTwoPyLo}}{{{fmt_interval_end(py['lo'])}}}")
    macros.append(rf"\newcommand{{\eTwoPyHi}}{{{fmt_interval_end(py['hi'])}}}")
    macros.append(rf"\newcommand{{\eTwoFileRead}}{{{fmt_s(file_read)}}}")
    macros.append(rf"\newcommand{{\eTwoScan}}{{{fmt_s(scan)}}}")
    macros.append(rf"\newcommand{{\eTwoFloorSum}}{{{fmt_s(floor_sum)}}}")
    macros.append(rf"\newcommand{{\eTwoRustOneNative}}{{{fmt_s(rust1)}}}")
    macros.append(rf"\newcommand{{\eTwoFloorFrac}}{{{floor_frac:.6f}}}")

    # Warm-cache Table / prose from the same layered JSON.
    rust_auto = layered["native"]["rust_auto"]
    cpp_auto = layered["native"]["cpp_auto"]
    rust_1n = layered["native"]["rust_1"]
    cpp_1n = layered["native"]["cpp_1"]
    py_full = layered["python"]["full_objects"]["readcon_python"]
    ratio_auto = layered["native"]["ratio_rust_over_cpp_auto"]
    ratio_one = layered["native"]["ratio_rust_over_cpp_1"]
    emt = float(layered["force_call"]["emt_seconds_median"])
    morse = float(layered["force_call"]["morse_in_process"]["seconds_per_call"])
    nframes = int(layered["file"]["frames"])
    us_per_frame = 1e6 * rust_auto["median_s"] / nframes

    def row_bits(block: dict) -> tuple[str, str, str, str]:
        mb = block["MB_per_s"]
        mb_s = sig(mb, 4) if mb >= 100 else sig(mb, 3)
        return (
            f"{block['median_s']:.6f}".rstrip("0").rstrip("."),
            mb_s,
            f"{block['bca_low_s']:.6f}".rstrip("0").rstrip("."),
            f"{block['bca_high_s']:.6f}".rstrip("0").rstrip("."),
        )

    ra_med, ra_mb, ra_lo, ra_hi = row_bits(rust_auto)
    ca_med, ca_mb, ca_lo, ca_hi = row_bits(cpp_auto)
    r1_med, r1_mb, r1_lo, r1_hi = row_bits(rust_1n)
    c1_med, c1_mb, c1_lo, c1_hi = row_bits(cpp_1n)
    py_med, py_mb, py_lo, py_hi = row_bits(py_full)
    macros += [
        rf"\newcommand{{\eTwoRustAutoMed}}{{{ra_med}}}",
        rf"\newcommand{{\eTwoRustAutoMB}}{{{ra_mb}}}",
        rf"\newcommand{{\eTwoRustAutoLo}}{{{ra_lo}}}",
        rf"\newcommand{{\eTwoRustAutoHi}}{{{ra_hi}}}",
        rf"\newcommand{{\eTwoCppAutoMed}}{{{ca_med}}}",
        rf"\newcommand{{\eTwoCppAutoMB}}{{{ca_mb}}}",
        rf"\newcommand{{\eTwoCppAutoLo}}{{{ca_lo}}}",
        rf"\newcommand{{\eTwoCppAutoHi}}{{{ca_hi}}}",
        rf"\newcommand{{\eTwoRustThrOneMed}}{{{r1_med}}}",
        rf"\newcommand{{\eTwoRustThrOneMB}}{{{r1_mb}}}",
        rf"\newcommand{{\eTwoRustThrOneLo}}{{{r1_lo}}}",
        rf"\newcommand{{\eTwoRustThrOneHi}}{{{r1_hi}}}",
        rf"\newcommand{{\eTwoCppThrOneMed}}{{{c1_med}}}",
        rf"\newcommand{{\eTwoCppThrOneMB}}{{{c1_mb}}}",
        rf"\newcommand{{\eTwoCppThrOneLo}}{{{c1_lo}}}",
        rf"\newcommand{{\eTwoCppThrOneHi}}{{{c1_hi}}}",
        rf"\newcommand{{\eTwoPyFullMed}}{{{py_med}}}",
        rf"\newcommand{{\eTwoPyFullMB}}{{{py_mb}}}",
        rf"\newcommand{{\eTwoPyFullLo}}{{{py_lo}}}",
        rf"\newcommand{{\eTwoPyFullHi}}{{{py_hi}}}",
        rf"\newcommand{{\eTwoUsPerFrame}}{{{us_per_frame:.2f}}}",
        rf"\newcommand{{\eTwoFramesPerS}}{{{rust_auto['frames_per_s']:.0f}}}",
        rf"\newcommand{{\eTwoRatioAutoMed}}{{{sig(ratio_auto['median'], 3)}}}",
        rf"\newcommand{{\eTwoRatioAutoLo}}{{{fmt_interval_end(ratio_auto['bca_low'])}}}",
        rf"\newcommand{{\eTwoRatioAutoHi}}{{{fmt_interval_end(ratio_auto['bca_high'])}}}",
        rf"\newcommand{{\eTwoRatioOneMed}}{{{sig(ratio_one['median'], 3)}}}",
        rf"\newcommand{{\eTwoEmtS}}{{{emt:.3e}}}",
        rf"\newcommand{{\eTwoEmtCalls}}{{{int(round(rust_auto['median_s'] / emt))}}}",
        rf"\newcommand{{\eTwoMorseS}}{{{morse:.3e}}}",
        rf"\newcommand{{\eTwoMorseCalls}}{{{int(round(rust_auto['median_s'] / morse))}}}",
        rf"\newcommand{{\eTwoPyBuild}}{{{fmt_s(py_full['median_s'])}}}",
    ]

    perf_table = [
        HEADER.rstrip("\n"),
        r"\begin{table}[ht]",
        r"\centering",
        r"\small",
        (
            r"\caption{Warm read of \(10{,}000\) frames, \(167{,}710{,}000\) bytes, "
            r"readcon-core 0.17.2 at commit~\readconcommitshort{}."
            r" Native rows are 15 repeats."
            r" The Python row is 9 repeats."
            r" The interval is a bias-corrected and accelerated 95 percent interval of the median.}"
        ),
        r"\label{tab:perf}",
        r"\begin{tabular}{lrrl}",
        r"\toprule",
        r"Reader & Median (s) & MB/s & Interval (s) \\",
        r"\midrule",
        rf"Rust, automatic pool & {ra_med} & {ra_mb} & [{ra_lo}, {ra_hi}] \\",
        rf"C++ wrapper, automatic pool & {ca_med} & {ca_mb} & [{ca_lo}, {ca_hi}] \\",
        rf"Rust, 1 thread & {r1_med} & {r1_mb} & [{r1_lo}, {r1_hi}] \\",
        rf"C++ wrapper, 1 thread & {c1_med} & {c1_mb} & [{c1_lo}, {c1_hi}] \\",
        rf"Python, full objects & {py_med} & {py_mb} & [{py_lo}, {py_hi}] \\",
        r"\bottomrule",
        r"\end{tabular}",
        r"\end{table}",
        "",
    ]
    (OUT / "e2_perf_table.tex").write_text("\n".join(perf_table) + "\n")

    # SI 2/4/8 Python paired-ratio BCa from the same JSON.
    for thr, name in ((2, "Two"), (4, "Four"), (8, "Eight")):
        block = bca["python_full"][str(thr)] if str(thr) in bca["python_full"] else None
        if block is None and thr == 8:
            block = bca["python_full"]
        assert block is not None
        macros.append(rf"\newcommand{{\eTwoPy{name}Med}}{{{sig(block['median'], 3)}}}")
        macros.append(rf"\newcommand{{\eTwoPy{name}Lo}}{{{fmt_interval_end(block['low'])}}}")
        macros.append(rf"\newcommand{{\eTwoPy{name}Hi}}{{{fmt_interval_end(block['high'])}}}")

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "e2_macros.tex").write_text("\n".join(macros) + "\n")

    # Supplement paragraph: only regenerated JSON numbers.
    py2 = bca["python_full"]["2"]
    py4 = bca["python_full"]["4"]
    py8 = bca["python_full"]["8"] if "8" in bca["python_full"] else bca["python_full"]
    supp = [
        HEADER.rstrip("\n"),
        rf"The \eTwoNScale-repeat pool sweep in Section~\ref{{sec:perf-read}} is the campaign on the same \(10{{,}}000\)-frame file.",
        rf"Paired one-thread/two-thread Python speedups have median \(\eTwoPyTwoMed\) with BCa interval \([\eTwoPyTwoLo, \eTwoPyTwoHi]\);",
        rf"four threads give \(\eTwoPyFourMed\) with \([\eTwoPyFourLo, \eTwoPyFourHi]\);",
        rf"eight threads give \(\eTwoPyEightMed\) with \([\eTwoPyEightLo, \eTwoPyEightHi]\).",
        rf"Those three intervals are the \texttt{{python\_full}} entries of \texttt{{e2\_speedup\_bca.json}} (\texttt{{pixi run cpc-e2-speedup-bca}}).",
        rf"The in-process file read is \(\eTwoFileRead\,\mathrm{{s}}\) and the boundary scan is \(\eTwoScan\,\mathrm{{s}}\).",
        rf"Their sum is \(\eTwoFloorSum\,\mathrm{{s}}\), which is \(\eTwoFloorFrac\) of the 15-repeat one-thread median \(\eTwoRustOneNative\,\mathrm{{s}}\).",
        r"That quotient is not the serial fraction of Figure~\ref{fig:roofline}.",
        r"The figure has no Amdahl curve.",
        "",
    ]
    # Keep unused locals referenced for clarity in generated comment.
    _ = (py2, py4, py8)
    (OUT / "e2_supp_threads.tex").write_text("\n".join(supp) + "\n")
    print("wrote", OUT / "e2_macros.tex")
    print("wrote", OUT / "e2_supp_threads.tex")
    print("wrote", OUT / "e2_perf_table.tex")
    print(
        "python 1→8:",
        py["med"],
        py["lo"],
        py["hi"],
        "rust:",
        rust["med"],
        rust["lo"],
        rust["hi"],
        "us/frame:",
        f"{us_per_frame:.2f}",
    )


if __name__ == "__main__":
    main()
