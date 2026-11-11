# Copyright (c) readcon-core developers
"""One asv class per readcon-db comparison.

Snakemake runs the interleaved session through drive.py. Each class times
one readcon-db call of that comparison.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import db_h2h


class _Db:
    timeout = 600
    number = 1
    repeat = 6
    rounds = 1
    warmup_time = 0
    op = "append"

    def setup(self):
        self.frames, self.images = db_h2h.load_inputs()
        self.dest = db_h2h.store_path("readcon_db", "asv")
        if self.op == "random":
            ready = db_h2h.store_path("readcon_db", "random")
            if not db_h2h._store_ready("readcon_db", ready):
                db_h2h.append_store("readcon_db", ready, self.frames, self.images)
            self.dest = ready
            self.picked = db_h2h.indices()
            check = db_h2h.check_frames("readcon_db", ready, [self.picked[0]])
            if not check["correct"]:
                raise RuntimeError(check["reason"])

    def time_readcon_db(self):
        if self.op == "append":
            db_h2h.append_store("readcon_db", self.dest, self.frames, self.images)
        else:
            db_h2h.read_random("readcon_db", self.dest, self.picked)

    def track_frames(self):
        return float(len(self.frames))


class DbAppend(_Db):
    op = "append"


class DbRandom(_Db):
    op = "random"
