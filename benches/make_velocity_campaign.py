#!/usr/bin/env python3
"""Write the velocity trajectory ``h2h.py`` times, and XYZ and GRO of the same frames.

The fixture is one CuH2 frame. Each atom gets a velocity from its index, the
frame is written once, and that text is repeated for 10,000 frames. XYZ and GRO
are the same frames through chemfiles. A second run of the same fixture writes
the same bytes.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import chemfiles
import numpy as np

import readcon

FRAMES = 10000


def velocities(count: int) -> np.ndarray:
    index = np.arange(count, dtype=np.float64)
    return np.column_stack(
        (
            0.001 * ((index % 17) + 1),
            -0.001 * ((index % 13) + 1),
            0.001 * ((index % 11) - 5),
        )
    )


def chemfiles_frame(symbols: list[str], positions: np.ndarray, velocity: np.ndarray, cell) -> chemfiles.Frame:
    frame = chemfiles.Frame()
    frame.resize(len(symbols))
    frame.positions[:] = positions
    frame.add_velocities()
    frame.velocities[:] = velocity
    for atom, symbol in zip(frame.topology.atoms, symbols):
        atom.name = symbol
    lengths, angles = cell
    frame.cell = chemfiles.UnitCell(
        (float(lengths[0]), float(lengths[1]), float(lengths[2])),
        (float(angles[0]), float(angles[1]), float(angles[2])),
    )
    return frame


def write_repeats(path: Path, frame: chemfiles.Frame, fmt: str) -> None:
    with chemfiles.Trajectory(str(path), "w", fmt) as traj:
        for _ in range(FRAMES):
            traj.write(frame)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--xyz", type=Path, required=True)
    parser.add_argument("--gro", type=Path, required=True)
    args = parser.parse_args()
    source = readcon.read_con(str(args.fixture), 1)
    if len(source) != 1:
        raise SystemExit(f"{args.fixture} has {len(source)} frames, not 1")
    frame = source[0]
    velocity = velocities(len(frame.atoms))
    for atom, row in zip(frame.atoms, velocity):
        atom.vx, atom.vy, atom.vz = (float(row[0]), float(row[1]), float(row[2]))
    text = readcon.write_con_string([frame])
    if not text.endswith("\n"):
        text += "\n"
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(text.encode() * FRAMES)
    positions = np.array([[atom.x, atom.y, atom.z] for atom in frame.atoms], dtype=np.float64)
    symbols = [atom.symbol for atom in frame.atoms]
    written = chemfiles_frame(symbols, positions, velocity, (frame.cell, frame.angles))
    write_repeats(args.xyz, written, "XYZ")
    write_repeats(args.gro, written, "GRO")


if __name__ == "__main__":
    main()
