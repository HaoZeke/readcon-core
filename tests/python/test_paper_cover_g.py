"""Outputs for the head-to-head, binary, probe, recount, wall, drift, survey and restart scripts."""

from __future__ import annotations

import copy
import importlib.util
import io
import json
import os
import runpy
import sys
import zstandard
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[2]
assert zstandard.ZSTD_VERSION == (1, 5, 7)
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverg_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def _raises(fn, match: str) -> None:
    with pytest.raises(SystemExit, match=match):
        fn()


def test_h2h_records_reject_a_broken_campaign(tmp_path, monkeypatch):
    h2h = load("h2h_tables.py")
    rec = h2h.records()
    h2h.check(rec)
    macros, py_rows, cpp_rows = h2h.macros_and_tables(rec)
    names = [name for name, _ in macros]
    assert "hhFrames" in names
    assert py_rows and cpp_rows
    assert len(names) == len(set(names))

    def edited(change):
        bad = copy.deepcopy(rec)
        change(bad)
        return bad

    _raises(
        lambda: h2h.check(
            edited(lambda r: r["payloads"]["velocity_campaign_rerun_identical"].__setitem__("con", False))
        ),
        "byte for byte",
    )
    _raises(
        lambda: h2h.check(edited(lambda r: r["python"]["coords"]["provenance"].__setitem__("peers", {}))),
        "other peers",
    )
    _raises(
        lambda: h2h.check(
            edited(
                lambda r: r["python"]["coords"]["provenance"]["build"].__setitem__(
                    "readcon_module_sha256", "0"
                )
            )
        ),
        "another readcon module",
    )
    _raises(
        lambda: h2h.check(
            edited(lambda r: r["python"]["vel"]["provenance"]["host"].__setitem__("kernel", "other-kernel"))
        ),
        "another host",
    )
    _raises(
        lambda: h2h.check(
            edited(lambda r: r["payloads"]["provenance"]["build"].__setitem__("library_digest", "0"))
        ),
        "library digests",
    )
    _raises(
        lambda: h2h.check(
            edited(
                lambda r: r["cpp"]["coords"]["provenance"]["library"].__setitem__(
                    "newer_sources", ["src/new.rs"]
                )
            )
        ),
        "older than",
    )

    frames = rec["payloads"]["frames"]

    def split_prints(r):
        cells = r["cpp"]["coords"]["cells"]
        cells[0]["readcon_coordinates_printed"] = {"frames": frames, "checksum": "1"}
        cells[1]["readcon_coordinates_printed"] = {"frames": frames, "checksum": "2"}

    _raises(lambda: h2h.check(edited(split_prints)), "print different")

    def wrong_frames(r):
        for cell in r["cpp"]["coords"]["cells"]:
            for key in list(cell):
                if key.endswith("_printed") and isinstance(cell[key], dict):
                    cell[key] = dict(cell[key], frames=1)

    _raises(lambda: h2h.check(edited(wrong_frames)), "readcon_coordinates_printed")

    def different_coordinates(r):
        for cell in r["cpp"]["coords"]["cells"]:
            cell["readcon_coordinates_printed"] = {"frames": frames, "checksum": "ours"}
            cell["chemfiles_xyz_printed"] = {"frames": frames, "checksum": "peer"}

    _raises(lambda: h2h.check(edited(different_coordinates)), "different coordinates")

    def both_campaigns_print_gro(r):
        vel = {(c["compiler"], c["cxx_flags"][0]): c for c in r["cpp"]["vel"]["cells"]}
        for cell in r["cpp"]["coords"]["cells"]:
            src = vel[(cell["compiler"], cell["cxx_flags"][0])]
            for key in (
                "chemfiles_gro",
                "readcon_with_velocities",
                "readcon_frames_with_velocities",
            ):
                cell[key] = src[key]

    _raises(lambda: h2h.macros_and_tables(edited(both_campaigns_print_gro)), "duplicate")

    out = tmp_path / "gen"
    monkeypatch.setattr(h2h, "OUT", out)
    monkeypatch.setattr(h2h, "LEDGER", tmp_path / "h2h_numbers.tsv")
    monkeypatch.setattr(sys, "argv", ["h2h_tables.py"])
    assert h2h.main() == 0
    text = (out / "h2h_macros.tex").read_text(encoding="utf-8")
    assert "hhFrames" in text
    assert (out / "h2h_python.tex").read_text(encoding="utf-8").count("readcon") > 0
    (out / "h2h_macros.tex").write_text("stale\n", encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["h2h_tables.py", "--check"])
    assert h2h.main() == 1


