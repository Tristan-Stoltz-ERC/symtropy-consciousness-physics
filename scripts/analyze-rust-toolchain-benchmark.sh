#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${RUSTC_TOOLCHAIN:-1.99.1}"
ROOT="$(git rev-parse --show-toplevel)"
RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
case "$RUSTC_VERSION" in
  "rustc "*) ;;
  *) echo "unable to identify rustc for toolchain $TOOLCHAIN: $RUSTC_VERSION" >&2; exit 2 ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
rustup run "$TOOLCHAIN" rustc \
  --edition=2021 \
  --deny warnings \
  -C opt-level=2 \
  "$ROOT/benchmarks/rust-toolchain/analyze.rs" \
  -o "$WORK/rust-toolchain-benchmark-analyzer"

"$WORK/rust-toolchain-benchmark-analyzer" "$@"
