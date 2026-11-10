# Coverage measurement

v14.2 is a checkpoint, not a pass. The v14.3 table in
`COVERAGE_TABLE.md` is newer. `scripts/coverage_table.py` writes it
from a fresh `cargo llvm-cov --branch` JSON export. Seven source
files are still OPEN because they are under 98 percent on lines or
branches. After the live-handle tests, ffi.rs is at 97.96 percent
lines and 97.44 percent branches. The v14.2 checkpoint list was:

- src/conbin.rs
- src/ffi.rs
- src/frame_index.rs
- src/iterators.rs
- src/parser.rs
- src/stream.rs
- src/writer.rs

A command-line write reported success when the buffered writer could
not flush. The command now flushes before it reports success, and the
path converter does the same. A full device returns an error from both.

The sections below are earlier measurements. They are not the v14.2
table.

ffi.rs was reported at 94.75 percent lines by a run of
`cargo llvm-cov --lib --tests`, and at 87.07 percent lines by a later
run of `cargo llvm-cov --lib`. Those runs do not measure the same
binaries. The library-only run counts the unit-test binary. Adding
integration tests also builds the cdylib. That cdylib is a second copy
of every `no_mangle` export, nothing calls it, and llvm-cov merges its
zero counts over the copy the tests did call. The percentage moves
even when the source and the tests do not.

`scripts/coverage.sh` runs the workspace with every target, then the
doctests. Those two flags cannot be combined, and the script keeps
both sets of profiles for one report. It then runs the C driver, and
the Fortran driver when `fpm` is on `PATH`, against the instrumented
static library. It builds the Python extension with
`cargo llvm-cov show-env` and `maturin develop`, runs
`pytest tests/python`, and writes one report.

## Run after the cold-arm tests

Files still under 98 percent on lines or branches:

| File | Lines | Branches |
| --- | --- | --- |
| array.rs | 99.20 | 100.00 |
| conbin.rs | 95.10 | 89.71 |
| convert.rs | 98.88 | 100.00 |
| extxyz_scan.rs | 100.00 | 95.59 |
| ffi.rs | 96.53 | 76.20 |
| frame_index.rs | 99.84 | 93.33 |
| index_proj.rs | 97.83 | 75.00 |
| iterators.rs | 98.73 | 93.71 |
| main.rs | 96.67 | 86.67 |
| parser.rs | 98.15 | 91.96 |
| python.rs | 89.39 | 77.14 |
| stream.rs | 98.66 | 92.86 |
| types.rs | 99.70 | 97.87 |
| units.rs | 100.00 | 96.49 |
| writer.rs | 97.75 | 92.35 |

Every other source file is at or above 98 percent on both bars.

## Run after the writer and unit tests

Files still under 98 percent on lines or branches:

| File | Lines | Branches |
| --- | --- | --- |
| conbin.rs | 94.88 | 92.19 |
| ffi.rs | 96.53 | 76.20 |
| frame_index.rs | 99.84 | 93.33 |
| iterators.rs | 98.76 | 93.88 |
| parser.rs | 98.15 | 91.96 |
| python.rs | 89.39 | 77.14 |
| stream.rs | 98.66 | 92.86 |
| types.rs | 99.70 | 97.87 |
| writer.rs | 97.93 | 97.02 |

Nineteen source files are at or above 98 percent on both bars.

## Run after the cold wide-file stream

Files still under 98 percent on lines or branches:

| File | Lines | Branches |
| --- | --- | --- |
| ffi.rs | 96.53 | 76.20 |
| frame_index.rs | 99.84 | 93.33 |
| iterators.rs | 98.73 | 93.71 |
| parser.rs | 98.15 | 91.96 |
| python.rs | 89.39 | 77.14 |
| stream.rs | 98.66 | 92.86 |
| writer.rs | 97.93 | 97.02 |

Sixteen source files are at or above 98 percent on both bars.
conbin.rs is at 99.65 percent lines and 99.46 percent branches.
types.rs is at 99.79 percent lines and 98.61 percent branches.

## Run 84 checkpoint

`paper/cpc/receipts/run84-taken.txt` is the summary
`coverage_features.sh` wrote for the mixed tree, before that commit
was split. python.rs is under 98 percent on both bars in that file.
The split replaced the mixed commit with one fix per removed guard.
The namespace child no longer uses a fixed profile path. It creates
a directory under `target/cov-live`, deletes it, and also runs with
the profile variable unset. This tip has not been measured again.

The seven paper-record tests still fail. The producers
`bench/reads.py`, `bench/cpp_reads.py` and `bench/r0174.py` have not
been run at this tip. The values the checks compare:

- `bench/results/r0174/reads.json` names release 0.17.4.
  `provenance.json` names the version in `Cargo.toml`.
- `cpp/build.json` hashes `readcon_read.cpp` as `1c37e6df28f3`.
  `bench/native/readcon_read.cpp` hashes to `65fd82243ef1`.
- `cpp/build.json` hashes `cpp_reads.py` as `775e2eda69a0`.
  `paper/cpc/bench/cpp_reads.py` hashes to `faebe04d52cb`.
- `cpp/build.json` records library digest `3b4d00ae2427e0e1`.
  `provenance.json` records `d94b6ad922e9980e`.
  The binary-format records name both, with one binary sha256.

Residual arms from that measurement:

- `src/chemfiles_import_imp.rs:920`. The test
  `imports_xyz_from_temp_file_via_chemfiles_io` asserts an oxygen at
  the origin and an oxygen whose z is above 0.5. The export left one
  side of the origin comparison at count 0.
- `src/conbin.rs:594`. `handle.join().is_err` inside
  `sum_pieces_in_order`. Callers: `sum_split_streams` at line 523 and
  `split_stream_sum` at line 641. The worker only receives a piece
  index and adds a range. A join error requires that worker to panic.
- `src/conbin.rs:690`. `err_rx.try_recv` inside `split_stream_sum`.
  Callers: `sum_split_streams` at line 619, and the tests at lines
  2000, 2623 and 2986. The zero side is an error string on that
  channel. The fault cases that fail an open do not send on it.
- `src/conbin.rs:2811`. The `Ok` arm of `stream_one_span` after the
  first round in `run_fault_suite`. The only caller is that loop.
  The page is unmapped, and the call returns the error the test
  asserts, so the `Ok` arm does not run.
- `src/conbin.rs:3138` and `src/conbin.rs:3214`. The profile directory
  starts with the temporary-directory prefixes. The only caller of
  `hold_coverage_profile` from the suite is `run_namespace_suite`.
  The child no longer builds a profile on those prefixes.
