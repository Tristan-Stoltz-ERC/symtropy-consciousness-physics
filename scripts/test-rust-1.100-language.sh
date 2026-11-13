#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.100.0}"
EXPECTED_VERSION="${2:-$TOOLCHAIN}"
ROOT="$(git rev-parse --show-toplevel)"
SOURCE="$ROOT/tests/toolchain/rust-1.100-language/never_type.rs"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $EXPECTED_VERSION "* ]] || {
  echo "expected Rust $EXPECTED_VERSION, got: $RUSTC_VERSION" >&2
  exit 2
}
[[ "$CARGO_VERSION" == "cargo $EXPECTED_VERSION "* ]] || {
  echo "expected Cargo $EXPECTED_VERSION, got: $CARGO_VERSION" >&2
  exit 2
}

for edition in 2018 2021 2024; do
  mkdir -p "$WORK/$edition"
  rustup run "$TOOLCHAIN" rustc \
    --edition="$edition" \
    --crate-type=lib \
    --deny=warnings \
    "$SOURCE" \
    --out-dir "$WORK/$edition"
  test -s "$WORK/$edition/libnever_type.rlib"
  echo "never_type_edition_$edition=PASS"
done

echo "never_type_stabilization=PASS"
