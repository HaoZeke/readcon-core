# Copyright (c) readcon developers
"""Recorded paths stay relative to the home directory when they can."""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from recorded_path import recorded_path


def test_recorded_path_is_relative_to_home():
    target = Path.home() / "h2h-work" / "data" / "sample.xyz"
    assert recorded_path(target) == "h2h-work/data/sample.xyz"


def test_recorded_path_uses_a_repo_scratch_directory():
    repo = Path(__file__).resolve().parents[2]
    target = repo / "target" / "h2h-scratch" / "sample.xyz"
    recorded = recorded_path(target)
    assert "tmp" not in recorded.split("/")
    assert recorded.endswith("target/h2h-scratch/sample.xyz")
