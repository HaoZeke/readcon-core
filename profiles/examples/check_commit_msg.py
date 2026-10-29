#!/usr/bin/env python3
"""Apply profiles/examples/commit-msg.toml ReviewRegister bans, then ProseGuard."""
from __future__ import annotations
import re, subprocess, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOML = Path(__file__).with_name("commit-msg.toml")
JSON = Path(__file__).with_name("commit-msg.proseguard.json")

def main(paths: list[str]) -> int:
    cfg = tomllib.loads(TOML.read_text())
    ban = [b.lower() for b in cfg["rgoswami.ReviewRegister"]["ban"]]
    rc = 0
    for path in paths:
        text = Path(path).read_text()
        findings = []
        for i, line in enumerate(text.splitlines(), 1):
            for word in re.findall(r"[A-Za-z0-9_@/-]+", line):
                if word.lower() in ban:
                    findings.append(f"{path}:{i}: rgoswami.ReviewRegister: banned {word!r}")
        if findings:
            print("\n".join(findings))
            rc = 1
        else:
            print(f"{path}: ReviewRegister clean")
        proc = subprocess.run(
            ["proseguard", "-c", str(JSON), path, "-f", "text"],
            text=True, capture_output=True,
        )
        sys.stdout.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        if proc.returncode not in (0, 1):
            rc = proc.returncode
        elif proc.returncode == 1:
            rc = 1
    return rc

if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:] or ["-"]))
