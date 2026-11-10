# Copyright (c) readcon developers
"""Store a path relative to the repository, then the home directory."""

from pathlib import Path


def recorded_path(path: Path) -> str:
    resolved = path.expanduser().resolve()
    roots = [
        Path(__file__).resolve().parents[2],
        Path.home().resolve(),
    ]
    for root in roots:
        try:
            return resolved.relative_to(root).as_posix()
        except ValueError:
            continue
    return resolved.name
