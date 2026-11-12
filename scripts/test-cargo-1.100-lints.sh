#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-beta}"
EXPECTED_VERSION="${2:-1.100.0}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || realpath "$(dirname "$0")/..")"
MANIFEST="$ROOT/tests/toolchain/cargo-lints/Cargo.toml"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
[[ "$RUSTC_VERSION" == "rustc $EXPECTED_VERSION "* ]] || {
  echo "expected Rust $EXPECTED_VERSION via $TOOLCHAIN; got: $RUSTC_VERSION" >&2
  exit 2
}

LOG="$WORK/cargo-lint.log"
set +e
rustup run "$TOOLCHAIN" cargo check \
  --manifest-path "$MANIFEST" \
  --offline \
  --message-format=short > "$LOG" 2>&1
status=$?
set -e
cat "$LOG"
[[ $status -eq 0 ]] || exit "$status"

if ! grep -Eiq 'unused[ _]dependency' "$LOG"; then
  echo "Cargo 1.100 did not report the deliberately unused path dependency." >&2
  exit 1
fi

echo "cargo_unused_dependency_lint=PASS"
