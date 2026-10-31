#!/usr/bin/env python3
"""Draw resources/test/cuh2.con side on, fixed atoms apart from free ones.

Writes tutorial_slab.png next to this script. Needs readcon and matplotlib.

Usage, from the repository root:
    python docs/orgmode/img/tutorial_slab.py
"""

from __future__ import annotations

from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

import readcon

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SRC = ROOT / "resources" / "test" / "cuh2.con"
OUT = HERE / "tutorial_slab.png"


def main() -> int:
    frame = readcon.read_first_frame(str(SRC))
    groups: dict[tuple[str, bool], list] = {}
    for atom in frame.atoms:
        groups.setdefault((atom.symbol, all(atom.fixed)), []).append(atom)

    style = {
        ("Cu", True): ("#94A3B8", 60, "Cu, fixed in x, y and z"),
        ("H", False): ("#4F46E5", 90, "H, free"),
    }
    fig, (side, top) = plt.subplots(1, 2, figsize=(7.2, 4.0), dpi=200)
    for key, atoms in sorted(groups.items()):
        color, size, label = style.get(key, ("#F59E0B", 60, f"{key[0]}, other"))
        for ax, coord in ((side, "z"), (top, "y")):
            ax.scatter(
                [a.x for a in atoms],
                [getattr(a, coord) for a in atoms],
                s=size,
                c=color,
                edgecolors="#1E1B4B",
                linewidths=0.4,
                label=f"{label} ({len(atoms)})",
                zorder=2 if key[1] else 3,
            )
        layers = len({round(a.z, 2) for a in atoms})
        print(f"{key[0]}\tfixed={key[1]}\tatoms={len(atoms)}\tz_layers={layers}")
    side.set_ylim(top=side.get_ylim()[1] + 1.5)
    side.set_title("side view", fontsize=10)
    side.set_ylabel("z / Å")
    top.set_title("top view", fontsize=10)
    top.set_ylabel("y / Å")
    for ax in (side, top):
        ax.set_xlabel("x / Å")
        ax.set_aspect("equal")
    handles, labels = side.get_legend_handles_labels()
    fig.legend(handles, labels, loc="lower center", ncols=2, fontsize=8, frameon=False)
    fig.suptitle(f"{SRC.name}: {len(frame)} atoms", fontsize=10)
    fig.tight_layout(rect=(0, 0.08, 1, 1))
    fig.savefig(OUT, metadata={"Software": None})
    print(f"wrote {OUT.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
