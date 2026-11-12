#!/usr/bin/env bash
set -Eeuo pipefail

# Compare exact Rust toolchains with identical source/lock inputs, separate
# target directories, warmed registry cache, offline measured builds, rotated
# trial order, resource measurements and raw simulation output digests.

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

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  die "benchmark requires a clean worktree; commit or separately archive local changes first"
fi

LOCK_BEFORE="$(sha256sum Cargo.lock | awk '{print $1}')"
for toolchain in "${TOOLCHAINS[@]}"; do
  rustc_version="$(rustup run "$toolchain" rustc --version)" || die "toolchain not installed: $toolchain"
  cargo_version="$(rustup run "$toolchain" cargo --version)" || die "Cargo missing for $toolchain"
  case "$toolchain" in
    beta) [[ "$rustc_version" == *"-beta"* || "$rustc_version" == *"1.100.0"* ]] || die "beta selector did not resolve to a beta/release compiler: $rustc_version" ;;
    *) printf '%s\n' "$rustc_version" | grep -F "$toolchain" >/dev/null || die "unexpected rustc version: $rustc_version" ;;
  esac
  printf 'toolchain=%s rustc=%s cargo=%s\n' "$toolchain" "$rustc_version" "$cargo_version"
  rustup run "$toolchain" cargo fetch --locked
done

mkdir -p "$RESULT_ROOT"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$RESULT_ROOT/run-$RUN_ID"
mkdir -p "$OUT/targets" "$OUT/logs" "$OUT/results" "$OUT/toolchains"
CSV="$OUT/results/measurements.csv"

for toolchain in "${TOOLCHAINS[@]}"; do
  name="${toolchain//[^A-Za-z0-9._-]/_}"
  rustup run "$toolchain" rustc -Vv > "$OUT/toolchains/$name-rustc.txt"
  rustup run "$toolchain" cargo -Vv > "$OUT/toolchains/$name-cargo.txt"
done

{
  echo "schema=luminous.rust-toolchain-benchmark.v0.2"
  echo "commit=$(git rev-parse HEAD)"
  echo "worktree=clean"
  echo "cargo_lock_sha256=$LOCK_BEFORE"
  echo "date_utc=$RUN_ID"
  echo "kernel=$(uname -srmo)"
  echo "cpu_model=$(lscpu 2>/dev/null | grep -m1 'Model name:' | sed 's/^[^:]*:[[:space:]]*//' || echo unavailable)"
  echo "cpu_count=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
  echo "memory=$(grep -m1 '^MemTotal:' /proc/meminfo 2>/dev/null || echo unavailable)"
  echo "filesystem=$(stat -f -c %T "$OUT" 2>/dev/null || echo unknown)"
  echo "mount=$(findmnt -no SOURCE,FSTYPE,OPTIONS -T "$OUT" 2>/dev/null || echo unavailable)"
  echo "cpu_governor=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unavailable)"
  echo "build_repeats=$BUILD_REPEATS"
  echo "runtime_repeats=$RUNTIME_REPEATS"
  echo "cargo_build_jobs=${CARGO_BUILD_JOBS:-default}"
  echo "cache_policy=fresh-target-per-toolchain-trial; shared-warmed-registry; page-cache-not-dropped"
  echo "cargo_incremental=0"
  echo "cargo_network=offline_during_measurement"
  echo "sccache=disabled_for_primary_comparison"
  echo "examples=${EXAMPLES[*]}"
} > "$OUT/metadata.txt"

printf 'toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256\n' > "$CSV"
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
TIME_BIN="$(type -P gtime || type -P time || true)"

for ((trial=1; trial<=BUILD_REPEATS; trial++)); do
  # Rotate toolchain order to reduce systematic thermal/order bias.
  for ((offset=0; offset<${#TOOLCHAINS[@]}; offset++)); do
    index=$(((trial - 1 + offset) % ${#TOOLCHAINS[@]}))
    toolchain="${TOOLCHAINS[$index]}"

    for example in "${EXAMPLES[@]}"; do
      name="${toolchain//[^A-Za-z0-9._-]/_}"
      target_dir="$OUT/targets/$name/trial-$trial/$example"
      log_prefix="$OUT/logs/$name-trial-$trial-$example"
      mkdir -p "$target_dir"

      start_ns="$(date +%s%N)"
      set +e
      if [[ -n "$TIME_BIN" ]]; then
        "$TIME_BIN" -f 'max_rss_kib=%M' -o "$log_prefix-build.time" \
          env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
          rustup run "$toolchain" cargo build \
            --release --locked --offline --example "$example" \
            --target-dir "$target_dir" \
            > "$log_prefix-build.log" 2>&1
        status=$?
      else
        env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
          rustup run "$toolchain" cargo build \
            --release --locked --offline --example "$example" \
            --target-dir "$target_dir" \
            > "$log_prefix-build.log" 2>&1
        status=$?
      fi
      set -e

      end_ns="$(date +%s%N)"
      [[ $status -eq 0 ]] || {
        tail -n 80 "$log_prefix-build.log" >&2 || true
        die "build failed: toolchain=$toolchain trial=$trial example=$example"
      }
      build_ms=$(((end_ns - start_ns) / 1000000))
      rss="$(awk -F= '$1=="max_rss_kib" {print $2}' "$log_prefix-build.time" 2>/dev/null || echo NA)"
      target_bytes="$(du -sb "$target_dir" 2>/dev/null | awk '{print $1}' || echo NA)"

      binary="$target_dir/release/examples/$example"
      [[ -x "$binary" ]] || die "expected benchmark executable missing: $binary"
      binary_bytes="$(stat -c %s "$binary" 2>/dev/null || echo NA)"

      for ((runtime_repeat=1; runtime_repeat<=RUNTIME_REPEATS; runtime_repeat++)); do
        stdout="$log_prefix-run-$runtime_repeat.stdout"
        stderr="$log_prefix-run-$runtime_repeat.stderr"
        start_ns="$(date +%s%N)"
        "$binary" > "$stdout" 2> "$stderr"
        end_ns="$(date +%s%N)"
        runtime_ms=$(((end_ns - start_ns) / 1000000))
        digest="$(sha256sum "$stdout" | awk '{print $1}')"
        printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
          "$toolchain" "$trial" "$example" "$build_ms" "$rss" \
          "$target_bytes" "$binary_bytes" "$runtime_repeat" "$runtime_ms" "$digest" >> "$CSV"
      done
      printf 'trial=%s toolchain=%s example=%s build_ms=%s rss_kib=%s\n' \
        "$trial" "$toolchain" "$example" "$build_ms" "$rss"
    done
  done
done

LOCK_AFTER="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || die "Cargo.lock changed despite --locked; rejecting result"
printf 'benchmark_status=complete\n' > "$OUT/STATUS.txt"
printf 'cargo_lock_sha256_after=%s\n' "$LOCK_AFTER" >> "$OUT/metadata.txt"
printf 'Completed. Raw results: %s\n' "$OUT"
printf 'No winner is declared; analyze paired trial distributions and output digests.\n'
