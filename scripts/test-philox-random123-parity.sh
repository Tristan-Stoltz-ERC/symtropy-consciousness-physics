#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="\${1:-1.100.0}"
CORPUS_COUNT="\${2:-4096}"
ROOT="$(git rev-parse --show-toplevel)"
RANDOM123_COMMIT="ff4443af61194bb57a042cd4945f2c73bdf1d0dd"
WORK="$(mktemp -d)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

command -v git >/dev/null 2>&1
command -v g++ >/dev/null 2>&1
command -v rustup >/dev/null 2>&1

# Fetch exactly the reviewed reference revision; do not follow a moving branch.
git -C "$WORK" init -q random123
git -C "$WORK/random123" remote add origin https://github.com/lemire/random123.git
git -C "$WORK/random123" fetch --quiet --depth=1 origin "$RANDOM123_COMMIT"
git -C "$WORK/random123" checkout --quiet --detach FETCH_HEAD
[[ "$(git -C "$WORK/random123" rev-parse HEAD)" == "$RANDOM123_COMMIT" ]] || {
  echo "Random123 checkout did not match the pinned revision" >&2
  exit 1
}

rustup run "$TOOLCHAIN" rustc -Vv > "$WORK/rustc-version.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$WORK/cargo-version.txt"
g++ --version | head -n1 > "$WORK/cxx-version.txt"

g++ -std=c++11 -O2 \
  -I"$WORK/random123/include" \
  "$ROOT/tests/reference/philox4x64_random123.cpp" \
  -o "$WORK/philox-reference"

rustup run "$TOOLCHAIN" cargo fetch --locked
rustup run "$TOOLCHAIN" cargo run --locked --offline \
  --example event_rng_reference_corpus -- "$CORPUS_COUNT" \
  > "$WORK/reference-corpus.txt"

"$WORK/philox-reference" < "$WORK/reference-corpus.txt"

printf 'random123_commit=%s\n' "$RANDOM123_COMMIT"
printf 'rustc=%s\n' "$(head -n1 "$WORK/rustc-version.txt")"
printf 'cargo=%s\n' "$(head -n1 "$WORK/cargo-version.txt")"
printf 'cxx=%s\n' "$(cat "$WORK/cxx-version.txt")"
printf 'corpus_sha256=%s\n' "$(sha256sum "$WORK/reference-corpus.txt" | awk '{print $1}')"
