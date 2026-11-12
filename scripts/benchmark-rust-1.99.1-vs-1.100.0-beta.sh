#!/usr/bin/env bash
set -Eeuo pipefail

# Exact release-candidate guard: never let the mutable beta channel silently
# advance to a different compiler while this benchmark is labelled 1.100.0.
RUSTC_VERSION="$(rustup run beta rustc --version)"
CARGO_VERSION="$(rustup run beta cargo --version)"
[[ "$RUSTC_VERSION" =~ ^rustc\ 1\.100\.0(-beta(\.[0-9]+)?)?\  ]] || {
  echo "Rust beta is not the 1.100.0 release candidate: $RUSTC_VERSION" >&2
  exit 2
}
[[ "$CARGO_VERSION" =~ ^cargo\ 1\.100\.0(-beta(\.[0-9]+)?)?\  ]] || {
  echo "Cargo beta does not match Rust 1.100.0: $CARGO_VERSION" >&2
  exit 2
}

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
TOOLCHAINS="1.99.1 beta" \
  exec bash benchmarks/rust-toolchain/run-paired.sh
