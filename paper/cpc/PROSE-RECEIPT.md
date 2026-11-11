# Raw records

These files are raw records.
They are not prose.

| Record | Writer |
| --- | --- |
| paper/cpc/receipts/replay-map.txt | paper/cpc/scripts/write_replay_map.py |
| paper/cpc/receipts/distrib_read/ | paper/cpc/scripts/mutate_distrib_read.py |
| paper/cpc/receipts/python/ | paper/cpc/scripts/mutate_python_arms.py |
| paper/cpc/receipts/import/ | paper/cpc/scripts/mutate_import_bonds.py |
| paper/cpc/receipts/zstd-writer-null-fail.txt | paper/cpc/scripts/write_log_receipts.py |
| paper/cpc/receipts/zstd-writer-error-pass.txt | paper/cpc/scripts/write_log_receipts.py |
| paper/cpc/receipts/zstd-writer-roundtrip-pass.txt | paper/cpc/scripts/write_log_receipts.py |
| paper/cpc/receipts/empty-chemfiles-frame-pass.txt | paper/cpc/scripts/write_log_receipts.py |
| paper/cpc/receipts/empty-chemfiles-frame-abort.txt | paper/cpc/scripts/write_log_receipts.py |
| paper/cpc/bench/results/r0174/reads.json | paper/cpc/bench/reads.py |
| paper/cpc/bench/results/r0174/memray.json | paper/cpc/bench/memray_peaks.py |
| paper/cpc/bench/results/r0174/provenance.json | paper/cpc/bench/r0174.py |
| paper/cpc/bench/results/r0174/h2h/ | paper/cpc/bench/h2h.py |
| paper/cpc/bench/results/r0174/binary_formats/ | paper/cpc/bench/binary_formats.py |
| paper/cpc/bench/results/r0174/cachegrind/ | paper/cpc/bench/cachegrind.py |

Each cargo receipt stores the raw rustc -V and cargo -V lines.
Those lines come from the pin named in Cargo.toml.
