"""Build a Rust program of the bench against the readcon-core tree ``READCON_SRC``, locked as the library is.

``build(name, source, features, extra)`` writes a crate under
``BENCH_WORK/<name>`` whose one binary is ``source``: the edition and
``[profile.release]`` of the library's ``Cargo.toml``, readcon-core by path
with ``features``, the dependencies of ``extra`` (name: TOML value), an empty
``[workspace]`` and a copy of the library's ``Cargo.lock``.  ``cargo build
--release`` must then lock every package the library locks at the library's
version, source and checksum; a package the library does not lock may come
only from ``extra``.  The record of the build names the manifest, the command,
the program's SHA-256 and the library digest, with host paths replaced as in
``portable.py``.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import tomllib

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "scripts"))
sys.path.insert(0, str(HERE))
import portable
import provenance

WORK = Path(os.environ.get("BENCH_WORK", HERE / "work" / "r0174"))


def readcon_src() -> Path:
    return Path(os.environ.get("READCON_SRC", "").strip() or provenance.REPO).resolve()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest(
    name: str, source: Path, features: list[str], extra: dict[str, str]
) -> str:
    src = readcon_src()
    library = tomllib.loads((src / "Cargo.toml").read_text())
    profile = library.get("profile", {}).get("release", {})
    lines = [
        "[package]",
        f'name = "{name}"',
        'version = "0.0.0"',
        f'edition = "{library["package"]["edition"]}"',
        "publish = false",
        "",
        "[[bin]]",
        f'name = "{name}"',
        f"path = {json.dumps(str(source))}",
        "",
        "[dependencies]",
        f"readcon-core = {{ path = {json.dumps(str(src))}, features = {json.dumps(features)} }}",
        *(f"{key} = {value}" for key, value in extra.items()),
        "",
        "[profile.release]",
        *(f"{key} = {json.dumps(value)}" for key, value in profile.items()),
        "",
        "[workspace]",
        "",
    ]
    return "\n".join(lines)


def locked(path: Path) -> dict[str, tuple]:
    return {
        f"{p['name']} {p['version']}": (p.get("source"), p.get("checksum"))
        for p in tomllib.loads(path.read_text())["package"]
    }


def build(
    name: str,
    source: Path,
    features: list[str],
    extra: dict[str, str] | None = None,
    env: dict[str, str] | None = None,
) -> dict:
    """Build ``source`` as the binary ``name``; return its path under ``program`` and the build record."""
    extra = extra or {}
    src = readcon_src()
    crate = WORK / name
    crate.mkdir(parents=True, exist_ok=True)
    text = manifest(name, source, features, extra)
    (crate / "Cargo.toml").write_text(text)
    (crate / "Cargo.lock").write_bytes((src / "Cargo.lock").read_bytes())
    target = WORK / f"target-{name}"
    command = ["cargo", "build", "--release"]
    print("+", " ".join(command), "in", crate, flush=True)
    subprocess.run(
        command,
        cwd=crate,
        check=True,
        env={**os.environ, **(env or {}), "CARGO_TARGET_DIR": str(target)},
    )
    library, program = locked(src / "Cargo.lock"), locked(crate / "Cargo.lock")
    program.pop(f"{name} 0.0.0")
    names = {key.rsplit(" ", 1)[0] for key in library}
    other = sorted(
        key
        for key, pin in program.items()
        if (key in library and pin != library[key])
        or (key not in library and key.rsplit(" ", 1)[0] in names)
    )
    if other:
        raise SystemExit(
            f"{name} locks other versions than {src / 'Cargo.lock'}: {other}"
        )
    added = sorted(key for key in program if key not in library)
    binary = target / "release" / name
    record = {
        "program": name,
        "source": str(source),
        "source_sha256": sha256(source),
        "manifest": text,
        "command": command,
        "features": features,
        "locked_packages": len(program),
        "locked_as_library": True,
        "packages_beyond_library": added,
        "library_digest": provenance.library_digest(src),
        "readcon_version": tomllib.loads((src / "Cargo.toml").read_text())["package"][
            "version"
        ],
        "rustc": subprocess.run(
            ["rustc", "--version"], capture_output=True, text=True, check=True
        ).stdout.strip(),
        "sha256": sha256(binary),
    }
    return {"path": binary, "record": portable.portable(record)}
