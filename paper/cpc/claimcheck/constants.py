#!/usr/bin/env python3
"""The natural time unit from the CODATA tables of NIST, and the ordering tests of the paper from SciPy and
statsmodels.

The paper says the natural time unit of eOn and ASE, ``(amu angstrom^2/eV)^0.5``, is 10.18 fs for the CODATA
2018 values of the atomic mass constant and the electronvolt, that a reader taking those velocities as
angstrom/fs scales the kinetic energy by the square of that number, that 11 runs against 11 admit an exact
two-sided Mann-Whitney p as low as 2.8e-6, and that Holm's procedure at level 0.05 rejects 96 of the 101
timing ratios.  This script

1. reads the atomic mass constant and the electronvolt from the CODATA 2018 table of NIST, and from the
   current table for comparison, and computes the unit from each;
2. compares those values with the constants of ``src/units.rs`` of READCON_SRC, with the macros of the
   paper, and with the unit of ``ase.units``, which follows CODATA 2014;
3. recomputes, from the runs of the records, the p of every ratio with ``scipy.stats.mannwhitneyu`` in its
   exact mode and the Holm decisions with ``statsmodels.stats.multitest.multipletests``, and compares them
   with ``scripts/ordering_tests.py``.

The record is ``data/claimcheck/constants.json``.
"""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cc_common as cc  # noqa: E402

TABLES = {
    "codata_2018": "https://physics.nist.gov/cuu/Constants/ArchiveASCII/allascii_2018.txt",
    "codata_current": "https://physics.nist.gov/cuu/Constants/Table/allascii.txt",
}
QUANTITIES = {"atomic mass constant": "m_u", "electron volt": "eV"}
ANGSTROM, FEMTOSECOND = 1e-10, 1e-15
MACROS = {
    "wkTimeFs": "src/figures/generated/worked_macros.tex",
    "velNaturalFs": "src/figures/generated/velocity_case.tex",
    "velNaturalFsBinary": "src/figures/generated/velocity_case.tex",
    "velFactorSq": "src/figures/generated/velocity_case.tex",
    "velEonFs": "src/figures/generated/velocity_case.tex",
    "ordRuns": "src/figures/generated/ordering_macros.tex",
    "ordClaims": "src/figures/generated/ordering_macros.tex",
    "ordRejected": "src/figures/generated/ordering_macros.tex",
    "ordAlpha": "src/figures/generated/ordering_macros.tex",
    "ordPMin": "src/figures/generated/ordering_macros.tex",
}


def table(path: Path) -> dict:
    """The value of each quantity of ``QUANTITIES`` in a NIST ``allascii`` table."""
    out = {}
    for line in path.read_text().splitlines():
        name = line[:60].strip()
        if name in QUANTITIES:
            value = re.match(r"\s*([-0-9. ]+(?:e[-+]?\d+)?)", line[60:]).group(1)
            out[QUANTITIES[name]] = {
                "line": line.rstrip(),
                "value": float(value.replace(" ", "")),
            }
    if set(out) != set(QUANTITIES.values()):
        raise SystemExit(f"{path.name} lacks {set(QUANTITIES.values()) - set(out)}")
    return out


def unit_fs(m_u: float, ev: float) -> float:
    return math.sqrt(m_u * ANGSTROM**2 / ev) / FEMTOSECOND


def macro(name: str) -> str:
    text = (cc.CPC / MACROS[name]).read_text()
    found = re.search(
        r"\\newcommand\{\\" + name + r"\}\{(.*)\}\s*$", text, re.MULTILINE
    )
    if not found:
        raise SystemExit(f"{MACROS[name]} has no macro {name}")
    return found.group(1)


def number(text: str) -> float:
    return float(
        re.sub(r"\\num\{([^}]*)\}", r"\1", text).replace(r"\times10^{", "e").rstrip("}")
    )


def library_constants(src: Path) -> dict:
    text = (src / "src" / "units.rs").read_text()
    out = {}
    for label, names in (("m_u", '&["u", "amu"'), ("eV", '&["ev"]')):
        found = (
            re.search(r"si_factor:\s*([0-9_.e-]+)", text[text.find(names) :])
            if names in text
            else None
        )
        out[label] = float(found.group(1).replace("_", "")) if found else None
    return out


