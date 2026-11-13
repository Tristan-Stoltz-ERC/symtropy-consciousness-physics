#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.100.0}"
OUT_ROOT="${2:-benchmark-results/rust-pgo}"
REPEATS="${PGO_RUNTIME_REPEATS:-10}"
EXAMPLES=(gradient_replacement cooperation_decomposition jphi_convergence)

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ "$TOOLCHAIN" == "1.100.0" ]] || die "This protocol is pinned to Rust 1.100.0; got $TOOLCHAIN"
[[ "$REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUNTIME_REPEATS must be a positive integer"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
[[ -x /usr/bin/time ]] || die "/usr/bin/time is required for peak-RSS measurements"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || die "worktree must be clean before PGO qualification"

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc 1.100.0 "* ]] || die "unexpected rustc: $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo 1.100.0 "* ]] || die "unexpected Cargo: $CARGO_VERSION"

HOST="$(rustup run "$TOOLCHAIN" rustc -vV | sed -n 's/^host: //p')"
SYSROOT="$(rustup run "$TOOLCHAIN" rustc --print sysroot)"
LLVM_PROFDATA="$SYSROOT/lib/rustlib/$HOST/bin/llvm-profdata"
[[ -x "$LLVM_PROFDATA" ]] || die "llvm-profdata not found at $LLVM_PROFDATA; install llvm-tools-preview for $TOOLCHAIN"

LOCK_SHA="$(sha256sum Cargo.lock | awk '{print $1}')"
SOURCE_SHA="$(git rev-parse HEAD)"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$OUT_ROOT/run-$RUN_ID"
mkdir -p "$OUT"/{baseline,instrumented,pgo,profiles,logs,results,targets,toolchains}
CSV="$OUT/results/measurements.csv"

rustup run "$TOOLCHAIN" rustc -Vv > "$OUT/toolchains/rustc.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$OUT/toolchains/cargo.txt"
"$LLVM_PROFDATA" --version > "$OUT/toolchains/llvm-profdata.txt"

{
  echo "schema=luminous.symtropy.instrumentation-pgo-cross-validation.v0.2"
  echo "source_commit=$SOURCE_SHA"
  echo "cargo_lock_sha256=$LOCK_SHA"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "host=$HOST"
  echo "kernel=$(uname -srmo)"
  echo "cpu_model=$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)"
  echo "cpu_count=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
  echo "memory=$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)"
  echo "filesystem=$(stat -f -c %T "$OUT" 2>/dev/null || echo unknown)"
  echo "cpu_governor=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)"
  echo "runtime_repeats_per_evaluation=$REPEATS"
  echo "examples=${EXAMPLES[*]}"
  echo "profile_policy=three-fold leave-one-example-out"
  echo "profile_mode=instrumentation"
  echo "sample_pgo=not run; separate perf/hardware-counter study"
  echo "sccache=disabled"
  echo "cargo_incremental=0"
  echo "dependency_resolution=locked"
  echo "runtime_order=alternating baseline/pgo within each trial"
} > "$OUT/metadata.txt"

printf 'mode,fold,example,repeat,elapsed_ms,max_rss_kib,artifact_bytes,stdout_sha256\n' > "$CSV"
printf 'mode,fold,example,build_ms,max_rss_kib,artifact_bytes\n' > "$OUT/results/builds.csv"

export CARGO_INCREMENTAL=0
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_ENCODED_RUSTFLAGS RUSTFLAGS || true

# Warm dependencies before measured builds; all subsequent Cargo commands run
# offline and locked. The warm-up itself is not a performance observation.
rustup run "$TOOLCHAIN" cargo fetch --locked
export CARGO_NET_OFFLINE=true

measure_build() {
  local mode="$1" fold="$2" example="$3" target="$4" flags="$5"
  local log="$OUT/logs/$mode-$fold-$example-build.log"
  local timefile="$OUT/logs/$mode-$fold-$example-build.time"
  local start end ms rss bytes
  start="$(date +%s%N)"
  if [[ -n "$flags" ]]; then
    env RUSTFLAGS="$flags" /usr/bin/time -f 'max_rss_kib=%M' -o "$timefile" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --example "$example" --target-dir "$target" > "$log" 2>&1
  else
    env -u RUSTFLAGS /usr/bin/time -f 'max_rss_kib=%M' -o "$timefile" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --example "$example" --target-dir "$target" > "$log" 2>&1
  fi
  end="$(date +%s%N)"
  ms=$(((end-start)/1000000))
  rss="$(awk -F= '$1=="max_rss_kib" {print $2}' "$timefile")"
  bytes="$(stat -c %s "$target/release/examples/$example")"
  printf '%s,%s,%s,%s,%s,%s\n' "$mode" "$fold" "$example" "$ms" "$rss" "$bytes" >> "$OUT/results/builds.csv"
  printf 'build mode=%s fold=%s example=%s elapsed_ms=%s max_rss_kib=%s artifact_bytes=%s\n' \
    "$mode" "$fold" "$example" "$ms" "$rss" "$bytes" | tee -a "$OUT/results/builds.txt"
}

run_once() {
  local mode="$1" fold="$2" example="$3" repeat="$4" binary="$5"
  local stdout="$OUT/logs/$mode-$fold-$example-run-$repeat.stdout"
  local stderr="$OUT/logs/$mode-$fold-$example-run-$repeat.stderr"
  local timefile="$OUT/logs/$mode-$fold-$example-run-$repeat.time"
  local start end elapsed rss digest bytes
  [[ -x "$binary" ]] || die "missing executable: $binary"
  start="$(date +%s%N)"
  /usr/bin/time -f 'max_rss_kib=%M' -o "$timefile" "$binary" > "$stdout" 2> "$stderr"
  end="$(date +%s%N)"
  elapsed=$(((end-start)/1000000))
  rss="$(awk -F= '$1=="max_rss_kib" {print $2}' "$timefile")"
  bytes="$(stat -c %s "$binary")"
  digest="$(sha256sum "$stdout" | awk '{print $1}')"
  printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$mode" "$fold" "$example" "$repeat" "$elapsed" "$rss" "$bytes" "$digest" >> "$CSV"
  printf '%s\n' "$digest"
}

run_paired_repeats() {
  local fold="$1" example="$2" baseline_binary="$3" pgo_binary="$4"
  local repeat base_digest pgo_digest
  for ((repeat=1; repeat<=REPEATS; repeat++)); do
    # Rotate the first binary each trial to reduce order/thermal bias.
    if (( repeat % 2 == 1 )); then
      base_digest="$(run_once baseline "$fold" "$example" "$repeat" "$baseline_binary")"
      pgo_digest="$(run_once pgo "$fold" "$example" "$repeat" "$pgo_binary")"
    else
      pgo_digest="$(run_once pgo "$fold" "$example" "$repeat" "$pgo_binary")"
      base_digest="$(run_once baseline "$fold" "$example" "$repeat" "$baseline_binary")"
    fi
    [[ "$base_digest" == "$pgo_digest" ]] || {
      echo "Scientific output digest changed for fold=$fold example=$example repeat=$repeat" >&2
      echo "baseline=$base_digest" >&2
      echo "pgo=$pgo_digest" >&2
      exit 1
    }
  done
}

median() {
  sort -n | awk '{ values[NR]=$1 } END {
    if (NR == 0) exit 1;
    if (NR % 2) printf "%.6f", values[(NR+1)/2];
    else printf "%.6f", (values[NR/2] + values[NR/2+1]) / 2;
  }'
}

# Build one unprofiled release binary for each example. Runtime observations
# will be interleaved with each fold's held-out PGO executable.
BASE_TARGET="$OUT/targets/baseline"
for example in "${EXAMPLES[@]}"; do
  measure_build baseline none "$example" "$BASE_TARGET" ""
done

# Three-fold leave-one-example-out PGO. Each profile uses the other two examples.
for heldout in "${EXAMPLES[@]}"; do
  fold="heldout-$heldout"
  profile_dir="$OUT/profiles/$fold"
  profile_target="$OUT/targets/instrumented-$fold"
  pgo_target="$OUT/targets/pgo-$fold"
  mkdir -p "$profile_dir"

  train=()
  for example in "${EXAMPLES[@]}"; do
    [[ "$example" == "$heldout" ]] || train+=("$example")
  done
  [[ "${#train[@]}" -eq 2 ]] || die "expected two training examples for $fold"

  # Instrument all example binaries, but execute only the two declared
  # training workloads while collecting the profile.
  for example in "${EXAMPLES[@]}"; do
    measure_build instrumented "$fold" "$example" "$profile_target" "-Cprofile-generate=$profile_dir"
  done
  for example in "${train[@]}"; do
    TRAIN_STDOUT="$OUT/instrumented/$fold-$example.stdout"
    TRAIN_STDERR="$OUT/instrumented/$fold-$example.stderr"
    set +e
    LLVM_PROFILE_FILE="$profile_dir/%m_%p.profraw" \
      "$profile_target/release/examples/$example" \
      > "$TRAIN_STDOUT" 2> "$TRAIN_STDERR"
    status=$?
    set -e
    if (( status != 0 )); then
      echo "PGO profile-training workload failed: fold=$fold example=$example status=$status" >&2
      echo "--- workload stderr ---" >&2
      tail -n 160 "$TRAIN_STDERR" >&2 || true
      echo "--- workload stdout ---" >&2
      tail -n 80 "$TRAIN_STDOUT" >&2 || true
      exit "$status"
    fi
  done

  shopt -s nullglob
  raw_profiles=("$profile_dir"/*.profraw)
  shopt -u nullglob
  [[ "${#raw_profiles[@]}" -gt 0 ]] || die "no raw profiles generated for $fold"
  profdata="$OUT/profiles/$fold.profdata"
  if ! "$LLVM_PROFDATA" merge -o "$profdata" "${raw_profiles[@]}" \
      > "$OUT/profiles/$fold-merge.stdout" \
      2> "$OUT/profiles/$fold-merge.stderr"; then
    echo "llvm-profdata merge failed for $fold" >&2
    tail -n 160 "$OUT/profiles/$fold-merge.stderr" >&2 || true
    exit 1
  fi
  if ! "$LLVM_PROFDATA" show --counts "$profdata" \
      > "$OUT/profiles/$fold-profile-summary.txt" \
      2> "$OUT/profiles/$fold-show.stderr"; then
    echo "llvm-profdata show failed for $fold" >&2
    tail -n 160 "$OUT/profiles/$fold-show.stderr" >&2 || true
    exit 1
  fi

  measure_build pgo "$fold" "$heldout" "$pgo_target" \
    "-Cprofile-use=$profdata -Cllvm-args=-pgo-warn-missing-function"
  run_paired_repeats "$fold" "$heldout" \
    "$BASE_TARGET/release/examples/$heldout" \
    "$pgo_target/release/examples/$heldout"

  base_digests="$(awk -F, -v f="$fold" '$1=="baseline" && $2==f {print $8}' "$CSV" | sort -u)"
  pgo_digests="$(awk -F, -v f="$fold" '$1=="pgo" && $2==f {print $8}' "$CSV" | sort -u)"
  [[ -n "$base_digests" && "$base_digests" == "$pgo_digests" ]] || die "repeated output digests are not identical for $fold"
  echo "heldout_output_identity_$fold=PASS" | tee -a "$OUT/results/claim-gates.txt"
done

# Descriptive median paired speedup by held-out workload. A ratio > 1 means
# PGO was faster. This is exploratory evidence, not a confidence interval.
{
  echo "# Held-out PGO runtime medians"
  echo
  echo "| Held-out example | Baseline median ms | PGO median ms | Median paired speedup |"
  echo "|---|---:|---:|---:|"
  for example in "${EXAMPLES[@]}"; do
    fold="heldout-$example"
    baseline_times="$(awk -F, -v f="$fold" '$1=="baseline" && $2==f {print $5}' "$CSV")"
    pgo_times="$(awk -F, -v f="$fold" '$1=="pgo" && $2==f {print $5}' "$CSV")"
    baseline_median="$(printf '%s\n' "$baseline_times" | median)"
    pgo_median="$(printf '%s\n' "$pgo_times" | median)"
    ratios="$(awk -F, -v f="$fold" '
      $2==f && $1=="baseline" {b[$4]=$5}
      $2==f && $1=="pgo" && ($4 in b) && $5>0 {print b[$4]/$5}
    ' "$CSV")"
    ratio_median="$(printf '%s\n' "$ratios" | median)"
    printf '| %s | %s | %s | %s |\n' "$example" "$baseline_median" "$pgo_median" "$ratio_median"
  done
} | tee "$OUT/results/heldout-medians.md" | tee -a "$GITHUB_STEP_SUMMARY"

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_SHA" == "$LOCK_AFTER" ]] || die "Cargo.lock changed during PGO qualification"
git diff --exit-code -- Cargo.lock >/dev/null || die "tracked Cargo.lock changed"
echo "cargo_lock_immutable=PASS" | tee -a "$OUT/results/claim-gates.txt"
echo "pgo_validation=COMPLETE_EXPLORATORY" > "$OUT/STATUS.txt"

{
  echo "## Instrumentation PGO cross-validation"
  echo
  echo "- Exact compiler: $RUSTC_VERSION"
  echo "- Source: $SOURCE_SHA"
  echo "- Cargo.lock SHA-256: $LOCK_SHA"
  echo "- Held-out workloads: ${EXAMPLES[*]}"
  echo "- Repeats per held-out condition: $REPEATS"
  echo "- Runtime order: alternating baseline/PGO within each repeat"
  echo "- Output identity gate: PASS for all folds"
  echo "- Claim ceiling: exploratory held-out runtime measurements only; not p99 and not a claim that PGO beats SOTA."
} | tee "$OUT/results/report.txt" | tee -a "$GITHUB_STEP_SUMMARY"

printf 'pgo_result_dir=%s\n' "$OUT"
