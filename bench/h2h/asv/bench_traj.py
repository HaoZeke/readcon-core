# Copyright (c) readcon-core developers
"""One ASV class per XYZ comparison.

Snakemake runs the interleaved session through drive.py. Each class times
one readcon Python read of that comparison's file so the class is a real
benchmark. timeout is the session bound for the large files.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import traj_h2h


class _Case:
    timeout = 3600
    number = 1
    repeat = 6
    rounds = 1
    warmup_time = 0
    case = ""

    def setup(self):
        traj_h2h.ensure_inputs()
        atoms, _frames, _stem = traj_h2h.case_map()[self.case]
        self.path = traj_h2h.input_path(self.case)
        self.atoms = atoms

    def time_read_readcon_py(self):
        traj_h2h.read_readcon(self.path)

    def track_n_atoms(self):
        got = traj_h2h.read_readcon(self.path)
        return float(got["n_atoms"])


class GridMillion(_Case):
    """10^6 copper atoms, one extended XYZ frame."""

    case = "grid-1e6"


class GridFrames(_Case):
    """100 copper atoms, 10000 extended XYZ frames."""

    case = "grid-10000"


class Traj32(_Case):
    """32 copper atoms, 100000 frames, written as XYZ."""

    case = "traj-32"
