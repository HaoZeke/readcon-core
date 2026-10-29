#!/usr/bin/env python3
"""Render the paper's benchmark structures with xyzrender and the Ruhi palette.

Panels:
  (a) resources/test/tiny_cuh2.con — four-atom survey frame
  (b) resources/test/cuh2.con — 218-atom timing frame (via XYZ)
  (c) middle image of the EMT Pt(100) band from cloud-runs-2/05

xyzrender cannot load the large CON directly (element-table KeyError on
cuh2.con), so those two panels go through a temporary XYZ written next to
the script outputs under figures/generated/work/ (gitignored). Colours and
panel chrome use chemparseplot Ruhi + Atkinson Hyperlegible.
"""
from __future__ import annotations

import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.font_manager as fm
import matplotlib.pyplot as plt
import numpy as np
from ase import Atoms
from ase.io import read, write
from matplotlib.offsetbox import AnnotationBbox, OffsetImage
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
OUT = ROOT / "src" / "figures" / "generated"
WORK = OUT / "work"
FONTS = ROOT / "fonts" / "atkinson-hyperlegible"
FAMILY = "Atkinson Hyperlegible"

sys.path.insert(0, str(ROOT / "scripts"))
import plot_figures as pf  # noqa: E402

import xyzrender as xr  # noqa: E402


def use_vendored_font() -> None:
    ttfs = sorted(FONTS.glob("*.ttf"))
    if not ttfs:
        sys.exit(f"no vendored {FAMILY} TTF files in {FONTS}")
    fm.fontManager.ttflist = [f for f in fm.fontManager.ttflist if f.name != FAMILY]
    for ttf in ttfs:
        fm.fontManager.addfont(str(ttf))


def render_png(source: Path | Atoms, dest: Path, *, canvas: int = 480) -> Path:
    WORK.mkdir(parents=True, exist_ok=True)
    if isinstance(source, Atoms):
        xyz = WORK / (dest.stem + ".xyz")
        write(str(xyz), source)
        molecule = xr.load(str(xyz))
    else:
        try:
            molecule = xr.load(str(source))
        except Exception:
            atoms = read(str(source))
            xyz = WORK / (source.stem + ".xyz")
            write(str(xyz), atoms)
            molecule = xr.load(str(xyz))
    svg_path = WORK / (dest.stem + ".svg")
    xr.render(
        molecule,
        output=str(svg_path),
        background="white",
        canvas_size=canvas,
        transparent=False,
    )
    # Drop atom-label <text> nodes: xyzrender embeds Arial/DejaVu, which
    # would otherwise enter the manuscript PDF. Panel titles use Atkinson.
    import re

    svg = svg_path.read_text(encoding="utf-8")
    svg = re.sub(r"<text\b[^>]*>.*?</text>", "", svg, flags=re.DOTALL)
    svg_path.write_text(svg, encoding="utf-8")
    import cairosvg

    cairosvg.svg2png(url=str(svg_path), write_to=str(dest), output_width=canvas, output_height=canvas)
    return dest


def pt_slab_middle() -> Atoms:
    band = REPO / "paper/cpc/data/cloud-runs-2/05/bands/uninterrupted.npz"
    z = np.load(band)
    mid = z["pos"].shape[0] // 2
    return Atoms(numbers=z["numbers"], positions=z["pos"][mid], cell=z["cell"], pbc=[True, True, True])


def main() -> None:
    use_vendored_font()
    import matplotlib as mpl

    mpl.rcParams.update(
        {
            "font.family": FAMILY,
            "font.sans-serif": [FAMILY],
            "mathtext.fontset": "custom",
            "mathtext.rm": FAMILY,
            "mathtext.it": FAMILY,
            "mathtext.bf": FAMILY,
            "pdf.fonttype": 42,
            "ps.fonttype": 42,
        }
    )
    OUT.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)

    panels = [
        ("(a) survey CuH2", REPO / "resources/test/tiny_cuh2.con", "tiny_cuh2.png"),
        ("(b) timing CuH2", REPO / "resources/test/cuh2.con", "cuh2.png"),
        ("(c) EMT Pt(100) hop", pt_slab_middle(), "pt_slab.png"),
    ]
    pngs = []
    for label, src, name in panels:
        dest = WORK / name
        render_png(src, dest)
        pngs.append((label, dest))

    fig, axes = plt.subplots(1, 3, figsize=(7.2, 2.6))
    for ax, (label, png) in zip(axes, pngs):
        ax.set_facecolor("white")
        for spine in ax.spines.values():
            spine.set_color(pf.TEAL)
            spine.set_linewidth(1.2)
        ax.set_xticks([])
        ax.set_yticks([])
        img = np.asarray(Image.open(png).convert("RGBA"))
        ax.imshow(img)
        ax.set_title(label, fontsize=9, color=pf.TEAL, pad=4, fontfamily=FAMILY)
    fig.patch.set_facecolor("white")
    fig.tight_layout(w_pad=0.4)
    out_pdf = OUT / "structures.pdf"
    fig.savefig(out_pdf, bbox_inches="tight")
    fig.savefig(OUT / "structures.png", dpi=160, bbox_inches="tight")
    plt.close(fig)
    print("wrote", out_pdf)


if __name__ == "__main__":
    main()
