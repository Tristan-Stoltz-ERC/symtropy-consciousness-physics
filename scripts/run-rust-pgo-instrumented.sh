#!/usr/bin/env bash
set -Eeuo pipefail

# Instrumentation-based PGO qualification. The profile is generated and
# consumed on the same exact source/compiler/target. This script emits raw
# paired measurements and fails closed if observable outputs change.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
EXPECTED_RUSTC="rustc ${TOOLCHAIN} "
EXPECTED_CARGO="cargo ${TOOLCHAIN} "
TRAIN_EXAMPLE="${PGO_TRAIN_EXAMPLE:-jphi_convergence}"
RUN_REPEATS="${PGO_RUN_REPEATS:-5}"
TARGET="${PGO_TARGET:-}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-rust-pgo}"
VALIDATION_EXAMPLES=()
read -r -a VALIDATION_EXAMPLES <<< "${PGO_VALIDATION_EXAMPLES:-gradient_replacement cooperation_decomposition}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ "$RUN_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUN_REPEATS must be a positive integer"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"
[[ -f Cargo.lock ]] || die "Cargo.lock is required"

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  die "PGO qualification requires a clean worktree"
fi

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "$EXPECTED_RUSTC"* ]] || die "expected exact Rust $TOOLCHAIN; got $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "$EXPECTED_CARGO"* ]] || die "expected exact Cargo $TOOLCHAIN; got $CARGO_VERSION"
RUSTC_VERBOSE="$(rustup run "$TOOLCHAIN" rustc -vV)"
HOST="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^host: //p')"
LLVM_VERSION="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')"
[[ -n "$HOST" && -n "$LLVM_VERSION" ]] || die "could not identify compiler host/LLVM version"
[[ -n "$TARGET" ]] || TARGET="$HOST"
[[ "$TARGET" == "$HOST" ]] || die "initial PGO pilot is host-native only; host=$HOST target=$TARGET"

case "$TRAIN_EXAMPLE" in
  gradient_replacement|cooperation_decomposition|jphi_convergence) ;;
  *) die "unsupported train example: $TRAIN_EXAMPLE" ;;
esac
EXAMPLES=("$TRAIN_EXAMPLE")
for example in "${VALIDATION_EXAMPLES[@]}"; do
  case "$example" in
    gradient_replacement|cooperation_decomposition|jphi_convergence) ;;
    *) die "unsupported validation example: $example" ;;
  esac
  if [[ "$example" != "$TRAIN_EXAMPLE" ]]; then EXAMPLES+=("$example"); fi
done

LOCK_BEFORE="$(sha256sum Cargo.lock | awk '{print $1}')"
SOURCE_COMMIT="$(git rev-parse HEAD)"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/$SOURCE_COMMIT-$TOOLCHAIN-$RUN_ID"
PROFILE_DIR="$OUT/profile-data"
BASE_TARGET="$OUT/target-no-pgo"
INSTR_TARGET="$OUT/target-instrumented"
OPT_TARGET="$OUT/target-pgo"
mkdir -p "$OUT"/{baseline,instrumented,pgo,logs,profiles,metadata}
mkdir -p "$PROFILE_DIR"

export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export CARGO_TERM_COLOR=never

rustup run "$TOOLCHAIN" cargo fetch --locked
rustup run "$TOOLCHAIN" cargo metadata --locked --format-version 1 --no-deps > "$OUT/metadata/cargo-metadata.json"

declare -A BASE_BUILD_MS PGO_BUILD_MS

build_example() {
  local variant="$1" target_dir="$2" example="$3" rustflags="$4"
  local log="$OUT/logs/$variant-build-$example.log"
  local start_ns end_ns status elapsed_ms

  start_ns="$(date +%s%N)"
  set +e
  if [[ -n "$rustflags" ]]; then
    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
      CARGO_TARGET_DIR="$target_dir" RUSTFLAGS="$rustflags" \
      rustup run "$TOOLCHAIN" cargo build \
        --release --locked --offline --target "$TARGET" --example "$example" \
        > "$log" 2>&1
    status=$?
  else
    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
      -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
      CARGO_TARGET_DIR="$target_dir" \
      rustup run "$TOOLCHAIN" cargo build \
        --release --locked --offline --target "$TARGET" --example "$example" \
        > "$log" 2>&1
    status=$?
  fi
  set -e
  end_ns="$(date +%s%N)"
  elapsed_ms=$(((end_ns - start_ns) / 1000000))
  if [[ $status -ne 0 ]]; then
    tail -n 100 "$log" >&2 || true
    die "$variant build failed for $example"
  fi
  printf '%s\n' "$elapsed_ms"
}

