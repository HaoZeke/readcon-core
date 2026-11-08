"""The text-format benchmarks of chemfiles and extxyz on their own systems, with readcon on the same atoms as CON.

chemfiles: ``files/water.xyz`` of the chemfiles benchmark repository (297 atoms in 100 frames, six decimals, written
by VMD) and its gzip ``water.xyz.gz``, read by chemfiles, ASE, MDAnalysis and Open Babel as its ``bench.py``
reads them.  The
XYZ file has no cell; ``water.nc`` of the same repository holds the same frames in binary32 with the box, so the
CON files take its box and print the six decimals of the XYZ file, plain and in gzip.

extxyz: the ``make_xyz`` systems of ``benchmarks/bench_read.py`` at extxyz v0.4.6, one frame of N Cu atoms with
forces, energy and step written by the cextxyz writer of the ase-extxyz plugin, for the N of the read table of its
README.  They are read by ASE's built-in extxyz reader, the cextxyz plugin, and ``extxyz.read_dicts`` with the regex
tokenizer (the README column) and with its default tokenizer.  readcon reads the same frame with its forces as CON,
in the eight decimals of the cextxyz writer.  ``make_xyz`` draws the forces from NumPy's global generator, seeded
here with the seed of ``interleave``.

readcon reads each CON file twice over: ``read_con``, which returns frames with every section and the positions
as the array ``ConFrame.xyz``, as the readcon cells of ``bench/readers.py`` do, and ``read_arrays``, which returns
the positions of every frame as one array that leaves Rust without a copy.

Every run is a fresh process (``child``) that imports the reader and reads the file once untimed, as
``bench/bench_child.py`` reads its warm-up payload, and then times one read of every frame, so neither a timing
holds the import of a library or its first-call setup.  A first run of each cell records its output or its
failure, and ``interleave.campaign`` gives the median and type-7 IQR.

    python claimcheck/cited_text.py
    python claimcheck/cited_text.py child READER PATH
"""

from __future__ import annotations

import gc
import gzip
import importlib.util
import json
import math
import os
import shutil
import sys
import time
from pathlib import Path

import numpy as np

CHEMFILES = {
    "commit": "0d6c7f4e6c6b3ff54ad54acdc5e5c92ff5aeae2b",
    "files": {
        "water.xyz": (
            1606000,
            "75a64a809cde8404c73dcb6f815483d571dddc8daa6fa5e1a78ed54cd1041268",
        ),
        "water.xyz.gz": (
            458269,
            "5a129e57c5663a3c6051b1343b86b222b31830c06a44d0e35992072e14ada719",
        ),
        "water.nc": (
            366588,
            "771f732764bd69eb323d86f62589ec2f8418bfe5d0a6375adb431c2bac580e0d",
        ),
        "README.md": (
            6070,
            "6271a4d93f3f88cfdb26b34f455b82b82e2c30d0a466923c93081cc0a5b6bd56",
        ),
        "bench.py": (
            6217,
            "002cd6a9f54a4cdfb99d023f26c264694f5789b9e0546c6a589c1830170fcffe",
        ),
    },
}
EXTXYZ = {
    "tag": "v0.4.6",
    "commit": "ebfe7b39db1e098aba418d4b8ac57795d12a4dac",
    "files": {
        "benchmarks/bench_read.py": (
            6123,
            "246bb56fdbd838824d18d0c9816727ddb3738d754ea1aaba5d69f354bcd202ff",
        ),
        "README.md": (
            26526,
            "820cbd408177c16f328b6ab39255100da216f85186d58774e4b8e29722729bc0",
        ),
    },
}
PLUGIN = {
    "url": "https://files.pythonhosted.org/packages/4c/c3/b20fea103b9a0e0bdf108dd2babd5fe3a39549a18ac15c4af906fb4c51a9/"
    "ase_extxyz-0.1.2-py3-none-any.whl",
    "file": "ase_extxyz-0.1.2-py3-none-any.whl",
    "bytes": 8744,
    "sha256": "00c7569d24eff4183659877a20334504eeae4483be8bf1ac478426684fe7cea5",
}
# The atom counts of the read table of the extxyz README.
SIZES = (10, 100, 1000, 4000, 16000, 64000, 200000)
WATER_READERS = (
    "chemfiles",
    "ase",
    "mdanalysis",
    "openbabel",
    "readcon",
    "readcon_arrays",
)
CU_READERS = (
    "ase_extxyz",
    "cextxyz",
    "read_dicts_regex",
    "read_dicts",
    "readcon",
    "readcon_arrays",
)
OURS = ("readcon", "readcon_arrays")


def r_chemfiles(path: str):
    import chemfiles

    chemfiles.set_warnings_callback(lambda message: None)
    records = []
    with chemfiles.Trajectory(path) as traj:
        for _ in range(traj.nsteps):
            frame = traj.read()
            records.append((frame.positions, frame))
    return records


