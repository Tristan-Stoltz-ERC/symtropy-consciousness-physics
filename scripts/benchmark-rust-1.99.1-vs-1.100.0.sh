#!/usr/bin/env bash
set -Eeuo pipefail

# Exact stable toolchain guard: do not substitute a beta, nightly or later patch.
RUSTC_VERSION="$(rustup run 1.100.0 rustc --version)"
CARGO_VERSION="$(rustup run 1.100.0 cargo --version)"
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