run_example() {
  local variant="$1" target_dir="$2" example="$3" trial="$4" build_ms="$5"
  local bin="$target_dir/$TARGET/release/examples/$example"
  local stdout="$OUT/$variant/$example-$trial.stdout"
  local stderr="$OUT/$variant/$example-$trial.stderr"
  local timing="$OUT/$variant/$example-$trial.time"
  local start_ns end_ns wall_ms rss_kib stdout_sha stderr_sha binary_bytes target_bytes
  [[ -x "$bin" ]] || die "executable missing: $bin"
  start_ns="$(date +%s%N)"
  if [[ -x /usr/bin/time ]]; then
    /usr/bin/time -f '%e,%M' -o "$timing" "$bin" > "$stdout" 2> "$stderr"
  else
    "$bin" > "$stdout" 2> "$stderr"
    : > "$timing"
  fi
  end_ns="$(date +%s%N)"
  wall_ms=$(((end_ns - start_ns) / 1000000))
  rss_kib="$(awk -F, 'NF==2 {printf "%.0f", $2}' "$timing" 2>/dev/null || true)"
  [[ -n "$rss_kib" ]] || rss_kib="NA"
  stdout_sha="$(sha256sum "$stdout" | awk '{print $1}')"
  stderr_sha="$(sha256sum "$stderr" | awk '{print $1}')"
  binary_bytes="$(stat -c %s "$bin")"
  target_bytes="$(du -sb "$target_dir" | awk '{print $1}')"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$variant" "$trial" "$example" "$build_ms" "$rss_kib" \
    "$target_bytes" "$binary_bytes" "1" "$wall_ms" "$stdout_sha" \
    >> "$OUT/metadata/runtime.csv"
  printf '%s,%s,%s,%s,%s\n' "$variant" "$trial" "$example" "$stdout_sha" "$stderr_sha" \
    >> "$OUT/metadata/output-digests.csv"
  printf '%s\n' "$stdout_sha"
}

printf 'toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256\n' > "$OUT/metadata/runtime.csv"
printf 'variant,trial,example,stdout_sha256,stderr_sha256\n' > "$OUT/metadata/output-digests.csv"
printf 'variant,example,build_ms\n' > "$OUT/metadata/builds.csv"

# 1. Build baseline release binaries without PGO.
for example in "${EXAMPLES[@]}"; do
  elapsed="$(build_example baseline "$BASE_TARGET" "$example" "")"
  BASE_BUILD_MS["$example"]="$elapsed"
  printf 'no-pgo,%s,%s\n' "$example" "$elapsed" >> "$OUT/metadata/builds.csv"
done

# 2. Instrument only the declared training executable. Explicit --target keeps
# Cargo from propagating RUSTFLAGS into build scripts.
elapsed="$(build_example instrumented "$INSTR_TARGET" "$TRAIN_EXAMPLE" "-Cprofile-generate=$PROFILE_DIR -Cdebuginfo=line-tables-only")"
INSTR_BIN="$INSTR_TARGET/$TARGET/release/examples/$TRAIN_EXAMPLE"
[[ -x "$INSTR_BIN" ]] || die "instrumented training executable missing: $INSTR_BIN"
printf 'instrumentation,%s,%s\n' "$TRAIN_EXAMPLE" "$elapsed" >> "$OUT/metadata/builds.csv"
for ((i=1; i<=2; i++)); do
  "$INSTR_BIN" > "$OUT/instrumented/training-$i.stdout" 2> "$OUT/instrumented/training-$i.stderr"
done

