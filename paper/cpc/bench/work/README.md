# Timing payloads

`frames-10000.con` is not stored as a unique binary in the archive by default.
Rebuild it from the repository root:

```bash
python3 - <<'PY'
from pathlib import Path
src = Path("resources/test/cuh2.con").read_text()
out = Path("paper/cpc/bench/work/frames-10000.con")
out.parent.mkdir(parents=True, exist_ok=True)
with out.open("w") as handle:
    for _ in range(10000):
        handle.write(src)
        if not src.endswith("\n"):
            handle.write("\n")
print(out, out.stat().st_size)
PY
```

The file is 167,710,000 bytes: ten thousand copies of the 218-atom copper-hydrogen frame.
