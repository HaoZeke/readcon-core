"""A read with no NumPy module keeps the parsed symbols."""

import os
import shutil
import subprocess
import sys
import tempfile

import readcon


def test_read_without_numpy_keeps_the_symbols():
    package = os.path.dirname(readcon.__file__)
    if os.path.basename(package) != "readcon":
        package = os.path.dirname(package)
    source = os.path.join(
        os.path.dirname(__file__), "..", "..", "resources", "test", "tiny_cuh2.con"
    )
    source = os.path.abspath(source)
    with tempfile.TemporaryDirectory() as tmp:
        shutil.copytree(package, os.path.join(tmp, "readcon"))
        code = (
            "import readcon\n"
            f"frames = readcon.read_con({source!r})\n"
            "assert len(frames) == 1\n"
            "assert len(frames[0]) == 4\n"
            "assert frames[0].atoms[0].symbol == 'Cu'\n"
            "assert frames[0].atoms[2].symbol == 'H'\n"
            "print('symbols-ok')\n"
        )
        env = os.environ.copy()
        env["PYTHONPATH"] = tmp
        env["PYTHONSAFEPATH"] = "1"
        env.pop("PYTHONHOME", None)
        proc = subprocess.run(
            [sys.executable, "-S", "-c", code],
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
    assert proc.returncode == 0, proc.stderr
    assert "symbols-ok" in proc.stdout
