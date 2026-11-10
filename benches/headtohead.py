#!/usr/bin/env python3
"""Peer files for one CON trajectory: h5py, ASE traj, extended XYZ and plain XYZ.

``h2h.py`` times these files beside the CON. ``ensure_peers`` reads the CON
with readcon, writes one ASE image per frame, and stores positions at
``position/value`` in ``traj.h5``.
"""

from __future__ import annotations

from pathlib import Path

import h5py
import numpy as np
from ase.io import write as ase_write

import readcon


def ensure_peers(con: Path, dest: Path, frames: int) -> dict[str, Path]:
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    loaded = readcon.read_con(str(con), 1)
    if len(loaded) != frames:
        raise SystemExit(f"{con} has {len(loaded)} frames, not {frames}")
    images = [frame.to_ase() for frame in loaded]
    xyz = dest / "traj.xyz"
    extxyz = dest / "traj.extxyz"
    traj = dest / "traj.traj"
    h5_path = dest / "traj.h5"
    ase_write(str(xyz), images, format="xyz")
    ase_write(str(extxyz), images, format="extxyz")
    ase_write(str(traj), images, format="traj")
    positions = np.stack([image.get_positions() for image in images])
    with h5py.File(h5_path, "w") as handle:
        handle.create_dataset("position/value", data=positions)
    return {"h5": h5_path, "traj": traj, "extxyz": extxyz, "xyz": xyz}
