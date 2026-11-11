#!/bin/bash
# Branch coverage on a date-pinned nightly, next to the pinned toolchain.
# The pin is Cargo.toml rust-version. This script does not add a toolchain file.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
cd "$root"
pin="$HOME/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin"
night="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin"
echo "pin rustc -V"
"$pin/rustc" -V
echo "pin cargo -V"
"$pin/cargo" -V
echo "night rustc -V"
"$night/rustc" -V
echo "night cargo -V"
"$night/cargo" -V
export PATH="$night:$PATH"
export CARGO_TARGET_DIR="$root/target/cov-branch"
# Date pin: the nightly installed on this machine, printed above.
cargo llvm-cov --branch --lib --features mpi,python -- --test-threads=1
