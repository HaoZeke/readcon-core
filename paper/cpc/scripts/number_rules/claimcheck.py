"""Rules for the numbers that the claim check prints in ``rg_supp.tex`` and its summary in ``rg_main.tex``.

A number that a cited work states comes from the lines of its pinned file that ``claimcheck/sources.py`` keeps
in ``data/claimcheck/sources.json``.  The systems of a benchmark come from the record of the check that ran it,
and the slab of the Sella example from the script that builds it.
"""

from __future__ import annotations

import re

from numbers_lib import Rule, grep, jget

DOC = "rg_supp.tex"
SOURCES = "data/claimcheck/sources.json"
TEXT = "data/claimcheck/cited_text.json"
SELLA_PY = "claimcheck/sella_con.py"


def matched(check: str) -> list[str]:
    """The lines that check ``check`` of the sources record kept, which must have found its statement."""
    found = [c for c in jget(SOURCES, "checks") if c["id"] == check]
    if len(found) != 1 or not found[0]["found"]:
        raise SystemExit(
            f"{SOURCES}: check {check} is missing or did not find its statement"
        )
    return [m["text"] for m in found[0]["matches"]]


def one(check: str, pattern: str) -> str:
    hits = [m.group(1) for text in matched(check) if (m := re.search(pattern, text))]
    if len(hits) != 1:
        raise SystemExit(
            f"{SOURCES}: {len(hits)} lines of check {check} match {pattern!r}"
        )
    return hits[0]


def libxtc_tables() -> tuple[list[int], list[float]]:
    """The atoms of the systems of Table S1 and the libxtc over xdrfile read rates of Tables S3 and S4."""
    atoms: list[int] = []
    rates: list[float] = []
    table, columns = None, []
    for text in matched("libxtc_paper_tables"):
        cells = re.split(r"\s{2,}", text.strip())
        if found := re.match(r"Table (S\d):", cells[0]):
            table, columns = found.group(1), []
        elif cells[0] == "System type":
            columns = cells
        elif table == "S1":
            atoms.append(int(cells[columns.index("Size")]))
        elif table in ("S3", "S4"):
            libxtc = next(i for i, c in enumerate(columns) if c.split()[0] == "libxtc")
            xdrfile = columns.index("xdrfile")
            rates.append(
                float(cells[libxtc].split("±")[0]) / float(cells[xdrfile].split("±")[0])
            )
    if len(atoms) != 3 or len(rates) != 6:
        raise SystemExit(
            f"{SOURCES}: libxtc_paper_tables gives {len(atoms)} systems and {len(rates)} rates"
        )
    return atoms, rates


ATOMS, RATES = libxtc_tables()
CU = sorted(int(n) for n in jget(TEXT, "cu"))

RULES = [
    Rule(
        "300",
        int(one("eon_akmc_pt_config", r"^temperature = (\d+)$")),
        SOURCES,
        "eon_akmc_pt_config, temperature of the config.ini of examples/akmc-pt",
        (f"{DOC}~AKMC example of eOn",),
    ),
    Rule(
        "0.95",
        float(one("eon_akmc_pt_config", r"^confidence = ([\d.]+)$")),
        SOURCES,
        "eon_akmc_pt_config, confidence of the config.ini of examples/akmc-pt",
        (f"{DOC}~AKMC example of eOn",),
    ),
    Rule(
        "111",
        int(grep(SELLA_PY, r'^    slab = fcc(\d+)\("Cu"')),
        SELLA_PY,
        "the ASE surface of the slab that the README example of Sella builds",
        (f"{DOC}~saddle search of its README",),
    ),
    Rule(
        "1.5",
        min(RATES),
        SOURCES,
        "libxtc_paper_tables, least libxtc over xdrfile frames per second of Tables S3 and S4",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "2.8",
        max(RATES),
        SOURCES,
        "libxtc_paper_tables, largest libxtc over xdrfile frames per second of Tables S3 and S4",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "20",
        int(
            one(
                "libxtc_paper_protocol",
                r"^\d+ frames were read (\d+) times for each format$",
            )
        ),
        SOURCES,
        "libxtc_paper_protocol, reads of each format",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "1000",
        int(
            one(
                "libxtc_paper_protocol",
                r"^(\d+) frames were read \d+ times for each format$",
            )
        ),
        SOURCES,
        "libxtc_paper_protocol, frames of each read",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "20138",
        min(ATOMS),
        SOURCES,
        "libxtc_paper_tables, smallest system of Table S1",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "210566",
        max(ATOMS),
        SOURCES,
        "libxtc_paper_tables, largest system of Table S1",
        (f"{DOC}~reports read rates",),
    ),
    Rule(
        "16",
        int(one("libxtc_decoder_commit", r"^Date: \w+, (\d+) \w+ \d+ ")),
        SOURCES,
        "libxtc_decoder_commit, day of the Date line of the patch",
        (f"{DOC}~replaced that stub on",),
    ),
    Rule(
        "2021",
        int(one("libxtc_decoder_commit", r"^Date: \w+, \d+ \w+ (\d+) ")),
        SOURCES,
        "libxtc_decoder_commit, year of the Date line of the patch",
        (f"{DOC}~replaced that stub on",),
    ),
    Rule(
        "2021",
        int(
            one(
                "libxtc_paper_protocol",
                r'<pub-date pub-type="epub">.*<year>(\d+)</year>',
            )
        ),
        SOURCES,
        "libxtc_paper_protocol, year of the electronic publication",
        (f"{DOC}~^The paper appeared on",),
    ),
    Rule(
        "2026.4",
        float(
            one("gromacs_2026.4_version", r"^project\(Gromacs VERSION (\d+\.\d+)\)$")
        ),
        SOURCES,
        "gromacs_2026.4_version, project version of the GROMACS commit",
        (f"{DOC}~the version that GROMACS",),
    ),
    Rule(
        "100",
        jget(TEXT, "water.conversion.frames"),
        TEXT,
        "water.conversion.frames",
        (f"{DOC}~benchmark repository of chemfiles",),
    ),
    Rule(
        "297",
        jget(TEXT, "water.conversion.atoms"),
        TEXT,
        "water.conversion.atoms",
        (f"{DOC}~benchmark repository of chemfiles",),
    ),
    Rule(
        "50",
        int(one("chemfiles_bench_protocol", r"^MAX_REPETITIONS = (\d+)$")),
        SOURCES,
        "chemfiles_bench_protocol, MAX_REPETITIONS of bench.py",
        (f"{DOC}~benchmark repository of chemfiles",),
    ),
    Rule("10", CU[0], TEXT, "smallest key of cu", (f"{DOC}~benchmark of extxyz",)),
    Rule(
        "200000",
        CU[-1],
        TEXT,
        "largest key of cu",
        (
            f"{DOC}~benchmark of extxyz",
            f"{DOC}~On the frame of",
            "rg_main.tex~Cu frame of extxyz",
        ),
    ),
]
