"""The first-frame reader keeps float32 coordinates."""

from pathlib import Path

import pytest

readcon = pytest.importorskip("readcon")


def test_read_first_frame_float32_positions(tmp_path: Path):
    source = Path(__file__).resolve().parents[2] / "resources" / "test" / "tiny_cuh2.con"
    path = tmp_path / "f32-positions.con"
    text = source.read_text(encoding="utf-8").replace(
        '{"con_spec_version":2}',
        '{"con_spec_version":2,"storage_dtypes":{"positions":"float32"}}',
        1,
    )
    path.write_text(text, encoding="utf-8")
    frame = readcon.read_first_frame(str(path))
    assert abs(float(frame.xyz[0, 0]) - 0.6394) < 1e-4
    assert abs(float(frame.xyz[0, 1]) - 0.9045) < 1e-4
    assert abs(float(frame.xyz[0, 2]) - 6.9753) < 1e-4