def test_binary_records_reject_a_broken_cell(tmp_path, monkeypatch):
    tables = load("binary_formats_tables.py")
    rec = tables.records()
    tables.check(rec)
    macros, rows, ratio_rows = tables.macros_and_rows(rec)
    assert any(name == "bfFrames" for name, _ in macros)
    assert rows and ratio_rows

    def edited(change):
        bad = copy.deepcopy(rec)
        change(bad)
        return bad

    _raises(
        lambda: tables.check(
            edited(lambda r: r["settle"]["provenance"]["build"].__setitem__("library_digest", "0"))
        ),
        "readcon_read builds",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["build"]["binaries"]["readcon_read"].__setitem__("sha256", "0"))
        ),
        "another readcon_read",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["build"]["readcon_read"].__setitem__("locked_as_library", False))
        ),
        "locks other versions",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["payloads"]["provenance"]["host"].__setitem__("kernel", "other-kernel"))
        ),
        "another host",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["payloads"]["rerun_identical"].__setitem__("traj.xtc", False))
        ),
        "byte for byte",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["payloads"]["writers"]["con_decimals"].__setitem__("traj_two.con", 9))
        ),
        "decimals",
    )
    _raises(
        lambda: tables.check(edited(lambda r: r["checks"]["cells"].pop("tng-all"))),
        "other cells",
    )
    _raises(
        lambda: tables.check(
            edited(lambda r: r["checks"]["cells"]["readcon-frames"]["printed"].__setitem__("frames", 1))
        ),
        "check run printed",
    )

    def mismatch(r):
        printed = dict(r["checks"]["cells"]["readcon-frames"]["printed"])
        printed["checksum"] = "no"
        r["timings"]["cells"]["readcon-frames"]["printed"] = printed

    _raises(lambda: tables.check(edited(mismatch)), "timed runs printed")
    _raises(
        lambda: tables.check(
            edited(lambda r: r["checks"]["cells"]["libxtc-frames"].__setitem__("passes", True))
        ),
        "passes",
    )
    _raises(
        lambda: tables.check(
            edited(
                lambda r: r["checks"]["libxtc_pep"]["read_libxtc"].__setitem__(
                    "max_nm_from_pep_npy", 1
                )
            )
        ),
        "libxtc_pep",
    )

    def quiet_pep(r):
        pep = r["checks"]["libxtc_pep"]
        pep["read_libxtc"]["max_nm_from_pep_npy"] = 0
        pep["read_xdrfile"]["max_nm_from_mdanalysis"] = 0
        pep["read_libxtc"]["atoms_off_mdanalysis"] = 0

    _raises(lambda: tables.check(edited(quiet_pep)), "MDAnalysis")
    _raises(
        lambda: tables.check(
            edited(lambda r: r["checks"]["tng_two_sets"]["frames"].__setitem__("returncode", 1))
        ),
        "tng_two_sets",
    )
    _raises(
        lambda: tables.macros_and_rows(
            edited(
                lambda r: r["checks"]["cells"]["libxtc-frames"].__setitem__(
                    "atom_positions_beyond_tolerance", 1
                )
            )
        ),
        "disagree",
    )
    _raises(
        lambda: tables.macros_and_rows(
            edited(lambda r: r["timings"]["cells"]["readcon-frames"].__setitem__("threads", 2))
        ),
        "one-thread",
    )

    out = tmp_path / "gen"
    monkeypatch.setattr(tables, "OUT", out)
    monkeypatch.setattr(tables, "LEDGER", tmp_path / "bf_numbers.tsv")
    monkeypatch.setattr(sys, "argv", ["binary_formats_tables.py"])
    assert tables.main() == 0
    assert "bfFrames" in (out / "bf_macros.tex").read_text(encoding="utf-8")
    assert "readcon-core" in (out / "bf_table.tex").read_text(encoding="utf-8")
    (out / "bf_table.tex").write_text("stale\n", encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["binary_formats_tables.py", "--check"])
    assert tables.main() == 1


