#!/usr/bin/env python3
"""Paper receipts that need a conda prefix, the eOn checkout, or ira_mod.

Unit tests do not own these runs. This script vendors a one-package conda
prefix, fetches the pinned eOn commit when the peer checkout is absent, and
records each result in paper/cpc/logs/paper-repro.log.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CPC = ROOT / "paper" / "cpc"
SCRIPTS = CPC / "scripts"
LOG = CPC / "logs" / "paper-repro.log"
EON = CPC / "bench" / "work" / "peers" / "eon"
EON_COMMIT = "579738a9387e8d0ea55496f45dd057812cfab892"
EON_URL = "https://github.com/TheochemUI/eOn.git"
PREFIX = CPC / "repro" / "conda-prefix"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def section(title: str, body: str) -> str:
    return f"## {title}\n{body.rstrip()}\n"


def export_env_receipt() -> str:
    env = load("export_env.py")
    out = CPC / "repro" / "environment"
    out.mkdir(parents=True, exist_ok=True)
    env.CONDA = PREFIX
    env.ENV = out
    code = env.main()
    text = (out / "conda-linux-64.txt").read_text(encoding="utf-8")
    if code not in (0, None):
        raise SystemExit(f"export_env returned {code}")
    if "@EXPLICIT" not in text:
        raise SystemExit("conda-linux-64.txt has no explicit list")
    if "example.invalid/zlib" not in text:
        raise SystemExit("vendored zlib record was dropped")
    return f"exit {code}\n{text}"


def fetch_eon() -> str:
    if EON.exists() and not (EON / ".git").is_dir():
        aside = EON.with_name("eon.not-a-checkout")
        if aside.exists():
            aside = EON.with_name(f"eon.not-a-checkout-{os.getpid()}")
        EON.rename(aside)
    EON.parent.mkdir(parents=True, exist_ok=True)
    if not (EON / ".git").is_dir():
        proc = subprocess.run(
            ["git", "clone", "--filter=blob:none", EON_URL, str(EON)],
            text=True,
            capture_output=True,
            timeout=180,
        )
        if proc.returncode != 0:
            raise SystemExit(f"eon clone failed\n{proc.stderr}")
    proc = subprocess.run(
        ["git", "checkout", EON_COMMIT],
        cwd=EON,
        text=True,
        capture_output=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"eon checkout failed\n{proc.stderr}")
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=EON, text=True
    ).strip()
    if head != EON_COMMIT:
        raise SystemExit(f"eon HEAD {head} is not {EON_COMMIT}")
    return f"HEAD {head}"


def eon_receipt() -> str:
    fetched = fetch_eon()
    eon = load("eon_build_log.py")
    code = eon.main()
    log = eon.LOG.read_text(encoding="utf-8") if eon.LOG.is_file() else ""
    if code not in (0, None):
        raise SystemExit(f"eon_build_log returned {code}\n{log}")
    return f"{fetched}\nexit {code}\n{log}"


def ira_receipt() -> str:
    import ira_mod  # noqa: F401

    plotted = load("plot_figures.py")
    plotted.main()
    figure = CPC / "src" / "figures" / "generated" / "restart.pdf"
    if not figure.is_file() or figure.stat().st_size == 0:
        raise SystemExit("plot_figures.main left no restart.pdf")
    return f"plot_figures.main finished\nrestart.pdf {figure.stat().st_size} bytes"


def tip_digest() -> str:
    prov = load("provenance.py")
    digest = prov.library_digest()
    return f"library_digest {digest}"


def main() -> int:
    parts = [
        section("tip", tip_digest()),
        section("export_env", export_env_receipt()),
        section("eon", eon_receipt()),
        section("ira", ira_receipt()),
    ]
    LOG.parent.mkdir(parents=True, exist_ok=True)
    LOG.write_text("\n".join(parts) + "\n", encoding="utf-8")
    print(LOG)
    return 0


if __name__ == "__main__":
    sys.exit(main())
