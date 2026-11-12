#!/usr/bin/env bash
set -Eeuo pipefail

# Compare exact Rust toolchains using locked dependencies, independent target
# directories, a shared warmed Cargo registry cache, and rotated trial order.
# No cache dropping, host mutation, dependency update, or live service access.

read -r -a TOOLCHAINS <<< "${TOOLCHAINS:-1.96.1 1.99.1}"
BUILD_REPEATS="${BUILD_REPEATS:-5}"
RUNTIME_REPEATS="${RUNTIME_REPEATS:-5}"
RESULT_ROOT="${RESULT_ROOT:-$PWD/benchmark-results/rust-toolchain}"
EXAMPLES=(gradient_replacement cooperation_decomposition jphi_convergence)

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ "$BUILD_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "BUILD_REPEATS must be a positive integer"
[[ "$RUNTIME_REPEATS" =~ ^[1-9][0-9]*$ ]] || die "RUNTIME_REPEATS must be a positive integer"
(( ${#TOOLCHAINS[@]} >= 2 )) || die "Supply at least two exact toolchains in TOOLCHAINS"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
command -v rustup >/dev/null 2>&1 || die "rustup is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"

for toolchain in "${TOOLCHAINS[@]}"; do
  rustc_version="$(rustup run "$toolchain" rustc --version)" || die "toolchain not installed: $toolchain"
  cargo_version="$(rustup run "$toolchain" cargo --version)" || die "Cargo missing for $toolchain"
  printf '%s\n' "$rustc_version" | grep -F "$toolchain" >/dev/null || die "unexpected rustc version: $rustc_version"
  printf 'toolchain=%s rustc=%s cargo=%s\n' "$toolchain" "$rustc_version" "$cargo_version"
  rustup run "$toolchain" cargo fetch --locked
done

mkdir -p "$RESULT_ROOT"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/run-$RUN_ID"
mkdir -p "$OUT/targets" "$OUT/logs" "$OUT/results"
CSV="$OUT/results/measurements.csv"

cat > "$OUT/metadata.txt" <<EOF
schema=luminous.rust-toolchain-benchmark.v0.1
commit=$(git rev-parse HEAD)
dirty_worktree=$(test -z "$(git status --porcelain)" && echo false || echo true)
date_utc=$RUN_ID
kernel=$(uname -srmo)
cpu_model=$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)
cpu_count=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)
memory=$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)
filesystem=$(stat -f -c %T "$OUT" 2>/dev/null || echo unknown)
mount=$(findmnt -no SOURCE,FSTYPE,OPTIONS -T "$OUT" 2>/dev/null || echo unavailable)
cpu_governor=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)
build_repeats=$BUILD_REPEATS
runtime_repeats=$RUNTIME_REPEATS
cache_policy=fresh-target-per-toolchain-trial; shared-warmed-cargo-registry; page-cache-not-dropped
cargo_incremental=0
cargo_network=offline_during_measurement
examples=${EXAMPLES[*]}
EOF

printf 'toolchain,trial,example,build_ms,runtime_repeat,runtime_ms,stdout_sha256\n' > "$CSV"

export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true

for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  # Rotate toolchain order to reduce systematic thermal/order bias.
  for ((offset=0; offset<${#TOOLCHAINS[@]}; offset++)); do
    index=$(((trial - 1 + offset) % ${#TOOLCHAINS[@]}))
    toolchain="${TOOLCHAINS[$index]}"

    for example in "${EXAMPLES[@]}"; do
      target_dir="$OUT/targets/$toolchain/trial-$trial/$example"
      log_prefix="$OUT/logs/$toolchain-trial-$trial-$example"
      mkdir -p "$target_dir"

      start_ns="$(date +%s%N)"
      rustup run "$toolchain" cargo build \
        --release --locked --offline --example "$example" \
        --target-dir "$target_dir" \
        > "$log_prefix-build.log" 2>&1
      end_ns="$(date +%s%N)"
      build_ms=$(((end_ns - start_ns) / 1000000))

      binary="$target_dir/release/examples/$example"
      [[ -x "$binary" ]] || die "expected benchmark executable missing: $binary"

      for ((runtime_repeat=1; runtime_repeat<=RUNTIME_REPEATS; runtime_repeat++)); do
        stdout="$log_prefix-run-$runtime_repeat.stdout"
        stderr="$log_prefix-run-$runtime_repeat.stderr"
        start_ns="$(date +%s%N)"
        "$binary" > "$stdout" 2> "$stderr"
        end_ns="$(date +%s%N)"
        runtime_ms=$(((end_ns - start_ns) / 1000000))
        digest="$(sha256sum "$stdout" | awk '{print $1}')"
        printf '%s,%s,%s,%s,%s,%s,%s\n' \
          "$toolchain" "$trial" "$example" "$build_ms" \
          "$runtime_repeat" "$runtime_ms" "$digest" >> "$CSV"
      done
      printf 'trial=%s toolchain=%s example=%s build_ms=%s\n' \
        "$trial" "$toolchain" "$example" "$build_ms"
    done
  done
done

printf 'Completed. Raw results: %s\n' "$OUT"
printf 'No winner is declared; analyze paired trial distributions and output digests.\n'