def test_recount_names_an_unknown_tree_and_a_disagreement(tmp_path, monkeypatch):
    recount = load("spec_corpus_recount.py")
    monkeypatch.setattr(
        recount.provenance, "source_identity", lambda: {"source_commit": "abc123"}
    )
    assert recount.tree_commit(recount.REPO) == "abc123"
    assert recount.tree_commit(tmp_path) == "unknown"
    text = recount.table(
        {
            "n_invalid": 2,
            "records": [{"id": "not-in-a-group", "kind": "valid"}],
        }
    )
    assert "Other legal & 1" in text
    assert "Illegal & 2" in text

    def disagreed(tree):
        return {
            "readcon_version": "0.18.0",
            "tree_commit": "abc123",
            "n_total": 2,
            "n_valid": 1,
            "n_invalid": 1,
            "n_accepted": 0,
            "n_rejected": 2,
            "n_disagreements": 1,
            "records": [
                {
                    "id": "not-in-a-group",
                    "kind": "valid",
                    "agrees_with_manifest": False,
                    "verdict": "reject",
                    "error_class": "Other",
                    "line": 3,
                }
            ],
        }

    monkeypatch.setattr(recount, "run", disagreed)
    monkeypatch.setattr(
        sys, "argv", ["spec_corpus_recount.py", "--out", str(tmp_path / "out")]
    )
    assert recount.main() == 1
    written = (tmp_path / "out" / "corpus_table.tex").read_text(encoding="utf-8")
    assert "Other legal" in written
    macros = (tmp_path / "out" / "corpus_recount_macros.tex").read_text(encoding="utf-8")
    assert "corpusConfDisagree" in macros and "{1}" in macros


def test_walls_stop_when_a_route_is_not_clean(tmp_path, monkeypatch):
    walls = load("run_restart_walls.py")
    root = tmp_path / "paper"
    (root / "data" / "failure").mkdir(parents=True)
    (root / "data" / "failure" / "restart_formats.csv").write_text(
        "route,error\n", encoding="utf-8"
    )
    monkeypatch.setattr(walls, "ROOT", root)
    with pytest.raises(SystemExit, match="clean row"):
        walls.main()


def test_walls_entry_runs_a_child_and_stops_on_a_bad_table(tmp_path, monkeypatch, capsys):
    import restart_formats as rf

    monkeypatch.setattr(rf, "endpoints", lambda: [])
    monkeypatch.setattr(rf, "make_band", lambda eps: [])
    monkeypatch.setattr(
        rf,
        "run",
        lambda imgs, start=0, stop=None: {
            "step": 1,
            "converged": True,
            "calls": 2,
        },
    )
    monkeypatch.setattr(sys, "argv", ["run_restart_walls.py", "--child", "uninterrupted"])
    runpy.run_path(str(SCRIPTS / "run_restart_walls.py"), run_name="__main__")
    child = json.loads(capsys.readouterr().out.strip().splitlines()[-1])
    assert child["force_calls"] == 2
    assert child["steps"] == 1
    table = ROOT / "paper" / "cpc" / "data" / "failure" / "restart_formats.csv"
    saved = table.read_bytes()
    table.write_text("route,error\n", encoding="utf-8")
    try:
        monkeypatch.setattr(sys, "argv", ["run_restart_walls.py"])
        with pytest.raises(SystemExit, match="clean row"):
            runpy.run_path(str(SCRIPTS / "run_restart_walls.py"), run_name="__main__")
    finally:
        table.write_bytes(saved)


