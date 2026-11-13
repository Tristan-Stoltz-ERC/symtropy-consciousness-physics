#!/usr/bin/env bash
set -Eeuo pipefail

# Instrumentation PGO with fresh paired build trials and repeated runtime trials.
# The profile is generated once from the declared workload, then used for each
# fresh candidate build. No source, lockfile, input or seed is changed.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
TRAIN_EXAMPLE="${PGO_TRAIN_EXAMPLE:-jphi_convergence}"
BUILD_REPEATS="${PGO_BUILD_REPEATS:-5}"
RUNTIME_REPEATS="${PGO_RUN_REPEATS:-5}"
TARGET="${PGO_TARGET:-}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-rust-pgo}"
VALIDATION_EXAMPLES=()
read -r -a VALIDATION_EXAMPLES <<< "${PGO_VALIDATION_EXAMPLES:-gradient_replacement cooperation_decomposition}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ "$BUILD_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_BUILD_REPEATS must be a positive integer"
[[ "$RUNTIME_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUN_REPEATS must be a positive integer"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"
[[ -f Cargo.lock ]] || die "Cargo.lock is required"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || die "PGO qualification requires a clean worktree"

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $TOOLCHAIN "* ]] || die "unexpected Rust toolchain: $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo $TOOLCHAIN "* ]] || die "unexpected Cargo toolchain: $CARGO_VERSION"
RUSTC_VERBOSE="$(rustup run "$TOOLCHAIN" rustc -vV)"
HOST="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^host: //p')"
LLVM_VERSION="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')"
[[ -n "$HOST" && -n "$LLVM_VERSION" ]] || die "could not identify compiler host/LLVM version"
[[ -n "$TARGET" ]] || TARGET="$HOST"
[[ "$TARGET" == "$HOST" ]] || die "initial PGO pilot is host-native only; host=$HOST target=$TARGET"

case "$TRAIN_EXAMPLE" in
  gradient_replacement|cooperation_decomposition|jphi_convergence) ;;
  *) die "unsupported training example: $TRAIN_EXAMPLE" ;;
esac
EXAMPLES=("$TRAIN_EXAMPLE")
for example in "${VALIDATION_EXAMPLES[@]}"; do
  case "$example" in gradient_replacement|cooperation_decomposition|jphi_convergence) ;; *) die "unsupported validation example: $example" ;; esac
  [[ "$example" == "$TRAIN_EXAMPLE" ]] || EXAMPLES+=("$example")
done

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
OUT="$RESULT_ROOT/$SOURCE_COMMIT-$TOOLCHAIN-$RUN_ID"
PROFILE_DIR="$OUT/profile-data"
INSTR_TARGET="$OUT/target-instrumented"
BASE_ROOT="$OUT/target-no-pgo"
PGO_ROOT="$OUT/target-pgo"
mkdir -p "$OUT"/{baseline,instrumented,pgo,logs,metadata} "$PROFILE_DIR"

export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export CARGO_TERM_COLOR=never
rustup run "$TOOLCHAIN" cargo fetch --locked
rustup run "$TOOLCHAIN" cargo metadata --locked --format-version 1 --no-deps > "$OUT/metadata/cargo-metadata.json"
CARGO_METADATA_SHA="$(sha256sum "$OUT/metadata/cargo-metadata.json" | awk '{print $1}')"

declare -A BASE_BUILD_MS BASE_BUILD_RSS BASE_BINARY BASE_BINARY_SHA BASE_BYTES
declare -A PGO_BUILD_MS PGO_BUILD_RSS PGO_BINARY PGO_BINARY_SHA PGO_BYTES
declare -A BASE_DIGEST PGO_DIGEST

