#!/usr/bin/env bash
# Build the timed artefacts, then run the extended XYZ comparison, the CON
# comparison, and the CON iterator timings in one session.
# The bench scripts in this commit are the ones that produce the numbers.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${H2H_PYTHON:-python3}"
OUT="${H2H_OUT:-$ROOT/target/h2h-logs}"
dir_build="build"
dir_stage="timed-stage"
export H2H_PYTHON="$PY"
export H2H_BUILD="${H2H_BUILD:-$ROOT/$dir_build/$dir_stage/out}"
export H2H_OUT="$OUT"
export H2H_REQUIRE_CLEAN=1
export H2H_REPEATS=10
mkdir -p "$OUT"
head_sha="$(git -C "$ROOT" rev-parse HEAD)"
tree_sha="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
printf 'HEAD %s\n' "$head_sha"
printf 'tree %s\n' "$tree_sha"
printf 'command %s\n' "bash scripts/run_timings.sh"
trap 'code=$?; printf "exit %s\n" "$code"' EXIT
"$PY" - start "$OUT/session-start.json" <<'PY'
import json, subprocess, sys
from datetime import datetime, timezone
from pathlib import Path
label, dest = sys.argv[1], Path(sys.argv[2])
boot = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="utf-8").strip()
uptime = Path("/proc/uptime").read_text(encoding="utf-8").split()[0]
wall = datetime.now(timezone.utc).isoformat(timespec="seconds")
listing = subprocess.check_output(["lscpu"], text=True)
stat_lines = [line for line in Path("/proc/stat").read_text(encoding="utf-8").splitlines() if line.startswith("cpu")]
loadavg = Path("/proc/loadavg").read_text(encoding="utf-8").strip()
print(f"session {label} boot_id {boot}", flush=True)
print(f"session {label} uptime_s {uptime}", flush=True)
print(f"session {label} wall_utc {wall}", flush=True)
print(f"session {label} loadavg {loadavg}", flush=True)
for line in stat_lines:
    print(f"session {label} proc_stat {line}", flush=True)
print(f"session {label} lscpu", flush=True)
sys.stdout.write(listing if listing.endswith("\n") else listing + "\n")
print(f"session {label} lscpu_end", flush=True)
dest.write_text(json.dumps({"boot_id": boot, "uptime_s": uptime, "wall_utc": wall, "lscpu": listing, "loadavg": loadavg, "proc_stat": stat_lines}) + "\n")
PY
bash "$ROOT/scripts/build_timed.sh"
cd "$ROOT/bench/h2h_timed/harness"
H2H_RESULTS="$OUT/extxyz.json" "$PY" campaign.py
"$PY" iterator_campaign.py
"$PY" - "$OUT" "$H2H_BUILD" "$ROOT" <<'PY'
import hashlib
import importlib.util
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

out = Path(sys.argv[1])
build_dir = Path(sys.argv[2])
root = Path(sys.argv[3]).resolve()
home = Path.home().resolve()

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()

def locate(text: str) -> Path:
    path = Path(text)
    if path.is_absolute():
        return path
    return root / path

def rel(path: Path) -> str:
    resolved = path.resolve()
    for base in (root, home):
        try:
            return resolved.relative_to(base).as_posix()
        except ValueError:
            continue
    return resolved.name

extxyz = json.loads((out / "extxyz.json").read_text(encoding="utf-8"))
summary_path = out / "iterator" / "summary.json"
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
    path = locate(item["path"])
    digest = sha256(path)
    if digest != item["sha256"]:
        raise SystemExit(f"after hash mismatch for {name}")
    after[name] = {"path": rel(path), "sha256": digest}
for key, info in build["arms"].items():
    digest = sha256(locate(info["binary_path"]))
    if digest != info["binary_sha256"]:
        raise SystemExit(f"after arm hash mismatch for {key}")
build["artefacts_after"] = after
extxyz["iterator"] = iterator
others = sample.extension_sha256()
extxyz.setdefault("extension_sha256", {}).update(others)
extxyz["script_sha256"] = iterator["script_sha256"]
extxyz["build"] = build
extxyz["python_wheels"] = common.timed_wheel_record()
start = json.loads((out / "session-start.json").read_text(encoding="utf-8"))
end_boot = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="utf-8").strip()
end_uptime = Path("/proc/uptime").read_text(encoding="utf-8").split()[0]
end_wall = datetime.now(timezone.utc).isoformat(timespec="seconds")
end_lscpu = subprocess.check_output(["lscpu"], text=True)
end_stat = [line for line in Path("/proc/stat").read_text(encoding="utf-8").splitlines() if line.startswith("cpu")]
end_load = Path("/proc/loadavg").read_text(encoding="utf-8").strip()
print(f"session end boot_id {end_boot}", flush=True)
print(f"session end uptime_s {end_uptime}", flush=True)
print(f"session end wall_utc {end_wall}", flush=True)
print(f"session end loadavg {end_load}", flush=True)
for line in end_stat:
    print(f"session end proc_stat {line}", flush=True)
print("session end lscpu", flush=True)
sys.stdout.write(end_lscpu if end_lscpu.endswith("\n") else end_lscpu + "\n")
print("session end lscpu_end", flush=True)
extxyz["session"] = {
    "start": start,
    "end": {
        "boot_id": end_boot,
        "uptime_s": end_uptime,
        "wall_utc": end_wall,
        "lscpu": end_lscpu,
        "loadavg": end_load,
        "proc_stat": end_stat,
    },
}
(out / "results.json").write_text(json.dumps(extxyz, indent=2) + "\n", encoding="utf-8")
print(f"wrote {rel(out / 'results.json')}", flush=True)
PY
