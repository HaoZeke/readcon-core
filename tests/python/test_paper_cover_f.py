"""Outputs for the failure matrix and the flat manuscript builder."""

from __future__ import annotations

import importlib.util
import json
import runpy
import sys
import types
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "paper" / "cpc" / "scripts"


def load(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location("coverf_" + Path(name).stem, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(SCRIPTS))
    spec.loader.exec_module(module)
    return module


def _fx():
    return {
        "symbols": ["H", "O"],
        "pos": np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]),
        "ids": np.array([1, 2]),
        "cell": np.eye(3) * 10.0,
        "mask": np.array([[True, False, False], [False, False, True]]),
        "vel": np.zeros((2, 3)),
        "masses": np.array([1.0, 16.0]),
        "pbc": [True, True, False],
    }


def test_openbabel_writer_and_reader_round_trip_labels(tmp_path, monkeypatch):
    matrix = load("failure_matrix.py")
    atoms = []

    class Pair:
        def SetAttribute(self, name):
            self.name = name

        def SetValue(self, value):
            self.value = value

        def GetValue(self):
            return self.value

    class Atom:
        def __init__(self):
            self.x = self.y = self.z = 0.0
            self.label = None

        def SetAtomicNum(self, number):
            self.number = number

        def SetVector(self, x, y, z):
            self.x, self.y, self.z = x, y, z

        def CloneData(self, data):
            self.label = data

        def GetX(self):
            return self.x

        def GetY(self):
            return self.y

        def GetZ(self):
            return self.z

        def GetData(self, key):
            return self.label

    class Unit:
        def SetData(self, *args):
            self.args = args

        def SetSpaceGroup(self, name):
            self.group = name

        def GetA(self):
            return 10.0

        def GetB(self):
            return 10.0

        def GetC(self):
            return 10.0

    class Mol:
        def __init__(self):
            self.atoms = []
            self.cell = None

        def BeginModify(self):
            return None

        def EndModify(self):
            return None

        def NewAtom(self):
            atom = Atom()
            self.atoms.append(atom)
            atoms.append(atom)
            return atom

        def CloneData(self, cell):
            self.cell = cell

        def HasData(self, kind):
            return self.cell is not None

        def GetData(self, kind):
            return self.cell

    stored = {}

    class Wrapped:
        def __init__(self, mol):
            self.OBMol = mol

        def write(self, fmt, path, overwrite=True):
            stored["fmt"] = fmt
            stored["mol"] = self.OBMol
            Path(path).write_text(fmt + "\n", encoding="utf-8")

    ob = types.SimpleNamespace(
        obErrorLog=types.SimpleNamespace(SetOutputLevel=lambda level: None),
        OBMol=Mol,
        OBPairData=Pair,
        OBUnitCell=Unit,
        GetAtomicNum=lambda symbol: {"H": 1, "O": 8}[symbol],
        vector3=lambda *coords: coords,
        OBMolAtomIter=lambda mol: iter(mol.atoms),
        toUnitCell=lambda cell: cell,
        UnitCell="cell",
    )
    pybel = types.SimpleNamespace(
        Molecule=Wrapped,
        readfile=lambda fmt, path: iter([Wrapped(stored["mol"])]),
    )
    babel = types.ModuleType("openbabel")
    babel.openbabel = ob
    babel.pybel = pybel
    monkeypatch.setitem(sys.modules, "openbabel", babel)
    monkeypatch.setitem(sys.modules, "openbabel.openbabel", ob)
    monkeypatch.setitem(sys.modules, "openbabel.pybel", pybel)

    dest = tmp_path / "mol.cif"
    matrix.w_openbabel("cif")(_fx(), dest)
    assert dest.read_text(encoding="utf-8") == "cif\n"
    assert [atom.label.GetValue() for atom in atoms] == ["H1", "O2"]
    read = matrix.r_openbabel("cif")(dest)
    assert read["labels"] == ["H1", "O2"]
    assert read["cell"].tolist() == [10.0, 10.0, 10.0]
    assert read["pbc"] == [True, True, True]


