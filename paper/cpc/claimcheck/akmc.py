"""Adaptive kinetic Monte Carlo with eOn on its own example, six seeds, and a read of every CON file the runs wrote.

The example is ``examples/akmc-pt`` of eOn v3.6.0, a Pt heptamer on a Pt(111) slab with the Morse potential,
read from the tag by URL, size and SHA-256.  The eon environment holds the conda-forge package of eOn 3.6.0.
Its run requirements leave out Python packages that the ``pyproject.toml`` of eOn v3.6.0 lists, so the record
first keeps the error of the server as packaged.  It then installs those packages, as the wheels PyPI serves
for the Python of the eon environment, pinned by SHA-256 in ``WHEELS``, into ``WORK/akmc/site``.

Each seed runs ``python -m eon -C config.ini`` in the eon environment in a directory of its own, with the config
of the example plus ``[Main] random_seed`` and ``[AKMC] max_kmc_steps = STEPS``.  As many seeds run at once as
the host has room for the client processes the example asks for (``number_of_CPUs``).  The record keeps, per
seed, the saddle searches of each state, the processes found, the KMC steps with their barriers and times, the
simulated time and the wall time.  Every ``.con`` file the runs wrote is then read by readcon of this checkout,
by readcon 0.17.3 (the release the eOn server ran with) and by the eon module of ASE (``con_readers``).
"""

from __future__ import annotations

import configparser
import glob
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import cc_common as cc
import con_readers
import tomllib

TAG = "v3.6.0"
RAW = f"https://raw.githubusercontent.com/TheochemUI/eOn/{TAG}"
EXAMPLE = {
    "config.ini": (
        397,
        "909bd006da8da44c4291666feab78f1b4bafd1e0e246f938b5b55930bb5f9c85",
    ),
    "pos.con": (
        15830,
        "90c8a40b9cfb21ccbf06f07a20fe8a07095555ef31cb30bfcbe074727b7a474d",
    ),
}
PYPROJECT = (2538, "69525afeb1559945b1043a568c95a909eb4ec0267c001632944ae3caef688047")
SEEDS = range(1, 7)
# Three KMC steps carry the heptamer through three states, and every state is searched to the confidence of the example.
STEPS = 3
PYTHON = "3.14"
WHEELS = {
    "readcon==0.17.3": "c099e9a149f2ae9081f0c7e13585355a7a0b0da3ab396b3b6ba892b22475a30f",
    "readcon-db==0.2.0": "0ccbbfab82b69b490b56323e05342e9116a8c3ba80ed549c9b417e0ecfc55e16",
    "vesin==0.6.2": "14ba0c65e602ffb59c63199761b70f6b96a1847913de56d9b60c186931b16f4f",
    "eon-schema==0.2.4": "87b5af17e0a01ac5d0714df49041ff9681e5bd2265e363edbb654aed64d5937a",
    "xxhash==4.0.2": "318e6c3a31d75ccacd1f9b864e3b857d44c9f4bed5e9d5fb5db1dc2fec82f019",
    "pydantic==2.14.2": "7efbf7507b937931cd32a5755acb00ea1cf5ccb1d4e8067a5850d5069e382118",
    "pydantic-core==2.50.2": "b29b7c5adf4b2be7c941f7f76432aa3269cd5052ff8610376b8ece24d7b02c51",
    "typing-extensions==4.16.0": "481caa481374e813c1b176ada14e97f1f67a4539ce9cfeb3f350d78d6370c2e8",
    "annotated-types==0.8.0": "f072f4d804ea359e4eaf198b1af7a8b0943881a87f31bb764f8bf219bb9419e0",
    "typing-inspection==0.4.5": "bc210535442b39e1bf2805e0c7a17dfd6a4ace760c84c7f7d16245d23a6b1c39",
}
PLATFORM = [
    "--python-version",
    PYTHON,
    "--implementation",
    "cp",
    "--abi",
    "cp" + PYTHON.replace(".", ""),
    "--platform",
    "manylinux2014_x86_64",
    "--platform",
    "manylinux_2_17_x86_64",
    "--platform",
    "manylinux_2_28_x86_64",
    "--only-binary=:all:",
    "--no-deps",
]


def eon_env(args: list, cwd: Path, site: Path | None = None) -> list:
    """``args`` in the eon environment, with ``site`` first on ``PYTHONPATH`` when given."""
    base = [
        "pixi",
        "run",
        "--locked",
        "--manifest-path",
        str(cc.CPC / "pixi.toml"),
        "-e",
        "eon",
    ]
    return base + (["env", f"PYTHONPATH={site}"] if site else []) + args


