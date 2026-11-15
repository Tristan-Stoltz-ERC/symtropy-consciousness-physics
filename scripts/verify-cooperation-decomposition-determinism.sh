#!/usr/bin/env bash
set -Eeuo pipefail

# Compare legacy and opt-in deterministic reduction modes using fresh
# processes, an identical seed partition, and byte-exact stdout/stderr checks.
# Legacy divergence is recorded as a diagnostic, never normalized or hidden.
# The deterministic candidate must be repeatable or the script fails.

REPEATS="$(printenv REPEATS || echo 5)"
SEED_START_INDEX="$(printenv SEED_START_INDEX || echo 0)"
SEED_COUNT="$(printenv SEED_COUNT || echo 20)"
KEEP_RESULTS="$(printenv KEEP_RESULTS || echo 0)"

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 2
}

[[ "$REPEATS" =~ ^([2-9]|[1-9][0-9]+)$ ]] || die "REPEATS must be an integer >= 2"
[[ "$SEED_START_INDEX" =~ ^[0-9]+$ ]] || die "SEED_START_INDEX must be a nonnegative integer"
[[ "$SEED_COUNT" =~ ^[1-9][0-9]*$ ]] || die "SEED_COUNT must be a positive integer"
[[ "$KEEP_RESULTS" == "0" || "$KEEP_RESULTS" == "1" ]] || die "KEEP_RESULTS must be 0 or 1"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || die "cargo is required"
command -v rustc >/dev/null 2>&1 || die "rustc is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"
command -v cmp >/dev/null 2>&1 || die "cmp is required"

TMP_ROOT="$(printenv TMPDIR || echo /tmp)"
OUT="$(mktemp -d "$TMP_ROOT/cooperation-determinism.XXXXXX")"
cleanup() {
  if [[ "$KEEP_RESULTS" == "1" ]]; then
    printf 'evidence_directory=%s\n' "$OUT"
  else
    rm -rf "$OUT"
  fi
}
trap cleanup EXIT

rustc -Vv > "$OUT/toolchain.txt"
cargo -Vv >> "$OUT/toolchain.txt"
{
  printf 'source_commit=%s\n' "$(git rev-parse HEAD)"
  printf 'seed_start_index=%s\nseed_count=%s\nrepeats=%s\n' \
    "$SEED_START_INDEX" "$SEED_COUNT" "$REPEATS"
  printf 'legacy_feature=disabled\ncandidate_feature=deterministic-reductions\n'
} > "$OUT/metadata.txt"

run_mode() {
  local mode="$1"
  local binary="$2"
  local repeatable=1
  local mode_dir="$OUT/$mode"
  mkdir -p "$mode_dir"

  for ((run=1; run<=REPEATS; run++)); do
    local stdout="$mode_dir/run-$run.stdout"
    local stderr="$mode_dir/run-$run.stderr"

    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
      SYMTROPY_SEED_START_INDEX="$SEED_START_INDEX" \
      SYMTROPY_SEED_COUNT="$SEED_COUNT" \
      "$binary" > "$stdout" 2> "$stderr"

    local stdout_hash
    local stderr_hash
    stdout_hash="$(sha256sum "$stdout" | awk '{print $1}')"
    stderr_hash="$(sha256sum "$stderr" | awk '{print $1}')"
    printf 'mode=%s run=%s stdout_sha256=%s stderr_sha256=%s\n' \
      "$mode" "$run" "$stdout_hash" "$stderr_hash"

    if (( run > 1 )); then
      if ! cmp -s "$mode_dir/run-1.stdout" "$stdout"; then
        repeatable=0
      fi
      if ! cmp -s "$mode_dir/run-1.stderr" "$stderr"; then
        repeatable=0
      fi
    fi
  done

  local status="PASS"
  if [[ "$repeatable" -ne 1 ]]; then
    status="NONDETERMINISTIC"
  fi
  printf '%s_repeatability=%s\n' "$mode" "$status" | tee -a "$OUT/metadata.txt"

  if [[ "$mode" == "deterministic" && "$repeatable" -ne 1 ]]; then
    die "deterministic-reductions mode produced differing fresh-process output"
  fi
}

env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  cargo build --release --locked --example cooperation_decomposition
LEGACY_BINARY="$OUT/cooperation-decomposition-legacy"
cp "$ROOT/target/release/examples/cooperation_decomposition" "$LEGACY_BINARY"
printf 'legacy_binary_sha256=%s\n' \
  "$(sha256sum "$LEGACY_BINARY" | awk '{print $1}')" >> "$OUT/metadata.txt"
run_mode legacy "$LEGACY_BINARY"

env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  cargo build --release --locked --features deterministic-reductions \
  --example cooperation_decomposition
DETERMINISTIC_BINARY="$OUT/cooperation-decomposition-deterministic"
cp "$ROOT/target/release/examples/cooperation_decomposition" "$DETERMINISTIC_BINARY"
printf 'deterministic_binary_sha256=%s\n' \
  "$(sha256sum "$DETERMINISTIC_BINARY" | awk '{print $1}')" >> "$OUT/metadata.txt"
run_mode deterministic "$DETERMINISTIC_BINARY"

printf 'determinism_status=PASS\n'
printf 'legacy_repeatability_is_diagnostic_only=true\n'
printf 'deterministic_seed_indices=[%s,%s)\n' \
  "$SEED_START_INDEX" "$((SEED_START_INDEX + SEED_COUNT))"