def test_failure_rows_record_a_reader_crash_and_a_bad_h5(tmp_path, monkeypatch):
    matrix = load("failure_matrix.py")
    monkeypatch.setattr(matrix, "OUT", tmp_path)

    class Boom(BaseException):
        pass

    def write(fx, path):
        path.write_text("written\n", encoding="utf-8")

    def read(path):
        raise Boom("stopped")

    monkeypatch.setattr(
        matrix,
        "ROUTES",
        [("CON 3.1", "readcon-core", "readcon-core", "con", write, read)],
    )
    import provenance

    monkeypatch.setattr(
        provenance,
        "record",
        lambda: {
            "peers": {
                name: "0"
                for name in (
                    "ase",
                    "chemfiles",
                    "MDAnalysis",
                    "h5py",
                    "ovito",
                    "openbabel",
                )
            }
        },
    )
    monkeypatch.setattr(matrix, "summary", lambda rows: {"routes": len(rows)})
    matrix.main()
    rows = json.loads((tmp_path / "matrix.json").read_text(encoding="utf-8"))["rows"]
    assert rows[0]["R1"] == "reader error"
    assert rows[0]["error"].startswith("Boom:")

    def bad_h5(path):
        path.write_bytes(b"h5")
        return None, []

    monkeypatch.setattr(matrix, "h5md_layout", bad_h5)
    monkeypatch.setattr(
        matrix,
        "ROUTES",
        [("H5MD, private mask dataset", "h5py", "h5py", "h5md", write, read)],
    )
    with pytest.raises(SystemExit, match="H5MD root version"):
        matrix.main()


def test_where_names_a_library_frame(tmp_path):
    matrix = load("failure_matrix.py")
    pkg = tmp_path / "site-packages"
    pkg.mkdir()
    (pkg / "demo.py").write_text("def boom():\n    raise RuntimeError('x')\n", encoding="utf-8")
    sys.path.insert(0, str(pkg))
    try:
        import demo

        with pytest.raises(RuntimeError) as caught:
            demo.boom()
    finally:
        sys.path.remove(str(pkg))
    located = matrix.where(caught.value)
    assert located.startswith("demo.py:")


