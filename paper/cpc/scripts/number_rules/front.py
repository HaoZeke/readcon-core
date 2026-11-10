"""Rules for the numbers printed in the prose of ``rg_main.tex``, and for the release and benchmark constants
that the differential and timing paragraphs of ``rg_supp.tex`` print."""

from __future__ import annotations

import inspect
import re

import scipy.constants
from numbers_lib import REPO, Rule, grep, jget, text

DOC = "rg_main.tex"
SUPP = "rg_supp.tex"
RESTART = "data/failure/restart_formats.json"
RESTART_PY = "scripts/restart_formats.py"
SURVEY_PY = "scripts/failure_matrix.py"
SPEC = "/docs/orgmode/spec.org"
IDS = "data/v31_large_ids.con"


def param(name: str) -> float:
    return jget(RESTART, f"parameters.{name}")


def driver(name: str) -> float:
    """``NAME = number`` in restart_formats.py; the record must carry the same value."""
    return float(grep(RESTART_PY, rf"^{name} = ([\d.]+)$"))


for key, name in (
    ("fmax_eV_A", "FMAX"),
    ("alpha_A2_eV", "ALPHA"),
    ("interrupt_step", "K"),
    ("step_cap", "NMAX"),
):
    if param(key) != driver(name):
        raise SystemExit(
            f"{RESTART}: parameters.{key} = {param(key)} but {RESTART_PY} sets {name} = {driver(name)}"
        )
if grep(RESTART_PY, r"^ENDPOINT_FMAX = (.+)$") != "FMAX / N_IMG":
    raise SystemExit(
        f"{RESTART_PY}: the endpoint tolerance is not the band tolerance divided by N_IMG"
    )
if param("endpoint_fmax_eV_A") != param("fmax_eV_A") / int(
    grep(RESTART_PY, r"^N_IMG = (\d+)")
):
    raise SystemExit(
        f"{RESTART}: endpoint_fmax_eV_A is not the band tolerance divided by N_IMG"
    )
if param("fmax_eV_A") != float(
    grep(RESTART_PY, r"^ASE_DEFAULT_FMAX = ([\d.]+)$")
) / int(grep(RESTART_PY, r"^N_IMG = (\d+)")):
    raise SystemExit(f"{RESTART}: fmax_eV_A is not ASE_DEFAULT_FMAX divided by N_IMG")
if "fmax=ENDPOINT_FMAX" not in text(RESTART_PY):
    raise SystemExit(f"{RESTART_PY}: BFGS does not use ENDPOINT_FMAX")
if param("spring_eV_A2") != float(
    grep(RESTART_PY, r"^SPRING, METHOD, CLIMB = ([\d.]+), ")
):
    raise SystemExit(f"{RESTART}: spring_eV_A2 disagrees with SPRING in {RESTART_PY}")
SITES = param("sites")
if (SITES[1][0] - SITES[0][0], SITES[1][1] - SITES[0][1]) != (1, 0):
    raise SystemExit(f"{RESTART}: sites {SITES} are not neighbouring hollows along x")


def miller(rel: str, cells: str) -> int:
    return int(grep(rel, rf's = fcc(\d+)\("Pt", \({re.escape(cells)}\)'))


def kib(rel: str, name: str) -> float:
    a, b = re.search(rf"{name}: \w+ = (\d+) \* (\d+);", text(rel)).groups()
    return int(a) * int(b) / 1024


def spec_version(version: str) -> float:
    return float(
        grep(SPEC, rf"^\|\s*({re.escape(version)})\s*\|\s*\d{{4}}-\d{{2}}-\d{{2}}\s*\|")
    )


def variance_contours() -> list[float]:
    """Fractions of the variance range that plot_landscape_surface draws as dashed lines."""
    import chemparseplot.plot.neb

    source = inspect.getsource(chemparseplot.plot.neb.plot_landscape_surface)
    threshold = float(re.search(r"variance_threshold=([\d.]+)", source).group(1))
    fixed = [float(f) for f in re.findall(r"v_min \+ ([\d.]+) \* v_range", source)]
    return sorted({threshold, *fixed})


def corpus_ids() -> list[int]:
    return sorted(
        {
            int(line.split()[4])
            for line in text(IDS).splitlines()
            if re.fullmatch(r"(\S+ ){4}\d{10,}", line)
        }
    )


U64_BITS = int(grep("/src/types.rs", r"pub atom_id: u(\d+),"))
ANGLE_LIMIT = float(grep("/src/parser.rs", r"\*angle >= ([\d.]+)\)"))


def si_factor(aliases: str) -> float:
    """``si_factor`` of the unit that src/units.rs inserts under ``aliases``."""
    pattern = re.escape(aliases) + r",\s*UnitValue \{\s*si_factor: ([\d_.e+-]+),"
    return float(grep("/src/units.rs", pattern).replace("_", ""))


