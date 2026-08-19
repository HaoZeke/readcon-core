# ReadCon.jl

Thin `ccall` bindings over `libreadcon_core` (same ABI as `include/readcon-core.h`).

The wrapper loads the shared library in this order:

1. Julia artifact `libreadcon_core` (`Artifacts.toml`, filled from `Artifacts.toml.in` after a clib GitHub Release)
2. `READCON_LIB_PATH` (file path to the cdylib)
3. `READCON_CORE_LIB` (same; both names are accepted)
4. A local `target/{release,debug}` build next to this package

Windows chemfiles is not in the Windows clib tarball.

## Run tests locally

1. Build the shared library from the **repository root**:

   ```bash
   cargo build --release --features chemfiles
   # optional fat matrix: chemfiles,zstd,metatensor
   export READCON_CORE_LIB="$PWD/target/release/libreadcon_core.so"
   # equivalent: export READCON_LIB_PATH="$PWD/target/release/libreadcon_core.so"
   export JULIA_LOAD_PATH="$PWD/julia/ReadCon:$JULIA_LOAD_PATH"
   ```

2. From `julia/ReadCon` (or with `JULIA_PROJECT` set):

   ```bash
   julia --project=. -e 'using Pkg; Pkg.test()'
   ```

If `libreadcon_core` is not on `LD_LIBRARY_PATH` / `READCON_CORE_LIB` /
`READCON_LIB_PATH` and no artifact is bound, tests that touch the FFI **fail
fast** with a clear load error (they do not silently skip ABI checks). Pure
Julia struct layout tests in `test/runtests.jl` still run.

## CI

Workflow `.github/workflows/ci_julia.yml` runs when Julia is available on the
runner: builds `libreadcon_core` with `chemfiles`, exports `READCON_CORE_LIB`,
then `Pkg.test()`. Agents without Julia should treat missing `julia` as an
environment limit, not an API gap—the package sources and tests remain in-tree.
