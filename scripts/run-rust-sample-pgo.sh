#!/usr/bin/env bash
set -Eeuo pipefail

# Sample-based PGO with perf branch-stack training and repeated fresh paired
# builds. Missing PMU/LLVM capability is NOT_EVALUABLE, not an optimization pass.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
EXAMPLE="${PGO_SAMPLE_EXAMPLE:-jphi_convergence}"
BUILD_REPEATS="${PGO_BUILD_REPEATS:-5}"
RUNTIME_REPEATS="${PGO_RUN_REPEATS:-5}"
PERF_EVENT="${PERF_EVENT:-branches:u}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-rust-sample-pgo}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
not_evaluable() { printf 'NOT_EVALUABLE: %s\n' "$*" >&2; exit 3; }
[[ "$BUILD_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_BUILD_REPEATS must be a positive integer"
[[ "$RUNTIME_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUN_REPEATS must be a positive integer"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || die "sample PGO requires a clean worktree"
case "$EXAMPLE" in gradient_replacement|cooperation_decomposition|jphi_convergence) ;; *) die "unsupported example: $EXAMPLE" ;; esac

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $TOOLCHAIN "* ]] || die "unexpected Rust toolchain: $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo $TOOLCHAIN "* ]] || die "unexpected Cargo toolchain: $CARGO_VERSION"
RUSTC_VERBOSE="$(rustup run "$TOOLCHAIN" rustc -vV)"
HOST="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^host: //p')"
LLVM_VERSION="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')"
[[ "$HOST" == "x86_64-unknown-linux-gnu" ]] || not_evaluable "sample PGO is initially x86_64 Linux only (host=$HOST)"
command -v perf >/dev/null 2>&1 || not_evaluable "Linux perf is not installed"
command -v llvm-profgen >/dev/null 2>&1 || not_evaluable "llvm-profgen is not installed"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"

PROFGEN_VERSION="$(llvm-profgen --version 2>&1 | head -n 3)"
LLVM_MAJOR="$(printf '%s\n' "$LLVM_VERSION" | cut -d. -f1)"
PROFGEN_MAJOR="$(printf '%s\n' "$PROFGEN_VERSION" | sed -nE 's/.*LLVM version: ([0-9]+).*/\1/p' | head -n1)"
[[ -n "$PROFGEN_MAJOR" && "$PROFGEN_MAJOR" == "$LLVM_MAJOR" ]] || not_evaluable "llvm-profgen LLVM major ($PROFGEN_MAJOR) does not match rustc LLVM major ($LLVM_MAJOR)"

LOCK_BEFORE="$(sha256sum Cargo.lock | awk '{print $1}')"
COMMIT="$(git rev-parse HEAD)"
SOURCE_TREE="$(git rev-parse 'HEAD^{tree}')"
KERNEL="$(uname -srmo)"
CPU_MODEL="$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)"
CPU_COUNT="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
MEMORY_TOTAL="$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)"
FILESYSTEM="$(stat -f -c %T "$PWD" 2>/dev/null || echo unknown)"
MOUNT_INFO="$(findmnt -no SOURCE,FSTYPE,OPTIONS -T "$PWD" 2>/dev/null || echo unavailable)"
CPU_GOVERNOR="$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)"
PERF_VERSION="$(perf --version 2>&1 | head -n1)"
PERF_PARANOID="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo unavailable)"
PROFGEN_BIN="$(command -v llvm-profgen)"
PROFGEN_SHA="$(sha256sum "$PROFGEN_BIN" | awk '{print $1}')"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/$COMMIT-$TOOLCHAIN-$RUN_ID"
PROFILE="$OUT/$EXAMPLE.prof"
PERFDATA="$OUT/$EXAMPLE.perf.data"
TRAIN_TARGET="$OUT/target-sample-training"
BASE_ROOT="$OUT/target-no-pgo"
PGO_ROOT="$OUT/target-sample-pgo"
mkdir -p "$OUT"/{baseline,sample-pgo,logs,metadata}

export CARGO_INCREMENTAL=0
export CARGO_TERM_COLOR=never
# Warm the exact locked graph before entering offline measurement mode.
env -u CARGO_NET_OFFLINE rustup run "$TOOLCHAIN" cargo fetch --locked
export CARGO_NET_OFFLINE=true
rustup run "$TOOLCHAIN" cargo metadata --locked --format-version 1 --no-deps > "$OUT/metadata/cargo-metadata.json"
CARGO_METADATA_SHA="$(sha256sum "$OUT/metadata/cargo-metadata.json" | awk '{print $1}')"

# The profiling and measured builds all use the same line-table setting.
set +e
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
  CARGO_TARGET_DIR="$TRAIN_TARGET" RUSTFLAGS="-Cdebuginfo=line-tables-only" \
  rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
    --target "$HOST" --example "$EXAMPLE" > "$OUT/logs/sample-profile-build.log" 2>&1
status=$?
set -e
[[ $status -eq 0 ]] || { tail -n 100 "$OUT/logs/sample-profile-build.log" >&2; die "sample profiling build failed"; }
TRAIN_BIN="$TRAIN_TARGET/$HOST/release/examples/$EXAMPLE"
[[ -x "$TRAIN_BIN" ]] || die "sample training binary missing: $TRAIN_BIN"

set +e
perf record -b -e "$PERF_EVENT" -o "$PERFDATA" -- "$TRAIN_BIN" \
  > "$OUT/logs/perf-training.stdout" 2> "$OUT/logs/perf-training.stderr"
status=$?
set -e
if [[ $status -ne 0 ]]; then
  not_evaluable "perf could not record branch samples (event=$PERF_EVENT); inspect $OUT/logs/perf-training.stderr"
fi
[[ -s "$PERFDATA" ]] || not_evaluable "perf produced no data"

llvm-profgen --binary="$TRAIN_BIN" --output="$PROFILE" --perfdata="$PERFDATA" \
  > "$OUT/logs/llvm-profgen.log" 2>&1 || {
    cat "$OUT/logs/llvm-profgen.log" >&2 || true
    die "llvm-profgen could not convert perf data"
  }
[[ -s "$PROFILE" ]] || die "sample profile is empty"
# Capture the training program again without perf's own diagnostic text in stderr.
"$TRAIN_BIN" > "$OUT/logs/training-check.stdout" 2> "$OUT/logs/training-check.stderr"
TRAIN_OUT="$(sha256sum "$OUT/logs/training-check.stdout" | awk '{print $1}')"
TRAIN_ERR="$(sha256sum "$OUT/logs/training-check.stderr" | awk '{print $1}')"

build_example() {
  local variant="$1" target_dir="$2" rustflags="$3" log="$4"
  local time_file="$log.time" start_ns end_ns status elapsed_ms rss_kib
  start_ns="$(date +%s%N)"
  set +e
  if [[ -x /usr/bin/time ]]; then
    /usr/bin/time -f '%M' -o "$time_file" \
      env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
      CARGO_TARGET_DIR="$target_dir" RUSTFLAGS="$rustflags" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --target "$HOST" --example "$EXAMPLE" > "$log" 2>&1
    status=$?
  else
    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
      CARGO_TARGET_DIR="$target_dir" RUSTFLAGS="$rustflags" \
      rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
        --target "$HOST" --example "$EXAMPLE" > "$log" 2>&1
    status=$?
    : > "$time_file"
  fi
  set -e
  end_ns="$(date +%s%N)"
  elapsed_ms=$(((end_ns-start_ns)/1000000))
  [[ $status -eq 0 ]] || { tail -n 120 "$log" >&2; die "$variant build failed"; }
  rss_kib="$(cat "$time_file" 2>/dev/null || true)"
  [[ "$rss_kib" =~ ^[0-9]+$ ]] || rss_kib=NA
  printf '%s,%s\n' "$elapsed_ms" "$rss_kib"
}

run_example() {
  local variant="$1" binary="$2" trial="$3" repeat="$4"
  local stdout="$OUT/$variant/run-$trial-$repeat.stdout" stderr="$OUT/$variant/run-$trial-$repeat.stderr"
  local start_ns end_ns wall_ms stdout_sha stderr_sha
  start_ns="$(date +%s%N)"
  "$binary" > "$stdout" 2> "$stderr"
  end_ns="$(date +%s%N)"
  wall_ms=$(((end_ns-start_ns)/1000000))
  stdout_sha="$(sha256sum "$stdout" | awk '{print $1}')"
  stderr_sha="$(sha256sum "$stderr" | awk '{print $1}')"
  printf '%s,%s,%s\n' "$wall_ms" "$stdout_sha" "$stderr_sha"
}

declare -A BASE_BUILD_MS BASE_BUILD_RSS PGO_BUILD_MS PGO_BUILD_RSS
declare -A BASE_BINARY BASE_BYTES PGO_BINARY PGO_BYTES
declare -A BASE_DIGEST PGO_DIGEST

printf 'toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256\n' > "$OUT/metadata/runtime.csv"
printf 'variant,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,binary_sha256\n' > "$OUT/metadata/builds.csv"
printf 'variant,trial,runtime_repeat,stdout_sha256,stderr_sha256\n' > "$OUT/metadata/output-digests.csv"

for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  base_dir="$BASE_ROOT/trial-$trial"
  pgo_dir="$PGO_ROOT/trial-$trial"
  if (( trial % 2 == 1 )); then
    BASE_BUILD="$(build_example no-pgo "$base_dir" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-build-$trial.log")"
    PGO_BUILD="$(build_example sample-pgo "$pgo_dir" "-Cdebuginfo=line-tables-only -Cprofile-sample-use=$PROFILE -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/sample-pgo-build-$trial.log")"
  else
    PGO_BUILD="$(build_example sample-pgo "$pgo_dir" "-Cdebuginfo=line-tables-only -Cprofile-sample-use=$PROFILE -Cllvm-args=-pgo-warn-missing-function" "$OUT/logs/sample-pgo-build-$trial.log")"
    BASE_BUILD="$(build_example no-pgo "$base_dir" "-Cdebuginfo=line-tables-only" "$OUT/logs/no-pgo-build-$trial.log")"
  fi
  BASE_BUILD_MS="${BASE_BUILD%%,*}"; BASE_BUILD_RSS="${BASE_BUILD##*,}"
  PGO_BUILD_MS="${PGO_BUILD%%,*}"; PGO_BUILD_RSS="${PGO_BUILD##*,}"
  BASE_BIN="$base_dir/$HOST/release/examples/$EXAMPLE"
  PGO_BIN="$pgo_dir/$HOST/release/examples/$EXAMPLE"
  [[ -x "$BASE_BIN" && -x "$PGO_BIN" ]] || die "paired sample-PGO binary missing"
  BASE_BINARY[$trial]="$(stat -c %s "$BASE_BIN")"; BASE_BINARY_SHA[$trial]="$(sha256sum "$BASE_BIN" | awk '{print $1}')"; BASE_BYTES[$trial]="$(du -sb "$base_dir" | awk '{print $1}')"
  PGO_BINARY[$trial]="$(stat -c %s "$PGO_BIN")"; PGO_BINARY_SHA[$trial]="$(sha256sum "$PGO_BIN" | awk '{print $1}')"; PGO_BYTES[$trial]="$(du -sb "$pgo_dir" | awk '{print $1}')"
  BASE_BUILD_MS[$trial]="$BASE_BUILD_MS"; BASE_BUILD_RSS[$trial]="$BASE_BUILD_RSS"
  PGO_BUILD_MS[$trial]="$PGO_BUILD_MS"; PGO_BUILD_RSS[$trial]="$PGO_BUILD_RSS"
  printf 'no-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$EXAMPLE" "$BASE_BUILD_MS" "$BASE_BUILD_RSS" "${BASE_BYTES[$trial]}" "${BASE_BINARY[$trial]}" "${BASE_BINARY_SHA[$trial]}" >> "$OUT/metadata/builds.csv"
  printf 'sample-pgo,%s,%s,%s,%s,%s,%s,%s\n' "$trial" "$EXAMPLE" "$PGO_BUILD_MS" "$PGO_BUILD_RSS" "${PGO_BYTES[$trial]}" "${PGO_BINARY[$trial]}" "${PGO_BINARY_SHA[$trial]}" >> "$OUT/metadata/builds.csv"

  for ((repeat=1; repeat<=RUNTIME_REPEATS; repeat++)); do
    if (((trial+repeat)%2==0)); then
      PGO_PAIR="$(run_example sample-pgo "$PGO_BIN" "$trial" "$repeat")"
      BASE_PAIR="$(run_example no-pgo "$BASE_BIN" "$trial" "$repeat")"
    else
      BASE_PAIR="$(run_example no-pgo "$BASE_BIN" "$trial" "$repeat")"
      PGO_PAIR="$(run_example sample-pgo "$PGO_BIN" "$trial" "$repeat")"
    fi
    BASE_WALL="${BASE_PAIR%%,*}"; BASE_REST="${BASE_PAIR#*,}"; BASE_OUT="${BASE_REST%%,*}"; BASE_ERR="${BASE_REST##*,}"
    PGO_WALL="${PGO_PAIR%%,*}"; PGO_REST="${PGO_PAIR#*,}"; PGO_OUT="${PGO_REST%%,*}"; PGO_ERR="${PGO_REST##*,}"
    [[ "$BASE_OUT" == "$PGO_OUT" && "$BASE_ERR" == "$PGO_ERR" ]] || die "sample PGO changed stdout/stderr for trial=$trial repeat=$repeat"
    [[ "$BASE_OUT" == "$TRAIN_OUT" && "$BASE_ERR" == "$TRAIN_ERR" ]] || die "training executable outputs differ from no-PGO measurement"
    for variant in no-pgo sample-pgo; do
      if [[ "$variant" == no-pgo ]]; then wall="$BASE_WALL"; out_sha="$BASE_OUT"; err_sha="$BASE_ERR"; build_ms="$BASE_BUILD_MS"; build_rss="$BASE_BUILD_RSS"; target_bytes="${BASE_BYTES[$trial]}"; binary_bytes="${BASE_BINARY[$trial]}"
      else wall="$PGO_WALL"; out_sha="$PGO_OUT"; err_sha="$PGO_ERR"; build_ms="$PGO_BUILD_MS"; build_rss="$PGO_BUILD_RSS"; target_bytes="${PGO_BYTES[$trial]}"; binary_bytes="${PGO_BINARY[$trial]}"
      fi
      printf '%s,%s,%s,%s,%s,%s\n' "$variant" "$trial" "$repeat" "$out_sha" "$err_sha" "$wall" >> "$OUT/metadata/output-digests.csv"
      printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' "$variant" "$trial" "$EXAMPLE" "$build_ms" "$build_rss" "$target_bytes" "$binary_bytes" "$repeat" "$wall" "$out_sha" >> "$OUT/metadata/runtime.csv"
    done
  done
done

# Require deterministic output across every trial and repeat per variant.
for variant in no-pgo sample-pgo; do
  count="$(awk -F, -v v="$variant" '$1==v {print $4 "," $5}' "$OUT/metadata/output-digests.csv" | sort -u | wc -l | tr -d ' ')"
  [[ "$count" == 1 ]] || die "$variant output drifted across paired repeats"
done

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked"

RUSTC_TOOLCHAIN="$TOOLCHAIN" bash scripts/analyze-rust-toolchain-benchmark.sh \
  "$OUT/metadata/runtime.csv" no-pgo sample-pgo > "$OUT/metadata/paired-analysis.md"

{
  echo "schema=luminous.rust-pgo-sample.v0.3"
  echo "source_commit=$COMMIT"
  echo "source_tree=$SOURCE_TREE"
  echo "toolchain=$TOOLCHAIN"
  echo "cargo_lock_sha256=$LOCK_BEFORE"
  echo "cargo_lock_sha256_after=$LOCK_AFTER"
  echo "cargo_metadata_sha256=$CARGO_METADATA_SHA"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "rustc_llvm=$LLVM_VERSION"
  echo "llvm_profgen=$PROFGEN_VERSION"
  echo "llvm_profgen_path=$PROFGEN_BIN"
  echo "llvm_profgen_sha256=$PROFGEN_SHA"
  echo "perf_version=$PERF_VERSION"
  echo "perf_event=$PERF_EVENT"
  echo "perf_event_paranoid=$PERF_PARANOID"
  echo "kernel=$KERNEL"
  echo "cpu_model=$CPU_MODEL"
  echo "cpu_count=$CPU_COUNT"
  echo "memory_total=$MEMORY_TOTAL"
  echo "filesystem=$FILESYSTEM"
  echo "mount_info=$MOUNT_INFO"
  echo "cpu_governor=$CPU_GOVERNOR"
  echo "target=$HOST"
  echo "training_example=$EXAMPLE"
  echo "perf_data_sha256=$(sha256sum "$PERFDATA" | awk '{print $1}')"
  echo "sample_profile_sha256=$(sha256sum "$PROFILE" | awk '{print $1}')"
  echo "sample_profile_bytes=$(stat -c %s "$PROFILE")"
  echo "build_repeats=$BUILD_REPEATS"
  echo "runtime_repeats_per_build=$RUNTIME_REPEATS"
  echo "target_directories=fresh_per_variant_trial"
  echo "build_flags=matched line-tables-only; profile flags are the treatment"
  echo "profile_sample_use_flags=-Cdebuginfo=line-tables-only -Cprofile-sample-use=<profile> -Cllvm-args=-pgo-warn-missing-function"
  echo "trial_order=rotated_for_build_and_runtime"
  echo "observable_output_equivalence=EXACT_STDOUT_AND_STDERR"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/metadata/run-manifest.txt"
printf 'PASS_MECHANICAL_SAMPLE_PROFILE_AND_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
bash scripts/seal-rust-pgo-evidence.sh "$OUT" metadata/run-manifest.txt
printf 'results=%s\n' "$OUT"
printf 'RUSTC_TOOLCHAIN=%s bash scripts/analyze-rust-toolchain-benchmark.sh %s/metadata/runtime.csv no-pgo sample-pgo\n' "$TOOLCHAIN" "$OUT"
