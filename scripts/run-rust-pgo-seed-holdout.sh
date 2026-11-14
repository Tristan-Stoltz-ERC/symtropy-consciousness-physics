#!/usr/bin/env bash
set -Eeuo pipefail

# Same-program PGO seed holdout:
# 1) Instrument only the training seed indices.
# 2) Validate instrumentation output against a no-PGO training control.
# 3) Evaluate both builds on disjoint held-out seed indices from the same example.
# 4) Pair fresh build trials and repeated held-out executions; preserve raw evidence.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
EXAMPLE="${PGO_SEED_HOLDOUT_EXAMPLE:-jphi_convergence}"
TRAIN_INDICES_RAW="${PGO_TRAIN_SEED_INDICES:-0,1,2,3,4,5,6,7,8,9}"
HOLDOUT_INDICES_RAW="${PGO_HOLDOUT_SEED_INDICES:-10,11,12,13,14,15,16,17,18,19}"
BUILD_REPEATS="${PGO_BUILD_REPEATS:-5}"
RUNTIME_REPEATS="${PGO_RUN_REPEATS:-5}"
TARGET="${PGO_TARGET:-}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-pgo-seed-holdout}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

[[ "$BUILD_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_BUILD_REPEATS must be a positive integer"
[[ "$RUNTIME_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUN_REPEATS must be a positive integer"
(( BUILD_REPEATS <= 1000 )) || die "PGO_BUILD_REPEATS exceeds the 1000 trial safety ceiling"
(( RUNTIME_REPEATS <= 1000 )) || die "PGO_RUN_REPEATS exceeds the 1000 repeat safety ceiling"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"
[[ -f Cargo.lock ]] || die "Cargo.lock is required"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || die "PGO qualification requires a clean worktree"

case "$EXAMPLE" in
  jphi_convergence|gradient_replacement|cooperation_decomposition) ;;
  *) die "unsupported same-program PGO example: $EXAMPLE" ;;
esac

validate_seed_list() {
  local label="$1"
  local raw="$2"
  local -a values
  local item joined=""
  declare -A seen=()
  [[ -n "$raw" ]] || die "$label seed-index list must not be empty"
  IFS=',' read -r -a values <<< "$raw"
  (( ${#values[@]} > 0 )) || die "$label seed-index list must not be empty"
  (( ${#values[@]} <= 10000 )) || die "$label seed-index list exceeds 10000 entries"
  for item in "${values[@]}"; do
    [[ "$item" =~ ^(0|[1-9][0-9]*)$ ]] || die "$label contains non-canonical seed index '$item'"
    [[ -z "${seen[$item]+present}" ]] || die "$label contains duplicate seed index '$item'"
    seen["$item"]=1
    if [[ -n "$joined" ]]; then joined+=",";
    fi
    joined+="$item"
  done
  printf '%s' "$joined"
}

TRAIN_INDICES="$(validate_seed_list training "$TRAIN_INDICES_RAW")"
HOLDOUT_INDICES="$(validate_seed_list holdout "$HOLDOUT_INDICES_RAW")"
declare -A TRAIN_SET=()
IFS=',' read -r -a TRAIN_ARRAY <<< "$TRAIN_INDICES"
IFS=',' read -r -a HOLDOUT_ARRAY <<< "$HOLDOUT_INDICES"
for index in "${TRAIN_ARRAY[@]}"; do TRAIN_SET["$index"]=1; done
for index in "${HOLDOUT_ARRAY[@]}"; do
  [[ -z "${TRAIN_SET[$index]+present}" ]] || die "training and holdout seed sets overlap at index $index"
done

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $TOOLCHAIN "* ]] || die "unexpected Rust toolchain: $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo $TOOLCHAIN "* ]] || die "unexpected Cargo toolchain: $CARGO_VERSION"
RUSTC_VERBOSE="$(rustup run "$TOOLCHAIN" rustc -vV)"
HOST="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^host: //p')"
LLVM_VERSION="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')"
[[ -n "$HOST" && -n "$LLVM_VERSION" ]] || die "could not identify compiler host/LLVM version"
[[ -n "$TARGET" ]] || TARGET="$HOST"
[[ "$TARGET" == "$HOST" ]] || die "this first seed-holdout PGO pilot is host-native only (host=$HOST target=$TARGET)"

LOCK_BEFORE="$(sha256sum Cargo.lock | awk '{print $1}')"
SOURCE_COMMIT="$(git rev-parse HEAD)"
SOURCE_TREE="$(git rev-parse 'HEAD^{tree}')"
KERNEL="$(uname -srmo)"
CPU_MODEL="$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)"
CPU_COUNT="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
MEMORY_TOTAL="$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)"
FILESYSTEM="$(stat -f -c %T "$PWD" 2>/dev/null || echo unknown)"
MOUNT_INFO="$(findmnt -no SOURCE,FSTYPE,OPTIONS -T "$PWD" 2>/dev/null || echo unavailable)"
CPU_GOVERNOR="$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/$SOURCE_COMMIT-$TOOLCHAIN-$EXAMPLE-$RUN_ID"
PROFILE_DIR="$OUT/profile-data"
INSTR_TARGET="$OUT/target-instrumented"
TRAIN_BASE_TARGET="$OUT/target-training-control"
BASE_ROOT="$OUT/target-no-pgo"
PGO_ROOT="$OUT/target-pgo"
mkdir -p "$OUT"/{training,instrumented,no-pgo,instrumentation-pgo,logs,metadata} "$PROFILE_DIR"

export CARGO_INCREMENTAL=0
export CARGO_TERM_COLOR=never
# Warm the exact locked graph before entering offline measurement mode.
env -u CARGO_NET_OFFLINE rustup run "$TOOLCHAIN" cargo fetch --locked
export CARGO_NET_OFFLINE=true
rustup run "$TOOLCHAIN" cargo metadata --locked --format-version 1 --no-deps > "$OUT/metadata/cargo-metadata.json"
CARGO_METADATA_SHA="$(sha256sum "$OUT/metadata/cargo-metadata.json" | awk '{print $1}')"

build_example() {
  local target_dir="$1" example="$2" rustflags="$3" log="$4"
  local time_file="$log.time" start_ns end_ns status elapsed_ms rss_kib
  mkdir -p "$target_dir"
  start_ns="$(date +%s%N)"
  set +e
  if [[ -x /usr/bin/time ]]; then
    if [[ -n "$rustflags" ]]; then
      /usr/bin/time -f '%M' -o "$time_file" \
        env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
        CARGO_TARGET_DIR="$target_dir" RUSTFLAGS="$rustflags" \
        rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
          --target "$TARGET" --example "$example" > "$log" 2>&1
    else
      /usr/bin/time -f '%M' -o "$time_file" \
        env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
        CARGO_TARGET_DIR="$target_dir" \
        rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
          --target "$TARGET" --example "$example" > "$log" 2>&1
    fi
    status=$?
  else
    if [[ -n "$rustflags" ]]; then
      env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
        CARGO_TARGET_DIR="$target_dir" RUSTFLAGS="$rustflags" \
        rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
          --target "$TARGET" --example "$example" > "$log" 2>&1
    else
      env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
        CARGO_TARGET_DIR="$target_dir" \
        rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
          --target "$TARGET" --example "$example" > "$log" 2>&1
    fi
    status=$?
    : > "$time_file"
  fi
  set -e
  end_ns="$(date +%s%N)"
  elapsed_ms=$(((end_ns-start_ns)/1000000))
  if [[ $status -ne 0 ]]; then tail -n 120 "$log" >&2 || true; die "build failed: $log"; fi
  rss_kib="$(cat "$time_file" 2>/dev/null || true)"
  [[ "$rss_kib" =~ ^[0-9]+$ ]] || rss_kib=NA
  printf '%s,%s\n' "$elapsed_ms" "$rss_kib"
}

run_seed_set() {
  local variant="$1" binary="$2" indices="$3" phase="$4" trial="$5" repeat="$6"
  local stdout="$OUT/$variant/$phase-$trial-$repeat.stdout"
  local stderr="$OUT/$variant/$phase-$trial-$repeat.stderr"
  local start_ns end_ns wall_ms stdout_sha stderr_sha
  start_ns="$(date +%s%N)"
  env -u SYMTROPY_SEED_START_INDEX -u SYMTROPY_SEED_COUNT SYMTROPY_SEED_INDICES="$indices" "$binary" > "$stdout" 2> "$stderr"
  end_ns="$(date +%s%N)"
  wall_ms=$(((end_ns-start_ns)/1000000))
  stdout_sha="$(sha256sum "$stdout" | awk '{print $1}')"
  stderr_sha="$(sha256sum "$stderr" | awk '{print $1}')"
  printf '%s,%s,%s\n' "$wall_ms" "$stdout_sha" "$stderr_sha"
}

printf 'toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256\n' > "$OUT/metadata/runtime.csv"
printf 'variant,phase,trial,repeat,seed_indices,runtime_ms,stdout_sha256,stderr_sha256\n' > "$OUT/metadata/output-digests.csv"
printf 'variant,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,binary_sha256\n' > "$OUT/metadata/builds.csv"

# Instrumentation profile training is restricted to the training index set.
INSTR_BUILD="$(build_example "$INSTR_TARGET" "$EXAMPLE" "-Cdebuginfo=line-tables-only -Cprofile-generate=$PROFILE_DIR" "$OUT/logs/instrumentation-training-build.log")"
INSTR_BIN="$INSTR_TARGET/$TARGET/release/examples/$EXAMPLE"
[[ -x "$INSTR_BIN" ]] || die "instrumented training binary missing"
TRAIN_OUT=()
TRAIN_ERR=()
for repeat in 1 2; do
  PAIR="$(run_seed_set instrumented "$INSTR_BIN" "$TRAIN_INDICES" training 0 "$repeat")"
  PAIR_REST="${PAIR#*,}"
  TRAIN_OUT+=( "${PAIR_REST%%,*}" )
  TRAIN_ERR+=( "${PAIR_REST##*,}" )
  printf 'instrumentation,training,0,%s,%s,%s,%s,%s\n' "$repeat" "$TRAIN_INDICES" "${PAIR%%,*}" "${PAIR_REST%%,*}" "${PAIR_REST##*,}" >> "$OUT/metadata/output-digests.csv"
done
[[ "${TRAIN_OUT[0]}" == "${TRAIN_OUT[1]}" && "${TRAIN_ERR[0]}" == "${TRAIN_ERR[1]}" ]] || die "instrumented training outputs are not deterministic"

# No-PGO training control is an independent executable build; it must reproduce
# the same training outputs before the derived profile can be evaluated.
TRAIN_BASE_BUILD="$(build_example "$TRAIN_BASE_TARGET" "$EXAMPLE" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-training-control-build.log")"
TRAIN_BASE_BIN="$TRAIN_BASE_TARGET/$TARGET/release/examples/$EXAMPLE"
[[ -x "$TRAIN_BASE_BIN" ]] || die "no-PGO training control binary missing"
TRAIN_BASE_PAIR="$(run_seed_set no-pgo "$TRAIN_BASE_BIN" "$TRAIN_INDICES" training-control 0 1)"
TRAIN_BASE_REST="${TRAIN_BASE_PAIR#*,}"
[[ "${TRAIN_BASE_REST%%,*}" == "${TRAIN_OUT[0]}" && "${TRAIN_BASE_REST##*,}" == "${TRAIN_ERR[0]}" ]] || die "instrumentation training output differs from no-PGO training control"
printf 'no-pgo,training-control,0,1,%s,%s,%s,%s\n' "$TRAIN_INDICES" "${TRAIN_BASE_PAIR%%,*}" "${TRAIN_BASE_REST%%,*}" "${TRAIN_BASE_REST##*,}" >> "$OUT/metadata/output-digests.csv"

rustup component add llvm-tools-preview --toolchain "$TOOLCHAIN"
SYSROOT="$(rustup run "$TOOLCHAIN" rustc --print sysroot)"
LLVM_PROFDATA="$(find "$SYSROOT/lib/rustlib" -type f -name llvm-profdata -perm -u+x -print -quit)"
[[ -x "$LLVM_PROFDATA" ]] || die "llvm-profdata not found in llvm-tools-preview"
PROFRAW_FILES=("$PROFILE_DIR"/*.profraw)
(( ${#PROFRAW_FILES[@]} > 0 )) || die "training produced no .profraw files"
"$LLVM_PROFDATA" merge --failure-mode=any -o "$PROFILE_DIR/merged.profdata" "${PROFRAW_FILES[@]}"
[[ -s "$PROFILE_DIR/merged.profdata" ]] || die "merged profile is empty"
LLVM_PROFDATA_VERSION="$("$LLVM_PROFDATA" --version 2>&1 | head -n1)"
"$LLVM_PROFDATA" show --all-functions "$PROFILE_DIR/merged.profdata" > "$OUT/metadata/profile-summary.txt"

declare -A BASE_BUILD_MS BASE_BUILD_RSS BASE_BYTES BASE_BINARY BASE_BINARY_SHA
declare -A PGO_BUILD_MS PGO_BUILD_RSS PGO_BYTES PGO_BINARY PGO_BINARY_SHA
declare -A BASE_DIGEST GLOBAL_DIGEST

# Paired fresh builds. Each build trial yields two independent release artifacts.
for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  base_dir="$BASE_ROOT/trial-$trial"
  pgo_dir="$PGO_ROOT/trial-$trial"
  if (( trial % 2 == 1 )); then
    BASE_BUILD="$(build_example "$base_dir" "$EXAMPLE" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-$trial.log")"
    PGO_BUILD="$(build_example "$pgo_dir" "$EXAMPLE" "-Cdebuginfo=line-tables-only -Cprofile-use=$PROFILE_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/instrumentation-pgo-$trial.log")"
  else
    PGO_BUILD="$(build_example "$pgo_dir" "$EXAMPLE" "-Cdebuginfo=line-tables-only -Cprofile-use=$PROFILE_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/instrumentation-pgo-$trial.log")"
    BASE_BUILD="$(build_example "$base_dir" "$EXAMPLE" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-$trial.log")"
  fi
  BASE_BUILD_MS="${BASE_BUILD%%,*}"; BASE_BUILD_RSS="${BASE_BUILD##*,}"
  PGO_BUILD_MS="${PGO_BUILD%%,*}"; PGO_BUILD_RSS="${PGO_BUILD##*,}"
  BASE_BIN="$base_dir/$TARGET/release/examples/$EXAMPLE"
  PGO_BIN="$pgo_dir/$TARGET/release/examples/$EXAMPLE"
  [[ -x "$BASE_BIN" && -x "$PGO_BIN" ]] || die "paired executable missing"
  BASE_BINARY["$trial"]="$(stat -c %s "$BASE_BIN")"
  BASE_BINARY_SHA["$trial"]="$(sha256sum "$BASE_BIN" | awk '{print $1}')"
  BASE_BYTES["$trial"]="$(du -sb "$base_dir" | awk '{print $1}')"
  PGO_BINARY["$trial"]="$(stat -c %s "$PGO_BIN")"
  PGO_BINARY_SHA["$trial"]="$(sha256sum "$PGO_BIN" | awk '{print $1}')"
  PGO_BYTES["$trial"]="$(du -sb "$pgo_dir" | awk '{print $1}')"
  BASE_BUILD_MS["$trial"]="$BASE_BUILD_MS"; BASE_BUILD_RSS["$trial"]="$BASE_BUILD_RSS"
  PGO_BUILD_MS["$trial"]="$PGO_BUILD_MS"; PGO_BUILD_RSS["$trial"]="$PGO_BUILD_RSS"
  printf 'no-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$EXAMPLE" "$BASE_BUILD_MS" "$BASE_BUILD_RSS" "${BASE_BYTES[$trial]}" "${BASE_BINARY[$trial]}" "${BASE_BINARY_SHA[$trial]}" >> "$OUT/metadata/builds.csv"
  printf 'instrumentation-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$EXAMPLE" "$PGO_BUILD_MS" "$PGO_BUILD_RSS" "${PGO_BYTES[$trial]}" "${PGO_BINARY[$trial]}" "${PGO_BINARY_SHA[$trial]}" >> "$OUT/metadata/builds.csv"
done

# Execute only the disjoint holdout schedule for runtime comparisons.
for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  base_bin="$BASE_ROOT/trial-$trial/$TARGET/release/examples/$EXAMPLE"
  pgo_bin="$PGO_ROOT/trial-$trial/$TARGET/release/examples/$EXAMPLE"
  for ((repeat=1; repeat<=RUNTIME_REPEATS; repeat++)); do
    if (((trial+repeat)%2==0)); then
      PGO_PAIR="$(run_seed_set instrumentation-pgo "$pgo_bin" "$HOLDOUT_INDICES" holdout "$trial" "$repeat")"
      BASE_PAIR="$(run_seed_set no-pgo "$base_bin" "$HOLDOUT_INDICES" holdout "$trial" "$repeat")"
    else
      BASE_PAIR="$(run_seed_set no-pgo "$base_bin" "$HOLDOUT_INDICES" holdout "$trial" "$repeat")"
      PGO_PAIR="$(run_seed_set instrumentation-pgo "$pgo_bin" "$HOLDOUT_INDICES" holdout "$trial" "$repeat")"
    fi
    BASE_MS="${BASE_PAIR%%,*}"; BASE_REST="${BASE_PAIR#*,}"
    BASE_OUT="${BASE_REST%%,*}"; BASE_ERR="${BASE_REST##*,}"
    PGO_MS="${PGO_PAIR%%,*}"; PGO_REST="${PGO_PAIR#*,}"
    PGO_OUT="${PGO_REST%%,*}"; PGO_ERR="${PGO_REST##*,}"
    [[ "$BASE_OUT" == "$PGO_OUT" && "$BASE_ERR" == "$PGO_ERR" ]] || {
      printf 'OUTPUT_REVIEW_REQUIRED: PGO changed held-out output for %s trial=%s repeat=%s\n' "$EXAMPLE" "$trial" "$repeat" | tee -a "$OUT/STATUS.txt"
      exit 3
    }
    for variant in no-pgo instrumentation-pgo; do
      if [[ "$variant" == no-pgo ]]; then
        current="$BASE_REST"; runtime_ms="$BASE_MS"; build_ms="${BASE_BUILD_MS[$trial]}"; rss="${BASE_BUILD_RSS[$trial]}"; target_bytes="${BASE_BYTES[$trial]}"; binary_bytes="${BASE_BINARY[$trial]}"
      else
        current="$PGO_REST"; runtime_ms="$PGO_MS"; build_ms="${PGO_BUILD_MS[$trial]}"; rss="${PGO_BUILD_RSS[$trial]}"; target_bytes="${PGO_BYTES[$trial]}"; binary_bytes="${PGO_BINARY[$trial]}"
      fi
      out_sha="${current%%,*}"; err_sha="${current##*,}"
      key="$variant:$trial"
      digest_pair="$out_sha,$err_sha"
      if [[ -z "${BASE_DIGEST[$key]+present}" ]]; then
        BASE_DIGEST["$key"]="$digest_pair"
      else
        [[ "${BASE_DIGEST[$key]}" == "$digest_pair" ]] || die "held-out output drift across repeats for $variant trial=$trial"
      fi
      if [[ -z "${GLOBAL_DIGEST[$variant]+present}" ]]; then
        GLOBAL_DIGEST["$variant"]="$digest_pair"
      else
        [[ "${GLOBAL_DIGEST[$variant]}" == "$digest_pair" ]] || die "held-out output drift across fresh builds for $variant"
      fi
      printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
        "$variant" "$trial" "$EXAMPLE" "$build_ms" "$rss" "$target_bytes" "$binary_bytes" "$repeat" "$runtime_ms" "$out_sha" >> "$OUT/metadata/runtime.csv"
      printf '%s,holdout,%s,%s,%s,%s,%s,%s\n' "$variant" "$trial" "$repeat" "$HOLDOUT_INDICES" "$runtime_ms" "$out_sha" "$err_sha" >> "$OUT/metadata/output-digests.csv"
    done
  done
done

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked"

RUSTC_TOOLCHAIN="$TOOLCHAIN" bash scripts/analyze-rust-toolchain-benchmark.sh \
  "$OUT/metadata/runtime.csv" no-pgo instrumentation-pgo \
  > "$OUT/metadata/paired-analysis.md"

TRAIN_COUNT="$(awk -F, '{print NF}' <<< "$TRAIN_INDICES")"
HOLDOUT_COUNT="$(awk -F, '{print NF}' <<< "$HOLDOUT_INDICES")"
{
  echo "schema=luminous.rust-pgo-same-program-seed-holdout.v0.1"
  echo "source_commit=$SOURCE_COMMIT"
  echo "source_tree=$SOURCE_TREE"
  echo "cargo_lock_sha256=$LOCK_BEFORE"
  echo "cargo_lock_sha256_after=$LOCK_AFTER"
  echo "cargo_metadata_sha256=$CARGO_METADATA_SHA"
  echo "kernel=$KERNEL"
  echo "cpu_model=$CPU_MODEL"
  echo "cpu_count=$CPU_COUNT"
  echo "memory_total=$MEMORY_TOTAL"
  echo "filesystem=$FILESYSTEM"
  echo "mount_info=$MOUNT_INFO"
  echo "cpu_governor=$CPU_GOVERNOR"
  echo "toolchain=$TOOLCHAIN"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "llvm=$LLVM_VERSION"
  echo "llvm_profdata=$LLVM_PROFDATA_VERSION"
  echo "target=$TARGET"
  echo "example=$EXAMPLE"
  echo "training_seed_indices=$TRAIN_INDICES"
  echo "training_seed_count=$TRAIN_COUNT"
  echo "holdout_seed_indices=$HOLDOUT_INDICES"
  echo "holdout_seed_count=$HOLDOUT_COUNT"
  echo "seed_formula=42+index*997"
  echo "profile_training_scope=training_seed_indices_only"
  echo "performance_measurement_scope=heldout_seed_indices_only"
  echo "training_baseline_output_match=EXACT_STDOUT_AND_STDERR"
  echo "heldout_baseline_pgo_output_match=EXACT_STDOUT_AND_STDERR"
  echo "profile_sha256=$(sha256sum "$PROFILE_DIR/merged.profdata" | awk '{print $1}')"
  echo "profile_bytes=$(stat -c %s "$PROFILE_DIR/merged.profdata")"
  echo "build_repeats=$BUILD_REPEATS"
  echo "runtime_repeats_per_build=$RUNTIME_REPEATS"
  echo "target_directories=fresh_per_variant_trial"
  echo "build_flags=matched line-tables-only; only PGO profile treatment differs"
  echo "trial_order=rotated for fresh builds and held-out executions"
  echo "cache_policy=fresh-target; warmed registry; offline measured builds; sccache disabled"
  echo "p99_claim=NOT_SUPPORTED_BY_DEFAULT_SAMPLE_COUNTS"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/metadata/run-manifest.txt"

printf 'PASS_MECHANICAL_PROFILE_AND_HELDOUT_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
bash scripts/seal-rust-pgo-evidence.sh "$OUT" metadata/run-manifest.txt
printf 'results=%s\n' "$OUT"
printf 'RUSTC_TOOLCHAIN=%s bash scripts/analyze-rust-toolchain-benchmark.sh %s/metadata/runtime.csv no-pgo instrumentation-pgo\n' "$TOOLCHAIN" "$OUT"
