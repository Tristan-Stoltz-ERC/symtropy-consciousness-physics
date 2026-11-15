#!/usr/bin/env bash
set -Eeuo pipefail

# Paired runtime and peak-RSS comparison for the historical and deterministic
# Cooperation Decomposition binaries. The workload seed partition and exact
# source/toolchain are held fixed; run order alternates to reduce thermal bias.
# No stdout normalization is permitted.

TRIALS="$(printenv TRIALS || echo 30)"
SEED_START_INDEX="$(printenv SEED_START_INDEX || echo 0)"
SEED_COUNT="$(printenv SEED_COUNT || echo 20)"
KEEP_RESULTS="$(printenv KEEP_RESULTS || echo 0)"
RESULT_PARENT="$(printenv RESULT_PARENT || printenv RUNNER_TEMP || printenv TMPDIR || echo /tmp)"
TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 2
}

[[ "$TRIALS" =~ ^([2-9]|[1-9][0-9]+)$ ]] || die "TRIALS must be an integer >= 2"
[[ "$SEED_START_INDEX" =~ ^[0-9]+$ ]] || die "SEED_START_INDEX must be a nonnegative integer"
[[ "$SEED_COUNT" =~ ^[1-9][0-9]*$ ]] || die "SEED_COUNT must be a positive integer"
[[ "$KEEP_RESULTS" == "0" || "$KEEP_RESULTS" == "1" ]] || die "KEEP_RESULTS must be 0 or 1"
[[ "$TOOLCHAIN" == "1.100.0" ]] || die "this paired overhead protocol is qualified only for Rust 1.100.0 stable"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
for tool in rustup sha256sum cmp awk date stat du; do
  command -v "$tool" >/dev/null 2>&1 || die "required command missing: $tool"
done
[[ -x /usr/bin/time ]] || die "/usr/bin/time is required for peak RSS measurement"

OUT="$(mktemp -d "$RESULT_PARENT/deterministic-reductions-overhead.XXXXXX")"
cleanup() {
  if [[ "$KEEP_RESULTS" == "1" ]]; then
    printf 'evidence_directory=%s\n' "$OUT"
  else
    rm -rf "$OUT"
  fi
}
trap cleanup EXIT

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $TOOLCHAIN "* ]] || die "unexpected rustc: $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo $TOOLCHAIN "* ]] || die "unexpected Cargo: $CARGO_VERSION"

rustup run "$TOOLCHAIN" rustc -Vv > "$OUT/rustc-version.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$OUT/cargo-version.txt"
git rev-parse HEAD > "$OUT/source-commit.txt"
sha256sum Cargo.lock src/coupling.rs examples/cooperation_decomposition.rs > "$OUT/input-sha256.txt"

{
  echo "schema=luminous.symtropy.deterministic-reductions-overhead.v0.1"
  echo "toolchain=$TOOLCHAIN"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "source_commit=$(git rev-parse HEAD)"
  echo "seed_start_index=$SEED_START_INDEX"
  echo "seed_count=$SEED_COUNT"
  echo "historical_seed_formula=42+997i"
  echo "paired_trials=$TRIALS"
  echo "warmup_runs_per_mode=2"
  echo "runtime_metric=wall_clock_nanoseconds"
  echo "memory_metric=maximum_resident_set_kib"
  echo "run_order=alternating_by_trial"
  echo "cache_policy=separate_fresh_build_targets; warmed_registry"
  echo "cargo_incremental=0"
  echo "sccache=disabled"
  echo "normalization=none"
  echo "cpu=$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)"
  echo "cpu_count=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
  echo "memory=$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)"
  echo "cpu_governor=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)"
  echo "kernel=$(uname -srmo)"
  echo "filesystem=$(stat -f -c %T "$OUT" 2>/dev/null || echo unknown)"
} > "$OUT/metadata.txt"

# Warm the exact locked dependency graph before switching to offline builds.
rustup run "$TOOLCHAIN" cargo fetch --locked

# Build both configurations into isolated target roots. Build time is stored
# separately from runtime timing and never mixed into runtime overhead.
start_ns="$(date +%s%N)"
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true \
  CARGO_TARGET_DIR="$OUT/target-legacy" \
  rustup run "$TOOLCHAIN" cargo build --release --locked --example cooperation_decomposition \
  > "$OUT/build-legacy.log" 2>&1
end_ns="$(date +%s%N)"
legacy_build_ms="$(((end_ns - start_ns) / 1000000))"

start_ns="$(date +%s%N)"
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true \
  CARGO_TARGET_DIR="$OUT/target-deterministic" \
  rustup run "$TOOLCHAIN" cargo build --release --locked --features deterministic-reductions \
    --example cooperation_decomposition \
  > "$OUT/build-deterministic.log" 2>&1
end_ns="$(date +%s%N)"
deterministic_build_ms="$(((end_ns - start_ns) / 1000000))"

