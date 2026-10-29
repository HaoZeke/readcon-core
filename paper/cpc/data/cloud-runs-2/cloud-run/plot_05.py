#!/usr/bin/env python3
"""Draw 05/figures/restart_formats.pdf from the committed 05 bands.

The panels come from scripts/plot_restart_formats.py with the Ruhi theme of
scripts/plot_figures.py. The colour cycle and the default colour map are set
to the Ruhi palette as well, so no artist falls back to matplotlib's defaults.

The theme face is the vendored Atkinson Hyperlegible in paper/cpc/fonts
(googlefonts/atkinson-hyperlegible 1cb3116, OFL 1.1). It replaces any
installed copy, so the figure does not depend on system fonts.

    pixi run -e figures cpc-figure-05
"""

from __future__ import annotations

import sys
from pathlib import Path

import matplotlib
import matplotlib.font_manager as fm
import numpy as np
from cycler import cycler

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
D05 = HERE.parent / "05"
FONTS = ROOT / "fonts" / "atkinson-hyperlegible"
FAMILY = "Atkinson Hyperlegible"
sys.path.insert(0, str(ROOT / "scripts"))

import plot_figures as pf  # noqa: E402
import plot_restart_formats as prf  # noqa: E402


def use_vendored_font():
    ttfs = sorted(FONTS.glob("*.ttf"))
    if not ttfs:
        sys.exit(f"no vendored {FAMILY} TTF files in {FONTS}")
    fm.fontManager.ttflist = [f for f in fm.fontManager.ttflist if f.name != FAMILY]
    for ttf in ttfs:
        fm.fontManager.addfont(str(ttf))
    found = Path(fm.findfont(FAMILY, fallback_to_default=False))
    if found.parent != FONTS:
        sys.exit(f"{FAMILY} resolved to {found}, not the vendored copy")


def main():
    use_vendored_font()
    x = np.linspace(0.0, 1.0, 256)
    if not np.array_equal(
        matplotlib.colormaps[pf.SEQUENTIAL.name](x), pf.SEQUENTIAL(x)
    ):
        sys.exit(f"colour map {pf.SEQUENTIAL.name} is not the Ruhi sequential map")
    matplotlib.rcParams["axes.prop_cycle"] = cycler(
        color=[pf.TEAL, pf.SKY, pf.MAGENTA, pf.CORAL, pf.SUNSHINE]
    )
    matplotlib.rcParams["image.cmap"] = pf.SEQUENTIAL.name
    prf.BANDS = D05 / "bands"
    pf.OUT = D05 / "figures"
    pf.OUT.mkdir(exist_ok=True)
    prf.main()


if __name__ == "__main__":
    main()
