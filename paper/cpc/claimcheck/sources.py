"""Statements of the paper that rest on the source of another code or on a cited paper, checked in that source.

Each check reads one file at a pinned commit, release or page, pinned by URL, size and SHA-256, and keeps every
line that matches its pattern with its line number; an HTML page is read as its text, an XML file keeps the
matched spans of its long lines, and a PDF is read as the text that ``pdftotext -layout`` gives.  A check passes
when the pattern matches and no line matches its ``absent`` pattern, and the record keeps both outcomes.

    python claimcheck/sources.py
"""

from __future__ import annotations

import html
import re

import akmc
import cc_common as cc
from cited_text import CHEMFILES

EON = "https://raw.githubusercontent.com/TheochemUI/eOn"
EON_V360 = "2d2adada2f64c4c5a063065abc0d9a8a56d74b2c"
EON_BOINC = "66629141fb31b72ed836b2f53ad51e5dbb049aed"
EON_SERVER_REMOVAL = "b1e1c5b8ab807652dfa0ec8bf5934cbba8b6da72"
METATOMIC_ASE = "b98ab002e81559ddea31dc095357c86c279ac4bd"
PLUMED = "e5e90a26cd7ba479d70b1240cd5bfd8cd027968e"
ASE_3290 = "f27c0005ae6a67ea419f996e728668865bfc1f86"
LIBXTC = "https://gitlab.com/impulse_md/libxtc"
LIBXTC_DECODER = "215f4d40d7a3c3d8daac82d5a9b9d647a2a06495"
LIBXTC_PAPER = "PMC8017739"
LIBXTC_SUPPLEMENT = (
    "https://static-content.springer.com/esm/art%3A10.1186%2Fs13104-021-05536-5/MediaObjects/"
    "13104_2021_5536_MOESM1_ESM.pdf"
)
GROMACS = "https://gitlab.com/gromacs/gromacs"
GROMACS_2026_4 = "6c69f720aa7378fddfee67f99d51aff21ccfb0a2"

