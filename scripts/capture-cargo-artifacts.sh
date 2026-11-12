#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-beta}"
OUTPUT="${2:?usage: $0 TOOLCHAIN OUTPUT_DIRECTORY}"
ROOT="$(git rev-parse --show-toplevel)"
mkdir -p "$OUTPUT"
OUTPUT="$(realpath "$OUTPUT")"
RAW="$OUTPUT/cargo-messages.jsonl"
DIAGNOSTICS="$OUTPUT/diagnostics.log"
MANIFEST="$OUTPUT/compiler-artifacts.jsonl"

cd "$ROOT"
rustup run "$TOOLCHAIN" cargo build \
  --locked --all-targets --offline \
  --message-format=json-render-diagnostics \
  > "$RAW" 2> "$DIAGNOSTICS"

command -v jq >/dev/null 2>&1 || {
  echo "jq is required to extract Cargo JSON messages" >&2
  exit 2
}

jq -c '
  select(.reason == "compiler-artifact")
  | {
      package_id,
      target,
      filenames,
      executable,
      fresh
    }
' "$RAW" > "$MANIFEST"

if [[ ! -s "$MANIFEST" ]]; then
  echo "Cargo emitted no compiler-artifact messages; refusing empty manifest." >&2
  cat "$DIAGNOSTICS" >&2 || true
  exit 1
fi

jq -s 'length' "$MANIFEST" | awk '$1 > 0 { exit 0 } { exit 1 }'
printf 'artifact_manifest=%s\n' "$MANIFEST"
printf 'artifact_records=%s\n' "$(wc -l < "$MANIFEST" | tr -d ' ')"
printf 'cargo_layout_policy=JSON_MESSAGES_NOT_PRIVATE_DIRECTORY_GUESSES\n'
