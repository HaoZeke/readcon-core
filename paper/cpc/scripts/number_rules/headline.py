"""Rules for the generated macros that the front matter and the Conclusions of ``rg_main.tex`` print.

Each rule reads the record the macro's generator reads.  The timing rules take
the cells of ``bench/results/r0174/reads.json``, the record ``r0174_tables.py``
reads, and divide the medians themselves.
"""

from __future__ import annotations

from numbers_lib import ROOT, Rule, grep, jget, text
from r0174_tables import RESULTS

R0174 = RESULTS.resolve().relative_to(ROOT).as_posix()
READS = f"{R0174}/reads.json"
CROSSCHECK = "data/corpus_crosscheck.json"
SD = "evidence/results/e1_restart.json"
E5 = "evidence/results/e5_prefactor.json"


def read_ratio(name: str, reader: str) -> Rule:
    cells = jget(READS, "cells")
    value = (
        cells["python:readcon:1"]["median_s"] / cells[f"python:{reader}:1"]["median_s"]
    )
    return Rule(
        "\\" + name,
        value,
        READS,
        f"cells python:readcon:1 median_s over cells python:{reader}:1 median_s",
    )


PREFACTORS = jget(E5, "prefactor_by_reading")
HELD_HZ = PREFACTORS["correct"]["prefactor_Hz"]
for route, item in PREFACTORS.items():
    if (
        route not in ("correct", "free")
        and f"{item['prefactor_Hz']:.2e}" != f"{HELD_HZ:.2e}"
    ):
        raise SystemExit(
            f"{E5}: the band read back through {route} returns {item['prefactor_Hz']} Hz, "
            f"not the held prefactor {HELD_HZ} Hz at three significant figures"
        )


def dlpack_ratio() -> float:
    view = copy = None
    for line in text("logs/dlpack-exchange.log").splitlines():
        if line.startswith("summary numpy from_dlpack n=32768 "):
            view = float(line.split("median=")[1].split()[0])
        elif line.startswith("summary numpy copy n=32768 "):
            copy = float(line.split("median=")[1].split()[0])
    if view is None or copy is None:
        raise SystemExit("logs/dlpack-exchange.log has no 32768 numpy medians")
    return view / copy


RULES = [
    Rule(
        r"\corpusFiles",
        jget(CROSSCHECK, "groups.all.files"),
        CROSSCHECK,
        "groups.all.files",
    ),
    Rule(
        r"\eFivePrefactorRatioFree",
        PREFACTORS["free"]["prefactor_Hz"] / HELD_HZ,
        E5,
        "prefactor_by_reading.free.prefactor_Hz over prefactor_by_reading.correct.prefactor_Hz",
    ),
    Rule(
        r"\dlpackAtoms",
        int(grep("scripts/bench_dlpack.py", r"SIZES = \(32, 128, 512, 2048, 8192, (\d+)\)")),
        "scripts/bench_dlpack.py",
        "last entry of SIZES, the atom count of the lead DLPack row",
    ),
    Rule(
        r"\dlpackRatio",
        dlpack_ratio(),
        "logs/dlpack-exchange.log",
        "median from_dlpack over median copy at 32768 atoms",
    ),
]