shopt -s nullglob
PROFRAW_FILES=("$PROFILE_DIR"/*.profraw)
(( ${#PROFRAW_FILES[@]} > 0 )) || die "training produced no .profraw files"
rustup component add llvm-tools-preview --toolchain "$TOOLCHAIN"
SYSROOT="$(rustup run "$TOOLCHAIN" rustc --print sysroot)"
LLVM_PROFDATA="$(find "$SYSROOT/lib/rustlib" -type f -name llvm-profdata -perm -u+x -print -quit)"
[[ -x "$LLVM_PROFDATA" ]] || die "llvm-profdata was not found in llvm-tools-preview"
"$LLVM_PROFDATA" merge --failure-mode=any -o "$PROFILE_DIR/merged.profdata" "${PROFRAW_FILES[@]}"
[[ -s "$PROFILE_DIR/merged.profdata" ]] || die "merged profile is empty"
"$LLVM_PROFDATA" show --all-functions "$PROFILE_DIR/merged.profdata" > "$OUT/metadata/profile-summary.txt"

# 3. Build using the exact profile. Cross-example runs are transfer probes, not
# an IID hold-out split; profile-coverage warnings remain visible in build logs.
for example in "${EXAMPLES[@]}"; do
  elapsed="$(build_example pgo "$OPT_TARGET" "$example" "-Cprofile-use=$PROFILE_DIR/merged.profdata -Cdebuginfo=line-tables-only -Cllvm-args=-pgo-warn-missing-function")"
  PGO_BUILD_MS["$example"]="$elapsed"
  printf 'instrumentation-pgo,%s,%s\n' "$example" "$elapsed" >> "$OUT/metadata/builds.csv"
done

# 4. Matched runtime observations; each workload uses the same repeat IDs.
declare -A BASE_OUT BASE_ERR PGO_OUT PGO_ERR
for example in "${EXAMPLES[@]}"; do
  for ((trial=1; trial<=RUN_REPEATS; trial++)); do
    base_sha="$(run_example no-pgo "$BASE_TARGET" "$example" "$trial" "${BASE_BUILD_MS[$example]}")"
    pgo_sha="$(run_example instrumentation-pgo "$OPT_TARGET" "$example" "$trial" "${PGO_BUILD_MS[$example]}")"
    base_err="$(awk -F, -v t="$trial" -v e="$example" '$1=="no-pgo" && $2==t && $3==e {print $5}' "$OUT/metadata/output-digests.csv")"
    pgo_err="$(awk -F, -v t="$trial" -v e="$example" '$1=="instrumentation-pgo" && $2==t && $3==e {print $5}' "$OUT/metadata/output-digests.csv")"
    if [[ $trial -eq 1 ]]; then
      BASE_OUT["$example"]="$base_sha"; BASE_ERR["$example"]="$base_err"
      PGO_OUT["$example"]="$pgo_sha"; PGO_ERR["$example"]="$pgo_err"
    else
      [[ "$base_sha" == "${BASE_OUT[$example]}" && "$base_err" == "${BASE_ERR[$example]}" ]] || die "no-PGO output is not deterministic across repeats: $example"
      [[ "$pgo_sha" == "${PGO_OUT[$example]}" && "$pgo_err" == "${PGO_ERR[$example]}" ]] || die "PGO output is not deterministic across repeats: $example"
    fi
  done
  [[ "${BASE_OUT[$example]}" == "${PGO_OUT[$example]}" && "${BASE_ERR[$example]}" == "${PGO_ERR[$example]}" ]] || {
    echo "OUTPUT_REVIEW_REQUIRED: PGO changed stdout/stderr digest for $example" | tee -a "$OUT/STATUS.txt"
    echo "No correctness/equivalence pass is claimed; inspect retained logs and rerun invariants." >&2
    exit 3
  }
done

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked"
{
  echo "schema=luminous.rust-pgo-instrumentation.v0.1"
  echo "source_commit=$SOURCE_COMMIT"
  echo "cargo_lock_sha256=$LOCK_BEFORE"
  echo "toolchain=$TOOLCHAIN"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "llvm=$LLVM_VERSION"
  echo "target=$TARGET"
  echo "training_example=$TRAIN_EXAMPLE"
  echo "validation_examples=${VALIDATION_EXAMPLES[*]}"
  echo "run_repeats=$RUN_REPEATS"
  echo "profile_sha256=$(sha256sum "$PROFILE_DIR/merged.profdata" | awk '{print $1}')"
  echo "profile_bytes=$(stat -c %s "$PROFILE_DIR/merged.profdata")"
  echo "cargo_lock_sha256_after=$LOCK_AFTER"
  echo "cache_policy=fresh-target-per-variant; warmed-registry; offline-measured-builds"
  echo "sccache=disabled"
  echo "observable_output_equivalence=EXACT_STDOUT_AND_STDERR_MATCH"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/metadata/run-manifest.txt"
printf 'PASS_MECHANICAL_PROFILE_AND_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
printf 'results=%s\n' "$OUT"
printf 'Analyze exploratory paired results with:\n'
printf 'RUSTC_TOOLCHAIN=%s bash scripts/analyze-rust-toolchain-benchmark.sh %s no-pgo instrumentation-pgo\n' "$TOOLCHAIN" "$OUT/metadata/runtime.csv"