def test_drift_masks_openbabel_and_a_rejected_corpus(tmp_path, monkeypatch, capsys):
    from ase import Atoms
    from ase.constraints import FixAtoms, FixCartesian

    drift = load("drift.py")
    atoms = Atoms("H", positions=[[0.0, 0.0, 0.0]])
    atoms.set_constraint(FixCartesian(0, mask=(True, False, False)))
    mask = drift._mask(atoms)
    assert mask.tolist() == [[False, True, True]]
    atoms.set_constraint(FixAtoms(indices=[0]))
    assert not drift._mask(atoms).any()

    import types

    class Atom:
        def GetX(self):
            return 1.0

        def GetY(self):
            return 2.0

        def GetZ(self):
            return 3.0

    class Mol:
        pass

    mol = Mol()
    ob = types.SimpleNamespace(
        obErrorLog=types.SimpleNamespace(SetOutputLevel=lambda level: None),
        OBMolAtomIter=lambda m: iter([Atom(), Atom()]),
    )
    pybel = types.SimpleNamespace(
        readfile=lambda fmt, path: iter([types.SimpleNamespace(OBMol=mol)])
    )
    babel = types.ModuleType("openbabel")
    babel.openbabel = ob
    babel.pybel = pybel
    monkeypatch.setitem(sys.modules, "openbabel", babel)
    monkeypatch.setitem(sys.modules, "openbabel.openbabel", ob)
    monkeypatch.setitem(sys.modules, "openbabel.pybel", pybel)
    (tmp_path / "mol.xyz").write_text("xyz\n", encoding="utf-8")
    read = drift.r_openbabel(tmp_path / "mol.xyz")
    assert read["pos"].tolist() == [[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]]

    def missing(name):
        raise drift.metadata.PackageNotFoundError(name)

    monkeypatch.setattr(drift.metadata, "version", missing)
    found = drift.versions()
    assert found["extxyz"] is None
    assert found["pyleri"] is None

    monkeypatch.setenv("READCON_LIB", str(tmp_path / "missing.so"))
    assert drift._lib() is None

    monkeypatch.delenv("READCON_LIB", raising=False)
    import readcon

    def reject(path):
        raise RuntimeError("stopped")

    monkeypatch.setattr(readcon, "read_con", reject)
    monkeypatch.setattr(drift, "OUT", tmp_path)
    drift.part_b()
    saved = json.loads((tmp_path / "drift_con.json").read_text(encoding="utf-8"))
    assert saved["disagreements"] > 0
    assert "disagreements" in capsys.readouterr().out


def test_drift_entry_rewrites_the_two_tables():
    paths = [
        ROOT / "paper" / "cpc" / "data" / "failure" / name
        for name in (
            "drift_extxyz.csv",
            "drift_extxyz.json",
            "drift_con.csv",
            "drift_con.json",
        )
    ]
    saved = {path: path.read_bytes() for path in paths}
    try:
        runpy.run_path(str(SCRIPTS / "drift.py"), run_name="__main__")
        assert paths[0].stat().st_size > 0
        assert paths[2].stat().st_size > 0
    finally:
        for path, data in saved.items():
            path.write_bytes(data)
    assert paths[0].read_bytes() == saved[paths[0]]


def test_survey_stops_when_the_frame_does_not_read_back(monkeypatch):
    listing = load("survey_listing.py")
    monkeypatch.setattr(listing.fm, "w_readcon", lambda fx, path: None)

    class Atom:
        atom_id = 1
        x = 0.0
        y = 0.0
        z = 0.0
        fixed = (False, False, False)

    class Frame:
        atoms = [Atom()]

    import readcon

    monkeypatch.setattr(readcon, "read_con", lambda path: [Frame()])
    with pytest.raises(SystemExit, match="does not read back"):
        listing.measure()


def test_survey_entry_rewrites_the_listing_from_the_record(monkeypatch):
    tex = ROOT / "paper" / "cpc" / "src" / "figures" / "generated" / "survey_listing.tex"
    saved = tex.read_bytes()
    monkeypatch.setattr(sys, "argv", ["survey_listing.py", "--tables"])
    try:
        runpy.run_path(str(SCRIPTS / "survey_listing.py"), run_name="__main__")
    finally:
        tex.write_bytes(saved)
    assert tex.read_bytes() == saved


