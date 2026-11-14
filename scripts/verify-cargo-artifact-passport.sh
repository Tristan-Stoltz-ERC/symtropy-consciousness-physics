#!/usr/bin/env bash
set -Eeuo pipefail
die() { echo "ERROR: $*" >&2; exit 2; }
[[ $# -ge 1 && -n "$1" ]] || die "usage: $0 OUTPUT [REPOSITORY_ROOT]"
output="$(realpath -e "$1")"
root=""
[[ $# -ge 2 && -n "$2" ]] && root="$(realpath -e "$2")"
target_dir="$output/target"
build_dir="$output/build"
passport="$output/passport.json"
sha() { sha256sum -- "$1" | awk '{print $1}'; }
[[ -f "$passport" && -f "$output/passport.sha256" ]] || die "passport or digest missing"
jq -e '.schema=="luminous.cargo-artifact-passport.v0.1"' "$passport" >/dev/null
[[ "$(awk '{print $1}' "$output/passport.sha256")" == "$(sha "$passport")" ]] || die "passport digest mismatch"

verify_hash() {
  local field="$1" file="$2"
  [[ -f "$file" ]] || die "missing input: $file"
  [[ "$(jq -r ".inputs.$field" "$passport")" == "$(sha "$file")" ]] || die "input digest mismatch: $field"
}
verify_hash cargo_metadata_sha256 "$output/cargo-metadata.json"
verify_hash cargo_messages_sha256 "$output/cargo-messages.jsonl"
verify_hash diagnostics_sha256 "$output/diagnostics.log"
verify_hash rustc_version_file_sha256 "$output/rustc-version.txt"
verify_hash cargo_version_file_sha256 "$output/cargo-version.txt"
verify_hash compiler_artifacts_file_sha256 "$output/compiler-artifacts.json"

expected_records="$(jq -r '.artifacts.compiler_artifact_count' "$passport")"
actual_records="$(jq 'length' "$output/compiler-artifacts.json")"
expected_files="$(jq -r '.artifacts.file_count' "$passport")"
actual_files="$(jq '[.[]|.files[]]|length' "$output/compiler-artifacts.json")"
[[ "$expected_records" == "$actual_records" && "$actual_records" -gt 0 ]] || die "empty or inconsistent compiler-artifact records"
[[ "$expected_files" == "$actual_files" && "$actual_files" -gt 0 ]] || die "empty or inconsistent artifact files"

# The manifest may only contain paths explicitly reported by Cargo.
reported="$output/reported-paths.verify"
inventory="$output/inventory-paths.verify"
: > "$reported"
while IFS= read -r filename; do
  [[ -n "$filename" ]] || continue
  resolved="$(realpath -e "$filename")"
  case "$resolved" in
    "$target_dir"/*) artifact_root="target"; base="$target_dir" ;;
    "$build_dir"/*) artifact_root="build"; base="$build_dir" ;;
    *) die "Cargo message path escapes isolated roots: $resolved" ;;
  esac
  relative="$(realpath --relative-to="$base" "$resolved")"
  [[ "$relative" != *$'\n'* && "$relative" != *$'\t'* ]] || die "unsupported control character in artifact path"
  printf '%s\t%s\n' "$artifact_root" "$relative" >> "$reported"
done < <(jq -r 'select(.reason=="compiler-artifact") | (.filenames[]? , .executable? // empty)' "$output/cargo-messages.jsonl" | LC_ALL=C sort -u)
jq -r '.artifacts.compiler_artifacts[]|.files[]|[.root,.path]|@tsv' "$passport" | LC_ALL=C sort -u > "$inventory"
LC_ALL=C sort -u "$reported" -o "$reported"
diff -u "$reported" "$inventory" >/dev/null || die "artifact inventory does not reconcile to Cargo JSON messages"
rm -f "$reported" "$inventory"

while IFS=$'\t' read -r artifact_root relative expected_sha expected_size; do
  [[ -n "$relative" ]] || continue
  case "$relative" in /*|../*|*/../*|*/..) die "unsafe relative artifact path: $relative" ;; esac
  case "$artifact_root" in target) base="$target_dir" ;; build) base="$build_dir" ;; *) die "unknown artifact root: $artifact_root" ;; esac
  resolved="$(realpath -e "$base/$relative")"
  case "$resolved" in "$base"/*) ;; *) die "artifact escapes recorded root: $relative" ;; esac
  [[ "$(sha "$resolved")" == "$expected_sha" ]] || die "artifact hash mismatch: $artifact_root/$relative"
  [[ "$(stat -c '%s' "$resolved")" == "$expected_size" ]] || die "artifact size mismatch: $artifact_root/$relative"
done < <(jq -r '.artifacts.compiler_artifacts[]|.files[]|[.root,.path,.sha256,(.size_bytes|tostring)]|@tsv' "$passport")

if [[ -n "$root" ]]; then
  [[ "$(git -C "$root" rev-parse HEAD)" == "$(jq -r '.source.git_commit' "$passport")" ]] || die "source commit mismatch"
  [[ "$(git -C "$root" rev-parse 'HEAD^{tree}')" == "$(jq -r '.source.git_tree' "$passport")" ]] || die "source tree mismatch"
  lock_path="$(jq -r '.source.lockfile_path' "$passport")"
  manifest_path="$(jq -r '.source.manifest_path' "$passport")"
  [[ "$(sha "$root/$lock_path")" == "$(jq -r '.source.cargo_lock_sha256' "$passport")" ]] || die "Cargo.lock digest mismatch"
  [[ "$(sha "$root/$manifest_path")" == "$(jq -r '.source.manifest_sha256' "$passport")" ]] || die "manifest digest mismatch"
  cargo_cfg="$(jq -r '.source.workspace_cargo_config_path // empty' "$passport")"
  if [[ -n "$cargo_cfg" ]]; then
    [[ "$(sha "$root/$cargo_cfg")" == "$(jq -r '.source.workspace_cargo_config_sha256' "$passport")" ]] || die "workspace Cargo config digest mismatch"
  fi
fi
echo "PASS_ARTIFACT_PASSPORT_INTEGRITY_ONLY"
printf 'compiler_artifact_records=%s\nartifact_files=%s\n' "$actual_records" "$actual_files"
