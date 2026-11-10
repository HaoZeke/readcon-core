"""The version check still rejects a crate version that escaped the allowlist."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def test_a_stray_crate_version_is_rejected():
    version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    stray = ROOT / "paper" / "cpc" / "DRIFT_VERSION.txt"
    stray.write_text(version + "\n", encoding="utf-8")
    subprocess.run(["git", "add", "-f", str(stray)], cwd=ROOT, check=True)
    try:
        done = subprocess.run(
            ["bash", "scripts/check_version_lockstep.sh"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        assert done.returncode != 0
        assert "DRIFT_VERSION.txt" in (done.stdout + done.stderr)
    finally:
        subprocess.run(["git", "reset", "-q", "HEAD", "--", str(stray)], cwd=ROOT, check=True)
        stray.unlink(missing_ok=True)


def test_the_public_tag_receipt_need_not_repeat_the_crate_version():
    release = (ROOT / "paper/cpc/src/release.tex").read_text(encoding="utf-8")
    version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    assert "0.18.0" in release
    assert version not in release
    done = subprocess.run(
        ["bash", "scripts/check_version_lockstep.sh"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    assert done.returncode == 0, done.stderr
