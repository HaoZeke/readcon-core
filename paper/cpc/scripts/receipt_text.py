"""Turn a captured test log into a receipt that names no host path.

The receipt scripts call this before they write. The log is not edited
after that write.
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def clean(text: str, root: Path | None = None) -> str:
    """Return ``text`` with this checkout and the home directory made relative."""
    root = ROOT if root is None else root
    home = str(Path.home())
    pairs = [(str(root), "."), (home, "home")]
    pairs.sort(key=lambda item: -len(item[0]))
    for src, dst in pairs:
        if src:
            text = text.replace(src, dst)
    return text


def rustup_bin() -> Path:
    return (
        Path.home()
        / ".rustup"
        / "toolchains"
        / "1.88.0-x86_64-unknown-linux-gnu"
        / "bin"
    )


def tool_path() -> str:
    import os

    return os.pathsep.join([str(rustup_bin()), "/usr/bin", "/bin"])


def versions() -> str:
    """Raw compiler lines from the pinned toolchain."""
    import os
    import subprocess

    env = os.environ.copy()
    env["PATH"] = tool_path()
    rustc = subprocess.check_output(["rustc", "-V"], env=env, text=True)
    cargo = subprocess.check_output(["cargo", "-V"], env=env, text=True)
    return f"rustc -V\n{rustc}cargo -V\n{cargo}"
