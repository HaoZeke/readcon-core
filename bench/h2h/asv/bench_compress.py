# Copyright (c) readcon-core developers
"""One asv class per compressed-CON comparison.

Snakemake runs the interleaved session through drive.py. Each class times
one readcon call of that comparison.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compress_h2h


class _Compress:
    timeout = 600
    number = 1
    repeat = 6
    rounds = 1
    warmup_time = 0
    case = "traj"
    op = "read"

    def setup(self):
        compress_h2h.ensure_inputs()
        self.codecs = compress_h2h.discover_codecs()
        self.path = compress_h2h.compressed_path(self.case, self.codecs[0])
        if self.op == "read":
            positions, cell = compress_h2h.read_readcon(self.path)
            check = compress_h2h.judge(positions, cell, self.case)
            if not check["correct"]:
                raise RuntimeError(check["reason"])

    def time_readcon(self):
        if self.op == "read":
            compress_h2h.load_readcon(self.path)
        else:
            frames = compress_h2h.load_frames(self.case)
            dest = compress_h2h.WORK / "outs" / f"asv-{self.case}.{self.path.suffixes[-1]}"
            if self.path.name.endswith(".con.gz"):
                dest = compress_h2h.WORK / "outs" / f"asv-{self.case}.con.gz"
            elif self.path.name.endswith(".con.zst"):
                dest = compress_h2h.WORK / "outs" / f"asv-{self.case}.con.zst"
            compress_h2h.write_readcon_py(frames, dest)

    def track_atoms(self):
        positions, _cell = compress_h2h.read_readcon(self.path)
        return float(positions.shape[1])


class CompressTrajRead(_Compress):
    case = "traj"
    op = "read"


class CompressTrajWrite(_Compress):
    case = "traj"
    op = "write"


class CompressCuRead(_Compress):
    case = "cu"
    op = "read"


class CompressCuWrite(_Compress):
    case = "cu"
    op = "write"
