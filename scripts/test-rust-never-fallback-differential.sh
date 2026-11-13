#!/usr/bin/env bash
set -Eeuo pipefail

BASELINE="${1:-1.99.1}"
CANDIDATE="${2:-1.100.0}"
ROOT="$(git rev-parse --show-toplevel)"
SOURCE="$ROOT/tests/toolchain/rust-1.100-language/never_fallback.rs"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

for toolchain in "$BASELINE" "$CANDIDATE"; do
  rustc_version="$(rustup run "$toolchain" rustc --version)"
  cargo_version="$(rustup run "$toolchain" cargo --version)"
  [[ "$rustc_version" == "rustc $toolchain "* ]] || {
    echo "unexpected rustc identity for $toolchain: $rustc_version" >&2
    exit 2
  }
  [[ "$cargo_version" == "cargo $toolchain "* ]] || {
    echo "unexpected Cargo identity for $toolchain: $cargo_version" >&2
    exit 2
  }
done

expect_compile() {
  local toolchain="$1" edition="$2" expected="$3"
  local log="$WORK/$toolchain-$edition.log"
  local cap_args=()
  # Rust 1.99 emits the never-fallback migration lint by default. Cap lint
  # severity only for the historical baseline compile so this probe measures
  # the inferred type; type errors remain errors. The 1.100 candidate is never
  # capped, so its !: Default failure must remain observable.
  if [[ "$toolchain" == "$BASELINE" ]]; then
    cap_args+=(--cap-lints allow)
  fi
  set +e
  rustup run "$toolchain" rustc \
    --edition="$edition" \
    --crate-type=bin \
    "${cap_args[@]}" \
    "$SOURCE" \
    -o "$WORK/probe-$toolchain-$edition" \
    > "$log" 2>&1
  local status=$?
  set -e

  if [[ "$expected" == "pass" && $status -ne 0 ]]; then
    cat "$log" >&2
    echo "FAIL: $toolchain edition $edition should compile, status=$status" >&2
    exit 1
  fi
  if [[ "$expected" == "fail" && $status -eq 0 ]]; then
    echo "FAIL: $toolchain edition $edition should reject historical unit fallback" >&2
    exit 1
  fi
  if [[ "$expected" == "fail" ]] && ! grep -Eq 'Default|default|trait bound|type annotations|cannot infer' "$log"; then
    cat "$log" >&2
    echo "FAIL: $toolchain edition $edition failed for an unrelated reason" >&2
    exit 1
  fi
  printf 'never_fallback_%s_edition_%s=%s\n' "$toolchain" "$edition" "$expected"
}

# Rust 1.99.1: old fallback remains in 2018/2021, while 2024 already uses !.
expect_compile "$BASELINE" 2018 pass
expect_compile "$BASELINE" 2021 pass
expect_compile "$BASELINE" 2024 fail

# Rust 1.100.0: fallback is ! in every edition.
expect_compile "$CANDIDATE" 2018 fail
expect_compile "$CANDIDATE" 2021 fail
expect_compile "$CANDIDATE" 2024 fail

echo "never_type_fallback_differential=PASS"