def main() -> int:
    import ase.units
    import scipy
    import scipy.stats
    import statsmodels
    from statsmodels.stats.multitest import multipletests

    sys.path.insert(0, str(cc.CPC / "scripts"))
    import ordering_tests

    downloads, codata = [], {}
    for label, url in TABLES.items():
        path, got = cc.fetch(
            url, Path(url).name if label == "codata_current" else "allascii_2018.txt"
        )
        downloads.append(got)
        values = table(path)
        codata[label] = {
            **values,
            "unit_fs": unit_fs(values["m_u"]["value"], values["eV"]["value"]),
        }
    tau = codata["codata_2018"]["unit_fs"]
    ase_fs = 1 / ase.units.fs
    ase_2018 = ase.units.create_units("2018")
    library = library_constants(cc.readcon_src())
    macros = {name: macro(name) for name in MACROS}
    unit = {
        "codata_2018_fs": tau,
        "codata_current_fs": codata["codata_current"]["unit_fs"],
        "factor_squared_codata_2018": tau**2,
        "library_constants": library,
        "library_constants_equal_codata_2018": library
        == {key: codata["codata_2018"][key]["value"] for key in ("m_u", "eV")},
        "ase_codata_version": ase.units.__codata_version__,
        "ase_fs": ase_fs,
        "ase_relative_difference": (ase_fs - tau) / tau,
        "ase_2018_fs": 1 / ase_2018["fs"],
        "paper_macros": {
            name: macros[name]
            for name in (
                "wkTimeFs",
                "velNaturalFs",
                "velNaturalFsBinary",
                "velFactorSq",
                "velEonFs",
            )
        },
        "velNaturalFs_matches": f"{tau:.9f}" == macros["velNaturalFs"],
        "wkTimeFs_matches": f"{tau:.2f}" == macros["wkTimeFs"],
        "velFactorSq_matches": f"{tau**2:.2f}" == macros["velFactorSq"],
        "parsed_relative_difference": (float(macros["velNaturalFsBinary"]) - tau) / tau,
        "eon_relative_difference": (float(macros["velEonFs"]) - tau) / tau,
    }

    rows = ordering_tests.claims()
    ours = [ordering_tests.mann_whitney(r["x"], r["y"]) for r in rows]
    theirs = [
        float(
            scipy.stats.mannwhitneyu(
                r["x"], r["y"], alternative="two-sided", method="exact"
            ).pvalue
        )
        for r in rows
    ]
    holm_ours = ordering_tests.holm(ours)
    alpha = float(macros["ordAlpha"])
    holm_theirs = [
        bool(v) for v in multipletests(theirs, alpha=alpha, method="holm")[0]
    ]
    n = {len(r["x"]) for r in rows} | {len(r["y"]) for r in rows}
    runs = n.pop() if len(n) == 1 else None
    p_min = 2 / math.comb(2 * runs, runs) if runs else None
    separated = (
        float(
            scipy.stats.mannwhitneyu(
                list(range(runs)),
                list(range(runs, 2 * runs)),
                alternative="two-sided",
                method="exact",
            ).pvalue
        )
        if runs
        else None
    )
    ordering = {
        "scipy": scipy.__version__,
        "statsmodels": statsmodels.__version__,
        "claims": len(rows),
        "runs_per_cell": runs,
        "max_relative_p_difference": max(abs(a - b) / b for a, b in zip(ours, theirs)),
        "rejected_ordering_tests": sum(holm_ours),
        "rejected_statsmodels": sum(holm_theirs),
        "same_decisions": holm_ours == holm_theirs,
        "p_min_exact": p_min,
        "p_min_scipy_separated_samples": separated,
        "paper_macros": {
            name: macros[name]
            for name in ("ordRuns", "ordClaims", "ordRejected", "ordAlpha", "ordPMin")
        },
        "macros_match": (
            str(len(rows)) == macros["ordClaims"]
            and str(sum(holm_theirs)) == macros["ordRejected"]
            and str(runs) == macros["ordRuns"]
            and f"{separated:.1e}" == f"{number(macros['ordPMin']):.1e}"
        ),
    }
    cc.write_record(
        "constants",
        {
            "claim": "The natural time unit is 10.18 fs for the CODATA 2018 constants, a misread scales the kinetic "
            "energy by its square, and the ordering tests of the paper are the exact Mann-Whitney test and "
            "Holm's procedure.",
            "codata": codata,
            "natural_time_unit": unit,
            "ordering": ordering,
        },
        sources=downloads,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
