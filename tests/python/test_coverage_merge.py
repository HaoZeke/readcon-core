"""The merged report keeps a count any binary recorded.

The old export lists an integration test before the library test binary.
Both define ``rkr_mpi_write_block_f``. The integration test's copy does
not match the profile, so llvm-cov reports line 382 of ``distrib_write.rs``
as 0 even though the library tests ran it 36 times.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "paper" / "cpc" / "scripts" / "fixtures" / "cov"
_spec = importlib.util.spec_from_file_location(
    "merge_coverage", ROOT / "paper" / "cpc" / "scripts" / "merge_coverage.py"
)
_merge = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_merge)
line_count = _merge.line_count
merge_documents = _merge.merge_documents
FILE = "src/distrib_write.rs"
LINE = 382


def _load(name: str) -> dict:
    return json.loads((FIXTURES / name).read_text(encoding="utf-8"))


def test_an_executed_c_abi_line_survives_the_merge():
    old_order = _load("distrib_write_old_order.json")
    test_binary = _load("distrib_write_test_binary.json")
    assert line_count(old_order, FILE, LINE) == 0
    assert line_count(test_binary, FILE, LINE) > 0
    merged = merge_documents([old_order, test_binary])
    assert line_count(merged, FILE, LINE) > 0