ANGSTROM_M = si_factor('&["angstrom", "a", "å"]')
ELECTRONVOLT_J = si_factor('&["ev"]')
if (ANGSTROM_M, ELECTRONVOLT_J) != (
    scipy.constants.angstrom,
    scipy.constants.electron_volt,
):
    raise SystemExit(
        "src/units.rs: the angstrom or the electronvolt is not the CODATA value of scipy.constants"
    )
AMU_KG = si_factor('&["u", "amu", "dalton", "da"]')
# scipy.constants follows the latest CODATA set, and the text names the 2018 value that the unit table holds.
if AMU_KG != 1.66053906660e-27:
    raise SystemExit(
        "src/units.rs: the atomic mass constant is not the CODATA 2018 value"
    )
BIG_IDS = corpus_ids()
ABOVE_2_53 = next(i for i in BIG_IDS if i < 2**U64_BITS - 1)
if ABOVE_2_53 - 1 != 2**53 or BIG_IDS[-1] != 2**U64_BITS - 1:
    raise SystemExit(
        f"{IDS}: expected the identifiers 2^53 + 1 and 2^64 - 1, found {BIG_IDS}"
    )

GRIND = "bench/results/r0174/cachegrind/cachegrind_results.json"
GRIND_FRAMES = int(
    re.fullmatch(
        r"parse_(\d+)_frames",
        next(
            k
            for k in jget(GRIND, "scenarios")
            if k.startswith("parse_") and k.endswith("_frames")
        ),
    ).group(1)
)

# benches/make_velocity_campaign.py and benches/headtohead.py are not in this
# tree. The z step they shared, 1e-6 angstrom per frame, is the assignment in
# benches/h5md_vs_con.py. Nothing in the manuscript prints that step, so the
# number rules do not read either missing file.

CONTOURS = variance_contours()
if CONTOURS[0] != 0.05 or len(CONTOURS) != 2:
    raise SystemExit(
        f"plot_landscape_surface draws variance contours at {CONTOURS}, not at 5 and 95 percent"
    )

if grep("/src/stream.rs", r"pub const DEFAULT_READ_SIZE: usize = 1 << (\d+);") != "20":
    raise SystemExit("the stream reader's DEFAULT_READ_SIZE is not 1 MiB")

def log_field(rel: str, key: str) -> int:
    prefix = key + " "
    for line in text(rel).splitlines():
        if line.startswith(prefix):
            return int(line.split()[1])
    raise SystemExit(f"{rel} has no {key} line")


def bootstrap_percent() -> int:
    for line in text("scripts/dlpack_interval.py").splitlines():
        if "np.percentile(boots, [" in line:
            lo, hi = (
                float(part)
                for part in line.split("[", 1)[1].split("]", 1)[0].split(",")
            )
            return int(hi - lo)
    raise SystemExit("scripts/dlpack_interval.py names no bootstrap percentiles")


