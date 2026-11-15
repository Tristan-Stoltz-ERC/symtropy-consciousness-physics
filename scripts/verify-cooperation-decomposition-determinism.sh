#!/usr/bin/env bash
set -Eeuo pipefail

# Rebuild the historical Cooperation Decomposition workload with the opt-in
# deterministic reduction path, then compare independent process executions.
# No stdout normalization is applied: any byte difference is a hard failure.

REPEATS="$(printenv REPEATS || echo 3)"
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
printf 'source_commit=%s\n' "$(git rev-parse HEAD)" > "$OUT/metadata.txt"
printf 'feature=deterministic-reductions\n' >> "$OUT/metadata.txt"
printf 'seed_start_index=%s\nseed_count=%s\nrepeats=%s\n' \
  "$SEED_START_INDEX" "$SEED_COUNT" "$REPEATS" >> "$OUT/metadata.txt"

cargo build --release --locked --features deterministic-reductions \
  --example cooperation_decomposition
BINARY="$ROOT/target/release/examples/cooperation_decomposition"
[[ -x "$BINARY" ]] || die "built Cooperation Decomposition executable is missing"
BINARY_SHA256="$(sha256sum "$BINARY" | awk '{print $1}')"
printf 'binary_sha256=%s\n' "$BINARY_SHA256" >> "$OUT/metadata.txt"

for ((run=1; run<=REPEATS; run++)); do
  stdout="$OUT/run-$run.stdout"
  stderr="$OUT/run-$run.stderr"
  env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
    SYMTROPY_SEED_START_INDEX="$SEED_START_INDEX" \
    SYMTROPY_SEED_COUNT="$SEED_COUNT" \
    "$BINARY" > "$stdout" 2> "$stderr"

  stdout_hash="$(sha256sum "$stdout" | awk '{print $1}')"
  stderr_hash="$(sha256sum "$stderr" | awk '{print $1}')"
  printf 'run=%s stdout_sha256=%s stderr_sha256=%s\n' \
    "$run" "$stdout_hash" "$stderr_hash"

  if (( run > 1 )); then
    cmp -s "$OUT/run-1.stdout" "$stdout" \
      || die "stdout differs between independent processes 1 and $run"
    cmp -s "$OUT/run-1.stderr" "$stderr" \
      || die "stderr differs between independent processes 1 and $run"
  fi
done

printf 'determinism_status=PASS\n'
printf 'process_runs=%s\nseed_indices=[%s,%s)\n' \
  "$REPEATS" "$SEED_START_INDEX" "$((SEED_START_INDEX + SEED_COUNT))"
