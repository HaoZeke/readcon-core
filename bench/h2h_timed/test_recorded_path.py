# Copyright (c) readcon developers
"""The next timed run stores paths relative to the home directory."""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from recorded_path import recorded_path


def test_recorded_path_is_relative_to_home():
    target = Path.home() / "h2h-work" / "data" / "sample.xyz"
    assert recorded_path(target) == "h2h-work/data/sample.xyz"


def test_recorded_path_keeps_a_path_outside_home():
    outside = Path("/tmp/outside-home-record")
    assert recorded_path(outside) == outside.resolve().as_posix()