def test_restart_rejects_a_bad_mask_a_bad_h5_and_a_failed_route(tmp_path, monkeypatch):
    previous = os.environ.get("RC_KERNELS")
    monkeypatch.setattr(
        sys, "argv", ["restart_formats.py", "--kernels", "nehalem", "tail"]
    )
    formats = load("restart_formats.py")
    try:
        assert formats.KERNELS == "nehalem"
        assert os.environ["RC_KERNELS"] == "nehalem"
    finally:
        if previous is None:
            os.environ.pop("RC_KERNELS", None)
        else:
            os.environ["RC_KERNELS"] = previous

    from ase import Atoms
    from ase.constraints import FixScaled

    atoms = Atoms("H", positions=[[0.0, 0.0, 0.0]], cell=[2.0, 2.0, 2.0], pbc=True)
    atoms.set_constraint(FixScaled(0, mask=(True, False, False)))
    mask = formats.fixed_mask(atoms)
    assert bool(mask[0, 0]) and not bool(mask[0, 1])
    atoms.set_cell([[2.0, 0.2, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]])
    with pytest.raises(ValueError, match="Cartesian"):
        formats.fixed_mask(atoms)

    class Odd:
        pass

    atoms.constraints = [Odd()]
    with pytest.raises(ValueError, match="unhandled"):
        formats.fixed_mask(atoms)

    folder = tmp_path / "h5"
    folder.mkdir()
    (folder / "band.h5md").write_bytes(b"h5")
    with pytest.raises(ValueError, match="H5MD"):
        formats.h5md_named("plain", folder)
    import failure_matrix as fm

    monkeypatch.setattr(fm, "h5md_layout", lambda path: (None, []))
    with pytest.raises(ValueError, match="no H5MD root"):
        formats.h5md_named("H5MD route", folder)
    monkeypatch.setattr(fm, "h5md_layout", lambda path: ("1.0", ["mask"]))
    with pytest.raises(ValueError, match="private"):
        formats.h5md_named("H5MD route", folder)

    monkeypatch.setattr(
        formats.numeric_pins,
        "in_effect",
        lambda: {
            "environment": {},
            "numpy_dispatch": [],
            "blas": ["openblas Nehalem kernels, 1 thread"],
        },
    )
    monkeypatch.setattr(formats, "OUT", tmp_path / "out")
    monkeypatch.setattr(
        formats,
        "run",
        lambda imgs, start=0, stop=None: {
            "step": 1,
            "converged": True,
            "barrier": 0.2,
            "fmax": 0.001,
            "calls": 3,
            "pos": [],
            "energies": [0.2],
            "forces": [],
        },
    )
    monkeypatch.setattr(formats, "ROUTES", [("boom", lambda imgs, dest: imgs)])

    def boom(*args):
        raise RuntimeError("stopped")

    monkeypatch.setattr(formats, "resume", boom)
    with pytest.raises(SystemExit) as caught:
        formats.main()
    assert caught.value.code == 1

    def resume(name, fn, eps, ref):
        res = {
            "step": 2,
            "converged": True,
            "barrier": 0.2,
            "pos": [np.zeros((1, 3))],
            "energies": [0.2],
            "forces": [np.zeros((1, 3))],
            "full": {"file_dpos_A": 0.0, "d_barrier_eV": 0.0, "max_disp_A": 0.0},
        }
        row = {key: "" for key in formats.FIELDS}
        row["route"] = name
        row["steps"] = 2
        row["error"] = ""
        return row, res

    monkeypatch.setattr(formats, "resume", resume)
    formats.KERNELS = "nehalem"
    formats.main()
    record = json.loads(
        (tmp_path / "out" / "restart_kernels" / "nehalem.json").read_text(encoding="utf-8")
    )
    assert record["kernel_set"] == "nehalem"
    assert record["routes"]["boom"]["steps"] == 2

    formats.KERNELS = None
    bands = tmp_path / "out" / "bands"
    bands.mkdir()
    (bands / "old.npz").write_bytes(b"old")
    formats.main()
    assert not (bands / "old.npz").exists()
    table = (tmp_path / "out" / "restart_formats.csv").read_text(encoding="utf-8")
    assert "uninterrupted" in table
    assert "boom" in table


