#!/usr/bin/env python3
"""Regenerate fairness and campaign-A/B peer TeX tables from committed CSVs."""
from __future__ import annotations

import csv
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "bench" / "results"
OUT = ROOT / "src" / "figures" / "generated"
HEADER = "% Written by paper/cpc/scripts/gen_fairness_peer_tables.py.\n"

PEER = {
    "chemfiles_xyz": "chemfiles XYZ",
    "h5py": "h5py",
    "mda_h5md": "MDAnalysis H5MD",
    "ase_eon": "ASE eon",
    "ase_extxyz": "ASE extended XYZ",
    "ase_traj": "ASE trajectory",
}
FIELDS = {
    "chemfiles_xyz": "coordinates",
    "h5py": "one binary array",
    "mda_h5md": "coordinates",
    "ase_eon": "same CON text",
    "ase_extxyz": "ASE atoms",
    "ase_traj": "ASE atoms",
}
PAYLOAD = {
    "frames-10000": "10000 frames",
    "slab": "Pt slab",
}


def sig(x: float, n: int = 3) -> str:
    """Format ``x`` to ``n`` significant figures, keeping trailing zeros.

    ``f\"{1.60249:.3g}\"`` collapses to ``1.6``; this returns ``1.60``.
    ``50.998`` with ``n=3`` returns ``51.0``.
    """
    if x == 0:
        return "0"
    from math import floor, isfinite, log10

    if not isfinite(x):
        return str(x)
    order = floor(log10(abs(x)))
    decimals = n - 1 - order
    if decimals < 0:
        # Large magnitude: round to significant integer digits.
        rounded = round(x, decimals)
        return f"{rounded:.0f}"
    rounded = round(x, decimals)
    if decimals == 0:
        return f"{int(rounded)}"
    return f"{rounded:.{decimals}f}"


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as fh:
        return list(csv.DictReader(fh))


def fairness_table() -> str:
    rows = read_csv(RESULTS / "ratios_v76_fairness.csv")
    body = [
        r"\begin{table}[ht]",
        r"\centering",
        r"\small",
        r"\caption{Paired ratios of the one-worker Python readcon-core time to each peer on \(10{,}000\) frames, so a value below 1 means readcon-core finished sooner.",
        r"Each cell is 30 interleaved pairs; the interval is the median of those paired ratios with its bias-corrected and accelerated 95 percent BCa interval.",
        r"Wall columns are the median wall-clock seconds of the same pairs (from \texttt{ratios\_v76\_fairness.csv}).",
        r"The evaluation count for every row is \(10{,}000\) frames.",
        r"Mode \texttt{full\_fields} keeps the parsed frames.",
        r"Mode \texttt{coords\_accum} runs the same parse and then accumulates only the first atom's \(x+y+z\).",
        r"The driver is \texttt{paper/cpc/bench/run\_fairness.py} (\texttt{pixi run cpc-fairness}).}",
        r"\label{tab:peers}",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{2.5pt}",
        r"\resizebox{\textwidth}{!}{%",
        r"\begin{tabular}{lllrrrl}",
        r"\toprule",
        r"Mode & Peer & Fields & readcon (s) & peer (s) & Ratio & Interval \\",
        r"\midrule",
    ]
    for row in rows:
        mode = row["mode"].replace("_", r"\_")
        peer = PEER[row["peer"]]
        fields = FIELDS[row["peer"]]
        rc = float(row["median_readcon_s"])
        pr = float(row["median_peer_s"])
        ratio = float(row["median_ratio"])
        lo = float(row["bca_low"])
        hi = float(row["bca_high"])
        body.append(
            rf"{mode} & {peer} & {fields} & {sig(rc)} & {sig(pr)} & {sig(ratio)} & "
            rf"[{sig(lo)}, {sig(hi)}] \\"
        )
    body += [r"\bottomrule", r"\end{tabular}}", r"\end{table}", ""]
    return "\n".join(body)


def campaign_table() -> str:
    a = read_csv(RESULTS / "ratios.csv")
    b = read_csv(RESULTS / "ratios_ase_formats.csv")
    # Wall times for B peers from ratios_ase; for A from ratios.csv.
    a_long = {
        r["peer"]: r
        for r in a
        if r["payload"] == "frames-10000" and r.get("bca_low")
    }
    a_slab = {
        r["peer"]: r
        for r in a
        if r["payload"] == "slab" and r.get("bca_low")
    }
    b_long = {
        r["peer"]: r
        for r in b
        if r["payload"] == "frames-10000" and r.get("bca_low")
    }
    b_slab = {
        r["peer"]: r
        for r in b
        if r["payload"] == "slab" and r.get("bca_low")
    }
    ase_a = float(a_long["ase_eon"]["median_ratio_readcon_over_peer"])
    ase_b = float(b_long["ase_eon"]["median_ratio_readcon_over_peer"])
    body = [
        r"\begin{table}[ht]",
        r"\centering",
        r"\small",
        (
            r"\caption{Campaign A and B peer ratios on the same host (family 6, model 173, \(2700\,\mathrm{MHz}\)), thirty interleaved pairs each. "
            r"Wall columns are median wall-clock seconds from the committed repeat CSVs (\texttt{ratios.csv}, \texttt{ratios\_ase\_formats.csv}). "
            r"The evaluation count on the long file is \(10{,}000\) frames; the slab rows are one frame. "
            rf"Campaign B adds ASE extended XYZ and the ASE trajectory; its ASE eon ratio on the \(10{{,}}000\)-frame file is "
            rf"{sig(ase_b)} against {sig(ase_a)} in A. "
            r"These rows are not the fairness modes of Table~\ref{tab:peers}.}"
        ),
        r"\label{tab:peers-ab}",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{2pt}",
        r"\resizebox{\textwidth}{!}{%",
        r"\begin{tabular}{lllrrrl}",
        r"\toprule",
        r"Payload & Peer & Fields & readcon (s) & peer (s) & Ratio & Interval \\",
        r"\midrule",
    ]

    def add(payload: str, peer: str, row: dict[str, str]) -> None:
        body.append(
            rf"{PAYLOAD[payload]} & {PEER[peer]} & {FIELDS[peer]} & "
            rf"{sig(float(row['median_readcon_s']))} & {sig(float(row['median_peer_s']))} & "
            rf"{sig(float(row['median_ratio_readcon_over_peer']))} & "
            rf"[{sig(float(row['bca_low']))}, {sig(float(row['bca_high']))}] \\"
        )

    for peer in ("ase_eon", "chemfiles_xyz", "h5py", "mda_h5md"):
        add("frames-10000", peer, a_long[peer])
    for peer in ("ase_eon", "chemfiles_xyz", "h5py", "mda_h5md"):
        add("slab", peer, a_slab[peer])
    for peer in ("ase_extxyz", "ase_traj"):
        add("frames-10000", peer, b_long[peer])
    for peer in ("ase_extxyz", "ase_traj"):
        add("slab", peer, b_slab[peer])
    body += [r"\bottomrule", r"\end{tabular}}", r"\end{table}", ""]
    return "\n".join(body)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "peer_table.tex").write_text(HEADER + fairness_table())
    (OUT / "peers_campaign_a.tex").write_text(HEADER + campaign_table())
    print("wrote", OUT / "peer_table.tex")
    print("wrote", OUT / "peers_campaign_a.tex")


if __name__ == "__main__":
    main()