def r_ase(path: str):
    import ase.io

    return [(atoms.positions, atoms) for atoms in ase.io.read(path, ":")]


def r_ase_extxyz(path: str):
    import ase.io

    return [
        (atoms.positions, atoms) for atoms in ase.io.read(path, ":", format="extxyz")
    ]


def r_cextxyz(path: str):
    import ase.io
    import ase_extxyz.io  # noqa: F401

    return [
        (atoms.positions, atoms) for atoms in ase.io.read(path, ":", format="cextxyz")
    ]


def r_mdanalysis(path: str):
    import MDAnalysis

    universe = MDAnalysis.Universe(path)
    return [(ts.positions.copy(), None) for ts in universe.trajectory]


def r_openbabel(path: str):
    """The ``OBConversion`` reads of ``bench_openbabel``, keeping the positions of every molecule."""
    from openbabel import openbabel as ob

    conversion, mol, records = ob.OBConversion(), ob.OBMol(), []
    more = conversion.ReadFile(mol, path)
    while more:
        records.append(
            (
                np.array(
                    [[a.GetX(), a.GetY(), a.GetZ()] for a in ob.OBMolAtomIter(mol)]
                ),
                mol,
            )
        )
        mol = ob.OBMol()
        more = conversion.Read(mol)
    return records


def dict_frames(frames) -> list:
    """``extxyz.read_dicts`` returns a list of frames, or the frame itself for a file of one frame."""
    return [
        (frame.arrays["pos"], frame)
        for frame in (frames if isinstance(frames, list) else [frames])
    ]


def r_read_dicts_regex(path: str):
    import extxyz

    return dict_frames(extxyz.read_dicts(path, use_regex=True))


def r_read_dicts(path: str):
    import extxyz

    return dict_frames(extxyz.read_dicts(path))


def r_readcon(path: str):
    import readcon

    return [(frame.xyz, frame) for frame in readcon.read_con(path)]


def r_readcon_arrays(path: str):
    """The (F, N, 3) positions of ``read_positions``, one block for every frame."""
    import readcon

    positions = np.asarray(readcon.read_positions(path, 1))
    return [(positions[i], positions) for i in range(positions.shape[0])]


READERS = {
    "chemfiles": r_chemfiles,
    "ase": r_ase,
    "ase_extxyz": r_ase_extxyz,
    "cextxyz": r_cextxyz,
    "mdanalysis": r_mdanalysis,
    "openbabel": r_openbabel,
    "read_dicts_regex": r_read_dicts_regex,
    "read_dicts": r_read_dicts,
    "readcon": r_readcon,
    "readcon_arrays": r_readcon_arrays,
}


def child(reader: str, path: str) -> None:
    fn = READERS[reader]
    fn(path)
    gc.collect()
    start = time.perf_counter()
    records = fn(path)
    seconds = time.perf_counter() - start
    positions = [np.asarray(p, dtype=np.float64) for p, _ in records]
    print(
        json.dumps(
            {
                "seconds": seconds,
                "frames": len(positions),
                "atoms": int(positions[0].shape[0]),
                "positions_sum": math.fsum(
                    float(v) for p in positions for v in p.ravel()
                ),
            }
        )
    )


def table_after(readme: Path, prefix: str) -> list[str]:
    """The first line of ``readme`` that starts with ``prefix`` and the first table after it, as written."""
    lines = readme.read_text().splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith(prefix))
    first = next(i for i in range(start, len(lines)) if lines[i].startswith("|"))
    end = next(
        (i for i in range(first, len(lines)) if not lines[i].startswith("|")),
        len(lines),
    )
    return [lines[start].rstrip()] + [line.rstrip() for line in lines[first:end]]


def in_id_order(frame) -> np.ndarray:
    """Positions of a CON frame in the order of their ids; CON groups the atoms of a frame by component."""
    xyz = np.asarray(frame.xyz, dtype=np.float64)
    ordered = np.empty_like(xyz)
    ordered[np.asarray(frame.atom_ids_array())] = xyz
    return ordered


