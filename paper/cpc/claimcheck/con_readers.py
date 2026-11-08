"""Read CON files with readcon of this checkout, with the readcon release on PyPI and with the eon module of ASE.

``verdicts(paths, root)`` returns, for each file, what each reader returned: the frames and the atoms of the
first frame, or the error text of a refused file.  For two readers that both read a file it gives the largest
difference of their positions; the two readcon builds also compare the SHA-256 of their positions as binary64.
The release runs in a child process whose ``PYTHONPATH`` starts with ``WORK/readcon-<version>``, where its
PyPI wheel for this Python, pinned by URL, size and SHA-256, is unpacked.  Run as a script, this module is
that child: it reads the files named in a JSON list and prints one JSON object.
"""

from __future__ import annotations

import collections
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

import cc_common as cc
import numpy as np

RELEASE = "0.17.3"
RELEASE_WHEEL = {
    "url": "https://files.pythonhosted.org/packages/7f/08/051666ea741c60c5bf2aef82449d0cebaf8134fa2b7d9b837baaef349670/"
    "readcon-0.17.3-cp312-cp312-manylinux2014_x86_64.manylinux_2_17_x86_64.whl",
    "name": "readcon-0.17.3-cp312-cp312-manylinux2014_x86_64.manylinux_2_17_x86_64.whl",
    "size": 971483,
    "sha": "8b126f449200e30718ecc909298739ee90f0d6afb376c2fa051d24b83cc40eea",
}


def error_text(error: Exception) -> str:
    return f"{type(error).__name__}: {error}"


def read_readcon(readcon, path: Path) -> tuple[dict, list | None]:
    try:
        frames = readcon.read_con(str(path))
    except Exception as error:  # noqa: BLE001
        return {"error": error_text(error)}, None
    positions = [
        np.ascontiguousarray(frame.coords_array(), dtype=np.float64) for frame in frames
    ]
    digest = hashlib.sha256(b"".join(p.tobytes() for p in positions)).hexdigest()
    return {
        "frames": len(frames),
        "atoms": int(positions[0].shape[0]) if positions else 0,
        "positions_sha256": digest,
    }, positions


def read_ase(path: Path) -> tuple[dict, list | None]:
    import ase.io

    try:
        images = ase.io.read(str(path), index=":", format="eon")
    except Exception as error:  # noqa: BLE001
        return {"error": error_text(error)}, None
    positions = [np.asarray(atoms.positions, dtype=np.float64) for atoms in images]
    return {
        "frames": len(images),
        "atoms": int(positions[0].shape[0]) if positions else 0,
    }, positions


def largest_difference(a: list | None, b: list | None) -> float | None:
    if (
        a is None
        or b is None
        or len(a) != len(b)
        or any(x.shape != y.shape for x, y in zip(a, b))
    ):
        return None
    return max(
        (float(np.max(np.abs(x - y))) if x.size else 0.0 for x, y in zip(a, b)),
        default=0.0,
    )


def release_site() -> tuple[Path, dict]:
    wheel, got = cc.fetch(
        RELEASE_WHEEL["url"],
        RELEASE_WHEEL["name"],
        RELEASE_WHEEL["sha"],
        RELEASE_WHEEL["size"],
    )
    site = cc.WORK / f"readcon-{RELEASE}"
    if not (site / "readcon").is_dir():
        site.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(wheel) as archive:
            archive.extractall(site)
    return site, got


def release_reads(paths: list[Path]) -> tuple[dict, dict]:
    site, got = release_site()
    with tempfile.NamedTemporaryFile(
        "w", suffix=".json", dir=cc.WORK, delete=False
    ) as handle:
        json.dump([str(p) for p in paths], handle)
        listing = handle.name
    try:
        path = os.pathsep.join(
            p for p in (str(site), os.environ.get("PYTHONPATH", "")) if p
        )
        env = {**os.environ, "PYTHONPATH": path}
        proc = subprocess.run(
            [sys.executable, str(Path(__file__).resolve()), listing],
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )
    finally:
        os.unlink(listing)
    if proc.returncode != 0:
        raise SystemExit(
            f"readcon {RELEASE} child exited {proc.returncode}: {proc.stderr[-2000:]}"
        )
    out = json.loads(proc.stdout.strip().splitlines()[-1])
    if out["version"] != RELEASE:
        raise SystemExit(f"the child imported readcon {out['version']}, not {RELEASE}")
    return out, got


def verdicts(paths: list[Path], root: Path) -> dict:
    """Every file under ``root`` in ``paths``, read by the three readers, and the counts of the outcomes."""
    import ase
    import readcon

    release, wheel = release_reads(paths)
    files = {}
    for path in paths:
        name = str(path.relative_to(root))
        here, here_positions = read_readcon(readcon, path)
        ase_row, ase_positions = read_ase(path)
        old = release["files"][str(path)]
        row = {
            "bytes": path.stat().st_size,
            "readcon": here,
            "readcon_release": old,
            "ase": ase_row,
            "readcon_vs_ase_largest_difference": largest_difference(
                here_positions, ase_positions
            ),
        }
        if "positions_sha256" in here and "positions_sha256" in old:
            row["readcon_release_same_positions"] = (
                here["positions_sha256"] == old["positions_sha256"]
            )
        files[name] = row

    def outcome(reader: str) -> dict:
        refused = collections.Counter(
            row[reader]["error"] for row in files.values() if "error" in row[reader]
        )
        return {
            "read": sum("error" not in row[reader] for row in files.values()),
            "refused": sum(refused.values()),
            "errors": dict(refused.most_common()),
        }

    differences = [
        row["readcon_vs_ase_largest_difference"]
        for row in files.values()
        if row["readcon_vs_ase_largest_difference"] is not None
    ]
    summary = {
        "files": len(files),
        "readcon": outcome("readcon"),
        "readcon_release": outcome("readcon_release"),
        "ase": outcome("ase"),
        "refused_by_readcon_only": sorted(
            n
            for n, r in files.items()
            if "error" in r["readcon"] and "error" not in r["readcon_release"]
        ),
        "refused_by_release_only": sorted(
            n
            for n, r in files.items()
            if "error" in r["readcon_release"] and "error" not in r["readcon"]
        ),
        "read_by_ase_refused_by_readcon": sorted(
            n
            for n, r in files.items()
            if "error" in r["readcon"] and "error" not in r["ase"]
        ),
        "readcon_and_release_same_positions": all(
            r.get("readcon_release_same_positions", True) for r in files.values()
        ),
        "readcon_vs_ase_files_compared": len(differences),
        "readcon_vs_ase_largest_difference": max(differences, default=None),
    }
    return {
        "readers": {
            "readcon": readcon.__version__,
            "readcon_release": release["version"],
            "readcon_release_wheel": wheel,
            "ase": ase.__version__,
        },
        "summary": summary,
        "files": files,
    }


def child(listing: str) -> int:
    import readcon

    paths = [Path(p) for p in json.loads(Path(listing).read_text())]
    files = {str(p): read_readcon(readcon, p)[0] for p in paths}
    print(json.dumps({"version": readcon.__version__, "files": files}))
    return 0


if __name__ == "__main__":
    raise SystemExit(child(sys.argv[1]))
