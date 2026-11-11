"""ASV arms for extended XYZ. timeout is 600 s. time_* is one call.

Snakemake runs bench/h2h/extxyz_h2h.py for the six serial repeats.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import extxyz_h2h


class _Extxyz:
    timeout = 600
    number = 1
    repeat = 6
    rounds = 1
    warmup_time = 0
    arm = ""

    def setup(self):
        extxyz_h2h.ensure_inputs()
        self.small = extxyz_h2h.input_path("small-1")
        self.large = extxyz_h2h.input_path("large-1")

    def time_read_small_1(self):
        extxyz_h2h.read_arm(self.arm, self.small)

    def time_read_large_1(self):
        extxyz_h2h.read_arm(self.arm, self.large)

    def track_n_atoms_small_1(self):
        got = extxyz_h2h.read_arm(self.arm, self.small)
        return float(got["n_atoms"])


class ReadconRust(_Extxyz):
    arm = "readcon_rust"


class ReadconPython(_Extxyz):
    arm = "readcon_py"


class AseExtxyzFair(_Extxyz):
    arm = "ase_extxyz"


class AseCextxyz(_Extxyz):
    arm = "ase_cextxyz"


class ExtxyzLibatoms(_Extxyz):
    arm = "extxyz"


class ExtxyzNg(_Extxyz):
    arm = "extxyz_ng"
