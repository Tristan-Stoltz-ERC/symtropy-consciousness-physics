#!/usr/bin/env bash
set -Eeuo pipefail

# Exact stable toolchain guard: do not substitute a beta, nightly or later patch.
BASE_RUSTC="$(rustup run 1.99.1 rustc --version)"
BASE_CARGO="$(rustup run 1.99.1 cargo --version)"
RUSTC_VERSION="$(rustup run 1.100.0 rustc --version)"
CARGO_VERSION="$(rustup run 1.100.0 cargo --version)"
[[ "$BASE_RUSTC" == "rustc 1.99.1 "* ]] || {
  echo "Rust 1.99.1 baseline is not installed/resolved: $BASE_RUSTC" >&2
  exit 2
}
[[ "$BASE_CARGO" == "cargo 1.99.1 "* ]] || {
  echo "Cargo 1.99.1 baseline does not match: $BASE_CARGO" >&2
  exit 2
}
[[ "$RUSTC_VERSION" == "rustc 1.100.0 "* ]] || {
  echo "Rust 1.100.0 stable is not installed/resolved: $RUSTC_VERSION" >&2
  exit 2
}
[[ "$CARGO_VERSION" == "cargo 1.100.0 "* ]] || {
  echo "Cargo does not match Rust 1.100.0: $CARGO_VERSION" >&2
  exit 2
}

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
TOOLCHAINS="1.99.1 1.100.0" \
  exec bash benchmarks/rust-toolchain/run-paired.sh
