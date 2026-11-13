#!/usr/bin/env bash
set -Eeuo pipefail

# Sample-based PGO pilot. Requires Linux perf with branch-stack support and
# llvm-profgen built from an LLVM release compatible with rustc's LLVM major.
# This path is intentionally opt-in because hardware PMU access is not
# consistently available on shared/hosted runners.

TOOLCHAIN="${RUST_TOOLCHAIN:-1.100.0}"
EXAMPLE="${PGO_SAMPLE_EXAMPLE:-jphi_convergence}"
RUN_REPEATS="${PGO_RUN_REPEATS:-5}"
PERF_EVENT="${PERF_EVENT:-branches:u}"
RESULT_ROOT="${PGO_RESULT_ROOT:-${TMPDIR:-/tmp}/symtropy-rust-sample-pgo}"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
not_evaluable() { printf 'NOT_EVALUABLE: %s\n' "$*" >&2; exit 3; }

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || die "sample PGO requires a clean worktree"
[[ "$RUN_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "PGO_SAMPLE_REPEATS must be a positive integer"
[[ "$EXAMPLE" == "gradient_replacement" || "$EXAMPLE" == "cooperation_decomposition" || "$EXAMPLE" == "jphi_convergence" ]] || die "unsupported example: $EXAMPLE"

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $TOOLCHAIN "* ]] || die "expected exact Rust $TOOLCHAIN, got $RUSTC_VERSION"
[[ "$CARGO_VERSION" == "cargo $TOOLCHAIN "* ]] || die "expected exact Cargo $TOOLCHAIN, got $CARGO_VERSION"
RUSTC_VERBOSE="$(rustup run "$TOOLCHAIN" rustc -vV)"
HOST="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^host: //p')"
LLVM_VERSION="$(printf '%s\n' "$RUSTC_VERBOSE" | sed -n 's/^LLVM version: //p')"
[[ "$HOST" == "x86_64-unknown-linux-gnu" ]] || not_evaluable "initial sample-PGO pilot supports x86_64-unknown-linux-gnu only (host=$HOST)"
command -v perf >/dev/null 2>&1 || not_evaluable "Linux perf is not installed"
command -v llvm-profgen >/dev/null 2>&1 || not_evaluable "llvm-profgen is not installed; install a matching LLVM toolchain"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"

PROFGEN_VERSION="$(llvm-profgen --version 2>&1 | head -n 3)"
LLVM_MAJOR="$(printf '%s\n' "$LLVM_VERSION" | cut -d. -f1)"
PROFGEN_MAJOR="$(printf '%s\n' "$PROFGEN_VERSION" | sed -nE 's/.*LLVM version: ([0-9]+).*/\1/p' | head -n1)"
[[ -n "$PROFGEN_MAJOR" && "$PROFGEN_MAJOR" == "$LLVM_MAJOR" ]] || not_evaluable "llvm-profgen LLVM major ($PROFGEN_MAJOR) does not match rustc LLVM major ($LLVM_MAJOR)"

LOCK_BEFORE="$(sha256sum Cargo.lock | awk '{print $1}')"
COMMIT="$(git rev-parse HEAD)"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/$COMMIT-$TOOLCHAIN-$RUN_ID"
PROFILE="$OUT/$EXAMPLE.prof"
PERFDATA="$OUT/$EXAMPLE.perf.data"
BASE_TARGET="$OUT/target-baseline"
SAMPLE_TARGET="$OUT/target-sample-pgo"
mkdir -p "$OUT"/{baseline,sample-pgo,logs}

export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
rustup run "$TOOLCHAIN" cargo fetch --locked

# The training binary includes source line tables so perf samples can be
# symbolized back to LLVM IR/code regions.
set +e
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
  CARGO_TARGET_DIR="$BASE_TARGET" RUSTFLAGS="-Cdebuginfo=line-tables-only" \
  rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
    --target "$HOST" --example "$EXAMPLE" > "$OUT/logs/sample-profile-build.log" 2>&1
status=$?
set -e
[[ $status -eq 0 ]] || { tail -n 100 "$OUT/logs/sample-profile-build.log" >&2; die "sample profiling build failed"; }
BIN="$BASE_TARGET/$HOST/release/examples/$EXAMPLE"
[[ -x "$BIN" ]] || die "sample training binary missing: $BIN"

# LLVM's documented Rust path uses a branch-stack perf recording and then
# llvm-profgen to create the sample profile. PMU denial is a non-evaluable
# capability condition, not a compiler or PGO pass.
set +e
perf record -b -e "$PERF_EVENT" -o "$PERFDATA" -- "$BIN" \
  > "$OUT/logs/perf-training.stdout" 2> "$OUT/logs/perf-training.stderr"
status=$?
set -e
if [[ $status -ne 0 ]]; then
  not_evaluable "perf could not record branch samples (event=$PERF_EVENT); inspect $OUT/logs/perf-training.stderr"
fi
[[ -s "$PERFDATA" ]] || not_evaluable "perf produced no data"

llvm-profgen --binary="$BIN" --output="$PROFILE" --perfdata="$PERFDATA" \
  > "$OUT/logs/llvm-profgen.log" 2>&1 || {
    cat "$OUT/logs/llvm-profgen.log" >&2 || true
    die "llvm-profgen could not convert perf data"
  }
[[ -s "$PROFILE" ]] || die "sample profile is empty"

# Rebuild from the same source, compiler, target and line-table debug settings.
set +e
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER -u CARGO_ENCODED_RUSTFLAGS \
  CARGO_TARGET_DIR="$SAMPLE_TARGET" \
  RUSTFLAGS="-Cdebuginfo=line-tables-only -Cprofile-sample-use=$PROFILE -Cllvm-args=-pgo-warn-missing-function" \
  rustup run "$TOOLCHAIN" cargo build --release --locked --offline \
    --target "$HOST" --example "$EXAMPLE" > "$OUT/logs/sample-pgo-build.log" 2>&1
status=$?
set -e
[[ $status -eq 0 ]] || { tail -n 100 "$OUT/logs/sample-pgo-build.log" >&2; die "sample-PGO rebuild failed"; }

OPT_BIN="$SAMPLE_TARGET/$HOST/release/examples/$EXAMPLE"
[[ -x "$OPT_BIN" ]] || die "sample-PGO executable missing: $OPT_BIN"
printf 'variant,trial,wall_ms,max_rss_kib,binary_bytes,stdout_sha256\n' > "$OUT/measurements.csv"

measure() {
  local variant="$1" bin="$2" trial="$3"
  local stdout="$OUT/$variant/run-$trial.stdout" stderr="$OUT/$variant/run-$trial.stderr"
  local timing="$OUT/$variant/run-$trial.time" start end wall rss digest
  start="$(date +%s%N)"
  if [[ -x /usr/bin/time ]]; then
    /usr/bin/time -f '%e,%M' -o "$timing" "$bin" > "$stdout" 2> "$stderr"
  else
    "$bin" > "$stdout" 2> "$stderr"
    : > "$timing"
  fi
  end="$(date +%s%N)"
  wall=$(((end-start)/1000000))
  rss="$(awk -F, 'NF==2 {printf "%.0f", $2}' "$timing" 2>/dev/null || true)"
  [[ -n "$rss" ]] || rss=NA
  digest="$(sha256sum "$stdout" | awk '{print $1}')"
  printf '%s,%s,%s,%s,%s,%s\n' "$variant" "$trial" "$wall" "$rss" "$(stat -c %s "$bin")" "$digest" >> "$OUT/measurements.csv"
}
BASE_BIN="$BASE_TARGET/$HOST/release/examples/$EXAMPLE"
[[ -x "$BASE_BIN" ]] || die "baseline binary missing"
for ((i=1; i<=RUN_REPEATS; i++)); do
  measure baseline "$BASE_BIN" "$i"
  measure sample-pgo "$OPT_BIN" "$i"
done

BASE_DIGESTS="$(awk -F, '$1=="baseline" {print $6}' "$OUT/measurements.csv" | sort -u | wc -l | tr -d ' ')"
PGO_DIGESTS="$(awk -F, '$1=="sample-pgo" {print $6}' "$OUT/measurements.csv" | sort -u | wc -l | tr -d ' ')"
BASE_FIRST="$(awk -F, '$1=="baseline" {print $6; exit}' "$OUT/measurements.csv")"
PGO_FIRST="$(awk -F, '$1=="sample-pgo" {print $6; exit}' "$OUT/measurements.csv")"
[[ "$BASE_DIGESTS" == 1 && "$PGO_DIGESTS" == 1 && "$BASE_FIRST" == "$PGO_FIRST" ]] || die "sample-PGO changed or destabilized the observable experiment output"

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked"
{
  echo "schema=luminous.rust-pgo-sample.v0.1"
  echo "source_commit=$COMMIT"
  echo "cargo_lock_sha256=$LOCK_BEFORE"
  echo "rustc=$RUSTC_VERSION"
  echo "cargo=$CARGO_VERSION"
  echo "rustc_llvm=$LLVM_VERSION"
  echo "llvm_profgen=$PROFGEN_VERSION"
  echo "target=$HOST"
  echo "training_example=$EXAMPLE"
  echo "perf_event=$PERF_EVENT"
  echo "perf_data_sha256=$(sha256sum "$PERFDATA" | awk '{print $1}')"
  echo "sample_profile_sha256=$(sha256sum "$PROFILE" | awk '{print $1}')"
  echo "sample_profile_bytes=$(stat -c %s "$PROFILE")"
  echo "run_repeats=$RUN_REPEATS"
  echo "observable_output_equivalence=EXACT_STDOUT_MATCH"
  echo "performance_claim=NOT_ESTABLISHED_BY_THIS_SCRIPT"
} > "$OUT/manifest.txt"
printf 'PASS_MECHANICAL_SAMPLE_PROFILE_AND_OUTPUT_CHECKS\n' > "$OUT/STATUS.txt"
printf 'results=%s\n' "$OUT"
