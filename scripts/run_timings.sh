#!/usr/bin/env bash
# Build the timed artefacts, then run the extended XYZ comparison, the CON
# comparison, and the CON iterator timings in one session.
# The bench scripts in this commit are the ones that produce the numbers.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${H2H_PYTHON:-$(python3 -c 'import sys; print(sys.executable)')}"
OUT="${H2H_OUT:-$("$PY" -c 'from pathlib import Path; print(Path.home() / "h2h-work" / "r29-logs")')}"
export H2H_PYTHON="$PY"
export H2H_BUILD="${H2H_BUILD:-/src/out}"
export H2H_OUT="$OUT"
export H2H_REQUIRE_CLEAN=1
mkdir -p "$OUT"
bash "$ROOT/scripts/build_timed.sh"
cd "$ROOT/bench/h2h_timed/harness"
H2H_RESULTS="$OUT/extxyz.json" "$PY" campaign.py
"$PY" iterator_campaign.py
"$PY" - "$OUT" "$H2H_BUILD" <<'PY'
import hashlib
import importlib.util
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
build_dir = Path(sys.argv[2])
home = Path.home().resolve().as_posix()

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()

def rel(path: Path) -> str:
    text = path.resolve().as_posix()
    prefix = home + "/"
    if text.startswith(prefix):
        return text[len(prefix):]
    return text

extxyz = json.loads((out / "extxyz.json").read_text(encoding="utf-8"))
summary_path = Path.home() / "h2h-work" / "r29-logs" / "iterator" / "summary.json"
iterator = json.loads(summary_path.read_text(encoding="utf-8"))
spec = importlib.util.spec_from_file_location("h2h_sample", Path("sample.py").resolve())
sample = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sample)
common_spec = importlib.util.spec_from_file_location("h2h_common", Path("common.py").resolve())
common = importlib.util.module_from_spec(common_spec)
common_spec.loader.exec_module(common)
build = json.loads((build_dir / "build.json").read_text(encoding="utf-8"))
after = {}
for name, item in build["artefacts_before"].items():
    path = Path(item["path"])
    digest = sha256(path)
    if digest != item["sha256"]:
        raise SystemExit(f"after hash mismatch for {name}")
    after[name] = {"path": rel(path), "sha256": digest}
for key, info in build["arms"].items():
    digest = sha256(Path(info["binary_path"]))
    if digest != info["binary_sha256"]:
        raise SystemExit(f"after arm hash mismatch for {key}")
build["artefacts_after"] = after
extxyz["iterator"] = iterator
others = sample.extension_sha256()
extxyz.setdefault("extension_sha256", {}).update(others)
extxyz["script_sha256"] = iterator["script_sha256"]
extxyz["build"] = build
extxyz["python_wheels"] = common.timed_wheel_record()
(out / "results.json").write_text(json.dumps(extxyz, indent=2) + "\n", encoding="utf-8")
print(f"wrote {rel(out / 'results.json')}", flush=True)
PY