build_example() {
  local target_dir="$1" example="$2" rustflags="$3" log="$4"
  local time_file="$log.time" start_ns end_ns status elapsed_ms rss_kib
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

run_example() {
  local variant="$1" binary="$2" example="$3" trial="$4" repeat="$5"
  local build_ms="$6" build_rss="$7" target_bytes="$8" binary_bytes="$9"
  local stdout="$OUT/$variant/$example-$trial-$repeat.stdout"
  local stderr="$OUT/$variant/$example-$trial-$repeat.stderr"
  local start_ns end_ns wall_ms stdout_sha stderr_sha
  start_ns="$(date +%s%N)"
  "$binary" > "$stdout" 2> "$stderr"
  end_ns="$(date +%s%N)"
  wall_ms=$(((end_ns-start_ns)/1000000))
  stdout_sha="$(sha256sum "$stdout" | awk '{print $1}')"
  stderr_sha="$(sha256sum "$stderr" | awk '{print $1}')"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$variant" "$trial" "$example" "$build_ms" "$build_rss" "$target_bytes" "$binary_bytes" "$repeat" "$wall_ms" "$stdout_sha" \
    >> "$OUT/metadata/runtime.csv"
  printf '%s,%s,%s,%s,%s,%s\n' "$variant" "$trial" "$example" "$repeat" "$stdout_sha" "$stderr_sha" \
    >> "$OUT/metadata/output-digests.csv"
  printf '%s,%s\n' "$stdout_sha" "$stderr_sha"
}

printf 'toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256\n' > "$OUT/metadata/runtime.csv"
printf 'variant,trial,example,runtime_repeat,stdout_sha256,stderr_sha256\n' > "$OUT/metadata/output-digests.csv"
printf 'variant,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,binary_sha256\n' > "$OUT/metadata/builds.csv"
printf 'variant,training_run,example,stdout_sha256,stderr_sha256\n' > "$OUT/metadata/training-digests.csv"

# Profile training build. Profile-generation flags are not applied to build scripts
# because Cargo receives an explicit target triple.
INSTR_BUILD="$(build_example "$INSTR_TARGET" "$TRAIN_EXAMPLE" "-Cprofile-generate=$PROFILE_DIR -Cdebuginfo=line-tables-only" "$OUT/logs/instrumentation-build.log")"
INSTR_BIN="$INSTR_TARGET/$TARGET/release/examples/$TRAIN_EXAMPLE"
[[ -x "$INSTR_BIN" ]] || die "instrumented training binary missing"
for ((i=1; i<=2; i++)); do
  "$INSTR_BIN" > "$OUT/instrumented/training-$i.stdout" 2> "$OUT/instrumented/training-$i.stderr"
  printf 'instrumentation,%s,%s,%s,%s\n' "$i" "$TRAIN_EXAMPLE" \
    "$(sha256sum "$OUT/instrumented/training-$i.stdout" | awk '{print $1}')" \
    "$(sha256sum "$OUT/instrumented/training-$i.stderr" | awk '{print $1}')" \
    >> "$OUT/metadata/training-digests.csv"
done
TRAIN_OUT1="$(awk -F, 'NR==2 {print $4}' "$OUT/metadata/training-digests.csv")"
TRAIN_ERR1="$(awk -F, 'NR==2 {print $5}' "$OUT/metadata/training-digests.csv")"
TRAIN_OUT2="$(awk -F, 'NR==3 {print $4}' "$OUT/metadata/training-digests.csv")"
TRAIN_ERR2="$(awk -F, 'NR==3 {print $5}' "$OUT/metadata/training-digests.csv")"
[[ "$TRAIN_OUT1" == "$TRAIN_OUT2" && "$TRAIN_ERR1" == "$TRAIN_ERR2" ]] || die "instrumented training output is not deterministic"

shopt -s nullglob
PROFRAW_FILES=("$PROFILE_DIR"/*.profraw)
(( ${#PROFRAW_FILES[@]} > 0 )) || die "training produced no .profraw files"
rustup component add llvm-tools-preview --toolchain "$TOOLCHAIN"
SYSROOT="$(rustup run "$TOOLCHAIN" rustc --print sysroot)"
LLVM_PROFDATA="$(find "$SYSROOT/lib/rustlib" -type f -name llvm-profdata -perm -u+x -print -quit)"
[[ -x "$LLVM_PROFDATA" ]] || die "llvm-profdata not found in llvm-tools-preview"
"$LLVM_PROFDATA" merge --failure-mode=any -o "$PROFILE_DIR/merged.profdata" "${PROFRAW_FILES[@]}"
[[ -s "$PROFILE_DIR/merged.profdata" ]] || die "merged profile is empty"
LLVM_PROFDATA_VERSION="$("$LLVM_PROFDATA" --version 2>&1 | head -n1)"
"$LLVM_PROFDATA" show --all-functions "$PROFILE_DIR/merged.profdata" > "$OUT/metadata/profile-summary.txt"

# Repeated paired fresh builds. Both variants use line-tables-only debuginfo;
# profiling flags are the treatment difference. Trial order is alternated.
for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  for example in "${EXAMPLES[@]}"; do
    base_dir="$BASE_ROOT/trial-$trial/$example"
    pgo_dir="$PGO_ROOT/trial-$trial/$example"
    mkdir -p "$base_dir" "$pgo_dir"
    if (( trial % 2 == 1 )); then
      BASE_BUILD="$(build_example "$base_dir" "$example" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-$trial-$example.log")"
      PGO_BUILD="$(build_example "$pgo_dir" "$example" "-Cdebuginfo=line-tables-only -Cprofile-use=$PROFILE_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/pgo-$trial-$example.log")"
    else
      PGO_BUILD="$(build_example "$pgo_dir" "$example" "-Cdebuginfo=line-tables-only -Cprofile-use=$PROFILE_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/pgo-$trial-$example.log")"
      BASE_BUILD="$(build_example "$base_dir" "$example" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-$trial-$example.log")"
    fi
    BASE_BUILD_MS="${BASE_BUILD%%,*}"; BASE_BUILD_RSS="${BASE_BUILD##*,}"
    PGO_BUILD_MS="${PGO_BUILD%%,*}"; PGO_BUILD_RSS="${PGO_BUILD##*,}"
    BASE_BIN="$base_dir/$TARGET/release/examples/$example"
    PGO_BIN="$pgo_dir/$TARGET/release/examples/$example"
    [[ -x "$BASE_BIN" && -x "$PGO_BIN" ]] || die "paired executable missing"
    BASE_BINARY["$trial:$example"]="$(stat -c %s "$BASE_BIN")"
    BASE_BINARY_SHA["$trial:$example"]="$(sha256sum "$BASE_BIN" | awk '{print $1}')"
    BASE_BYTES["$trial:$example"]="$(du -sb "$base_dir" | awk '{print $1}')"
    PGO_BINARY["$trial:$example"]="$(stat -c %s "$PGO_BIN")"
    PGO_BINARY_SHA["$trial:$example"]="$(sha256sum "$PGO_BIN" | awk '{print $1}')"
    PGO_BYTES["$trial:$example"]="$(du -sb "$pgo_dir" | awk '{print $1}')"
    BASE_BUILD_MS["$trial:$example"]="$BASE_BUILD_MS"; BASE_BUILD_RSS["$trial:$example"]="$BASE_BUILD_RSS"
    PGO_BUILD_MS["$trial:$example"]="$PGO_BUILD_MS"; PGO_BUILD_RSS["$trial:$example"]="$PGO_BUILD_RSS"
    printf 'no-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$example" "$BASE_BUILD_MS" "$BASE_BUILD_RSS" "${BASE_BYTES[$trial:$example]}" "${BASE_BINARY[$trial:$example]}" "${BASE_BINARY_SHA[$trial:$example]}" >> "$OUT/metadata/builds.csv"
    printf 'instrumentation-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$example" "$PGO_BUILD_MS" "$PGO_BUILD_RSS" "${PGO_BYTES[$trial:$example]}" "${PGO_BINARY[$trial:$example]}" "${PGO_BINARY_SHA[$trial:$example]}" >> "$OUT/metadata/builds.csv"
  done
done

# Runtime repeats are nested within each paired build trial. Execution order
# rotates by trial + repeat. Each raw output is retained and hashed.
for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  for example in "${EXAMPLES[@]}"; do
    base_dir="$BASE_ROOT/trial-$trial/$example"
    pgo_dir="$PGO_ROOT/trial-$trial/$example"
    key="$trial:$example"
    for ((repeat=1; repeat<=RUNTIME_REPEATS; repeat++)); do
      if (((trial+repeat)%2==0)); then
        PGO_PAIR="$(run_example instrumentation-pgo "$pgo_dir/$TARGET/release/examples/$example" "$example" "$trial" "$repeat" "${PGO_BUILD_MS[$key]}" "${PGO_BUILD_RSS[$key]}" "${PGO_BYTES[$key]}" "${PGO_BINARY[$key]}")"
        BASE_PAIR="$(run_example no-pgo "$base_dir/$TARGET/release/examples/$example" "$example" "$trial" "$repeat" "${BASE_BUILD_MS[$key]}" "${BASE_BUILD_RSS[$key]}" "${BASE_BYTES[$key]}" "${BASE_BINARY[$key]}")"
      else
        BASE_PAIR="$(run_example no-pgo "$base_dir/$TARGET/release/examples/$example" "$example" "$trial" "$repeat" "${BASE_BUILD_MS[$key]}" "${BASE_BUILD_RSS[$key]}" "${BASE_BYTES[$key]}" "${BASE_BINARY[$key]}")"
        PGO_PAIR="$(run_example instrumentation-pgo "$pgo_dir/$TARGET/release/examples/$example" "$example" "$trial" "$repeat" "${PGO_BUILD_MS[$key]}" "${PGO_BUILD_RSS[$key]}" "${PGO_BYTES[$key]}" "${PGO_BINARY[$key]}")"
      fi
      BASE_OUT="${BASE_PAIR%%,*}"; BASE_ERR="${BASE_PAIR##*,}"
      PGO_OUT="${PGO_PAIR%%,*}"; PGO_ERR="${PGO_PAIR##*,}"
      [[ "$BASE_OUT" == "$PGO_OUT" && "$BASE_ERR" == "$PGO_ERR" ]] || {
        echo "OUTPUT_REVIEW_REQUIRED: PGO changed stdout/stderr for $example trial=$trial repeat=$repeat" | tee -a "$OUT/STATUS.txt"
        exit 3
      }
      for variant in no-pgo instrumentation-pgo; do
        if [[ "$variant" == no-pgo ]]; then current_out="$BASE_OUT"; current_err="$BASE_ERR"; else current_out="$PGO_OUT"; current_err="$PGO_ERR"; fi
        dkey="$variant:$example"
        if [[ -z "${BASE_DIGEST[$dkey]+set}" ]]; then
          BASE_DIGEST["$dkey"]="$current_out,$current_err"
        else
          [[ "${BASE_DIGEST[$dkey]}" == "$current_out,$current_err" ]] || die "output drift across repeats for $variant / $example"
        fi
      done
      if [[ "$example" == "$TRAIN_EXAMPLE" ]]; then
        [[ "$BASE_OUT" == "$TRAIN_OUT1" && "$BASE_ERR" == "$TRAIN_ERR1" ]] || die "instrumented training execution differs from no-PGO baseline"
      fi
    done
  done
done

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked"

RUSTC_TOOLCHAIN="$TOOLCHAIN" bash scripts/analyze-rust-toolchain-benchmark.sh \
  "$OUT/metadata/runtime.csv" no-pgo instrumentation-pgo \
  > "$OUT/metadata/paired-analysis.md"

{
  echo "schema=luminous.rust-pgo-instrumentation.v0.3"
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
  echo "profile_generate_flags=-Cprofile-generate=<profile-dir> -Cdebuginfo=line-tables-only"
  echo "profile_use_flags=-Cdebuginfo=line-tables-only -Cprofile-use=<merged.profdata> -Cllvm-args=-pgo-warn-missing-function"
  echo "training_example=$TRAIN_EXAMPLE"
  echo "validation_examples=${VALIDATION_EXAMPLES[*]}"
  echo "build_repeats=$BUILD_REPEATS"
  echo "runtime_repeats_per_build=$RUNTIME_REPEATS"
  echo "profile_sha256=$(sha256sum "$PROFILE_DIR/merged.profdata" | awk '{print $1}')"
  echo "profile_bytes=$(stat -c %s "$PROFILE_DIR/merged.profdata")"
  echo "target_directories=fresh_per_variant_trial_example"
  echo "build_flags=matched line-tables-only; only PGO profile treatment differs"
  echo "trial_order=rotated_for_build_and_runtime"
  echo "cache_policy=fresh-target; warmed registry; offline measured builds; sccache disabled"
  echo "observable_output_equivalence=EXACT_STDOUT_AND_STDERR"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/metadata/run-manifest.txt"
printf 'PASS_MECHANICAL_PROFILE_AND_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
bash scripts/seal-rust-pgo-evidence.sh "$OUT" metadata/run-manifest.txt
printf 'results=%s\n' "$OUT"
printf 'RUSTC_TOOLCHAIN=%s bash scripts/analyze-rust-toolchain-benchmark.sh %s/metadata/runtime.csv no-pgo instrumentation-pgo\n' "$TOOLCHAIN" "$OUT"