CHECKS = [
    {
        "id": "eon_communicators_v3.6.0",
        "statement": "the communicators that eOn 3.6.0 ships",
        "url": f"{EON}/{EON_V360}/eon/communicator.py",
        "bytes": 29200,
        "sha256": "6bfcb24d3bc3c27ef0bb228dee8676bb894800c1af0e15ebc6a2d6621c664604",
        "pattern": r"^class \w+\(Communicator\)|config\.comm_type (==|in) ",
    },
    {
        "id": "eon_communicators_2018",
        "statement": "the communicators of eOn before the 2018 removal of BOINC",
        "url": f"{EON}/{EON_BOINC}/eon/communicator.py",
        "bytes": 49613,
        "sha256": "cf8a135c90c4c1c1f3285142e56c2ed9d7cc736c52c6dd475da14b89ee65bd53",
        "pattern": r"^class \w+\(Communicator\)",
    },
    {
        "id": "eon_boinc_client_removal",
        "statement": "the commit that removed BOINC from the eOn client",
        "url": f"https://github.com/TheochemUI/eOn/commit/{EON_BOINC}.patch",
        "bytes": 17311,
        "sha256": "2c2d92e59bf75f16e7ab9eb59ebc929c1a77bd4c379548f4c50d135cc7f8041f",
        "pattern": r"^(Date|Subject): ",
    },
    {
        "id": "eon_boinc_server_removal",
        "statement": "the commit that removed the BOINC communicator from the server",
        "url": f"https://github.com/TheochemUI/eOn/commit/{EON_SERVER_REMOVAL}.patch",
        "bytes": 46406,
        "sha256": "e8ffe454b4134f27c8c068a7f942d7d5286bf10503e70cdf794777e8ad3883ab",
        "pattern": r"^(Date|Subject): |^-class \w+\(Communicator\)",
    },
    {
        "id": "eon_readcon_wrap_v3.6.0",
        "statement": "the readcon-core that eOn 3.6.0 builds",
        "url": f"{EON}/{EON_V360}/subprojects/readcon-core.wrap",
        "bytes": 185,
        "sha256": "390ac78c3fcc4ef53076a3e355b522185ff1a11204922c629ce4f742a859ae3a",
        "pattern": r"^(url|revision) = ",
    },
    {
        "id": "h5md_version_pair",
        "statement": "H5MD stores its version as a major and minor pair",
        "url": "https://www.nongnu.org/h5md/releases/h5md-1.1.0.html",
        "bytes": 73048,
        "sha256": "722eb7890872ff60fb28f066d62a1faa053e59c9265f30c9f0881074be2d453f",
        "html": True,
        "pattern": r"version An attribute, of Integer datatype and of simple dataspace of rank 1 and size 2[^.]*\."
        r"|the version attribute contains only x and y",
    },
    {
        "id": "vesin_metatomic_ase",
        "statement": "vesin computes the neighbour lists of the ASE interface of metatomic",
        "url": f"https://raw.githubusercontent.com/metatensor/metatomic/{METATOMIC_ASE}/python/metatomic_ase/src/"
        "metatomic_ase/_neighbors.py",
        "bytes": 5706,
        "sha256": "d2c0082cffa8ec17e3faa2e82d716218adefb18a210ba1e2f83dbe70ba5e83d0",
        "pattern": r"^import vesin|vesin\.metatomic\.NeighborList\(",
    },
    {
        "id": "vesin_plumed_metatomic",
        "statement": "vesin computes the neighbour lists of the PLUMED interface",
        "url": f"https://raw.githubusercontent.com/plumed/plumed2/{PLUMED}/src/metatomic/metatomic.cpp",
        "bytes": 40105,
        "sha256": "d414c25cb1e2aea3cc5a8e7596d6b81deb8ebb065e2602712721267a898cda37",
        "pattern": r'#include "vesin\.h"|vesin::VesinNeighborList neighbors_',
    },
    {
        "id": "vesin_eon_metatomic",
        "statement": "vesin computes the neighbour lists of the eOn interface",
        "url": f"{EON}/{EON_V360}/client/potentials/Metatomic/MetatomicPotential.cpp",
        "bytes": 33499,
        "sha256": "33fbf98b6c1c46f1f48772cd1ecf37f3c15689dcd792b57a88dd185db1583c70",
        "pattern": r'#include "vesin\.h"|vesin_neighbors\(',
    },
    {
        "id": "charmm_formats_in_source",
        "statement": "CHARMM defines its binary files only in its source code",
        "url": "https://academiccharmm.org/documentation/version/c50b1/io",
        "bytes": 81129,
        "sha256": "3bfd216b0e30bca2aaba5262a49036ca2e0622e88107a8359b6b61069b8ba1bf",
        "html": True,
        "pattern": r"The precise format of all these files is described only in the source code[^.]*\.",
    },
    {
        "id": "ase_eon_reader_periodicity",
        "statement": "the eon reader of ASE 3.29.0 builds Atoms without pbc",
        "url": f"https://gitlab.com/ase/ase/-/raw/{ASE_3290}/ase/io/eon.py",
        "bytes": 10532,
        "sha256": "9a66e913b8cd6d0e3793cd74d7f1961db0da2e29b3673e3d7ee3760fae928b4f",
        "pattern": r"^    return Atoms\(|^        (symbols|positions|masses|cell|constraint)=",
        "absent": r"pbc",
    },
    {
        "id": "libxtc_v1.0.0_stub",
        "statement": "the frame decoder of libxtc at its tag v1.0.0 returns false",
        "url": f"{LIBXTC}/-/raw/v1.0.0/libxtc/xtc_unpack.cpp",
        "bytes": 1721,
        "sha256": "583d5c2ab201e7da497b5ecad06cccad43d8bbb62d4aa96d6b47b6e1e5faf72f",
        "pattern": r"^bool unpack_frame\(|^    return false;",
    },
    {
        "id": "libxtc_decoder_commit",
        "statement": "the decoder of libxtc replaced its stub on 16 April 2021",
        "url": f"{LIBXTC}/-/commit/{LIBXTC_DECODER}.patch",
        "bytes": 20392,
        "sha256": "177a9e41d20f3554fb446581546662a2a4cdcc55c0f366b1f671687a8b3a63ad",
        "pattern": r"^(From [0-9a-f]{40} |Date: |Subject: )",
    },
    {
        "id": "gromacs_2026.4_tng",
        "statement": "GROMACS 2026.4 bundles TNG 1.8.4",
        "url": f"{GROMACS}/-/raw/{GROMACS_2026_4}/src/external/tng_io/BuildTNG.cmake",
        "bytes": 4760,
        "sha256": "84258b5b0c44e8b85cfe771c90c7ec275d6a9502c036416cd5a92bd71f38b6c5",
        "pattern": r'^set\(TNG_(MAJOR_VERSION|MINOR_VERSION|VERSION_PATCH_LEVEL) "',
    },
    {
        "id": "gromacs_2026.4_version",
        "statement": "the GROMACS commit of the TNG check is GROMACS 2026.4",
        "url": f"{GROMACS}/-/raw/{GROMACS_2026_4}/CMakeLists.txt",
        "bytes": 43551,
        "sha256": "971117d68471918e95dbcc1c43d366981461648c50a56d0c0b0f94a999b5bd9a",
        "pattern": r"^project\(Gromacs VERSION ",
    },
    {
        "id": "eon_akmc_pt_config",
        "statement": "the AKMC example of eOn 3.6.0 runs the Morse potential of Pt at 300 K "
        "with a confidence of 0.95",
        "url": f"{akmc.RAW}/examples/akmc-pt/config.ini",
        "bytes": akmc.EXAMPLE["config.ini"][0],
        "sha256": akmc.EXAMPLE["config.ini"][1],
        "pattern": r"^(temperature|potential|confidence) = ",
    },
    {
        "id": "chemfiles_bench_protocol",
        "statement": "the benchmark of chemfiles publishes the mean of up to 50 reads "
        "after up to three warm-up reads",
        "url": f"https://raw.githubusercontent.com/chemfiles/benchmarks/{CHEMFILES['commit']}/bench.py",
        "bytes": CHEMFILES["files"]["bench.py"][0],
        "sha256": CHEMFILES["files"]["bench.py"][1],
        "pattern": r"^MAX_(REPETITIONS|TIME_SECONDS) = |^    for _ in range\(\d+\):$|^    runtime /= repetitions$",
    },
    {
        "id": "libxtc_paper_protocol",
        "statement": "the libxtc paper reads 1000 frames 20 times, appeared on 1 April 2021 "
        "and gives its data on request",
        "url": f"https://www.ebi.ac.uk/europepmc/webservices/rest/{LIBXTC_PAPER}/fullTextXML",
        "bytes": 39221,
        "sha256": "36f0bab828f02b6259c6a947124cdb74aff853bab8bf8f29fc0e656ef6860a3e",
        "xml": True,
        "pattern": r"\d+ frames were read \d+ times for each format"
        r'|<pub-date pub-type="epub"><day>\d+</day><month>\d+</month><year>\d+</year>'
        r"|available from the corresponding author on reasonable request",
    },
    {
        "id": "libxtc_paper_tables",
        "statement": "the systems, machines and read rates of the supplementary tables of "
        "the libxtc paper",
        "url": LIBXTC_SUPPLEMENT,
        "bytes": 154847,
        "sha256": "a0d4fec2485a3445f5d32611279d25a092d269c2e8a99c413962c0343c4347c9",
        "pdf": True,
        "pattern": r"^\s*Table S\d:|^System type |^(Small|Medium|Big) +\d|^CPU ",
    },
]
KINDS = ("html", "xml", "pdf")