def test_flat_builder_reports_a_bad_log_and_a_second_document(tmp_path, monkeypatch):
    em = load("make_em_flat.py")
    real_compile = em.compile_flat
    texbin = tmp_path / "texbin"
    texbin.mkdir()
    tool = texbin / "pdflatex"
    tool.write_text("#!/bin/sh\necho 'pdfTeX 3.14 (TeX Live 2025)'\n", encoding="utf-8")
    tool.chmod(0o755)
    monkeypatch.setattr(em.arxiv, "supported_unicode", lambda texbin: set())

    def sources(root: Path, main: str) -> None:
        root.mkdir()
        (root / "rg_main.tex").write_text(main, encoding="utf-8")
        (root / "rg_supp.tex").write_text("supp\n", encoding="utf-8")
        (root / "rg_main.aux").write_text("\n", encoding="utf-8")
        (root / "rg_supp.aux").write_text("\n", encoding="utf-8")
        (root / "rg_main.bbl").write_text("\\bibitem{a}\n", encoding="utf-8")
        spec = root / "spec.org"
        spec.write_text("spec\n", encoding="utf-8")
        em.arxiv.SPEC_ORG = spec

    bad = tmp_path / "bad"
    sources(
        bad,
        "\\" + "externaldocument{other}\n\\" + "externaldocument{rg_supp}\n\\" + "bibliography{rg_main}\n",
    )
    monkeypatch.setattr(sys, "argv", ["make_em_flat.py", "--out", str(tmp_path / "out-bad"), "--src", str(bad), "--texbin", str(texbin)])
    with pytest.raises(em.arxiv.PackError, match="externaldocument"):
        em.main()

    good = tmp_path / "good"
    sources(good, "\\" + "bibliography{rg_main}\n")
    (tmp_path / "out" / "em-flat").mkdir(parents=True)
    (tmp_path / "out" / "em-flat" / "old.txt").write_text("old\n", encoding="utf-8")

    def compiled(flat, texbin, env):
        return (
            "log",
            [1, 0, 0],
            {
                "found": {
                    "error": {"fatal": True, "count": 1, "examples": ["Undefined control sequence"]},
                    "labels changed": {"fatal": True, "count": 2},
                    "undefined reference": {"fatal": True, "count": 3},
                    "undefined citation": {"fatal": True, "count": 0},
                },
                "kinds": {},
                "info": {"pages": 2},
            },
        )

    monkeypatch.setattr(em, "compile_flat", compiled)
    monkeypatch.setattr(
        sys, "argv", ["make_em_flat.py", "--out", str(tmp_path / "out"), "--src", str(good), "--texbin", str(texbin)]
    )
    assert em.main() == 1
    report = json.loads((tmp_path / "out" / "em-flat-report.json").read_text(encoding="utf-8"))
    assert any(item.startswith("pdflatex exit codes") for item in report["failures"])
    assert "undefined reference: 3" in report["failures"]
    assert not (tmp_path / "out" / "em-flat" / "old.txt").exists()

    def first_aid(flat, texbin, env):
        return (
            "log",
            [0, 0, 0],
            {
                "found": {
                    "error": {"fatal": True, "count": 1, "examples": ["listings First Aid"]},
                    "labels changed": {"fatal": True, "count": 0},
                    "undefined reference": {"fatal": True, "count": 0},
                    "undefined citation": {"fatal": True, "count": 0},
                },
                "kinds": {},
                "info": {"pages": 1},
            },
        )

    monkeypatch.setattr(em, "compile_flat", first_aid)
    monkeypatch.setattr(
        sys, "argv", ["make_em_flat.py", "--out", str(tmp_path / "aid"), "--src", str(good), "--texbin", str(texbin)]
    )
    assert em.main() == 0
    aid = json.loads((tmp_path / "aid" / "em-flat-report.json").read_text(encoding="utf-8"))
    assert any("First Aid" in note for note in aid["notes"])
    assert aid["failures"] == []

    def labels(flat, texbin, env):
        return (
            "log",
            [0, 0, 0],
            {
                "found": {
                    "error": {"fatal": True, "count": 0, "examples": []},
                    "labels changed": {"fatal": True, "count": 4},
                    "undefined reference": {"fatal": False, "count": 2},
                    "undefined citation": {"fatal": False, "count": 1},
                },
                "kinds": {},
                "info": {"pages": 1},
            },
        )

    monkeypatch.setattr(em, "compile_flat", labels)
    monkeypatch.setattr(
        sys, "argv", ["make_em_flat.py", "--out", str(tmp_path / "labels"), "--src", str(good), "--texbin", str(texbin)]
    )
    assert em.main() == 1
    listed = json.loads((tmp_path / "labels" / "em-flat-report.json").read_text(encoding="utf-8"))
    assert "labels changed: 4" in listed["failures"]
    assert "undefined reference: 2" in listed["failures"]
    assert "undefined citation: 1" in listed["failures"]

    missing = tmp_path / "no-spec.org"
    saved = em.arxiv.SPEC_ORG
    em.arxiv.SPEC_ORG = missing
    try:
        with pytest.raises(em.arxiv.PackError, match="is missing"):
            em.write_flat(good, tmp_path / "flat-miss", good, {"rewrites": []})
    finally:
        em.arxiv.SPEC_ORG = saved

    em.arxiv.SPEC_ORG = good / "spec.org"
    try:
        placed = em.write_flat(good, tmp_path / "flat-spec", good, {"rewrites": []})
    finally:
        em.arxiv.SPEC_ORG = saved
    assert placed.count("spec.org") == 1

    monkeypatch.setattr(em, "compile_flat", real_compile)
    quiet = tmp_path / "quietbin"
    quiet.mkdir()
    compiler = quiet / "pdflatex"
    compiler.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    compiler.chmod(0o755)
    bare = tmp_path / "nopdf"
    bare.mkdir()
    (bare / "rg_main.tex").write_text(
        "\\documentclass{article}\\begin{document}x\\end{document}\n",
        encoding="utf-8",
    )
    _log, codes, _comp = em.compile_flat(bare, quiet, {})
    assert codes == [0, 0, 0]
    assert not (bare.parent / "em-flat-pdflatex.pdf").exists()

    monkeypatch.setattr(
        sys,
        "argv",
        ["make_em_flat.py", "--out", str(tmp_path / "entry"), "--texbin", str(tmp_path / "missing-bin")],
    )
    with pytest.raises(SystemExit) as caught:
        runpy.run_path(str(SCRIPTS / "make_em_flat.py"), run_name="__main__")
    assert caught.value.code == 2