def packaged_requirements() -> dict:
    conda_meta = sorted(
        glob.glob(
            str(cc.CPC / ".pixi" / "envs" / "eon" / "conda-meta" / "eon-3.6.0-*.json")
        )
    )
    if len(conda_meta) != 1:
        raise SystemExit(
            f"expected one conda-meta record of eon 3.6.0, found {len(conda_meta)}"
        )
    meta = json.loads(Path(conda_meta[0]).read_text())
    return {
        "package": f"{meta['name']} {meta['version']} {meta['build']}",
        "channel": meta.get("channel"),
        "depends": meta["depends"],
    }


def missing_from_package(pyproject: Path, depends: list[str]) -> dict:
    declared = tomllib.loads(pyproject.read_text())["project"]["dependencies"]

    def name(spec: str) -> str:
        return (
            spec.replace(">", " ")
            .replace("<", " ")
            .replace("=", " ")
            .split()[0]
            .lower()
            .replace("_", "-")
        )

    packaged = {name(spec) for spec in depends}
    return {
        "pyproject_dependencies": declared,
        "not_in_package": [d for d in declared if name(d) not in packaged],
    }


def install_server_packages(work: Path) -> tuple[Path, dict]:
    wheels = cc.WORK / "downloads" / "eon-server-wheels"
    site = work / "site"
    requirements = work / "requirements.txt"
    requirements.write_text(
        "".join(f"{spec} --hash=sha256:{digest}\n" for spec, digest in WHEELS.items())
    )
    pip = [sys.executable, "-m", "pip"]
    download = cc.run(
        pip
        + ["download", "--require-hashes", *PLATFORM, "-d", wheels, "-r", requirements]
    )
    install = cc.run(
        pip
        + [
            "install",
            "--require-hashes",
            *PLATFORM,
            "--no-index",
            "--find-links",
            wheels,
            "--target",
            site,
            "-r",
            requirements,
        ]
    )
    files = sorted(wheels.glob("*.whl"))
    return site, {
        "requirements": list(WHEELS),
        "download": cc.brief(download),
        "install": cc.brief(install),
        "wheels": [
            {"file": f.name, "bytes": f.stat().st_size, "sha256": cc.sha256(f)}
            for f in files
        ],
    }


def seed_config(example: Path, seed: int) -> str:
    config = configparser.ConfigParser()
    config.optionxform = str
    config.read(example)
    config["Main"]["random_seed"] = str(seed)
    config["AKMC"]["max_kmc_steps"] = str(STEPS)
    lines = []
    for section in config.sections():
        lines.append(f"[{section}]")
        lines += [f"{key} = {value}" for key, value in config[section].items()]
        lines.append("")
    return "\n".join(lines)


def spread(values: list[float], suffix: str) -> dict | None:
    """Median and type-7 quartiles of ``values``, which need two values or more."""
    return cc.interleave.summary(values, suffix) if len(values) > 1 else None


def table_rows(path: Path, header_lines: int) -> list[list[str]]:
    if not path.is_file():
        return []
    return [
        line.split()
        for line in path.read_text().splitlines()[header_lines:]
        if line.strip()
    ]


def seed_result(run: Path) -> dict:
    states = sorted(
        (p for p in (run / "states").iterdir() if p.is_dir() and p.name.isdigit()),
        key=lambda p: int(p.name),
    )
    per_state = []
    for state in states:
        searches = table_rows(state / "search_results.txt", 2)
        kinds: dict[str, int] = {}
        for row in searches:
            kind = row[-1].split("-")[0]
            kinds[kind] = kinds.get(kind, 0) + 1
        per_state.append(
            {
                "state": int(state.name),
                "searches": len(searches),
                "results": kinds,
                "processes": len(table_rows(state / "processtable", 1)),
            }
        )
    steps = [
        {
            "step": int(r[0]),
            "reactant": int(r[1]),
            "product": int(r[3]),
            "step_time_s": float(r[4]),
            "total_time_s": float(r[5]),
            "barrier_eV": float(r[6]),
            "rate_per_s": float(r[7]),
        }
        for r in table_rows(run / "dynamics.txt", 2)
    ]
    info = configparser.ConfigParser()
    info.read(run / "info.txt")
    simulated = info.getfloat("Simulation Information", "time_simulated", fallback=None)
    return {
        "kmc_steps": len(steps),
        "steps": steps,
        "simulated_time_s": simulated,
        "states": per_state,
        "searches": sum(s["searches"] for s in per_state),
    }


