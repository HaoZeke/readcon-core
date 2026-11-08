#!/usr/bin/env python3
"""The macros and tables of the claim checks of the CPC manuscript, from the records of ``data/claimcheck``.

* ``claimcheck_macros.tex``    the counts, times and ratios that the main text and the claim-check section print;
* ``claimcheck_files.tex``     per archive, the CON files and the files readcon and ASE read (``eon_files``);
* ``claimcheck_akmc.tex``      per seed of the AKMC example of eOn, the steps, states, searches, processes, simulated
                               time and wall time (``akmc``);
* ``claimcheck_xtb.tex``       the runs of the xtb band in eOn: times, force calls, the band against the energies
                               eOn prints, the CON files each reader reads, and the band files from run to run
                               (``xtb_neb``);
* ``claimcheck_sella.tex``     per route of the Sella slab, the periodic flags read, the steps, the saddle energy,
                               the saddle against the one from memory, and the median and IQR (``sella_con``);
* ``claimcheck_libxtc.tex``    per trajectory and reader, the largest difference from xdrfile and the atom-frames
                               beyond the XTC precision (``binary_cited``);
* ``claimcheck_tng.tex``       per TNG example file, build and read, the frames read or the error, and the
                               largest difference from the all-frames read of the TNG of the campaign
                               (``binary_cited``);
* ``claimcheck_binary.tex``    per file and reader, the median and IQR of the all-frames read and the ratio of
                               readcon to it (``binary_cited``);
* ``claimcheck_text.tex``      per water file of the chemfiles benchmark and reader, the median and IQR and the
                               ratio of readcon to it, beside the time the benchmark publishes (``cited_text``);
* ``claimcheck_extxyz.tex``    the same per Cu frame of the extxyz benchmark (``cited_text``);
* ``claimcheck_sources.tex``   per source check of a statement, the statement, the pinned file and the lines it
                               holds (``sources``);
* ``claimcheck_cited.tex``     the same per source check of a number of a cited work that the claim check
                               quotes, the checks of ``CITED`` (``sources``).

A ratio is the readcon median over the median of the other reader, as in every other table of the paper.  The
script stops when a record is missing, when a source check failed, when a run of a record exited with an error,
when the readcon route of the Sella slab or a readcon read of a converted file differs from its source, or when a
result the prose states exactly comes out otherwise: the two reads of the TNG of the campaign, the image against
its CON file, and the band files of xtb on one thread.

    python scripts/claimcheck_tables.py
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from binary_formats_tables import figures, grouped

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src" / "figures" / "generated"
RECORDS = ROOT / "data" / "claimcheck"
NAMES = (
    "eon_files",
    "akmc",
    "xtb_neb",
    "sella_con",
    "sources",
    "binary_cited",
    "cited_text",
    "decimals",
    "constants",
)
RECORD = "data/claimcheck/{}.json"
HEAD = "% Written by paper/cpc/scripts/claimcheck_tables.py from {}.\n"
TEXT_READERS = {
    "chemfiles": "chemfiles",
    "ase": "ASE",
    "mdanalysis": "MDAnalysis",
    "openbabel": "Open Babel",
    "ase_extxyz": "ASE extxyz",
    "cextxyz": "cextxyz plugin",
    "read_dicts_regex": r"\texttt{read\_dicts}, regex",
    "read_dicts": r"\texttt{read\_dicts}",
    "readcon": r"readcon \texttt{read\_con}",
    "readcon_arrays": r"readcon \texttt{read\_arrays}",
}
OURS = ("readcon", "readcon_arrays")
BINARY_READERS = {
    "xdrfile": "xdrfile",
    "libxtc": "libxtc",
    "libxtc-plain": "libxtc, plain flags",
    "tng": "TNG",
    "readcon": "readcon CON, two decimals",
    "readcon-image": "readcon image",
}
CITED = (
    "gromacs_2026.4_version",
    "eon_akmc_pt_config",
    "chemfiles_bench_protocol",
    "libxtc_paper_protocol",
    "libxtc_paper_tables",
)


def load() -> dict[str, dict]:
    missing = [name for name in NAMES if not (RECORDS / f"{name}.json").is_file()]
    if missing:
        raise SystemExit(f"data/claimcheck lacks {missing}")
    return {name: json.loads((RECORDS / f"{name}.json").read_text()) for name in NAMES}


def head(*names: str) -> str:
    """The header of a generated file written from the records ``names``, each named by its path."""
    paths = [RECORD.format(n) for n in names]
    return HEAD.format(
        paths[0] if len(paths) == 1 else ", ".join(paths[:-1]) + " and " + paths[-1]
    )


def runs(numbers: list[int]) -> str:
    """Sorted line numbers, with each run of consecutive lines as one range."""
    spans: list[list[int]] = []
    for n in numbers:
        if spans and n == spans[-1][1] + 1:
            spans[-1][1] = n
        else:
            spans.append([n, n])
    return ", ".join(str(a) if a == b else f"{a}--{b}" for a, b in spans)


def enot(x: float, digits: int = 2) -> str:
    """``x`` to ``digits`` significant figures in the e-notation of siunitx."""
    if x == 0:
        return "0"
    mantissa, exponent = f"{x:.{digits - 1}e}".split("e")
    return f"{mantissa}e{int(exponent)}"


def tex(text: str) -> str:
    return (
        text.replace("\\", r"\textbackslash{}")
        .replace("_", r"\_")
        .replace("&", r"\&")
        .replace("%", r"\%")
        .replace("#", r"\#")
        .replace("$", r"\$")
        .replace("{", r"\{")
        .replace("}", r"\}")
        .replace("~", r"\~{}")
        .replace("^", r"\^{}")
    )


def stack(align: str, *lines: str) -> str:
    """A header cell of one line per entry of ``lines``."""
    joined = r"\\".join(lines)
    return rf"\begin{{tabular}}[b]{{@{{}}{align}@{{}}}}{joined}\end{{tabular}}"


def table(head: str, spec: str, header: str, rows: list[str]) -> str:
    return "\n".join(
        [
            head.rstrip("\n"),
            rf"\begin{{tabular}}{{{spec}}}",
            r"\toprule",
            header + r"\\",
            r"\midrule",
            *rows,
            r"\bottomrule",
            r"\end{tabular}",
            "",
        ]
    )


def check(rec: dict[str, dict]) -> None:
    failed = [c["id"] for c in rec["sources"]["checks"] if not c["found"]]
    if failed:
        raise SystemExit(f"sources.json: the checks {failed} failed")
    akmc = rec["akmc"]["summary"]
    if akmc["runs_exit_0"] != akmc["runs"]:
        raise SystemExit("akmc.json: a seed exited with an error")
    reads = rec["akmc"]["con_reads"]["summary"]
    if sorted(reads["refused_by_readcon_only"]) != sorted(
        akmc["copies_of_the_example_pos_con"]
    ) or reads["readcon"]["refused"] != len(akmc["copies_of_the_example_pos_con"]):
        raise SystemExit("akmc.json: readcon refused a file that the runs wrote")
    hosts = {json.dumps(r["hardware"], sort_keys=True) for r in rec.values()}
    if len(hosts) != 1:
        raise SystemExit(f"data/claimcheck: the records ran on {len(hosts)} hosts")
    sella = rec["sella_con"]["summary"]
    if not (
        sella["readcon"]["same_start_as_memory"]
        and sella["readcon"]["same_saddle_as_memory"]
    ):
        raise SystemExit(
            "sella_con.json: the readcon route left the start or the saddle of the run from memory"
        )
    cited = rec["cited_text"]
    if cited["water"]["conversion"]["con_vs_xyz_largest_difference_A"] != 0.0:
        raise SystemExit("cited_text.json: the CON copy of water.xyz differs from it")
    if any(
        entry["con_positions_largest_difference_A"]
        or entry["con_forces_largest_difference"]
        for entry in cited["cu"].values()
    ):
        raise SystemExit("cited_text.json: a CON copy of a Cu file differs from it")
    binary = rec["binary_cited"]
    bundled = next(
        c for c in rec["sources"]["checks"] if c["id"] == "gromacs_2026.4_tng"
    )
    bundled = ".".join(
        re.search(r'"(\d+)"', m["text"]).group(1) for m in bundled["matches"]
    )
    if bundled != binary["tng_builds"]["campaign"]["version"]:
        raise SystemExit(
            f"sources.json: GROMACS 2026.4 bundles TNG {bundled}, not the TNG of the campaign"
        )
    for name, entry in binary["checks"].items():
        if "con" in entry and entry["con"]["positions"]["atom_frames_beyond"]:
            raise SystemExit(
                f"binary_cited.json: the CON copy of {name} leaves its two-decimal rounding"
            )
        for mode in ("all", "frames") if "tng_read" in entry else ():
            if (
                entry["tng_read"][f"campaign-{mode}"]["vs_campaign_all"].get(
                    "largest_difference_A"
                )
                != 0.0
            ):
                raise SystemExit(
                    f"binary_cited.json: the {mode} read of the TNG of the campaign differs on {name}"
                )
    if not all(
        image["checksum_of_the_con_read"] for image in binary["images"].values()
    ):
        raise SystemExit(
            "binary_cited.json: an image reads other positions than its CON file"
        )
    one = rec["xtb_neb"]["one_thread"]
    if (
        one["bands"]["distinct_neb_con_sha256"] != 1
        or not one["same_printed_as_the_campaign"]
    ):
        raise SystemExit("xtb_neb.json: the runs on one thread differ")


def searched(state: dict) -> int:
    """The saddle searches of one AKMC state: its rows of ``search_results.txt`` but the reverse process, the one
    row whose result is ``reverse from N``."""
    return state["searches"] - sum(
        n for result, n in state["results"].items() if result.isdigit()
    )


def median(timing: dict, cell: str) -> float:
    return timing["cells"][cell]["median_s"]


def macros(rec: dict[str, dict]) -> dict[str, list[tuple[str, str]]]:
    """The macros of ``claimcheck_macros.tex``, grouped by the record each is read from."""
    akmc = rec["akmc"]
    summary = akmc["summary"]
    per_state = [
        searched(s)
        for r in akmc["runs"]
        for s in r["states"]
        if s["state"] in {step["reactant"] for step in r["steps"]}
    ]
    per_run = [sum(searched(s) for s in r["states"]) for r in akmc["runs"]]
    simulated = [r["simulated_time_s"] for r in akmc["runs"]]
    files = rec["eon_files"]["archives"]
    eon, workflow = (
        files["eon-v3.6.0"]["summary"],
        files["neb-orchestrator-data"]["summary"],
    )
    example = files["eon-v3.6.0"]["files"]["examples/akmc-pt/pos.con"]["ase"]
    missing = akmc["package_missing"]
    error = akmc["server_as_packaged"]["stderr_tail"].strip().splitlines()[-1]
    hardware = akmc["hardware"]
    ordering = rec["constants"]["ordering"]
    sella = rec["sella_con"]["summary"]
    xtb = rec["xtb_neb"]
    xtb_cell = xtb["timing"]["cells"]["eon_xtb"]
    sources = {c["id"]: c for c in rec["sources"]["checks"]}
    removal = next(
        m["text"]
        for m in sources["eon_boinc_server_removal"]["matches"]
        if m["text"].startswith("Date")
    )
    wrap = next(
        m["text"]
        for m in sources["eon_readcon_wrap_v3.6.0"]["matches"]
        if m["text"].startswith("revision")
    )
    decimals = rec["decimals"]["total"]
    constants = rec["constants"]["natural_time_unit"]
    binary = rec["binary_cited"]
    md = binary["checks"]["md.xtc"]
    pep = binary["checks"]["pep.xtc"]
    ratios = binary["ratios"]
    builds = binary["tng_builds"]
    tag = next(b for b in builds if b != "campaign")
    tng = {name: e for name, e in binary["checks"].items() if name.endswith(".tng")}
    tag_frames = [
        e["tng_read"][f"{tag}-frames"]["vs_campaign_all"] for e in tng.values()
    ]
    tag_off = {
        c["atom_frames_beyond"] // e["tng_read"]["campaign-all"]["printed"]["atoms"]
        for c, e in zip(tag_frames, tng.values())
    }
    if len(tag_off) != 1:
        raise SystemExit(
            f"binary_cited.json: the frame reads of TNG {tag} miss {sorted(tag_off)} frames"
        )
    tag_all = [e["tng_read"][f"{tag}-all"]["printed"] for e in tng.values()]
    tag_error = next(p["error"] for p in tag_all if "error" in p)
    text = rec["cited_text"]["ratios"]
    versions = rec["cited_text"]["versions"]
    out = {
        "akmc": [
            (
                "ccHost",
                tex(
                    f"{hardware['uname_m']}, {hardware['nproc']} CPUs, {hardware['cpu_model']}"
                )
                .replace("(R)", r"\textregistered{}")
                .replace("(TM)", r"\texttrademark{}"),
            ),
            ("ccAkmcMissing", str(len(missing["not_in_package"]))),
            ("ccAkmcDeps", str(len(missing["pyproject_dependencies"]))),
            ("ccAkmcPackageError", tex(error)),
            ("ccAkmcConCopies", str(len(summary["copies_of_the_example_pos_con"]))),
            ("ccAkmcRuns", str(summary["runs"])),
            ("ccAkmcSteps", str(akmc["max_kmc_steps"])),
            ("ccAkmcSearches", grouped(sum(per_run))),
            ("ccAkmcStateMin", grouped(min(per_state))),
            ("ccAkmcStateMax", grouped(max(per_state))),
            ("ccAkmcRunMin", grouped(min(per_run))),
            ("ccAkmcRunMax", grouped(max(per_run))),
            ("ccAkmcSimMin", enot(min(simulated))),
            ("ccAkmcSimMax", enot(max(simulated))),
            ("ccAkmcConFiles", grouped(summary["con_files_written"])),
            ("ccAkmcConRead", grouped(akmc["con_reads"]["summary"]["readcon"]["read"])),
        ],
        "eon_files": [
            ("ccAkmcAtoms", grouped(int(example["atoms"]))),
            ("ccEonFiles", grouped(eon["files"])),
            ("ccEonAseRead", grouped(eon["ase"]["read"])),
            ("ccWorkflowFiles", grouped(workflow["files"])),
            ("ccWorkflowRead", grouped(workflow["readcon"]["read"])),
            ("ccWorkflowAseRead", grouped(workflow["ase"]["read"])),
        ],
        "sella_con": [
            ("ccSellaSteps", str(sella["readcon"]["steps"])),
            ("ccSellaEnergy", figures(float(sella["readcon"]["energy"]), 5)),
            ("ccSellaMedian", figures(sella["readcon"]["median_s"], 2)),
            ("ccSellaAseSteps", str(sella["ase"]["steps"])),
            ("ccSellaAseEnergy", figures(float(sella["ase"]["energy"]), 5)),
            ("ccSellaAseMedian", figures(sella["ase"]["median_s"], 3)),
            (
                "ccSellaAseRatio",
                figures(sella["ase"]["median_s"] / sella["readcon"]["median_s"], 2),
            ),
        ],
        "xtb_neb": [
            ("ccXtbForceCalls", grouped(xtb_cell["printed"]["force_calls"])),
            ("ccXtbImages", str(len(xtb_cell["printed"]["image_energies_eV"]))),
            ("ccXtbBarrier", figures(xtb["band"]["highest_energy_eV"], 4)),
            ("ccXtbMedian", figures(median(xtb["timing"], "eon_xtb"), 2)),
            ("ccXtbIqr", figures(xtb_cell["iqr_s"], 2)),
            ("ccXtbRuns", str(xtb_cell["n"])),
            ("ccXtbBandRuns", str(xtb["band_files_of_the_campaign"]["runs"])),
            (
                "ccXtbDistinct",
                str(xtb["band_files_of_the_campaign"]["distinct_neb_con_sha256"]),
            ),
            ("ccXtbOneThreadRuns", str(xtb["one_thread"]["bands"]["runs"])),
            ("ccXtbResultsDiff", enot(xtb["band"]["largest_difference_eV"])),
            ("ccXtbConFiles", grouped(xtb["con_reads"]["summary"]["files"])),
            (
                "ccXtbSpread",
                enot(xtb["band_files_of_the_campaign"]["largest_position_spread_A"]),
            ),
            (
                "ccXtbEnergySpread",
                enot(xtb["band_files_of_the_campaign"]["largest_energy_spread_eV"]),
            ),
        ],
        "sources": [
            ("ccBoincYear", re.search(r"\b(19|20)\d\d\b", removal).group(0)),
            ("ccEonWrap", tex(wrap.split("=", 1)[1].strip())),
        ],
        "decimals": [
            ("ccDecimals", grouped(decimals["finite_decimals"])),
            ("ccDecimalStrings", grouped(decimals["strings"])),
            (
                "ccDecimalMismatches",
                grouped(
                    decimals["read_arrays_bit_mismatches"]
                    + decimals["read_con_string_bit_mismatches"]
                ),
            ),
            ("ccWriterValues", grouped(decimals["writer_distinct_values"])),
            (
                "ccWriterFailures",
                grouped(
                    decimals["writer_round_trip_failures"]
                    + decimals["writer_longer_than_shortest"]
                    + decimals["writer_same_length_other_digits"]
                ),
            ),
        ],
        "constants": [
            ("ccNaturalFs", f"{constants['codata_2018_fs']:.9f}"),
            ("ccScipy", tex(ordering["scipy"])),
            ("ccStatsmodels", tex(ordering["statsmodels"])),
            ("ccAseCodata", constants["ase_codata_version"]),
            ("ccAseUnitDiff", enot(abs(constants["ase_relative_difference"]))),
        ],
        "binary_cited": [
            (
                "ccLibxtcMdPercent",
                figures(md["read_libxtc"]["vs_xdrfile"]["percent_beyond"], 2)
                if md["read_libxtc"]["vs_xdrfile"]["compared"]
                else "--",
            ),
            (
                "ccLibxtcPlainMdPercent",
                figures(md["read_libxtc_plain"]["vs_xdrfile"]["percent_beyond"], 2)
                if md["read_libxtc_plain"]["vs_xdrfile"]["compared"]
                else "--",
            ),
            (
                "ccLibxtcPepPercent",
                figures(pep["read_libxtc"]["vs_xdrfile"]["percent_beyond"], 2)
                if pep["read_libxtc"]["vs_xdrfile"]["compared"]
                else "--",
            ),
            ("ccLibxtcMdAtoms", grouped(md["read_libxtc"]["printed"]["atoms"])),
            ("ccLibxtcMdFrames", str(md["read_libxtc"]["printed"]["frames"])),
            (
                "ccLibxtcMdMax",
                figures(md["read_libxtc"]["vs_xdrfile"]["largest_difference_A"], 4),
            ),
            (
                "ccLibxtcPepOff",
                grouped(pep["read_libxtc"]["vs_xdrfile"]["atom_frames_beyond"]),
            ),
            (
                "ccLibxtcPepPositions",
                grouped(pep["read_libxtc"]["vs_xdrfile"]["atom_frames"]),
            ),
            ("ccRatioLibxtcXdrfile", figures(ratios["libxtc_over_xdrfile_md"], 2)),
            ("ccRatioMdXdrfile", figures(ratios["readcon_two_over_xdrfile_md"], 2)),
            ("ccRatioMdLibxtc", figures(ratios["readcon_two_over_libxtc_md"], 2)),
            (
                "ccRatioMdImageXdrfile",
                figures(ratios["readcon_image_over_xdrfile_md"], 2),
            ),
            ("ccRatioArgonTng", figures(ratios["readcon_two_over_tng_argon"], 2)),
            ("ccRatioWaterTng", figures(ratios["readcon_two_over_tng_water"], 2)),
            (
                "ccRatioArgonImageTng",
                figures(ratios["readcon_image_over_tng_argon"], 2),
            ),
            (
                "ccRatioWaterImageTng",
                figures(ratios["readcon_image_over_tng_water"], 2),
            ),
            ("ccTngVersion", builds["campaign"]["version"]),
            ("ccTngCommit", builds["campaign"]["commit"][:7]),
            ("ccTngTag", tex(tag)),
            ("ccTngTagCommit", builds[tag]["commit"][:7]),
            ("ccTngTagFramesOff", str(tag_off.pop())),
            (
                "ccTngTagMax",
                figures(max(c["largest_difference_A"] for c in tag_frames), 3),
            ),
            ("ccTngTagError", tex(tag_error.split(":", 1)[1].strip())),
            ("ccTngFiles", str(len(tng))),
            (
                "ccTngArgonAtoms",
                grouped(
                    tng["argon_npt_compressed.tng"]["tng_read"]["campaign-all"][
                        "printed"
                    ]["atoms"]
                ),
            ),
            (
                "ccTngWaterAtoms",
                grouped(
                    tng["water_npt_compressed_trjconv.tng"]["tng_read"]["campaign-all"][
                        "printed"
                    ]["atoms"]
                ),
            ),
            (
                "ccTngFrames",
                str(
                    tng["argon_npt_compressed.tng"]["tng_read"]["campaign-all"][
                        "printed"
                    ]["frames"]
                ),
            ),
        ],
        "cited_text": [
            (
                "ccRatioWaterChemfiles",
                figures(text["readcon_over_chemfiles-water.xyz"], 2),
            ),
            ("ccRatioWaterAse", figures(text["readcon_over_ase-water.xyz"], 2)),
            ("ccRatioWaterMda", figures(text["readcon_over_mdanalysis-water.xyz"], 2)),
            (
                "ccRatioWaterGzChemfiles",
                figures(text["readcon_over_chemfiles-water.xyz.gz"], 2),
            ),
            ("ccRatioWaterGzAse", figures(text["readcon_over_ase-water.xyz.gz"], 2)),
            ("ccRatioCuDicts", figures(text["readcon_over_read_dicts-cu200000"], 2)),
            ("ccRatioCuAse", figures(text["readcon_over_ase_extxyz-cu200000"], 2)),
            ("ccRatioCuPlugin", figures(text["readcon_over_cextxyz-cu200000"], 2)),
            (
                "ccRatioWaterArraysChemfiles",
                figures(text["readcon_arrays_over_chemfiles-water.xyz"], 2),
            ),
            (
                "ccRatioWaterGzArraysChemfiles",
                figures(text["readcon_arrays_over_chemfiles-water.xyz.gz"], 2),
            ),
            (
                "ccRatioWaterArraysAse",
                figures(text["readcon_arrays_over_ase-water.xyz"], 2),
            ),
            (
                "ccRatioWaterArraysMda",
                figures(text["readcon_arrays_over_mdanalysis-water.xyz"], 2),
            ),
            (
                "ccRatioWaterArraysOpenbabel",
                figures(text["readcon_arrays_over_openbabel-water.xyz"], 2),
            ),
            (
                "ccRatioCuArraysDicts",
                figures(text["readcon_arrays_over_read_dicts-cu200000"], 2),
            ),
            (
                "ccRatioCuArraysAse",
                figures(text["readcon_arrays_over_ase_extxyz-cu200000"], 2),
            ),
            (
                "ccRatioCuArraysPlugin",
                figures(text["readcon_arrays_over_cextxyz-cu200000"], 2),
            ),
            *[
                (f"ccVer{name}", tex(versions[dist]))
                for name, dist in (
                    ("Chemfiles", "chemfiles"),
                    ("Ase", "ase"),
                    ("Mda", "MDAnalysis"),
                    ("Openbabel", "openbabel"),
                    ("Extxyz", "extxyz"),
                    ("AseExtxyz", "ase-extxyz"),
                )
            ],
        ],
    }
    names = [n for group in out.values() for n, _ in group]
    if len(names) != len(set(names)) or any(
        not re.fullmatch(r"[A-Za-z]+", n) for n in names
    ):
        raise SystemExit("macro names must be distinct and letters only")
    if set(out) != set(NAMES):
        raise SystemExit(
            f"the macros come from {sorted(out)}, not from the records {sorted(NAMES)}"
        )
    return out


def files_table(rec: dict) -> str:
    rows = []
    readers = {
        k: rec["archives"]["eon-v3.6.0"]["readers"][k]
        for k in ("readcon", "readcon_release", "ase")
    }
    if any(
        a["readers"] != rec["archives"]["eon-v3.6.0"]["readers"]
        for a in rec["archives"].values()
    ):
        raise SystemExit("eon_files.json: the archives were read by different readers")
    for archive, label in (
        ("eon-v3.6.0", "eOn 3.6.0 release"),
        ("neb-orchestrator-data", "workflow archive"),
    ):
        s = rec["archives"][archive]["summary"]
        same = (
            "yes"
            if s["readcon_vs_ase_largest_difference"] == 0.0
            else enot(s["readcon_vs_ase_largest_difference"])
        )
        rows.append(
            f"{label} & {grouped(s['files'])} & {grouped(s['readcon_release']['read'])} & "
            f"{grouped(s['readcon']['read'])} & {grouped(s['ase']['read'])} & "
            f"{grouped(s['readcon_vs_ase_files_compared'])} & {same}\\\\"
        )
    return table(
        head("eon_files"),
        "@{}lrrrrrl@{}",
        f"Archive & {stack('r', 'CON', 'files')} & {stack('r', 'readcon', readers['readcon_release'])} & "
        f"{stack('r', 'readcon', readers['readcon'])} & {stack('r', 'ASE', readers['ase'])} & "
        f"{stack('r', 'Read by', 'both')} & {stack('l', 'Same', 'positions')}",
        rows,
    )


def akmc_table(rec: dict) -> str:
    rows = []
    for run in rec["runs"]:
        states = len(run["states"])
        processes = sum(
            s["processes"] - (s["searches"] - searched(s)) for s in run["states"]
        )
        rows.append(
            f"{run['seed']} & {run['kmc_steps']} & {states} & "
            f"{grouped(sum(searched(s) for s in run['states']))} & "
            f"{grouped(processes)} & \\num{{{enot(run['simulated_time_s'])}}} & "
            f"{figures(run['wall_seconds'], 3)}\\\\"
        )
    return table(
        head("akmc"),
        "@{}rrrrrrr@{}",
        r"Seed & KMC steps & States & Searches & Processes & Simulated time (s) & Wall time (s)",
        rows,
    )


def xtb_table(rec: dict) -> str:
    cell = rec["timing"]["cells"]["eon_xtb"]
    band, campaign, one = (
        rec["band"],
        rec["band_files_of_the_campaign"],
        rec["one_thread"]["bands"],
    )
    summary, readers = rec["con_reads"]["summary"], rec["con_reads"]["readers"]
    labels = {
        "readcon": f"readcon {readers['readcon']}",
        "readcon_release": f"readcon {readers['readcon_release']}",
        "ase": f"ASE {readers['ase']}",
    }
    read = ", ".join(
        f"{label} {grouped(summary[key]['read'])}" for key, label in labels.items()
    )
    rows = [
        f"Timed runs & {cell['n']}\\\\",
        f"Median, IQR (s) & {figures(cell['median_s'], 3)}, {figures(cell['iqr_s'], 2)}\\\\",
        f"Force calls & {grouped(cell['printed']['force_calls'])}\\\\",
        f"Images & {band['frames']}\\\\",
        f"Highest image, energy (eV) & {band['highest_image']}, {figures(band['highest_energy_eV'], 4)}\\\\",
        f"Band file against \\texttt{{results.dat}} (eV) & \\num{{{enot(band['largest_difference_eV'])}}}\\\\",
        f"CON files of a run, read & {grouped(summary['files'])}; {read}\\\\",
        f"Distinct band files, default threads & {campaign['distinct_neb_con_sha256']} of {campaign['runs']}\\\\",
        (
            f"Largest spread, positions (\\AA), energies (eV) & "
            f"\\num{{{enot(campaign['largest_position_spread_A'])}}}, "
            f"\\num{{{enot(campaign['largest_energy_spread_eV'])}}}\\\\"
        ),
        f"Distinct band files, one thread & {one['distinct_neb_con_sha256']} of {one['runs']}\\\\",
    ]
    return table(head("xtb_neb"), "@{}ll@{}", r"Quantity & Value", rows)


def sella_table(rec: dict) -> str:
    rows = []
    labels = {
        "memory": "in memory",
        "readcon": "readcon CON file",
        "ase": "ASE CON file",
    }
    for route in ("memory", "readcon", "ase"):
        s = rec["summary"][route]
        pbc = "".join("T" if flag else "F" for flag in s["pbc"])
        same = "yes" if s["same_saddle_as_memory"] else "no"
        rows.append(
            f"{labels[route]} & {pbc} & {s['steps']} & {figures(float(s['energy']), 5)} & {same} & "
            f"{figures(s['median_s'], 3)} & {figures(s['iqr_s'], 2)}\\\\"
        )
    return table(
        head("sella_con"),
        "@{}lcrrcrr@{}",
        f"Start & Periodic & Steps & {stack('r', 'Saddle', 'energy (eV)')} & "
        f"{stack('c', 'Saddle of', 'memory')} & {stack('r', 'Median', '(s)')} & {stack('r', 'IQR', '(s)')}",
        rows,
    )


def libxtc_table(rec: dict) -> str:
    rows = []
    for name in ("md.xtc", "pep.xtc"):
        entry = rec["checks"][name]
        readers = [
            ("MDAnalysis", entry["mdanalysis_vs_xdrfile"]),
            ("libxtc, setup.py flags", entry["read_libxtc"]["vs_xdrfile"]),
            ("libxtc, plain flags", entry["read_libxtc_plain"]["vs_xdrfile"]),
        ]
        if name == "pep.xtc":
            readers.append(
                (r"\texttt{pep.npy} of the libxtc tests", entry["pep_npy_vs_xdrfile"])
            )
        for label, cmp in readers:
            if not cmp["compared"]:
                rows.append(
                    rf"\texttt{{{name}}} & {label} & \multicolumn{{3}}{{l}}{{no read}}\\"
                )
                continue
            rows.append(
                rf"\texttt{{{name}}} & {label} & \num{{{enot(cmp['largest_difference_A'])}}} & "
                f"{grouped(cmp['atom_frames_beyond'])} of {grouped(cmp['atom_frames'])} & "
                f"{figures(cmp['percent_beyond'], 2) if cmp['percent_beyond'] else '0'}\\\\"
            )
    return table(
        head("binary_cited"),
        "@{}llrrr@{}",
        r"File & Reader & Largest difference (\AA) & Atom-frames beyond & Percent",
        rows,
    )


def binary_reader(cell: str, tng: str) -> str:
    if cell.endswith("-image"):
        return BINARY_READERS["readcon-image"]
    if cell.startswith("tng-"):
        return f"{BINARY_READERS['tng']} {tng}"
    return BINARY_READERS[
        "libxtc-plain" if cell.startswith("libxtc-plain") else cell.split("-")[0]
    ]


def binary_table(rec: dict) -> str:
    cells = rec["timing"]["cells"]
    groups = (
        ("md.xtc", ("xdrfile-md", "libxtc-md", "libxtc-plain-md"), "md"),
        ("argon", ("tng-argon",), "argon"),
        ("water", ("tng-water",), "water"),
    )
    rows = []
    for label, theirs, stem in groups:
        con, image = cells[f"readcon-{stem}-two"], cells[f"readcon-{stem}-image"]
        for cell in (*theirs, f"readcon-{stem}-two", f"readcon-{stem}-image"):
            c = cells[cell]
            ratios = (
                f"{figures(con['median_s'] / c['median_s'], 2)} & {figures(image['median_s'] / c['median_s'], 2)}"
                if cell in theirs
                else "&"
            )
            rows.append(
                f"{label} & {binary_reader(cell, rec['tng_builds']['campaign']['version'])} & "
                f"{grouped(c['printed']['atoms'])} & "
                f"{c['printed']['frames']} & {figures(c['median_s'] * 1e3, 3)} & "
                f"{figures(c['iqr_s'] * 1e3, 2)} & {ratios}\\\\"
            )
    return table(
        head("binary_cited"),
        "@{}llrrrrrr@{}",
        f"File & Reader & Atoms & Frames & {stack('r', 'Median', '(ms)')} & {stack('r', 'IQR', '(ms)')} & "
        f"{stack('r', 'CON', 'ratio')} & {stack('r', 'Image', 'ratio')}",
        rows,
    )


def tng_table(rec: dict) -> str:
    rows = []
    builds = rec["tng_builds"]
    for name, entry in rec["checks"].items():
        if not name.endswith(".tng"):
            continue
        reads = [
            (r"\texttt{read\_tng}", "all", entry["read_tng_of_the_campaign"], None)
        ]
        reads += [
            (
                f"TNG {builds[b]['version']}, \\texttt{{{builds[b]['commit'][:7]}}}",
                mode,
                read["printed"],
                read["vs_campaign_all"],
            )
            for key, read in entry["tng_read"].items()
            for b, mode in [key.rsplit("-", 1)]
        ]
        system = name.split("_", 1)[0]
        for label, mode, printed, cmp in reads:
            if "error" in printed:
                rows.append(
                    rf"{system} & {label} & {mode} & "
                    rf"\multicolumn{{3}}{{l}}{{\texttt{{{tex(printed['error'].split(':', 1)[1].strip())}}}}}\\"
                )
                continue
            beyond = (
                f"{grouped(cmp['atom_frames_beyond'])} of {grouped(cmp['atom_frames'])}"
                if cmp
                else ""
            )
            largest = f"\\num{{{enot(cmp['largest_difference_A'])}}}" if cmp else ""
            rows.append(
                rf"{system} & {label} & {mode} & {printed['frames']} & {largest} & {beyond}\\"
            )
    difference = stack("r", "Largest", r"difference (\AA)")
    return table(
        head("binary_cited"),
        "@{}lllrrr@{}",
        f"File & Reader & Read & Frames & {difference} & {stack('r', 'Atom-frames', 'beyond')}",
        rows,
    )


def published(rec: dict) -> dict[str, dict[str, str]]:
    """The milliseconds the benchmark of each cited work publishes, by cell of ``cited_text``."""
    out: dict[str, dict[str, str]] = {}
    linux = rec["published"]["chemfiles_linux"]
    head = [c.strip() for c in linux[1].strip("|").split("|")]
    for line in linux[3:]:
        cols = [c.strip() for c in line.strip("|").split("|")]
        if cols[0] in ("water.xyz", "water.xyz.gz"):
            for reader, key in (
                ("chemfiles", "chemfiles"),
                ("ase", "ase"),
                ("mdanalysis", "MDAnalysis"),
                ("openbabel", "openbabel"),
            ):
                col = next(i for i, h in enumerate(head) if h.startswith(key))
                out.setdefault(f"{reader}-{cols[0]}", {})["ms"] = cols[col].split(
                    " - "
                )[0]
    for line in rec["published"]["extxyz_read"][3:]:
        cols = [c.strip() for c in line.strip("|").split("|")]
        n = int(cols[0].replace(" ", ""))
        for reader, col in (("ase_extxyz", 2), ("cextxyz", 3), ("read_dicts_regex", 4)):
            out.setdefault(f"{reader}-cu{n}", {})["ms"] = (
                cols[col].removesuffix(" ms").strip()
            )
    for line in rec["published"]["extxyz_tokenizer"][3:]:
        cols = [c.strip() for c in line.strip("|").split("|")]
        out.setdefault(f"read_dicts-cu{int(cols[0].replace(' ', ''))}", {})["ms"] = (
            cols[2].removesuffix(" ms").strip()
        )
    return out


def text_table(rec: dict, benchmark: str) -> str:
    cells, first, given = rec["timing"]["cells"], rec["first_runs"], published(rec)
    if benchmark == "chemfiles":
        systems = [("water.xyz", "water"), ("water.xyz.gz", "water, gzip")]
    else:
        systems = [
            (f"cu{n}", f"Cu {grouped(int(n))}")
            for n in sorted({c.split("-cu")[1] for c in first if "-cu" in c}, key=int)
        ]
    rows = []
    for key, label in systems:
        names = [c for c in first if c.endswith(f"-{key}")]
        for cell in names:
            reader = TEXT_READERS[first[cell]["reader"]]
            pub = given.get(cell, {}).get("ms", "")
            if cell not in cells:
                rows.append(
                    f"{label} & {reader} & \\multicolumn{{4}}{{l}}{{exit {first[cell]['exit']}}} & {pub}\\\\"
                )
                continue
            ratios = " & ".join(
                ""
                if first[cell]["reader"] in OURS
                else figures(
                    cells[f"{ours}-{key}"]["median_s"] / cells[cell]["median_s"], 2
                )
                for ours in OURS
            )
            rows.append(
                f"{label} & {reader} & {figures(cells[cell]['median_s'] * 1e3, 3)} & "
                f"{figures(cells[cell]['iqr_s'] * 1e3, 2)} & {ratios} & {pub}\\\\"
            )
    con, arrays = (
        stack("r", r"\texttt{read\_con}", "ratio"),
        stack("r", r"\texttt{read\_arrays}", "ratio"),
    )
    return table(
        head("cited_text"),
        "@{}llrrrrr@{}",
        f"System & Reader & {stack('r', 'Median', '(ms)')} & {stack('r', 'IQR', '(ms)')} & {con} & "
        f"{arrays} & {stack('r', 'Published', '(ms)')}",
        rows,
    )


def sources_table(rec: dict, cited: bool) -> str:
    unknown = sorted(set(CITED) - {c["id"] for c in rec["checks"]})
    if unknown:
        raise SystemExit(f"data/claimcheck/sources.json has no checks {unknown}")
    rows = []
    for c in rec["checks"]:
        if (c["id"] in CITED) != cited:
            continue
        where = re.sub(r"^https?://", "", c["url"])
        lines = (
            runs(sorted({m["line"] for m in c["matches"] if m.get("line")}))
            or "page text"
        )
        rows.append(rf"{tex(c['statement'])} & \path{{{where}}} & {lines}\\")
    return table(
        head("sources"),
        r"@{}>{\raggedright\arraybackslash}p{0.34\linewidth}"
        r">{\raggedright\arraybackslash}p{0.48\linewidth}>{\raggedright\arraybackslash}p{0.1\linewidth}@{}",
        r"Statement & Pinned file & Lines",
        rows,
    )


def main() -> int:
    rec = load()
    check(rec)
    values = macros(rec)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "claimcheck_macros.tex").write_text(
        head(*values)
        + "% snapper:off\n"
        + "".join(
            f"% from {RECORD.format(record)}\n"
            + "".join(f"\\newcommand{{\\{n}}}{{{v}}}\n" for n, v in group)
            for record, group in values.items()
        )
    )
    outputs = {
        "claimcheck_files.tex": files_table(rec["eon_files"]),
        "claimcheck_akmc.tex": akmc_table(rec["akmc"]),
        "claimcheck_xtb.tex": xtb_table(rec["xtb_neb"]),
        "claimcheck_sella.tex": sella_table(rec["sella_con"]),
        "claimcheck_libxtc.tex": libxtc_table(rec["binary_cited"]),
        "claimcheck_tng.tex": tng_table(rec["binary_cited"]),
        "claimcheck_binary.tex": binary_table(rec["binary_cited"]),
        "claimcheck_text.tex": text_table(rec["cited_text"], "chemfiles"),
        "claimcheck_extxyz.tex": text_table(rec["cited_text"], "extxyz"),
        "claimcheck_sources.tex": sources_table(rec["sources"], cited=False),
        "claimcheck_cited.tex": sources_table(rec["sources"], cited=True),
    }
    for name, body in outputs.items():
        (OUT / name).write_text(body)
    print(
        f"wrote {sum(len(group) for group in values.values())} macros and {', '.join(outputs)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
