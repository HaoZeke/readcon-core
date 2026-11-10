# Coverage measurement

`scripts/coverage.sh` is the only measurement used for the line and
branch bars.

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
