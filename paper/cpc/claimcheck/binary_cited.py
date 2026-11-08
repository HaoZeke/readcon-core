"""libxtc and TNG on the trajectories their authors ship, against xdrfile, MDAnalysis and readcon.

The libraries and drivers are those of ``bench/binary_formats.py``, built by its ``build`` step into
``WORK/binary_cited`` from the pinned sources there: xdrfile 1.1.4, libxtc with the flags of its ``setup.py``,
which include ``-ffast-math`` and ``-march=native``, and the TNG library.  One more libxtc driver,
``read_libxtc_plain``, compiles the same kernel without those two flags.

libxtc: ``examples/md.xtc`` of the libxtc source, 20,138 atoms in 201 frames, the atom count of the Small system
of krylovLibxtcEfficientLibrary2021, and ``tests/data/pep.xtc`` with the reference ``pep.npy`` of its tests, read
by ``read_xdrfile``, both libxtc drivers and MDAnalysis.  TNG: ``argon_npt_compressed.tng`` and
``water_npt_compressed_trjconv.tng`` of the ``example_files`` of the TNG repository at the commit of the campaign,
which store the positions and the box every 5000 frames.  ``read_tng`` of the campaign takes files of stride 1
only, so ``tng_read.c`` reads them in both modes, built against the TNG library of the campaign, the one the cells
time, and against the last tag of TNG, ``TNG_TAG``.  ``readcon_read`` writes the positions that xdrfile and TNG
return as CON with two decimals, the 0.01 angstrom of the 0.001 nm of both formats, and as the binary image of
that CON file.  The cells of ``interleave.campaign`` time every reader on every file with all frames held, on one
thread, a fresh process for every run.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

import cc_common as cc
import numpy as np
import tomllib

HERE = Path(__file__).resolve().parent
WORKDIR = cc.WORK / "binary_cited"
os.environ["BENCH_WORK"] = str(WORKDIR / "bench")
os.environ["BENCH_RESULTS"] = str(WORKDIR / "results")
import binary_formats as bf

TNG_COMMIT = bf.SOURCES["tng"]["commit"]
TNG_TAG = {
    "tag": "v1.8.2",
    "commit": "3cd824df250a72f2660f5ac8a6d17f1c327947b8",
    "bytes": 335789,
    "url": "https://gitlab.com/gromacs/tng/-/archive/3cd824df250a72f2660f5ac8a6d17f1c327947b8/"
    "tng-3cd824df250a72f2660f5ac8a6d17f1c327947b8.tar.gz",
    "sha256": "12da3945c485c39558e9e08c52ceb3268bb5720cdef38f62fc981a65ee914373",
}
TNG_FILES = {
    "argon_npt_compressed.tng": (
        403693,
        "507a0838da979333d7eeac1403c12cb8fb1ebe4c4cbc3bcdae0bca5f69321773",
    ),
    "water_npt_compressed_trjconv.tng": (
        879275,
        "3b99de8af7436c3995b3e554a0c7cbab4ea2aa60186dfe742e3efdc8f87c96c6",
    ),
}
UNSAFE = ("-ffast-math", "-march=native")
XTC_TOLERANCE = bf.TOLERANCE["traj.xtc"][0]
TWO_DECIMALS = bf.TOLERANCE["traj_two.con"][0]


def dumped(
    driver: str, path: Path, tmp: Path, mode: str = "all"
) -> tuple[dict, np.ndarray | None, np.ndarray | None]:
    try:
        printed, pos, box = bf.dump(driver, mode, 1, path, tmp)
    except subprocess.CalledProcessError as error:
        return (
            {"error": f"exit {error.returncode}: {(error.stderr or '')[-800:]}"},
            None,
            None,
        )
    return printed, pos, box


def against(got: np.ndarray | None, ref: np.ndarray, tolerance: float) -> dict:
    if got is None or got.shape != ref.shape:
        return {"compared": False}
    off = np.abs(got - ref).reshape(-1, 3).max(axis=1) > tolerance
    return {
        "compared": True,
        "largest_difference_A": float(np.abs(got - ref).max()),
        "tolerance_A": tolerance,
        "atom_frames": int(off.size),
        "atom_frames_beyond": int(off.sum()),
        "percent_beyond": round(100.0 * off.sum() / off.size, 3),
    }


def mdanalysis_xtc(path: Path) -> np.ndarray:
    from MDAnalysis.lib.formats.libmdaxdr import XTCFile

    with XTCFile(str(path)) as xtc:
        return np.concatenate(
            [xtc.read().x.astype(np.float64).ravel() * 10.0 for _ in range(len(xtc))]
        )


def build_plain(root: Path) -> list:
    lx = root / "libxtc"
    flags = [f for f in bf.LIBXTC_FLAGS if f not in UNSAFE]
    kernel = bf.BUILD / "obj" / "xtc_unpack_plain.o"
    steps = [
        [bf.GXX, *flags, "-c", lx / "xtc_unpack.cpp", "-o", kernel],
        [
            bf.GXX,
            *bf.C_FLAGS,
            "-std=c++11",
            "-fopenmp",
            "-I",
            bf.DRIVERS,
            "-I",
            lx,
            bf.DRIVERS / "read_libxtc.cpp",
            kernel,
            "-o",
            bf.BIN / "read_libxtc_plain",
        ],
    ]
    for cmd in steps:
        cc.run(cmd)
    return [
        [Path(str(c)).name if c in (bf.GCC, bf.GXX) else str(c) for c in cmd]
        for cmd in steps
    ]


def build_tng(root: Path, commit: str) -> dict:
    """libtng_io.a of the TNG source ``root`` with the flags of the native-reader campaign, and ``tng_read.c``
    against it as ``BIN/tng_read_COMMIT``."""
    out = WORKDIR / f"tng-{commit[:8]}"
    shutil.rmtree(out, ignore_errors=True)
    (out / "obj").mkdir(parents=True)
    (out / "include" / "tng").mkdir(parents=True)
    version = bf.tng_version(root)
    text = (root / "include" / "tng" / "version.h.in").read_text()
    for key, value in version.items():
        text = text.replace(f"@{key}@", value)
    (out / "include" / "tng" / "version.h").write_text(text)
    objs, steps = [], []
    for source in sorted((root / "src" / "compression").glob("*.c")) + [
        root / "src/lib/tng_io.c",
        root / "src/lib/md5.c",
    ]:
        obj = out / "obj" / f"tng_{source.stem}.o"
        steps.append(
            [
                bf.GCC,
                *bf.C_FLAGS,
                *bf.TNG_FLAGS,
                "-I",
                root / "include",
                "-I",
                out / "include",
                "-c",
                source,
                "-o",
                obj,
            ]
        )
        objs.append(obj)
    steps.append(["ar", "rcs", out / "libtng_io.a", *objs])
    steps.append(
        [
            bf.GCC,
            *bf.C_FLAGS,
            *bf.TNG_FLAGS,
            "-I",
            bf.DRIVERS,
            "-I",
            root / "include",
            "-I",
            out / "include",
            HERE / "tng_read.c",
            out / "libtng_io.a",
            "-lz",
            "-lm",
            "-o",
            bf.BIN / f"tng_read_{commit[:8]}",
        ]
    )
    for cmd in steps:
        cc.run(cmd)
    return {
        "commit": commit,
        "version": version["TNG_IO_VERSION"],
        "driver": f"tng_read_{commit[:8]}",
        "commands": [
            [Path(str(c)).name if c in (bf.GCC, bf.GXX) else str(c) for c in cmd]
            for cmd in steps
        ],
    }


def write_two(pos: np.ndarray, box: np.ndarray, out: Path, tmp: Path) -> dict:
    ref, cell = tmp / f"{out.stem}.f64", tmp / f"{out.stem}_box.f64"
    pos.tofile(ref)
    box.tofile(cell)
    done = cc.run([bf.BIN / "readcon_read", "write-con", ref, cell, out, "2"])
    printed, back, back_box = dumped("readcon_read", out, tmp)
    return {
        "file": out.name,
        "bytes": out.stat().st_size,
        "write": cc.brief(done),
        "read_back": printed,
        "positions": against(back, pos, TWO_DECIMALS),
        "box": against(back_box, box, TWO_DECIMALS),
    }


def build_readcon_cited() -> dict:
    """The cited-check driver, built like ``binary_formats.build_readcon`` against this source."""
    source = HERE / "rust" / "readcon_cited.rs"
    crate = bf.WORK / "readcon_read"
    crate.mkdir(parents=True, exist_ok=True)
    library = tomllib.loads((bf.SRC / "Cargo.toml").read_text())
    profile = library.get("profile", {}).get("release", {})
    lines = [
        "[package]",
        'name = "binary-formats-readcon"',
        'version = "0.0.0"',
        f'edition = "{library["package"]["edition"]}"',
        "publish = false",
        "",
        "[[bin]]",
        'name = "readcon_read"',
        f"path = {json.dumps(str(source))}",
        "",
        "[dependencies]",
        f'readcon-core = {{ path = {json.dumps(str(bf.SRC))}, features = ["parallel"] }}',
        "",
        "[profile.release]",
        *(f"{key} = {json.dumps(value)}" for key, value in profile.items()),
        "",
        "[workspace]",
        "",
    ]
    (crate / "Cargo.toml").write_text("\n".join(lines))
    return _build_cited_crate(crate)


def _build_cited_crate(crate: Path) -> dict:
    shutil.copyfile(bf.SRC / "Cargo.lock", crate / "Cargo.lock")
    bf.run(
        ["cargo", "build", "--release"],
        cwd=crate,
        env={**os.environ, "CARGO_TARGET_DIR": str(bf.WORK / "target")},
    )
    library, driver = bf.locked(bf.SRC / "Cargo.lock"), bf.locked(crate / "Cargo.lock")
    driver.pop("binary-formats-readcon 0.0.0")
    other = sorted(name for name in driver if driver[name] != library.get(name))
    if other:
        raise SystemExit(
            f"readcon_cited locks other packages than the library: {other}"
        )
    binary = bf.WORK / "target" / "release" / "readcon_read"
    shutil.copy2(binary, bf.BIN / "readcon_read")
    return {
        "command": ["cargo", "build", "--release"],
        "locked_packages": len(driver),
        "locked_as_library": True,
        "source": "claimcheck/rust/readcon_cited.rs",
    }


def main() -> int:
    bf.build_readcon = build_readcon_cited
    bf.build()
    root = bf.WORK / "src" / bf.SOURCES["libxtc"]["dir"]
    plain = build_plain(root)
    sources = []
    archive, got = cc.fetch(
        TNG_TAG["url"],
        f"tng-{TNG_TAG['commit']}.tar.gz",
        TNG_TAG["sha256"],
        TNG_TAG["bytes"],
    )
    sources.append(got)
    unpacked = WORKDIR / "src"
    shutil.rmtree(unpacked, ignore_errors=True)
    unpacked.mkdir(parents=True)
    with tarfile.open(archive) as tar:
        tar.extractall(unpacked, filter="data")
    tng_builds = {
        "campaign": build_tng(bf.WORK / "src" / bf.SOURCES["tng"]["dir"], TNG_COMMIT),
        TNG_TAG["tag"]: build_tng(
            unpacked / f"tng-{TNG_TAG['commit']}", TNG_TAG["commit"]
        ),
    }
    tng = {}
    for name, (size, sha) in TNG_FILES.items():
        tng[name], got = cc.fetch(
            f"https://gitlab.com/gromacs/tng/-/raw/{TNG_COMMIT}/example_files/{name}",
            f"tng-{TNG_COMMIT[:8]}-{name}",
            sha,
            size,
        )
        sources.append(got)
    files = WORKDIR / "files"
    shutil.rmtree(files, ignore_errors=True)
    files.mkdir(parents=True)
    md = files / "md.xtc"
    shutil.copy(root / "examples" / "md.xtc", md)
    pep = root / "tests" / "data" / "pep.xtc"
    checks: dict = {}
    with tempfile.TemporaryDirectory(dir=WORKDIR) as tmp_name:
        tmp = Path(tmp_name)
        for label, path in (("md.xtc", md), ("pep.xtc", pep)):
            xdr_printed, xdr_pos, xdr_box = dumped("read_xdrfile", path, tmp)
            mda = mdanalysis_xtc(path)
            entry = {
                "bytes": path.stat().st_size,
                "sha256": cc.sha256(path),
                "read_xdrfile": xdr_printed,
                "mdanalysis_vs_xdrfile": against(mda, xdr_pos, XTC_TOLERANCE),
            }
            for driver in ("read_libxtc", "read_libxtc_plain"):
                printed, pos, _ = dumped(driver, path, tmp)
                entry[driver] = {
                    "printed": printed,
                    "vs_xdrfile": against(pos, xdr_pos, XTC_TOLERANCE),
                    "vs_mdanalysis": against(pos, mda, XTC_TOLERANCE),
                }
            if label == "pep.xtc":
                reference = (
                    np.load(root / "tests" / "data" / "pep.npy")
                    .astype(np.float64)
                    .ravel()
                    * 10.0
                )
                entry["pep_npy_vs_xdrfile"] = against(reference, xdr_pos, XTC_TOLERANCE)
                entry["pep_npy_vs_read_libxtc"] = against(
                    reference, dumped("read_libxtc", path, tmp)[1], XTC_TOLERANCE
                )
            else:
                entry["con"] = write_two(xdr_pos, xdr_box, files / "md_two.con", tmp)
            checks[label] = entry
        for name, path in tng.items():
            entry = {
                "bytes": path.stat().st_size,
                "sha256": cc.sha256(path),
                "read_tng_of_the_campaign": dumped("read_tng", path, tmp)[0],
            }
            reads = {}
            for build, built in tng_builds.items():
                for mode in ("all", "frames"):
                    reads[f"{build}-{mode}"] = dumped(built["driver"], path, tmp, mode)
            pos, box = reads["campaign-all"][1], reads["campaign-all"][2]
            if pos is None:
                raise SystemExit(
                    f"TNG {TNG_COMMIT[:8]} does not read {name}: {reads['campaign-all'][0]}"
                )
            entry["tng_read"] = {
                key: {"printed": printed, "vs_campaign_all": against(got, pos, 0.0)}
                for key, (printed, got, _) in reads.items()
            }
            stem = name.split("_")[0]
            entry["con"] = write_two(pos, box, files / f"{stem}_two.con", tmp)
            checks[name] = entry
    images = {}
    for stem in ("md", "argon", "water"):
        image = files / f"{stem}_two.conbin"
        images[stem] = {
            "file": image.name,
            "write": cc.brief(
                cc.run(
                    [
                        bf.BIN / "readcon_read",
                        "write-conbin",
                        files / f"{stem}_two.con",
                        image,
                    ]
                )
            ),
            "bytes": image.stat().st_size,
        }

    def cell(driver: str, path: Path, mode: str = "all"):
        return lambda: bf.summary(
            bf.run(
                [bf.BIN / driver, "time", mode, "1", path],
                capture_output=True,
                text=True,
            ).stdout
        )

    cells = {
        "xdrfile-md": cell("read_xdrfile", md),
        "libxtc-md": cell("read_libxtc", md),
        "libxtc-plain-md": cell("read_libxtc_plain", md),
        "readcon-md-two": cell("readcon_read", files / "md_two.con"),
        "readcon-md-image": cell("readcon_read", files / "md_two.conbin", "image"),
        "tng-argon": cell(
            tng_builds["campaign"]["driver"], tng["argon_npt_compressed.tng"]
        ),
        "readcon-argon-two": cell("readcon_read", files / "argon_two.con"),
        "readcon-argon-image": cell(
            "readcon_read", files / "argon_two.conbin", "image"
        ),
        "tng-water": cell(
            tng_builds["campaign"]["driver"], tng["water_npt_compressed_trjconv.tng"]
        ),
        "readcon-water-two": cell("readcon_read", files / "water_two.con"),
        "readcon-water-image": cell(
            "readcon_read", files / "water_two.conbin", "image"
        ),
    }
    timing = cc.campaign(cells)
    median = {name: timing["cells"][name]["median_s"] for name in cells}
    for stem, image in images.items():
        image["checksum_of_the_con_read"] = (
            timing["cells"][f"readcon-{stem}-image"]["printed"]["checksum"]
            == timing["cells"][f"readcon-{stem}-two"]["printed"]["checksum"]
        )
    build = bf.load("build.json")
    cc.write_record(
        "binary_cited",
        {
            "claim": "libxtc and TNG read the files their authors ship; readcon reads the same positions as CON",
            "sources": build["sources"],
            "libxtc_flags": bf.LIBXTC_FLAGS,
            "libxtc_plain_build": plain,
            "tng_builds": tng_builds,
            "checks": checks,
            "images": images,
            "timing": timing,
            "ratios": {
                "readcon_two_over_xdrfile_md": median["readcon-md-two"]
                / median["xdrfile-md"],
                "readcon_two_over_libxtc_md": median["readcon-md-two"]
                / median["libxtc-md"],
                "readcon_image_over_xdrfile_md": median["readcon-md-image"]
                / median["xdrfile-md"],
                "readcon_image_over_libxtc_md": median["readcon-md-image"]
                / median["libxtc-md"],
                "libxtc_over_xdrfile_md": median["libxtc-md"] / median["xdrfile-md"],
                "readcon_two_over_tng_argon": median["readcon-argon-two"]
                / median["tng-argon"],
                "readcon_image_over_tng_argon": median["readcon-argon-image"]
                / median["tng-argon"],
                "readcon_two_over_tng_water": median["readcon-water-two"]
                / median["tng-water"],
                "readcon_image_over_tng_water": median["readcon-water-image"]
                / median["tng-water"],
            },
        },
        sources,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
