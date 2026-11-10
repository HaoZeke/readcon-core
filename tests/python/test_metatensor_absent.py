"""The extension built without metatensor returns a typed status."""

import ctypes
from pathlib import Path

import readcon

FEATURE_DISABLED = -11
NULL_POINTER = -1
NAMES = (
    "rkr_frame_metatensor_positions_block",
    "rkr_frame_metatensor_velocities_block",
    "rkr_frame_metatensor_forces_block",
    "rkr_frame_metatensor_atom_energies_block",
)


def _library():
    folder = Path(readcon.__file__).resolve().parent
    objects = sorted(folder.glob("*.so"))
    assert objects, folder
    lib = ctypes.CDLL(str(objects[0]))
    return lib


def test_a_missing_metatensor_block_is_a_typed_status():
    lib = _library()
    for name in NAMES:
        fn = getattr(lib, name)
        fn.restype = ctypes.c_int
        fn.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p)]
        slot = ctypes.c_void_p(1)
        status = fn(None, ctypes.byref(slot))
        assert status in (FEATURE_DISABLED, NULL_POINTER), (name, status)
        if status == FEATURE_DISABLED:
            assert not slot.value, (name, slot.value)
            absent = fn(None, None)
            assert absent == FEATURE_DISABLED, (name, absent)
