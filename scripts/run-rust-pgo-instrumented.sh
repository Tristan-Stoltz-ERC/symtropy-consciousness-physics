#!/usr/bin/env bash
set -Eeuo pipefail

# Instrumentation-based PGO qualification for the Symtropy simulation.
# The profile is generated and consumed on the same exact source/compiler/target.
# The script makes no performance claim; it emits raw paired measurements and
# fails closed if any observable stdout digest changes under PGO.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
EXPECTED_RUSTC="rustc ${TOOLCHAIN} "
EXPECTED_CARGO="cargo ${TOOLCHAIN} "
TRAIN_EXAMPLE="${PGO_TRAIN_EXAMPLE:-jphi_convergence}"
RUN_REPEATS="${PGO_RUN_REPEATS:-3}"
TARGET="${PGO_TARGET:-}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-rust-pgo}"
VALIDATION_EXAMPLES=(${PGO_VALIDATION_EXAMPLES:-gradient_replacement cooperation_decomposition})

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ "$RUN_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_RUN_REPEATS must be a positive integer"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"
command -v cargo >/dev/null 2>&1 || die "Cargo shim is required"
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
if [[ -z "$TARGET" ]]; then TARGET="$HOST"; fi
[[ "$TARGET" == "$HOST" ]] || die "this initial PGO pilot is host-native only; host=$HOST target=$TARGET"

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
  [[ "$example" == "$TRAIN_EXAMPLE" ]] || EXAMPLES+=("$example")
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

# Warm the exact locked dependency graph before the measured offline builds.
rustup run "$TOOLCHAIN" cargo fetch --locked
rustup run "$TOOLCHAIN" cargo metadata --locked --format-version 1 --no-deps > "$OUT/metadata/cargo-metadata.json"

build_example() {
  local variant="$1" target_dir="$2" example="$3" rustflags="$4"
  local log="$OUT/logs/$variant-build-$example.log"
  local start_ns end_ns status elapsed_ms

  start_ns="$(date +%s%N)"
  set +e
  if [[ -n "$rustflags" ]]; then
    env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
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
  local variant="$1" target_dir="$2" example="$3" trial="$4"
  local bin="$target_dir/$TARGET/release/examples/$example"
  local stdout="$OUT/$variant/$example-$trial.stdout"
  local stderr="$OUT/$variant/$example-$trial.stderr"
  local timing="$OUT/$variant/$example-$trial.time"
  local start_ns end_ns wall_ms rss_kib stdout_sha stderr_sha
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
  printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$variant" "$trial" "$example" "$wall_ms" "$rss_kib" \
    "$(stat -c %s "$bin")" \
    "$(du -sb "$target_dir" | awk '{print $1}')" \
    "$stdout_sha" >> "$OUT/metadata/runtime.csv"
  printf '%s\n' "$stdout_sha"
}

printf 'variant,trial,example,wall_ms,max_rss_kib,binary_bytes,target_bytes,stdout_sha256\n' > "$OUT/metadata/runtime.csv"
printf 'variant,example,build_ms\n' > "$OUT/metadata/builds.csv"

# 1. Baseline release binaries without PGO.
for example in "${EXAMPLES[@]}"; do
  elapsed="$(build_example baseline "$BASE_TARGET" "$example" "")"
  printf 'no-pgo,%s,%s\n' "$example" "$elapsed" >> "$OUT/metadata/builds.csv"
done

# 2. Instrument one declared training workload. --target is intentional:
# Cargo does not pass RUSTFLAGS to build scripts when an explicit target is used.
elapsed="$(build_example instrumented "$INSTR_TARGET" "$TRAIN_EXAMPLE" "-Cprofile-generate=$PROFILE_DIR -Cdebuginfo=line-tables-only")"
printf 'instrumentation,%s,%s\n' "$TRAIN_EXAMPLE" "$elapsed" >> "$OUT/metadata/builds.csv"
INSTR_BIN="$INSTR_TARGET/$TARGET/release/examples/$TRAIN_EXAMPLE"
[[ -x "$INSTR_BIN" ]] || die "instrumented training executable missing: $INSTR_BIN"
for ((i=1; i<=2; i++)); do
  "$INSTR_BIN" > "$OUT/instrumented/training-$i.stdout" 2> "$OUT/instrumented/training-$i.stderr"
done

shopt -s nullglob
PROFRAW_FILES=("$PROFILE_DIR"/*.profraw)
(( ${#PROFRAW_FILES[@]} > 0 )) || die "training produced no .profraw files"
rustup component add llvm-tools-preview --toolchain "$TOOLCHAIN"
SYSROOT="$(rustup run "$TOOLCHAIN" rustc --print sysroot)"
LLVM_PROFDATA="$(find "$SYSROOT/lib/rustlib" -type f -name llvm-profdata -perm -u+x -print -quit)"
[[ -x "$LLVM_PROFDATA" ]] || die "llvm-profdata was not found in the exact toolchain's llvm-tools-preview"
"$LLVM_PROFDATA" merge --failure-mode=any -o "$PROFILE_DIR/merged.profdata" "${PROFRAW_FILES[@]}"
[[ -s "$PROFILE_DIR/merged.profdata" ]] || die "merged profile is empty"
"$LLVM_PROFDATA" show --all-functions "$PROFILE_DIR/merged.profdata" > "$OUT/metadata/profile-summary.txt"

# 3. Build the same examples using the exact profile just produced.
for example in "${EXAMPLES[@]}"; do
  elapsed="$(build_example pgo "$OPT_TARGET" "$example" "-Cprofile-use=$PROFILE_DIR/merged.profdata -Cdebuginfo=line-tables-only -Cllvm-args=-pgo-warn-missing-function")"
  printf 'instrumentation-pgo,%s,%s\n' "$example" "$elapsed" >> "$OUT/metadata/builds.csv"
done

# 4. Matched runtime observations. Each variant gets the same repeat count.
declare -A FIRST_BASE FIRST_PGO
for example in "${EXAMPLES[@]}"; do
  for ((trial=1; trial<=RUN_REPEATS; trial++)); do
    base_sha="$(run_example no-pgo "$BASE_TARGET" "$example" "$trial")"
    pgo_sha="$(run_example instrumentation-pgo "$OPT_TARGET" "$example" "$trial")"
    if [[ $trial -eq 1 ]]; then
      FIRST_BASE["$example"]="$base_sha"
      FIRST_PGO["$example"]="$pgo_sha"
    else
      [[ "$base_sha" == "${FIRST_BASE[$example]}" ]] || die "no-PGO output is not deterministic across repeats: $example"
      [[ "$pgo_sha" == "${FIRST_PGO[$example]}" ]] || die "PGO output is not deterministic across repeats: $example"
    fi
  done
  [[ "${FIRST_BASE[$example]}" == "${FIRST_PGO[$example]}" ]] || {
    echo "OUTPUT_REVIEW_REQUIRED: PGO changed stdout digest for $example" | tee -a "$OUT/STATUS.txt"
    echo "No correctness/equivalence pass is claimed; inspect retained CSV/stderr and rerun invariant tests." >&2
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
  echo "observable_output_equivalence=EXACT_STDOUT_MATCH"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/metadata/run-manifest.txt"
printf 'PASS_MECHANICAL_PROFILE_AND_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
printf 'results=%s\n' "$OUT"
printf 'Analyze exploratory paired results with:\n'
printf 'RUSTC_TOOLCHAIN=%s bash scripts/analyze-rust-toolchain-benchmark.sh %s  no-pgo instrumentation-pgo\n' "$TOOLCHAIN" "$OUT/metadata/runtime.csv"
