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

`scripts/coverage.sh` runs one command for the workspace:

```
cargo llvm-cov --workspace --all-targets --doc \
  --features parallel,zstd,grammar --branch
```

It then runs the C driver against the instrumented static library,
builds the Python extension with `cargo llvm-cov show-env` and
`maturin develop`, runs `pytest tests/python`, and writes one report.