def main() -> int:
    work = cc.WORK / "akmc"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    sources = []
    example = {}
    for name, (size, sha) in EXAMPLE.items():
        example[name], got = cc.fetch(
            f"{RAW}/examples/akmc-pt/{name}", f"eon-{TAG}-akmc-pt-{name}", sha, size
        )
        sources.append(got)
    pyproject, got = cc.fetch(
        f"{RAW}/pyproject.toml", f"eon-{TAG}-pyproject.toml", PYPROJECT[1], PYPROJECT[0]
    )
    sources.append(got)

    packaged = packaged_requirements()
    missing = missing_from_package(pyproject, packaged["depends"])
    as_packaged = work / "as-packaged"
    as_packaged.mkdir()
    shutil.copy(example["config.ini"], as_packaged / "config.ini")
    shutil.copy(example["pos.con"], as_packaged / "pos.con")
    packaged_run = cc.run(
        eon_env(["python", "-m", "eon", "-C", "config.ini"], as_packaged),
        cwd=as_packaged,
        check=False,
    )

    site, installed = install_server_packages(work)
    versions = {
        "eonclient": cc.run(eon_env(["eonclient", "--version"], work))[
            "stdout"
        ].strip(),
        "server": cc.run(
            eon_env(
                [
                    "python",
                    "-c",
                    "import eon, readcon; print(eon.__version__, readcon.__version__)",
                ],
                work,
                site,
            )
        )["stdout"].strip(),
    }

    example_config = configparser.ConfigParser()
    example_config.read(example["config.ini"])
    clients = example_config.getint("Communicator", "number_of_CPUs")
    parallel = max(1, (os.cpu_count() or 1) // clients)
    pending = list(SEEDS)
    running: dict[int, tuple[subprocess.Popen, float, Path]] = {}
    finished: dict[int, dict] = {}
    while pending or running:
        while pending and len(running) < parallel:
            seed = pending.pop(0)
            run = work / f"seed-{seed}"
            run.mkdir()
            (run / "config.ini").write_text(seed_config(example["config.ini"], seed))
            shutil.copy(example["pos.con"], run / "pos.con")
            cmd = eon_env(["python", "-m", "eon", "-C", "config.ini"], run, site)
            log = (run / "server.out").open("w")
            print("+", " ".join(cmd), f"(seed {seed})", flush=True)
            proc = subprocess.Popen(cmd, cwd=run, stdout=log, stderr=subprocess.STDOUT)
            running[seed] = (proc, time.perf_counter(), run)
        time.sleep(5)
        for seed, (proc, start, run) in list(running.items()):
            if proc.poll() is None:
                continue
            seconds = round(time.perf_counter() - start, 3)
            out = (run / "server.out").read_text()
            print(f"seed {seed}: exit {proc.returncode} after {seconds} s", flush=True)
            finished[seed] = {
                "seed": seed,
                "command": [str(c) for c in proc.args],
                "exit": proc.returncode,
                "wall_seconds": seconds,
                "server_output_tail": out[-1500:],
                **seed_result(run),
            }
            del running[seed]

    runs = [finished[s] for s in SEEDS]
    written = sorted(
        Path(p)
        for p in glob.glob(str(work / "seed-*" / "**" / "*.con"), recursive=True)
    )
    example_sha = EXAMPLE["pos.con"][1]
    copies = [str(p.relative_to(work)) for p in written if cc.sha256(p) == example_sha]
    reads = con_readers.verdicts(written, work)
    ok = [r for r in runs if r["exit"] == 0]
    cc.write_record(
        "akmc",
        {
            "claim": "An eOn AKMC run spawns saddle searches by the thousand, and readcon reads the CON files it writes",
            "example": f"examples/akmc-pt of eOn {TAG}",
            "package": packaged,
            "package_missing": missing,
            "server_as_packaged": cc.brief(packaged_run),
            "server_packages": installed,
            "versions": versions,
            "seeds": list(SEEDS),
            "max_kmc_steps": STEPS,
            "seeds_at_once": parallel,
            "client_processes_per_seed": clients,
            "runs": runs,
            "summary": {
                "runs": len(runs),
                "runs_exit_0": len(ok),
                "searches_total": sum(r["searches"] for r in ok),
                "searches_per_run": spread([r["searches"] for r in ok], ""),
                "searches_per_state": spread(
                    [
                        s["searches"]
                        for r in ok
                        for s in r["states"]
                        if s["state"] < r["kmc_steps"]
                    ],
                    "",
                ),
                "simulated_time_s": spread([r["simulated_time_s"] for r in ok], "_s"),
                "wall_seconds": spread([r["wall_seconds"] for r in ok], "_s"),
                "con_files_written": len(written),
                "copies_of_the_example_pos_con": copies,
            },
            "con_reads": reads,
        },
        sources,
    )
    return 0 if len(ok) == len(runs) else 1


if __name__ == "__main__":
    raise SystemExit(main())
