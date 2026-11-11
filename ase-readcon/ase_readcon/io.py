"""ASE formats ``readcon-con`` and ``readcon-convel``.

ASE looks up :data:`readcon_con_format` and :data:`readcon_convel_format`.
The format names become ``read_readcon_con`` and ``write_readcon_con``
(hyphens replaced by underscores).

A read walks the file with ``readcon.iter_con(stream=True)`` and parses
only the frames the slice asks for. A negative index uses
``count_frames`` and ``ConTrajectory``, which parse one frame at a time.
"""

from __future__ import annotations

import os
from collections.abc import Iterator

import numpy as np
import readcon
from ase.atoms import Atoms
from ase.utils.plugins import ExternalIOFormat

readcon_con_format = ExternalIOFormat(
    desc="CON trajectory via readcon",
    code="+S",
    module="ase_readcon.io",
)

readcon_convel_format = ExternalIOFormat(
    desc="convel trajectory via readcon",
    code="+S",
    module="ase_readcon.io",
)

_RESERVED = {
    "con_spec_version",
    "con_spec_minor",
    "sections",
    "pbc",
    "lattice_vectors",
    "units",
    "section_arity",
}


def read_readcon_con(filename, index=-1, **kwargs):
    """Yield Atoms for a CON file. ``index`` is an int, slice, or ASE string."""
    yield from _read(filename, index)


def read_readcon_convel(filename, index=-1, **kwargs):
    """Yield Atoms for a convel file, including velocities."""
    yield from _read(filename, index)


def write_readcon_con(filename, images, append=False, **kwargs):
    """Write Atoms as CON geometry. Velocities are left out of the file."""
    _write(filename, images, velocities=False, append=append)


def write_readcon_convel(filename, images, append=False, **kwargs):
    """Write Atoms as convel, including velocities."""
    _write(filename, images, velocities=True, append=append)


def _read(filename, index) -> Iterator[Atoms]:
    path = os.fspath(filename)
    selection = _as_slice(index)
    step = 1 if selection.step is None else selection.step
    if step == 0:
        raise ValueError("slice step cannot be zero")
    needs_length = step < 0 or _negative(selection.start) or _negative(selection.stop)
    if needs_length:
        start, stop, step = selection.indices(readcon.count_frames(path))
        traj = readcon.ConTrajectory(path, write_index=False)
        for frame_index in range(start, stop, step):
            yield _frame_to_atoms(traj[frame_index])
        return
    start = 0 if selection.start is None else selection.start
    stop = selection.stop
    frames = readcon.iter_con(path, stream=True)
    if start:
        frames.skip(start)
    cursor = start
    while stop is None or cursor < stop:
        frame = next(frames, None)
        if frame is None:
            break
        yield _frame_to_atoms(frame)
        gap = step - 1
        if gap:
            skipped = frames.skip(gap)
            cursor += 1 + skipped
            if skipped < gap:
                break
        else:
            cursor += 1


def _write(filename, images, velocities: bool, append: bool) -> None:
    path = os.fspath(filename)
    if isinstance(images, Atoms):
        images = [images]
    frames = []
    if append and os.path.isfile(path) and os.path.getsize(path):
        frames.extend(readcon.iter_con(path, stream=True))
    frames.extend(_atoms_to_frame(atoms, velocities) for atoms in images)
    readcon.write_con(path, frames)


def _frame_to_atoms(frame) -> Atoms:
    atoms = frame.to_ase()
    info = atoms.info
    for key, value in dict(frame.metadata).items():
        if key not in info:
            info[key] = value
    return atoms


def _atoms_to_frame(atoms: Atoms, velocities: bool):
    frame = readcon.ConFrame.from_ase(atoms)
    if not velocities:
        for atom in frame.atoms:
            atom.vx = None
            atom.vy = None
            atom.vz = None
    meta = dict(frame.metadata)
    for key, value in atoms.info.items():
        if key in _RESERVED or key in meta:
            continue
        plain = _plain(value)
        if plain is not _SKIP:
            meta[key] = plain
    frame.metadata = meta
    return frame


class _Skip:
    pass


_SKIP = _Skip()


def _plain(value):
    if isinstance(value, np.generic):
        value = value.item()
    if value is None or isinstance(value, (str, bool, int, float)):
        if isinstance(value, float) and not np.isfinite(value):
            return _SKIP
        return value
    if isinstance(value, np.ndarray):
        return _plain(value.tolist())
    if isinstance(value, (list, tuple)):
        items = []
        for item in value:
            plain = _plain(item)
            if plain is _SKIP:
                return _SKIP
            items.append(plain)
        return items
    if isinstance(value, dict):
        out = {}
        for key, item in value.items():
            plain = _plain(item)
            if plain is not _SKIP:
                out[str(key)] = plain
        return out
    return _SKIP


def _negative(value) -> bool:
    return value is not None and value < 0


def _as_slice(index) -> slice:
    if index is None or index == ":":
        return slice(None)
    if isinstance(index, str):
        from ase.utils import string2index

        return _as_slice(string2index(index))
    if isinstance(index, int):
        if index < 0:
            return slice(index, None)
        return slice(index, index + 1)
    if isinstance(index, slice):
        return index
    raise TypeError(f"unsupported index {index!r}")