def test_referee_reports_a_crash_a_null_iterator_and_a_stale_record(tmp_path, monkeypatch, capsys):
    monkeypatch.delenv("READCON_LIB", raising=False)
    monkeypatch.delenv("READCON_CLI", raising=False)
    probes = load("referee_probes.py")
    lib = ROOT / "target" / "release" / "libreadcon_core.so"
    cli = ROOT / "target" / "cli" / "release" / "readcon-core"
    probes.LIB = lib
    probes.CLI = cli
    probes._CLIB.clear()

    assert str(probes.raised(lambda: None)) == "no error"
    failed = probes.raised(lambda: (_ for _ in ()).throw(ValueError("x")))
    assert failed.type == "ValueError"
    with pytest.raises(SystemExit):
        probes.raised(lambda: (_ for _ in ()).throw(SystemExit(3)))

    missing = tmp_path / "gone"
    assert probes.state(missing, b"a") == "removed"
    missing.write_bytes(b"ab")
    assert probes.state(missing, b"a") == "changed to 2 bytes"

    rejected = probes.cli_outcome(1, "Error: frame 1: BadNumber at line 4: nope\n")
    assert rejected["outcome"] == "reject" and rejected["line"] == 4
    bare = probes.cli_outcome(1, "Error: frame 1: BadNumber: nope\n")
    assert bare["line"] is None
    assert probes.cli_outcome(0, "Successfully parsed 2 valid frame\n")["frames"] == 2
    assert probes.cli_outcome(101, "panicked at the frame\n")["outcome"] == "crash"
    assert probes.cli_outcome(2, "nope\n")["class"] == "unreadable"

    import readcon

    def interrupt(text, *args):
        raise KeyboardInterrupt

    monkeypatch.setattr(readcon, "read_con_string", interrupt)
    with pytest.raises(KeyboardInterrupt):
        probes.read("readcon", "x", tmp_path)
    with pytest.raises(KeyboardInterrupt):
        probes.verdict(lambda: (_ for _ in ()).throw(KeyboardInterrupt()))
    monkeypatch.undo()
    monkeypatch.delenv("READCON_LIB", raising=False)
    monkeypatch.delenv("READCON_CLI", raising=False)

    def crash():
        raise RuntimeError("boom")

    assert probes.verdict(crash).startswith("crash RuntimeError")
    assert probes.verdict(lambda: (_ for _ in ()).throw(ValueError("bad"))).startswith("reject")
    assert probes.verdict(lambda: 2) == "accept 2"

    monkeypatch.setattr(sys, "stdin", io.TextIOWrapper(io.BytesIO(b"not a con\n")))
    child = probes.child("readcon")
    assert child["outcome"] in {"reject", "crash"}
    assert "rss_kib" in child and child["seconds"] >= 0
    monkeypatch.setattr(sys, "stdin", io.TextIOWrapper(io.BytesIO(b"not a con\n")))
    core = probes.child("readcon-core")
    assert "rss_kib" in core

    with pytest.raises(SystemExit, match="does not hold"):
        probes.register(
            "cover-g",
            "tests",
            "CON-notaclause.1",
            "expected",
            lambda folder: ("", True),
            command="read",
        )
    existing = probes.PROBES[0]
    with pytest.raises(SystemExit, match="twice"):
        probes.register(
            existing["id"],
            existing["source"],
            existing["rule"],
            existing["expected"],
            lambda folder: ("", True),
            command="read",
        )

    assert "missing" in probes.built_stale(tmp_path / "no-library")
    real_newer = probes.provenance.newer_sources
    monkeypatch.setattr(probes.provenance, "newer_sources", lambda path: ["src/lib.rs"])
    assert "older than" in probes.built_stale(lib)
    monkeypatch.setattr(probes.provenance, "newer_sources", real_newer)

    assert probes.c_verdict(None) == "reject: the C iterator is NULL"
    library = probes.clib()
    sample = next((ROOT / "paper" / "cpc" / "data" / "corpus31").glob("*.con"))
    accepted = probes.c_verdict(library.read_con_file_iterator(str(sample).encode()))
    assert accepted.startswith("accept") or accepted.startswith("reject")
    (tmp_path / "broken.con").write_text("not a frame\n", encoding="utf-8")
    refused = probes.c_verdict(
        library.read_con_file_iterator(str(tmp_path / "broken.con").encode())
    )
    assert refused.startswith("reject")

    try:
        probes.c_frame(1, 1)
    except RuntimeError as exc:
        assert "add_atom" in str(exc)

    seen = probes.every_path(tmp_path, sample.read_text(encoding="utf-8"), threads=(1,), stem="one")
    assert seen
    mixed = probes.one_answer({"a": "accept 1", "b": "reject Other"})
    assert mixed.startswith("a:")
    observed, agreed = probes.same_everywhere(
        tmp_path, "not a frame\n", probes.Want("reject", classes=("Other",)), threads=(1,), stem="bad"
    )
    assert observed
    assert agreed is False

    class _Vel:
        def get_velocities(self):
            return np.zeros((1, 3))

    class _Frame:
        velocity_time_unit = probes.NATURAL
        metadata = {"units": {"time": "fs"}}

        def to_ase(self):
            return _Vel()

        def velocities_array(self):
            return np.zeros((1, 3))

    real_read = readcon.read_con_string
    monkeypatch.setattr(readcon, "read_con_string", lambda text: [_Frame()])
    label, ok = probes.legacy_label("frame")
    assert ok is True
    assert "equal" in label and "fs" in label
    monkeypatch.setattr(readcon, "read_con_string", real_read)
    text, passed = probes.c_iterator_two_frames_blank(tmp_path)
    assert "statuses" in text
    assert isinstance(passed, bool)
    minor, kept = probes.minor_kept(-1)
    assert minor.startswith("read")
    assert kept is False or isinstance(kept, bool)

    saved_cli = probes.CLI
    probes.CLI = tmp_path / "missing-cli"
    try:
        for fn in (
            probes.cli_matches_plain,
            probes.cli_truncated_last_frame,
            probes.cli_rewrite_refused,
            probes.cli_gzip_multi_member,
            probes.cli_truncated_gzip,
        ):
            message, good = fn(tmp_path, "two.con") if fn is probes.cli_matches_plain else fn(tmp_path)
            assert good is False
            assert "missing" in message
    finally:
        probes.CLI = saved_cli

    results = [
        {"id": "a", "pass": True, "runs": [1]},
        {"id": "b", "pass": True, "runs": [1]},
    ]
    fresh = probes.comparable(probes.record(results))
    swapped = json.loads(json.dumps(fresh))
    swapped["probes"] = list(reversed(swapped["probes"]))
    record = tmp_path / "record.json"
    record.write_text(json.dumps(swapped), encoding="utf-8")
    problems = probes.stale_against_record(results, record)
    assert any("another order" in item for item in problems)

    monkeypatch.setattr(sys, "argv", ["referee_probes.py", "--check", "--out", str(tmp_path / "out")])
    with pytest.raises(SystemExit):
        probes.main()
    monkeypatch.setattr(sys, "argv", ["referee_probes.py", "--record", str(record)])
    with pytest.raises(SystemExit):
        probes.main()
    monkeypatch.setattr(sys, "stdin", io.TextIOWrapper(io.BytesIO(b"not a con\n")))
    monkeypatch.setattr(sys, "argv", ["referee_probes.py", "--child", "readcon"])
    assert probes.main() == 0
    assert "IncompleteHeader" in capsys.readouterr().out
    monkeypatch.setattr(sys, "stdin", io.TextIOWrapper(io.BytesIO(b"not a con\n")))
    with pytest.raises(SystemExit) as caught:
        runpy.run_path(str(SCRIPTS / "referee_probes.py"), run_name="__main__")
    assert caught.value.code == 0

    saved_cli = probes.CLI
    probes.CLI = tmp_path / "missing-cli"
    try:
        unavailable = probes.cli_read(tmp_path, "input.con")
    finally:
        probes.CLI = saved_cli
    assert unavailable["outcome"] == "unavailable"
    assert "missing" in unavailable["message"]

    def explode(text, *args):
        raise RuntimeError("broke")

    monkeypatch.setattr(readcon, "read_con_string", explode)
    crashed = probes.read("readcon", "text", tmp_path)
    assert crashed["outcome"] == "crash" and crashed["class"] == "RuntimeError"
    monkeypatch.setattr(readcon, "read_con_string", real_read)

    class Stopped:
        returncode = 3
        stdout = b""
        stderr = b"died\n"

    monkeypatch.setattr(probes.subprocess, "run", lambda *args, **kwargs: Stopped())
    isolated = probes.read_isolated("readcon", "text", tmp_path)
    assert isolated == {
        "outcome": "crash",
        "class": "exit 3",
        "message": "died",
    }

    library = probes.clib()

    def refuse(*args):
        return -1

    library.rkr_frame_add_atom = refuse
    with pytest.raises(RuntimeError, match="add_atom"):
        probes.c_frame(1, 2)
    probes._CLIB.clear()

    class _Pbc:
        def __init__(self, pbc):
            self.pbc = pbc

        def to_ase(self):
            return self

    def read_pbc(text):
        raw = text if isinstance(text, str) else ""
        pbc = (True, False, True) if "false" in raw.lower() else (True, True, True)
        return [_Pbc(pbc)]

    monkeypatch.setattr(readcon, "read_con_string", read_pbc)
    monkeypatch.setattr(
        readcon.ConFrame, "from_ase", staticmethod(lambda atoms: _Pbc(tuple(atoms.pbc))), raising=False
    )
    monkeypatch.setattr(readcon, "write_con_string", lambda frames: "con\n{}\n")
    absent, declared_ok = probes.pbc_frame_attribute(tmp_path)
    assert "absent:" in absent and "declared:" in absent
    assert declared_ok is True
    booleans, flags_ok = probes.pbc_booleans_and_default(tmp_path)
    assert "ConFrame.pbc" in booleans
    assert isinstance(flags_ok, bool)
    slab, slab_ok = probes.pbc_slab_ase_round_trip(tmp_path)
    assert "ase.Atoms slab" in slab
    assert isinstance(slab_ok, bool)

    class _Writer:
        def get_velocities(self):
            return np.zeros((1, 3))

    class _Kept:
        velocity_time_unit = probes.NATURAL
        metadata = {"units": {"time": "fs"}}
        pbc = (True, True, True)

        def to_ase(self):
            return _Writer()

        def velocities_array(self):
            return np.zeros((1, 3))

    def write_frames(frames):
        line = json.dumps(
            {"con_spec_version": 3, "con_spec_minor": 0, "units": {"time": "fs"}}
        )
        return "con\n" + line + "\n"

    monkeypatch.setattr(readcon, "read_con_string", lambda text: [_Kept()])
    monkeypatch.setattr(readcon, "read_con", lambda path: [_Kept()])
    monkeypatch.setattr(readcon, "write_con_string", write_frames)
    rewritten, rewrite_ok = probes.v30_fs_label_rewrite(tmp_path)
    assert rewrite_ok is True
    assert "fs" in rewritten
    spellings, spelling_ok = probes.natural_spellings(tmp_path)
    assert spelling_ok is True
    assert probes.NATURAL in spellings
    corpus, corpus_ok = probes.corpus_v30_fs_label(tmp_path)
    assert corpus_ok is True
    assert "3" in corpus
    kept_text, kept_ok = probes.pre_31_label_kept(tmp_path)
    assert kept_ok is True
    assert "v30_convel_fs.convel" in kept_text
    minor_text, minor_ok = probes.pre_31_minor_kept(tmp_path)
    assert "rewritten as version" in minor_text
    assert isinstance(minor_ok, bool)

    monkeypatch.setattr(probes, "run_all", lambda: [{"id": "a", "pass": True, "runs": [1]}])
    monkeypatch.setattr(probes, "failures", lambda results: [])
    monkeypatch.setattr(probes, "stale_module", lambda: [])
    monkeypatch.setattr(probes, "stale_against_record", lambda results, path: [])
    record = tmp_path / "matched.json"
    record.write_text("{}\n", encoding="utf-8")
    monkeypatch.setattr(
        sys, "argv", ["referee_probes.py", "--check", "--record", str(record)]
    )
    assert probes.main() == 0
    assert "matches this run" in capsys.readouterr().out

    probes._CLIB.clear()
    probes.LIB = tmp_path / "missing-library"
    with pytest.raises(probes.Unavailable, match="missing"):
        probes.clib()
