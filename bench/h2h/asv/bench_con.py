# Copyright (c) readcon-core developers
"""ASV entry for the CON and convel head-to-head.

Snakemake runs the full serial session through drive.py. These methods
time one readcon call on the survey frame so the class is a real benchmark.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import support


class ConRead:
    timeout = 600
    number = 1
    repeat = 6
    rounds = 1
    warmup_time = 0

    def setup(self):
        import readcon

        self.readcon = readcon
        self.path = support.WORK / "con" / "inputs" / "survey.con"

    def time_read_survey(self):
        self.readcon.read_con(str(self.path))

    def track_n_atoms(self):
        frames = self.readcon.read_con(str(self.path))
        return len(frames[0].atoms)
