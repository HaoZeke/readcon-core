"""The type table and the symbol map match a per-atom loop."""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent / "harness"))

from sample import map_symbols, type_lut


def _loop(ids: np.ndarray, names: dict[int, int]) -> np.ndarray:
    out = np.empty(np.asarray(ids).shape, dtype=np.int64)
    flat_ids = np.asarray(ids, dtype=np.int64).ravel()
    flat_out = out.ravel()
    for index, type_id in enumerate(flat_ids):
        flat_out[index] = names[int(type_id)]
    return out


def test_type_lut_matches_the_loop() -> None:
    names = {1: 29, 2: 1, 7: 8}
    ids = np.array([[1, 2, 1, 7], [2, 7, 1, 1]], dtype=np.int64)
    lut = type_lut(names)
    assert np.array_equal(lut[ids], _loop(ids, names))


def test_map_symbols_calls_once_per_distinct_symbol() -> None:
    calls: list[str] = []

    def atomic_number(symbol: str) -> int:
        calls.append(symbol)
        return {"Cu": 29, "H": 1}[symbol]

    got = map_symbols(["Cu", "H", "Cu", "Cu"], atomic_number)
    assert calls == ["Cu", "H"]
    assert np.array_equal(got, np.array([29, 1, 29, 29], dtype=np.int64))