RULES: list[Rule] = [
    Rule(
        "100",
        miller(RESTART_PY, "3, 3, 3"),
        RESTART_PY,
        "Miller index of the fcc100 Pt slab in endpoints()",
        (
            f"{DOC}~climbing-image|restart|band|EMT",
            "figures/generated/restart_main.tex",
            "rg_supp.tex~restart study",
        ),
    ),
    Rule(
        "100",
        miller(SURVEY_PY, "2, 2, 2"),
        SURVEY_PY,
        "Miller index of the fcc100 Pt slab in fixture()",
        (
            f"{DOC}~survey|The frame is a Pt",
            "figures/generated/survey_table.tex",
            "rg_supp.tex~survey frame",
        ),
    ),
    Rule(
        "180",
        ANGLE_LIMIT,
        "/src/parser.rs",
        "upper bound of the cell-angle check (*angle >= 180.0 is rejected)",
    ),
    Rule(
        "2^{64}",
        2**U64_BITS,
        "/src/types.rs",
        "range of the atom_id integer type (u64): 2^bits",
        (DOC,),
    ),
    Rule(
        "-1",
        -1,
        "/src/types.rs",
        "the largest u64 identifier is 2^64 - 1; this is the -1 of that expression",
        (f"{DOC}~2\\^\\{{64\\}}-1",),
    ),
    Rule(
        "140",
        int(grep("/src/iterators.rs", r"const MUTATIONS: usize = (\d+);"))
        * sum(
            1
            for base in ("resources/conformance", "resources/test")
            for path in (REPO / base).rglob("*")
            if path.suffix in {".con", ".convel"}
        ),
        "/src/iterators.rs",
        "conformance and test files times MUTATIONS in the differential",
        (f"{DOC}~differential cases", f"{SUPP}~140 cases"),
    ),
    Rule(
        "61",
        int(
            grep(
                "/src/iterators.rs",
                r"for size in \[1(?:usize)?, (\d+), crate::stream::DEFAULT_READ_SIZE\]",
            )
        ),
        "/src/iterators.rs",
        "middle read size of the differential stream reads, bytes",
        (f"{SUPP}~read sizes of",),
    ),
    Rule(
        "305",
        int(grep("/src/iterators.rs", r"const MAPPED_STRIDE: usize = (\d+);")),
        "/src/iterators.rs",
        "stride of the padded mapped reads in the differential",
        (f"{SUPP}~One case in every",),
    ),
    Rule(
        "3.1",
        spec_version("3.1"),
        SPEC,
        "version column of the spec version-history table",
        (DOC,),
    ),
    Rule(
        "0.002",
        param("endpoint_fmax_eV_A"),
        RESTART,
        "parameters.endpoint_fmax_eV_A",
        (DOC, "rg_supp.tex"),
    ),
    Rule(
        "0.01",
        param("fmax_eV_A"),
        RESTART,
        "parameters.fmax_eV_A",
        (DOC, "rg_supp.tex", "figures/generated/restart_main.tex"),
    ),
    Rule(
        "0.02",
        param("alpha_A2_eV"),
        RESTART,
        "parameters.alpha_A2_eV",
        (DOC, "rg_supp.tex"),
    ),
    Rule(
        "2\\times 10^{-4}",
        param("alpha_A2_eV") * param("fmax_eV_A"),
        RESTART,
        "parameters.alpha_A2_eV times parameters.fmax_eV_A: one step at the band tolerance, in angstrom",
        (f"{DOC}~moves an atom by", f"{DOC}~under that step"),
    ),
    Rule(
        "1.6605\\times 10^{-27}",
        AMU_KG,
        "/src/units.rs",
        "si_factor of amu, in kg",
        (f"{DOC}~1\\.6605",),
    ),
    Rule(
        "10^{-20}",
        ANGSTROM_M**2,
        "/src/units.rs",
        "square of the si_factor of angstrom, in m^2",
        (f"{DOC}~1\\.6605",),
    ),
    Rule(
        "1.6022\\times 10^{-19}",
        ELECTRONVOLT_J,
        "/src/units.rs",
        "si_factor of ev, in J",
        (f"{DOC}~1\\.6605",),
    ),
    Rule(
        "0.1",
        param("spring_eV_A2"),
        RESTART,
        "parameters.spring_eV_A2",
        (f"{DOC}~spring constant",),
    ),
    Rule(
        "25",
        param("interrupt_step"),
        RESTART,
        "parameters.interrupt_step",
        (DOC, "rg_supp.tex", "figures/generated/restart_main.tex"),
    ),
    Rule(
        "4000",
        param("step_cap"),
        RESTART,
        "parameters.step_cap",
        (DOC, "rg_supp.tex", "figures/generated/restart_main.tex"),
    ),
    Rule(
        "100000",
        log_field("logs/reader-exchange.log", "frames"),
        "logs/reader-exchange.log",
        "frames of the reader-exchange trajectory",
        (
            f"{DOC}~100000",
            "figures/generated/host_table.tex~100000",
            f"{SUPP}~100000",
        ),
    ),
    Rule(
        "32",
        log_field("logs/reader-exchange.log", "atoms"),
        "logs/reader-exchange.log",
        "atoms of the reader-exchange trajectory",
        (f"{DOC}~32 copper",),
    ),
    Rule(
        "40",
        int(grep("bench/db_v2.py", r"for frame in range\((\d+)\)")),
        "bench/db_v2.py",
        "frames written by write_con",
        (f"{DOC}~40 frames",),
    ),
    Rule(
        "64",
        log_field("logs/mpi.log", "atoms"),
        "logs/mpi.log",
        "atoms of the distributed read",
        (f"{DOC}~64 copper", "figures/generated/mpi_host_table.tex~64"),
    ),
    Rule(
        "4000",
        log_field("logs/mpi.log", "frames"),
        "logs/mpi.log",
        "frames of the distributed read",
        ("figures/generated/mpi_host_table.tex~4000",),
    ),
    Rule(
        "1000",
        int(grep("scripts/plot_restored.py", r'n_frames"\] == "(\d+)"')),
        "scripts/plot_restored.py",
        "n_frames of the stored database copies that the disk figure draws",
        (f"{DOC}~1000-frame",),
    ),
    Rule(
        "95",
        bootstrap_percent(),
        "scripts/dlpack_interval.py",
        "width of the bootstrap percentile interval, percent",
        (f"{DOC}~bootstrap",),
    ),
    Rule(
        "3.12",
        float(grep("environment/conda-linux-64.txt", r"python-(\d+\.\d+)\.\d+")),
        "environment/conda-linux-64.txt",
        "Python major.minor of the conda environment",
        (f"{SUPP}~Python 3",),
    ),
]
