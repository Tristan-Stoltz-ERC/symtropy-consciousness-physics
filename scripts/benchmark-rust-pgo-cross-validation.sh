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
command -v llvm-profdata >/dev/null 2>&1 || true
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
  echo "schema=luminous.symtropy.instrumentation-pgo-cross-validation.v0.1"
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
} > "$OUT/metadata.txt"

printf 'mode,fold,example,repeat,elapsed_ms,max_rss_kib,artifact_bytes,stdout_sha256\n' > "$CSV"

export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=false
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_ENCODED_RUSTFLAGS || true

# Warm dependencies before the measured builds, then use offline resolution.
rustup run "$TOOLCHAIN" cargo fetch --locked
export CARGO_NET_OFFLINE=true

measure_build() {
  local mode="$1" fold="$2" example="$3" target="$4" flags="$5"
  local log="$OUT/logs/$mode-$fold-$example-build.log"
  local timefile="$OUT/logs/$mode-$fold-$example-build.time"
  local start end ms rss bytes
  start="$(date +%s%N)"
  if [[ -n "$flags" ]]; then
    RUSTFLAGS="$flags" /usr/bin/time -f 'max_rss_kib=%M' -o "$timefile" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --example "$example" --target-dir "$target" > "$log" 2>&1
  else
    /usr/bin/time -f 'max_rss_kib=%M' -o "$timefile" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --example "$example" --target-dir "$target" > "$log" 2>&1
  fi
  end="$(date +%s%N)"
  ms=$(((end-start)/1000000))
  rss="$(awk -F= '$1=="max_rss_kib" {print $2}' "$timefile")"
  bytes="$(stat -c %s "$target/release/examples/$example")"
  printf 'build mode=%s fold=%s example=%s elapsed_ms=%s max_rss_kib=%s artifact_bytes=%s\n' \
    "$mode" "$fold" "$example" "$ms" "$rss" "$bytes" | tee -a "$OUT/results/builds.txt"
}

run_repeats() {
  local mode="$1" fold="$2" example="$3" binary="$4"
  local repeat start end elapsed sha stdout stderr
  [[ -x "$binary" ]] || die "missing executable: $binary"
  for ((repeat=1; repeat<=REPEATS; repeat++)); do
    stdout="$OUT/logs/$mode-$fold-$example-run-$repeat.stdout"
    stderr="$OUT/logs/$mode-$fold-$example-run-$repeat.stderr"
    start="$(date +%s%N)"
    "$binary" > "$stdout" 2> "$stderr"
    end="$(date +%s%N)"
    elapsed=$(((end-start)/1000000))
    sha="$(sha256sum "$stdout" | awk '{print $1}')"
    printf '%s,%s,%s,%s,%s,NA,%s,%s\n' \
      "$mode" "$fold" "$example" "$repeat" "$elapsed" \
      "$(stat -c %s "$binary")" "$sha" >> "$CSV"
  done
}

# One unprofiled release build per example; those binaries provide the matched
# baseline runtime for each held-out fold.
BASE_TARGET="$OUT/targets/baseline"
for example in "${EXAMPLES[@]}"; do
  measure_build baseline none "$example" "$BASE_TARGET" ""
  run_repeats baseline none "$example" "$BASE_TARGET/release/examples/$example"
done

# Three-fold cross validation. Each fold trains an instrumentation profile on
# two workloads and evaluates the third workload only after rebuilding with
# that profile. No held-out workload contributes to its own fold's profile.
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

  # Instrument all three example artifacts, but execute only the two declared
  # training workloads while collecting the profile.
  for example in "${EXAMPLES[@]}"; do
    measure_build instrumented "$fold" "$example" "$profile_target" "-Cprofile-generate=$profile_dir"
  done
  for example in "${train[@]}"; do
    LLVM_PROFILE_FILE="$profile_dir/%m_%p.profraw" \
      "$profile_target/release/examples/$example" \
      > "$OUT/instrumented/$fold-$example.stdout" \
      2> "$OUT/instrumented/$fold-$example.stderr"
  done

  shopt -s nullglob
  raw_profiles=("$profile_dir"/*.profraw)
  shopt -u nullglob
  [[ "${#raw_profiles[@]}" -gt 0 ]] || die "no raw profiles generated for $fold"
  profdata="$OUT/profiles/$fold.profdata"
  "$LLVM_PROFDATA" merge -o "$profdata" "${raw_profiles[@]}"
  "$LLVM_PROFDATA" show --counts "$profdata" > "$OUT/profiles/$fold-profile-summary.txt"

  measure_build pgo "$fold" "$heldout" "$pgo_target" \
    "-Cprofile-use=$profdata -Cllvm-args=-pgo-warn-missing-function"
  run_repeats pgo "$fold" "$heldout" "$pgo_target/release/examples/$heldout"

  # PGO and baseline must preserve the exact deterministic stdout for the same
  # held-out workload. If not, fail rather than trading scientific semantics
  # for a faster binary.
  base_digest="$(awk -F, -v e="$heldout" '$1=="baseline" && $3==e && $4==1 {print $8}' "$CSV")"
  pgo_digests="$(awk -F, -v f="$fold" '$1=="pgo" && $2==f {print $8}' "$CSV" | sort -u)"
  [[ -n "$base_digest" && "$pgo_digests" == "$base_digest" ]] || {
    echo "Scientific output digest changed for $fold." >&2
    echo "baseline=$base_digest" >&2
    echo "pgo=$pgo_digests" >&2
    exit 1
  }
  echo "heldout_output_identity_$fold=PASS" | tee -a "$OUT/results/claim-gates.txt"
done

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
  echo "- Repeats per measured condition: $REPEATS"
  echo "- Output identity gate: PASS for all folds"
  echo "- Claim ceiling: exploratory held-out runtime measurements only; not p99 and not a claim that PGO beats SOTA."
} | tee "$OUT/results/report.txt" | tee -a "$GITHUB_STEP_SUMMARY"

printf 'pgo_result_dir=%s\n' "$OUT"
