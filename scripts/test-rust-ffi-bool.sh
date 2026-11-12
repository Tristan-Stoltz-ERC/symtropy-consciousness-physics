#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.99.1}"
EXPECTED_VERSION="${2:-$TOOLCHAIN}"
ROOT="$(git rev-parse --show-toplevel)"
RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
HOST="$(rustup run "$TOOLCHAIN" rustc -vV | sed -n 's/^host: //p')"

if [[ "$EXPECTED_VERSION" == "1.100.0" ]]; then
  [[ "$RUSTC_VERSION" =~ ^rustc\ 1\.100\.0(-beta(\.[0-9]+)?)?\  ]] || {
    echo "expected Rust 1.100.0 (optionally beta-suffixed) via $TOOLCHAIN; got: $RUSTC_VERSION" >&2
    exit 2
  }
else
  [[ "$RUSTC_VERSION" == "rustc $EXPECTED_VERSION "* ]] || {
    echo "expected Rust $EXPECTED_VERSION via $TOOLCHAIN; got: $RUSTC_VERSION" >&2
    exit 2
  }
fi

[[ "$HOST" == "x86_64-unknown-linux-gnu" ]] || {
  echo "this ABI probe requires x86_64-unknown-linux-gnu; got $HOST" >&2
  exit 2
}
command -v cc >/dev/null 2>&1 || {
  echo "a C compiler (cc) is required" >&2
  exit 2
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

rustup run "$TOOLCHAIN" rustc \
  --edition=2021 \
  --crate-name luminous_ffi_bool_probe \
  --crate-type=cdylib \
  -C panic=abort \
  "$ROOT/tests/toolchain/ffi_bool.rs" \
  -o "$WORK/libluminous_ffi_bool_probe.so"

cc -c "$ROOT/tests/toolchain/ffi_bool_raw.S" \
  -o "$WORK/ffi_bool_raw.o"

cc "$ROOT/tests/toolchain/ffi_bool.c" \
  "$WORK/ffi_bool_raw.o" \
  -L"$WORK" \
  -Wl,-rpath,"$WORK" \
  -lluminous_ffi_bool_probe \
  -o "$WORK/ffi-bool-probe"

"$WORK/ffi-bool-probe"
