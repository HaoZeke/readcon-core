# Copyright (c) readcon developers
"""Store a path relative to the home directory."""

from pathlib import Path


def recorded_path(path: Path) -> str:
    resolved = path.expanduser().resolve()
    home = Path.home().resolve()
    try:
        return resolved.relative_to(home).as_posix()
    except ValueError:
        return resolved.as_posix()
