#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="\${1:-1.100.0}"
BLOCKS="\${2:-4096}"
ROOT="$(git rev-parse --show-toplevel)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

die() { echo "ERROR: $*" >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || die "sha256sum required"
command -v cmp >/dev/null 2>&1 || die "cmp required"
rustup run "$TOOLCHAIN" rustc -Vv > "$WORK/rustc-version.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$WORK/cargo-version.txt"
[[ "$(head -n1 "$WORK/rustc-version.txt")" == "rustc $TOOLCHAIN "* ]] || die "rustc version mismatch"
[[ "$(head -n1 "$WORK/cargo-version.txt")" == "cargo $TOOLCHAIN "* ]] || die "Cargo version mismatch"

rustup run "$TOOLCHAIN" cargo fetch --locked
for mode in raw event; do
  rustup run "$TOOLCHAIN" cargo run --locked --offline --release \
    --example event_rng_stat_stream -- "$mode" "$BLOCKS" \
    > "$WORK/$mode-1.bin" 2> "$WORK/$mode-1.meta"
  rustup run "$TOOLCHAIN" cargo run --locked --offline --release \
    --example event_rng_stat_stream -- "$mode" "$BLOCKS" \
    > "$WORK/$mode-2.bin" 2> "$WORK/$mode-2.meta"

  expected_bytes="$((BLOCKS * 32))"
  actual_bytes="$(stat -c '%s' "$WORK/$mode-1.bin")"
  [[ "$actual_bytes" == "$expected_bytes" ]] || die "$mode stream byte count mismatch"
  cmp -s "$WORK/$mode-1.bin" "$WORK/$mode-2.bin" || die "$mode stream is not repeatable"
  grep -F "serialization=little_endian_u64" "$WORK/$mode-1.meta" >/dev/null
  printf '%s_blocks=%s\n' "$mode" "$BLOCKS"
  printf '%s_bytes=%s\n' "$mode" "$actual_bytes"
  printf '%s_sha256=%s\n' "$mode" "$(sha256sum "$WORK/$mode-1.bin" | awk '{print $1}')"
  printf '%s_repeatability=PASS\n' "$mode"
done

if cmp -s "$WORK/raw-1.bin" "$WORK/event-1.bin"; then
  die "raw and event-addressed schedules unexpectedly produced identical streams"
fi

printf 'stream_toolchain=%s\n' "$TOOLCHAIN"
printf 'rustc=%s\n' "$(head -n1 "$WORK/rustc-version.txt")"
printf 'cargo=%s\n' "$(head -n1 "$WORK/cargo-version.txt")"
printf 'stream_integrity_status=PASS\n'
printf 'note=deterministic_stream_smoke_not_a_statistical_battery\n'