def lines_of(path, kind: str | None) -> list[str]:
    if kind == "pdf":
        return cc.run(["pdftotext", "-layout", path, "-"])["stdout"].split("\n")
    text = path.read_text(encoding="utf-8")
    if kind == "html":
        return [
            html.unescape(re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", text))).strip()
        ]
    return text.splitlines()


def main() -> int:
    results, sources = [], []
    for check in CHECKS:
        kind = next((k for k in KINDS if check.get(k)), None)
        name = f"source-{check['id']}" + (f".{kind}" if kind else "")
        path, got = cc.fetch(check["url"], name, check["sha256"], check["bytes"])
        sources.append(got)
        pattern = re.compile(check["pattern"])
        lines = lines_of(path, kind)
        matches = []
        for number, line in enumerate(lines, start=1):
            if kind == "html":
                matches += [{"text": m.group(0)} for m in pattern.finditer(line)]
            elif kind == "xml":
                matches += [
                    {"line": number, "text": m.group(0)} for m in pattern.finditer(line)
                ]
            elif pattern.search(line):
                matches.append({"line": number, "text": line.rstrip()})
        result = {
            "id": check["id"],
            "statement": check["statement"],
            "url": check["url"],
            "sha256": got["sha256"],
            "pattern": check["pattern"],
            "matches": matches,
        }
        if kind == "pdf":
            result["text_by"] = cc.run(["pdftotext", "-v"])["stderr"].splitlines()[0]
        if "absent" in check:
            result["absent"] = check["absent"]
            result["lines_matching_absent"] = sum(
                bool(re.search(check["absent"], line)) for line in lines
            )
        result["found"] = bool(matches) and not result.get("lines_matching_absent", 0)
        results.append(result)
        print(
            check["id"],
            "found" if result["found"] else "NOT FOUND",
            len(matches),
            flush=True,
        )
    cc.write_record(
        "sources",
        {
            "claim": "statements of the paper that rest on the source of another code or on a cited paper",
            "checks": results,
            "found": sum(r["found"] for r in results),
            "checked": len(results),
        },
        sources,
    )
    return 0 if all(r["found"] for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
