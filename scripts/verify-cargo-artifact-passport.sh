#!/usr/bin/env bash
set -Eeuo pipefail

OUTPUT="${1:?usage: $0 OUTPUT_DIRECTORY [REPOSITORY_ROOT]}"
ROOT="${2-}"
OUTPUT="$(realpath -e "$OUTPUT")"
TARGET_DIR="$OUTPUT/target"
PASSPORT="$OUTPUT/passport.json"
sha256_file() { sha256sum -- "$1" | awk '{print $1}'; }

[[ -f "$PASSPORT" && -f "$OUTPUT/passport.sha256" ]] || { echo "passport or digest file missing" >&2; exit 2; }
jq -e '.schema=="luminous.cargo-artifact-passport.v0.1"' "$PASSPORT" >/dev/null
[[ "$(awk '{print $1}' "$OUTPUT/passport.sha256")" == "$(sha256_file "$PASSPORT")" ]] || {
  echo "passport digest mismatch" >&2; exit 1;
}

verify_hash() {
  local field="$1" file="$2"
  [[ "$(jq -r ".inputs.$field" "$PASSPORT")" == "$(sha256_file "$file")" ]] || {
    echo "input digest mismatch: $file" >&2; return 1;
  }
}
verify_hash cargo_metadata_sha256 "$OUTPUT/cargo-metadata.json"
verify_hash cargo_messages_sha256 "$OUTPUT/cargo-messages.jsonl"
verify_hash diagnostics_sha256 "$OUTPUT/diagnostics.log"
verify_hash rustc_version_file_sha256 "$OUTPUT/rustc-version.txt"
verify_hash cargo_version_file_sha256 "$OUTPUT/cargo-version.txt"
verify_hash compiler_artifacts_file_sha256 "$OUTPUT/compiler-artifacts.json"

expected_records="$(jq -r '.artifacts.compiler_artifact_count' "$PASSPORT")"
actual_records="$(jq 'length' "$OUTPUT/compiler-artifacts.json")"
[[ "$expected_records" == "$actual_records" && "$actual_records" -gt 0 ]] || {
  echo "artifact record count mismatch or empty artifact manifest" >&2; exit 1;
}

while IFS=
if [[ -n "$ROOT" ]]; then
  ROOT="$(realpath -e "$ROOT")"
  [[ "$(git -C "$ROOT" rev-parse HEAD)" == "$(jq -r '.source.git_commit' "$PASSPORT")" ]] || { echo "source commit mismatch" >&2; exit 1; }
  [[ "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" == "$(jq -r '.source.git_tree' "$PASSPORT")" ]] || { echo "source tree mismatch" >&2; exit 1; }
  lock_path="$(jq -r '.source.lockfile_path' "$PASSPORT")"
  manifest_path="$(jq -r '.source.manifest_path' "$PASSPORT")"
  [[ "$(sha256_file "$ROOT/$lock_path")" == "$(jq -r '.source.cargo_lock_sha256' "$PASSPORT")" ]] || { echo "Cargo.lock mismatch" >&2; exit 1; }
  [[ "$(sha256_file "$ROOT/$manifest_path")" == "$(jq -r '.source.manifest_sha256' "$PASSPORT")" ]] || { echo "manifest mismatch" >&2; exit 1; }
fi

printf 'PASS_ARTIFACT_PASSPORT_INTEGRITY_ONLY\n'
printf 'compiler_artifact_records=%s\n' "$actual_records"
printf 'artifact_files=%s\n' "$(jq -r '.artifacts.file_count' "$PASSPORT")"
\t' read -r artifact_root relative expected_sha expected_size; do
  [[ -n "$relative" ]] || continue
  case "$relative" in /*|../*|*/../*|*/..) echo "unsafe artifact path: $relative" >&2; exit 1 ;; esac
  case "$artifact_root" in
    target) artifact_base="$TARGET_DIR" ;;
    build) artifact_base="$OUTPUT/build" ;;
    *) echo "unknown Cargo artifact root: $artifact_root" >&2; exit 1 ;;
  esac
  resolved="$(realpath -e "$artifact_base/$relative")"
  case "$resolved" in "$artifact_base"/*) ;; *) echo "artifact escapes recorded Cargo root: $relative" >&2; exit 1 ;; esac
  [[ "$(sha256_file "$resolved")" == "$expected_sha" ]] || { echo "artifact hash mismatch: $relative" >&2; exit 1; }
  [[ "$(stat -c '%s' -- "$resolved")" == "$expected_size" ]] || { echo "artifact size mismatch: $relative" >&2; exit 1; }
done < <(jq -r '.artifacts.compiler_artifacts[] | .files[] | [.root,.path,.sha256,(.size_bytes|tostring)] | @tsv' "$PASSPORT")

if [[ -n "$ROOT" ]]; then
  ROOT="$(realpath -e "$ROOT")"
  [[ "$(git -C "$ROOT" rev-parse HEAD)" == "$(jq -r '.source.git_commit' "$PASSPORT")" ]] || { echo "source commit mismatch" >&2; exit 1; }
  [[ "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" == "$(jq -r '.source.git_tree' "$PASSPORT")" ]] || { echo "source tree mismatch" >&2; exit 1; }
  lock_path="$(jq -r '.source.lockfile_path' "$PASSPORT")"
  manifest_path="$(jq -r '.source.manifest_path' "$PASSPORT")"
  [[ "$(sha256_file "$ROOT/$lock_path")" == "$(jq -r '.source.cargo_lock_sha256' "$PASSPORT")" ]] || { echo "Cargo.lock mismatch" >&2; exit 1; }
  [[ "$(sha256_file "$ROOT/$manifest_path")" == "$(jq -r '.source.manifest_sha256' "$PASSPORT")" ]] || { echo "manifest mismatch" >&2; exit 1; }
fi

printf 'PASS_ARTIFACT_PASSPORT_INTEGRITY_ONLY\n'
printf 'compiler_artifact_records=%s\n' "$actual_records"
printf 'artifact_files=%s\n' "$(jq -r '.artifacts.file_count' "$PASSPORT")"