def water_con(nc_path: Path, xyz_path: Path, out: Path) -> dict:
    import ase.io
    import readcon
    from scipy.io import netcdf_file

    frames = ase.io.read(str(xyz_path), ":")
    with netcdf_file(str(nc_path), "r", mmap=False) as nc:
        lengths = np.array(nc.variables["cell_lengths"].data, dtype=np.float64)
        angles = np.array(nc.variables["cell_angles"].data, dtype=np.float64)
        coords = np.array(nc.variables["coordinates"].data, dtype=np.float64)
    gap = float(
        max(np.abs(coords[i] - atoms.positions).max() for i, atoms in enumerate(frames))
    )
    for atoms, length, angle in zip(frames, lengths, angles):
        atoms.set_cell([*length, *angle])
        atoms.pbc = True
    con = [readcon.ConFrame.from_ase(atoms) for atoms in frames]
    readcon.write_con(str(out), con, precision=6)
    readcon.write_con(str(out) + ".gz", con, precision=6)
    back = readcon.read_con(str(out))
    return {
        "frames": len(frames),
        "atoms": len(frames[0]),
        "box_from": "water.nc",
        "xyz_vs_nc_largest_difference_A": gap,
        "cells": sorted(
            {
                tuple(map(float, length)) + tuple(map(float, angle))
                for length, angle in zip(lengths, angles)
            }
        ),
        "con_bytes": out.stat().st_size,
        "con_gz_bytes": Path(str(out) + ".gz").stat().st_size,
        "con_vs_xyz_largest_difference_A": float(
            max(
                np.abs(in_id_order(b) - a.positions).max() for a, b in zip(frames, back)
            )
        ),
    }


def no_cell_round_trip(xyz_path: Path, out: Path) -> dict:
    """ConFrame.from_ase on the first frame as ASE reads the XYZ file, with no cell, written and read back."""
    import ase.io
    import readcon

    atoms = ase.io.read(str(xyz_path), 0)
    readcon.write_con(str(out), [readcon.ConFrame.from_ase(atoms)], precision=6)
    lines = out.read_text().splitlines()
    try:
        readcon.read_con(str(out))
        verdict = "read"
    except OSError as error:
        verdict = f"{type(error).__name__}: {error}"
    return {
        "ase_cell": atoms.cell.array.tolist(),
        "ase_pbc": atoms.pbc.tolist(),
        "written_lines_3_4": lines[2:4],
        "read_back": verdict,
    }


def make_cu(bench_read: Path, work: Path, seed: int) -> None:
    """The ``make_xyz`` files and their CON copies; runs with the plugin on ``PYTHONPATH``, as ``make_xyz`` writes
    through it."""
    import ase.io
    import readcon

    spec = importlib.util.spec_from_file_location("extxyz_bench_read", bench_read)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    out = {}
    for n in SIZES:
        xyz = work / f"cu_{n}.xyz"
        np.random.seed(seed)
        module.make_xyz(xyz, n)
        atoms = ase.io.read(str(xyz), format="extxyz")
        frame = readcon.ConFrame.from_ase(atoms)
        frame.set_energy(float(atoms.get_potential_energy()))
        con = work / f"cu_{n}.con"
        readcon.write_con(str(con), [frame], precision=8)
        back = readcon.read_con(str(con))[0]
        out[n] = {
            "xyz": xyz.name,
            "xyz_bytes": xyz.stat().st_size,
            "con": con.name,
            "con_bytes": con.stat().st_size,
            "con_positions_largest_difference_A": float(
                np.abs(np.asarray(back.xyz) - atoms.positions).max()
            ),
            "con_forces_largest_difference": float(
                np.abs(np.asarray(back.forces_array()) - atoms.get_forces()).max()
            ),
            "con_has_forces": bool(back.has_forces),
        }
    print(json.dumps(out))


