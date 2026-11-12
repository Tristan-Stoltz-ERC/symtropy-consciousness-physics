#!/usr/bin/env bash
set -Eeuo pipefail

# Exact release-candidate guard: never let the mutable beta channel silently
# advance to a different compiler while this benchmark is labelled 1.100.0.
EXPECTED_PREFIX="rustc 1.100.0 "
RUSTC_VERSION="$(rustup run beta rustc --version)"
[[ "$RUSTC_VERSION" == "$EXPECTED_PREFIX"* ]] || {
  echo "Rust 1.100.0 beta is not the current beta toolchain: $RUSTC_VERSION" >&2
  exit 2
}

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
TOOLCHAINS="1.99.1 beta" \
  exec bash benchmarks/rust-toolchain/run-paired.sh
