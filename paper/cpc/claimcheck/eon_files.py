"""Every CON file of the eOn 3.6.0 release and of a published reaction-path dataset, read by three readers.

The eOn release is the source tarball of tag v3.6.0, the asset that the conda-forge recipe of eOn 3.6.0 pins by
the same SHA-256.  The dataset is the Materials Cloud archive of the reaction-path workflow of
goswamiReproducibleOrchestrationBest2026 (record z41cv-q4h43), with the endpoints and bands of its five
reactions.  Each ``.con`` member of the two archives is read by readcon of this checkout, by the readcon release
on PyPI and by the eon module of ASE (``con_readers``); the record keeps every verdict and error text.
"""

from __future__ import annotations

import shutil
import tarfile
from pathlib import Path

import cc_common as cc
import con_readers

ARCHIVES = {
    "eon-v3.6.0": {
        "url": "https://github.com/TheochemUI/eOn/releases/download/v3.6.0/eon-v3.6.0.tar.xz",
        "file": "eon-v3.6.0.tar.xz",
        "bytes": 16488468,
        "sha256": "5e9008c6926b78563ee90f17189c4ad371314ddc02ac559ccec50a0e85d9bd7e",
    },
    "neb-orchestrator-data": {
        "url": "https://archive.materialscloud.org/api/records/z41cv-q4h43/files/neb_orchestrator_data.tar.xz/content",
        "file": "neb_orchestrator_data.tar.xz",
        "bytes": 25697528,
        "sha256": "a95dd4bf30b0bcee8892efeb48ddc39115510c02ef7e9b5ba9792108b92a8971",
        "doi": "10.24435/materialscloud:15-7q",
    },
}


def con_members(archive: Path, out: Path) -> list[Path]:
    with tarfile.open(archive) as tar:
        members = [
            m for m in tar.getmembers() if m.isfile() and m.name.endswith(".con")
        ]
        tar.extractall(out, members=members, filter="data")
    return sorted(out / m.name for m in members)


def main() -> int:
    work = cc.WORK / "eon_files"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    sources, archives = [], {}
    for label, pin in ARCHIVES.items():
        path, got = cc.fetch(pin["url"], pin["file"], pin["sha256"], pin["bytes"])
        sources.append(got)
        root = work / label
        files = con_members(path, root)
        archives[label] = {
            "doi": pin.get("doi"),
            "con_files": len(files),
            **con_readers.verdicts(files, root),
        }
        print(label, archives[label]["summary"], flush=True)
    cc.write_record(
        "eon_files",
        {
            "claim": "readcon reads the CON files that eOn ships and that a published workflow released",
            "archives": archives,
            "summary": {label: a["summary"] for label, a in archives.items()},
        },
        sources,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
