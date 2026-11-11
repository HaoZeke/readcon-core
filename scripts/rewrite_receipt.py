#!/usr/bin/env python3
"""Rewrite host paths in a coverage receipt and record the rewrite."""

import sys
from pathlib import Path

PREFIXES = (
    ("/usr/local/cargo/registry", "cargo-registry"),
    ("/usr/local/cargo", "cargo"),
    ("/usr/local/rustup", "rustup"),
)


def rewrite(text: str, root: str) -> tuple[str, list[str]]:
    used = []
    if root and root in text:
        text = text.replace(root + "/", "./")
        text = text.replace(root, ".")
        used.append(f"{root}/ -> ./")
    for old, new in PREFIXES:
        if old in text:
            text = text.replace(old, new)
            used.append(f"{old} -> {new}")
    if "/home/ubuntu" in text:
        text = text.replace("/home/ubuntu", "home")
        used.append("/home/ubuntu -> home")
    if "/src/" in text:
        text = text.replace("/src/", "src/")
        used.append("/src/ -> src/")
    return text, used


def main() -> None:
    path = Path(sys.argv[1])
    root = sys.argv[2] if len(sys.argv) > 2 else ""
    text = path.read_text(encoding="utf-8", errors="replace")
    text, used = rewrite(text, root)
    note = "rewrote " + ("; ".join(used) if used else "no host path")
    marker = "note " + note
    if marker not in text:
        if "\nexit " in text:
            head, tail = text.rsplit("\nexit ", 1)
            text = head + "\n" + marker + "\nexit " + tail
        else:
            if not text.endswith("\n"):
                text += "\n"
            text += marker + "\n"
    path.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