LEGACY_BINARY="$OUT/cooperation-decomposition-legacy"
DETERMINISTIC_BINARY="$OUT/cooperation-decomposition-deterministic"
cp "$OUT/target-legacy/release/examples/cooperation_decomposition" "$LEGACY_BINARY"
cp "$OUT/target-deterministic/release/examples/cooperation_decomposition" "$DETERMINISTIC_BINARY"
chmod 0555 "$LEGACY_BINARY" "$DETERMINISTIC_BINARY"

legacy_binary_sha="$(sha256sum "$LEGACY_BINARY" | awk '{print $1}')"
deterministic_binary_sha="$(sha256sum "$DETERMINISTIC_BINARY" | awk '{print $1}')"
{
  echo "legacy_build_ms=$legacy_build_ms"
  echo "deterministic_build_ms=$deterministic_build_ms"
  echo "legacy_binary_sha256=$legacy_binary_sha"
  echo "deterministic_binary_sha256=$deterministic_binary_sha"
  echo "legacy_binary_bytes=$(stat -c %s "$LEGACY_BINARY")"
  echo "deterministic_binary_bytes=$(stat -c %s "$DETERMINISTIC_BINARY")"
} >> "$OUT/metadata.txt"

mkdir -p "$OUT/warmup" "$OUT/runs"
CSV="$OUT/measurements.csv"
printf 'trial,first_mode,legacy_ms,deterministic_ms,legacy_rss_kib,deterministic_rss_kib,legacy_stdout_sha256,deterministic_stdout_sha256,legacy_stderr_sha256,deterministic_stderr_sha256\n' > "$CSV"

run_one() {
  local mode="$1"
  local trial="$2"
  local phase="$3"
  local binary="$LEGACY_BINARY"
  [[ "$mode" == "legacy" ]] || binary="$DETERMINISTIC_BINARY"

  local prefix="$OUT/$phase/$mode-$trial"
  local start_ns end_ns runtime_ms rss_kib
  start_ns="$(date +%s%N)"
  /usr/bin/time -f '%M' -o "$prefix.rss-kib" \
    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
      SYMTROPY_SEED_START_INDEX="$SEED_START_INDEX" \
      SYMTROPY_SEED_COUNT="$SEED_COUNT" \
      "$binary" > "$prefix.stdout" 2> "$prefix.stderr"
  end_ns="$(date +%s%N)"
  runtime_ms="$(((end_ns - start_ns) / 1000000))"
  rss_kib="$(cat "$prefix.rss-kib")"
  [[ "$runtime_ms" =~ ^[0-9]+$ && "$rss_kib" =~ ^[0-9]+$ ]] || die "invalid timing or RSS for $mode trial $trial"

  local stdout_sha stderr_sha
  stdout_sha="$(sha256sum "$prefix.stdout" | awk '{print $1}')"
  stderr_sha="$(sha256sum "$prefix.stderr" | awk '{print $1}')"
  printf '%s\t%s\t%s\t%s\t%s\n' "$mode" "$trial" "$runtime_ms" "$rss_kib" "$stdout_sha"
  printf '%s\n' "$stderr_sha" > "$prefix.stderr.sha256"
}

for warm in 1 2; do
  run_one legacy "$warm" warmup > "$OUT/warmup/legacy-$warm.summary"
  run_one deterministic "$warm" warmup > "$OUT/warmup/deterministic-$warm.summary"
done

for ((trial=1; trial<=TRIALS; trial++)); do
  legacy_summary="$OUT/runs/legacy-$trial.summary"
  deterministic_summary="$OUT/runs/deterministic-$trial.summary"

  if (( trial % 2 == 1 )); then
    first_mode="legacy"
    run_one legacy "$trial" runs > "$legacy_summary"
    run_one deterministic "$trial" runs > "$deterministic_summary"
  else
    first_mode="deterministic"
    run_one deterministic "$trial" runs > "$deterministic_summary"
    run_one legacy "$trial" runs > "$legacy_summary"
  fi

  IFS=$'\t' read -r _ _ legacy_ms legacy_rss legacy_stdout < "$legacy_summary"
  IFS=$'\t' read -r _ _ deterministic_ms deterministic_rss deterministic_stdout < "$deterministic_summary"
  legacy_stderr="$(cat "$OUT/runs/legacy-$trial.stderr.sha256")"
  deterministic_stderr="$(cat "$OUT/runs/deterministic-$trial.stderr.sha256")"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$trial" "$first_mode" "$legacy_ms" "$deterministic_ms" \
    "$legacy_rss" "$deterministic_rss" "$legacy_stdout" \
    "$deterministic_stdout" "$legacy_stderr" "$deterministic_stderr" >> "$CSV"
  printf 'paired_trial=%s first=%s legacy_ms=%s deterministic_ms=%s legacy_rss_kib=%s deterministic_rss_kib=%s\n' \
    "$trial" "$first_mode" "$legacy_ms" "$deterministic_ms" "$legacy_rss" "$deterministic_rss"
done

git diff --exit-code -- Cargo.lock
[[ -z "$(git status --porcelain -- Cargo.lock)" ]] || die "Cargo.lock changed during measurement"
printf 'measurement_status=COMPLETE\n' > "$OUT/STATUS.txt"
printf 'measurements_csv=%s\n' "$CSV"
printf 'evidence_directory=%s\n' "$OUT"