def main() -> int:
    import cc_common as cc
    import interleave

    work = cc.WORK / "cited_text"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    sources, paths = [], {}
    for name, (size, sha) in CHEMFILES["files"].items():
        url = f"https://raw.githubusercontent.com/chemfiles/benchmarks/{CHEMFILES['commit']}/files/{name}"
        if name in ("README.md", "bench.py"):
            url = f"https://raw.githubusercontent.com/chemfiles/benchmarks/{CHEMFILES['commit']}/{name}"
        paths[f"chemfiles/{name}"], got = cc.fetch(
            url, f"chemfiles-benchmarks-{CHEMFILES['commit'][:8]}-{name}", sha, size
        )
        sources.append(got)
    for name, (size, sha) in EXTXYZ["files"].items():
        url = f"https://raw.githubusercontent.com/libAtoms/extxyz/{EXTXYZ['commit']}/{name}"
        paths[f"extxyz/{name}"], got = cc.fetch(
            url, f"extxyz-{EXTXYZ['tag']}-{name.replace('/', '-')}", sha, size
        )
        sources.append(got)
    wheel, got = cc.fetch(
        PLUGIN["url"], PLUGIN["file"], PLUGIN["sha256"], PLUGIN["bytes"]
    )
    sources.append(got)
    site = work / "site"
    install = cc.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--no-deps",
            "--no-index",
            "--target",
            site,
            wheel,
        ]
    )
    water = work / "water"
    water.mkdir()
    shutil.copy(paths["chemfiles/water.xyz"], water / "water.xyz")
    shutil.copy(paths["chemfiles/water.xyz.gz"], water / "water.xyz.gz")
    with gzip.open(water / "water.xyz.gz", "rb") as handle:
        gz_same = handle.read() == (water / "water.xyz").read_bytes()
    conversion = water_con(
        paths["chemfiles/water.nc"], water / "water.xyz", water / "water.con"
    )
    no_cell = no_cell_round_trip(water / "water.xyz", work / "no_cell.con")
    env = {
        "PYTHONPATH": os.pathsep.join(
            [str(site), os.environ.get("PYTHONPATH", "")]
        ).rstrip(os.pathsep)
    }
    cu = work / "cu"
    cu.mkdir()
    cu_conversion = cc.child(
        [
            sys.executable,
            Path(__file__).resolve(),
            "make-cu",
            paths["extxyz/benchmarks/bench_read.py"],
            cu,
            interleave.SEED,
        ],
        env=env,
    )
    plan = {}
    for name in ("water.xyz", "water.xyz.gz"):
        for reader in WATER_READERS:
            path = water / (name.replace(".xyz", ".con") if reader in OURS else name)
            plan[f"{reader}-{name}"] = (reader, path)
    for n in SIZES:
        for reader in CU_READERS:
            plan[f"{reader}-cu{n}"] = (
                reader,
                cu / (f"cu_{n}.con" if reader in OURS else f"cu_{n}.xyz"),
            )
    first, cells = {}, {}
    for cell, (reader, path) in plan.items():
        cmd = [sys.executable, Path(__file__).resolve(), "child", reader, path]
        done = cc.run(cmd, env=env, check=False)
        first[cell] = {"reader": reader, "file": path.name, "exit": done["exit"]}
        if done["exit"] == 0:
            first[cell]["printed"] = json.loads(done["stdout"].strip().splitlines()[-1])
            cells[cell] = lambda cmd=cmd: cc.child(cmd, env=env)
        else:
            first[cell]["stderr_tail"] = done["stderr"][-1500:]
    timing = cc.campaign(cells)
    median = {cell: timing["cells"][cell]["median_s"] for cell in cells}

    def ratio(cell: str, other: str) -> float | None:
        return (
            median[cell] / median[other] if cell in median and other in median else None
        )

    ratios = {}
    for ours in OURS:
        for name in ("water.xyz", "water.xyz.gz"):
            for reader in WATER_READERS[: -len(OURS)]:
                ratios[f"{ours}_over_{reader}-{name}"] = ratio(
                    f"{ours}-{name}", f"{reader}-{name}"
                )
        for n in SIZES:
            for reader in CU_READERS[: -len(OURS)]:
                ratios[f"{ours}_over_{reader}-cu{n}"] = ratio(
                    f"{ours}-cu{n}", f"{reader}-cu{n}"
                )
    # The conda build of Open Babel installs no distribution metadata, so its library states its version.
    versions = cc.run(
        [
            sys.executable,
            "-c",
            (
                "import importlib.metadata as m; import json; "
                "from openbabel import openbabel as ob; print(json.dumps({**{d: m.version(d) for d in "
                "('chemfiles', 'ase', 'MDAnalysis', 'extxyz', 'ase-extxyz', 'readcon', 'numpy')}, "
                "'openbabel': ob.OBReleaseVersion()}))"
            ),
        ],
        env=env,
    )
    cc.write_record(
        "cited_text",
        {
            "claim": "chemfiles and extxyz on the systems of their own benchmarks, readcon on the same atoms as CON",
            "versions": json.loads(versions["stdout"].strip().splitlines()[-1]),
            "plugin_install": cc.brief(install),
            "published": {
                "chemfiles_linux": table_after(
                    paths["chemfiles/README.md"], "### Ubuntu Linux"
                ),
                "chemfiles_macos": table_after(
                    paths["chemfiles/README.md"], "### macOS"
                ),
                "extxyz_read": table_after(
                    paths["extxyz/README.md"], "Benchmark on a single-frame file"
                ),
                "extxyz_tokenizer": table_after(
                    paths["extxyz/README.md"], "`use_regex=False` in `read_dicts`"
                ),
            },
            "water": {"gzip_holds_the_xyz_bytes": gz_same, "conversion": conversion},
            "no_cell_round_trip": no_cell,
            "cu": cu_conversion,
            "first_runs": first,
            "timing": timing,
            "ratios": ratios,
        },
        sources,
    )
    return 0


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "child":
        child(sys.argv[2], sys.argv[3])
        raise SystemExit(0)
    if len(sys.argv) == 5 and sys.argv[1] == "make-cu":
        make_cu(Path(sys.argv[2]), Path(sys.argv[3]), int(sys.argv[4]))
        raise SystemExit(0)
    raise SystemExit(main())
