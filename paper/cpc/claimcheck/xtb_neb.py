"""eOn hands CON geometries to a Fortran electronic structure code: the HCN to HNC band with xtb.

The endpoints and the band settings are those that the reaction-path workflow of
goswamiReproducibleOrchestrationBest2026 published for HCN to HNC (Materials Cloud record z41cv-q4h43,
``results/endpoints/hcn_isom`` and ``results/neb/hcn_isom/config.ini``).  The config is kept as published but for
the potential: ``[Potential] potential = xtb`` with ``[XTBPot] paramset = GFN2xTB`` in place of the machine-learned
model, so the C++ client of eOn 3.6.0 (the eon environment) passes each image to the Fortran library of xtb.
A cell of ``interleave`` runs ``eonclient`` in a fresh directory, a fresh process for every run; a run prints
the termination, the force calls and the energies of the images that ``results.dat`` prints, which have to agree
from run to run.  The band files of every run are compared after the campaign, and two more runs with
``OMP_NUM_THREADS=1`` give the band file of a run on one thread twice.  The CON files of the last run are read by
the three readers of ``con_readers``, and the energies that the band file carries are compared with those that
``results.dat`` prints.
"""

from __future__ import annotations

import configparser
import io
import shutil
import tarfile
import time
from pathlib import Path

import cc_common as cc
import con_readers
import eon_files
import numpy as np

MEMBERS = {
    "reactant.con": "results/endpoints/hcn_isom/reactant.con",
    "product.con": "results/endpoints/hcn_isom/product.con",
    "published.ini": "results/neb/hcn_isom/config.ini",
}


def results(path: Path) -> dict:
    rows = {}
    for line in path.read_text().splitlines():
        parts = line.split()
        if len(parts) >= 2:
            rows[parts[1]] = parts[0]
    return rows


def main() -> int:
    import readcon

    work = cc.WORK / "xtb_neb"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    pin = eon_files.ARCHIVES["neb-orchestrator-data"]
    archive, got = cc.fetch(pin["url"], pin["file"], pin["sha256"], pin["bytes"])
    inputs = work / "inputs"
    inputs.mkdir()
    with tarfile.open(archive) as tar:
        for name, member in MEMBERS.items():
            (inputs / name).write_bytes(tar.extractfile(member).read())
    config = configparser.ConfigParser()
    config.optionxform = str
    config.read(inputs / "published.ini")
    published_potential = dict(config["Potential"])
    config.remove_section("Metatomic")
    config["Potential"]["potential"] = "xtb"
    config["XTBPot"] = {"paramset": "GFN2xTB"}
    text = io.StringIO()
    config.write(text)
    (inputs / "config.ini").write_text(text.getvalue())

    count = {"runs": 0}

    def eonclient(run: Path, env: dict | None = None) -> dict:
        run.mkdir()
        for name in ("reactant.con", "product.con", "config.ini"):
            shutil.copy(inputs / name, run / name)
        start = time.perf_counter()
        done = cc.run(
            [
                "pixi",
                "run",
                "--locked",
                "--manifest-path",
                cc.CPC / "pixi.toml",
                "-e",
                "eon",
                "eonclient",
            ],
            cwd=run,
            env=env,
        )
        wall = time.perf_counter() - start
        rows = results(run / "results.dat")
        images = int(float(rows["number_of_images"])) + 2
        energies = [rows[f"image{i}_energy"] for i in range(images)]
        return {
            "seconds": float(rows["time_seconds"]),
            "wall_seconds": wall,
            "exit": done["exit"],
            "termination": rows["termination_reason_text"],
            "potential": rows["potential_type"],
            "force_calls": int(rows["total_force_calls"]),
            "image_energies_eV": energies,
        }

    def cell() -> dict:
        count["runs"] += 1
        return eonclient(work / f"run-{count['runs']:02d}")

    def bands(runs: list[Path]) -> dict:
        frames = [readcon.read_con(str(run / "neb.con")) for run in runs]
        positions = np.stack(
            [
                np.concatenate([np.asarray(f.xyz).ravel() for f in band])
                for band in frames
            ]
        )
        energies = np.array(
            [[float(f.metadata["energy"]) for f in band] for band in frames]
        )
        return {
            "runs": len(runs),
            "distinct_neb_con_sha256": len(
                {cc.sha256(run / "neb.con") for run in runs}
            ),
            "largest_position_spread_A": float(
                (positions.max(axis=0) - positions.min(axis=0)).max()
            ),
            "largest_energy_spread_eV": float(
                (energies.max(axis=0) - energies.min(axis=0)).max()
            ),
        }

    timing = cc.campaign({"eon_xtb": cell})
    campaign_bands = bands([work / f"run-{i:02d}" for i in range(1, count["runs"] + 1)])
    serial = [work / f"serial-{i}" for i in (1, 2)]
    serial_printed = [eonclient(run, env={"OMP_NUM_THREADS": "1"}) for run in serial]
    last = work / f"run-{count['runs']:02d}"
    written = sorted(last.glob("*.con"))
    reads = con_readers.verdicts(written, last)
    band = readcon.read_con(str(last / "neb.con"))
    printed = [
        float(e) for e in timing["cells"]["eon_xtb"]["printed"]["image_energies_eV"]
    ]
    carried = [float(frame.metadata["relative_energy"]) for frame in band]
    client = cc.run(
        [
            "pixi",
            "run",
            "--locked",
            "--manifest-path",
            cc.CPC / "pixi.toml",
            "-e",
            "eon",
            "eonclient",
            "--version",
        ],
        cwd=work,
    )
    peak = max(range(len(printed)), key=printed.__getitem__)
    cc.write_record(
        "xtb_neb",
        {
            "claim": "eOn passes CON geometries to the Fortran library of xtb and writes a band that readcon reads",
            "eonclient": client["stdout"].strip(),
            "inputs": {
                name: {"member": MEMBERS[name], "sha256": cc.sha256(inputs / name)}
                for name in MEMBERS
            }
            | {
                "config.ini": {
                    "sha256": cc.sha256(inputs / "config.ini"),
                    "text": text.getvalue(),
                }
            },
            "published_potential": published_potential,
            "timing": timing,
            "band_files_of_the_campaign": campaign_bands,
            "one_thread": {
                "env": {"OMP_NUM_THREADS": "1"},
                "bands": bands(serial),
                "same_printed_as_the_campaign": all(
                    {k: v for k, v in p.items() if k not in ("seconds", "wall_seconds")}
                    == timing["cells"]["eon_xtb"]["printed"]
                    for p in serial_printed
                ),
            },
            "band": {
                "frames": len(band),
                "energies_in_band_file_eV": carried,
                "results_dat_energies_eV": printed,
                "largest_difference_eV": max(
                    abs(a - b) for a, b in zip(carried, printed)
                )
                if len(carried) == len(printed)
                else None,
                "highest_image": peak,
                "highest_energy_eV": printed[peak],
                "last_image_energy_eV": printed[-1],
            },
            "con_reads": reads,
        },
        [got],
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
