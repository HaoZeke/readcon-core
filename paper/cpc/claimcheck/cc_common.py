"""Shared parts of the claim-check runs: pinned downloads, records, fresh-process cells and Rust builds.

Every claim-check script reads what it checks from a download pinned by URL, size and SHA-256, from the
readcon-core tree ``READCON_SRC`` or from the programs of the pixi environments, and writes one record,
``data/claimcheck/<name>.json``.  A record holds the script, the commands it ran, the downloads it read, the
provenance of the readcon build and the host (``provenance.record``), and the results; host paths are replaced
as in ``bench/portable.py``, and the directory of the programs of the host system becomes ``$SYSTEM_BIN``.
Downloads and builds go to ``CC_WORK`` (default ``claimcheck/work``, which git
ignores), and a download that is already there is checked against its pin instead of fetched again.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
CPC = HERE.parent
sys.path.insert(0, str(CPC / "scripts"))
sys.path.insert(0, str(CPC / "bench"))
import interleave  # noqa: E402
import portable  # noqa: E402
import provenance  # noqa: E402

WORK = Path(os.environ.get("CC_WORK", HERE / "work")).resolve()
RECORDS = CPC / "data" / "claimcheck"
USER_AGENT = "readcon-cpc-claimcheck"
# The programs of the host system, outside every pixi environment; records name the directory $SYSTEM_BIN.
SYSTEM_BIN = Path("/usr/bin")


def readcon_src() -> Path:
    return Path(os.environ.get("READCON_SRC", "").strip() or provenance.REPO).resolve()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def fetch(
    url: str, name: str, sha: str | None = None, size: int | None = None
) -> tuple[Path, dict]:
    """``url`` saved as ``WORK/downloads/name``; stop unless it has ``size`` bytes and SHA-256 ``sha``.

    Without a pin the first download sets it, and the record keeps what was read."""
    path = WORK / "downloads" / name
    if not path.is_file():
        path.parent.mkdir(parents=True, exist_ok=True)
        part = path.with_name(path.name + ".part")
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        for attempt in range(5):
            try:
                with (
                    urllib.request.urlopen(request, timeout=120) as response,
                    part.open("wb") as out,
                ):
                    shutil.copyfileobj(response, out)
                break
            except urllib.error.HTTPError as error:
                if error.code < 500 or attempt == 4:
                    raise
                time.sleep(2 * (attempt + 1))
            except urllib.error.URLError:
                if attempt == 4:
                    raise
                time.sleep(2 * (attempt + 1))
        part.rename(path)
    got = {
        "url": url,
        "file": name,
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
    }
    if sha is not None and got["sha256"] != sha:
        raise SystemExit(f"{name}: SHA-256 {got['sha256']}, pinned {sha}")
    if size is not None and got["bytes"] != size:
        raise SystemExit(f"{name}: {got['bytes']} bytes, pinned {size}")
    return path, got


def run(
    cmd: list, cwd: Path | None = None, env: dict | None = None, check: bool = True
) -> dict:
    """Run ``cmd``; return its command line, exit status, seconds and its output.

    ``stdout`` and ``stderr`` hold all of it, for the caller to parse; ``brief`` keeps the tails for a record."""
    start = time.perf_counter()
    proc = subprocess.run(
        [str(c) for c in cmd],
        cwd=cwd,
        env={**os.environ, **(env or {})},
        capture_output=True,
        text=True,
        check=False,
    )
    out = {
        "command": [str(c) for c in cmd],
        "exit": proc.returncode,
        "seconds": round(time.perf_counter() - start, 3),
        "stdout": proc.stdout,
        "stderr": proc.stderr,
    }
    print("+", " ".join(out["command"]), f"-> exit {proc.returncode}", flush=True)
    if check and proc.returncode != 0:
        print(proc.stdout[-4000:], proc.stderr[-4000:], sep="\n", flush=True)
        raise SystemExit(f"{cmd[0]} exited {proc.returncode}")
    return out


def brief(done: dict, tail: int = 2000) -> dict:
    """A result of ``run`` for a record: the command, exit status, seconds and the tails of its output."""
    return {
        "command": done["command"],
        "exit": done["exit"],
        "seconds": done["seconds"],
        "stdout_tail": done["stdout"][-tail:],
        "stderr_tail": done["stderr"][-tail:],
    }


def child(cmd: list, env: dict | None = None) -> dict:
    """One fresh process that prints one JSON object on its last line of output."""
    proc = subprocess.run(
        [str(c) for c in cmd],
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, **(env or {})},
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"{' '.join(str(c) for c in cmd)} exited {proc.returncode}: {proc.stderr[-2000:]}"
        )
    return json.loads(proc.stdout.strip().splitlines()[-1])


def campaign(cells: dict, **kwargs) -> dict:
    """``interleave.campaign`` of the paper: interleaved rounds of fresh processes, median and type-7 IQR."""
    return interleave.campaign(cells, **kwargs)


def rustc(source: Path, out: Path, flags: list[str] | None = None) -> dict:
    """Compile one Rust file with the ``rustc`` of the environment into ``out``."""
    out.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "rustc",
        "--edition",
        "2021",
        "-O",
        *(flags or []),
        str(source),
        "-o",
        str(out),
    ]
    done = run(cmd)
    version = subprocess.run(
        ["rustc", "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    return {
        "command": done["command"],
        "rustc": version,
        "source_sha256": sha256(source),
        "sha256": sha256(out),
    }


def write_record(name: str, body: dict, sources: list[dict] | None = None) -> Path:
    """``data/claimcheck/<name>.json`` with the script, the downloads and the provenance of this run."""
    RECORDS.mkdir(parents=True, exist_ok=True)
    record = {
        "check": name,
        "script": f"paper/cpc/claimcheck/{name}.py",
        "downloads": sources or [],
        "provenance": provenance.record(),
        "hardware": provenance.hardware(),
        **body,
    }
    record = {key: value for key, value in record.items() if value is not None}
    pairs = sorted(
        portable.prefixes() + [(str(SYSTEM_BIN), "$SYSTEM_BIN")],
        key=lambda pair: -len(pair[0]),
    )
    path = RECORDS / f"{name}.json"
    path.write_text(
        json.dumps(portable.portable(record, pairs), indent=1, sort_keys=False) + "\n"
    )
    print(f"wrote {path.relative_to(CPC)}", flush=True)
    return path


def con_frame(xs: list[str], symbol: str = "H", ys: list[str] | None = None) -> str:
    """One CON frame of ``len(xs)`` atoms of one type, whose x tokens are ``xs`` as given."""
    head = [
        "claimcheck",
        "",
        "10.0 10.0 10.0",
        "90.0 90.0 90.0",
        "0 0",
        "0 0 0",
        "1",
        str(len(xs)),
        "1.0",
        symbol,
        "Coordinates of Component 1",
    ]
    ys = ys or ["0"] * len(xs)
    return (
        "\n".join(head + [f"{x} {y} 0 0 {i}" for i, (x, y) in enumerate(zip(xs, ys))])
        + "\n"
    )
