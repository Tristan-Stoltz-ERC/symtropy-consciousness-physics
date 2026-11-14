#!/usr/bin/env bash
set -Eeuo pipefail
OUT="${1:?usage: $0 OUTPUT_DIR [REPOSITORY_ROOT]}"
ROOT="${2-}"
OUT="$(realpath -e "$OUT")"
TARGET="$OUT/target"
BUILD="$OUT/build"
PASS="$OUT/passport.json"
die(){ echo "ERROR: $*" >&2; exit 1; }
sha(){ sha256sum -- "$1" | awk '{print $1}'; }

[[ -f "$PASS" && -f "$OUT/passport.sha256" ]] || die "passport/digest missing"
jq -e '.schema=="luminous.cargo-artifact-passport.v0.1"' "$PASS" >/dev/null || die "unsupported schema"
[[ "$(awk '{print $1}' "$OUT/passport.sha256")" == "$(sha "$PASS")" ]] || die "passport digest mismatch"

check_input(){
  local field="$1" file="$2" expected
  [[ -f "$file" ]] || die "missing input $file"
  expected="$(jq -r ".inputs.$field" "$PASS")"
  [[ "$expected" == "$(sha "$file")" ]] || die "input digest mismatch: $file"
}
check_input cargo_metadata_sha256 "$OUT/cargo-metadata.json"
check_input cargo_messages_sha256 "$OUT/cargo-messages.jsonl"
check_input diagnostics_sha256 "$OUT/diagnostics.log"
check_input rustc_version_file_sha256 "$OUT/rustc-version.txt"
check_input cargo_version_file_sha256 "$OUT/cargo-version.txt"
check_input compiler_artifacts_file_sha256 "$OUT/compiler-artifacts.json"

expected="$(jq -r '.artifacts.compiler_artifact_count' "$PASS")"
actual="$(jq 'length' "$OUT/compiler-artifacts.json")"
[[ "$actual" -gt 0 && "$expected" == "$actual" ]] || die "compiler artifact count mismatch"

while IFS=$'\t' read -r root rel digest size; do
  [[ -n "$rel" ]] || continue
  case "$root" in target) base="$TARGET";; build) base="$BUILD";; *) die "unknown artifact root: $root";; esac
  case "$rel" in ""|/*|../*|*/../*|*/..) die "unsafe artifact path: $rel";; esac
  [[ "$rel" != *$'\t'* && "$rel" != *$'\n'* ]] || die "artifact path contains control chars"
  file="$(realpath -e "$base/$rel")"
  case "$file" in "$base"/*) ;; *) die "artifact path escapes $root root: $rel";; esac
  [[ -f "$file" ]] || die "artifact missing: $rel"
  [[ "$(sha "$file")" == "$digest" ]] || die "artifact digest mismatch: $rel"
  [[ "$(stat -c '%s' "$file")" == "$size" ]] || die "artifact size mismatch: $rel"
done < <(jq -r '.artifacts.compiler_artifacts[] | .files[] | [.root,.path,.sha256,(.size_bytes|tostring)] | @tsv' "$PASS")

if [[ -n "$ROOT" ]]; then
  ROOT="$(realpath -e "$ROOT")"
  [[ "$(git -C "$ROOT" rev-parse HEAD)" == "$(jq -r '.source.git_commit' "$PASS")" ]] || die "source commit mismatch"
  [[ "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" == "$(jq -r '.source.git_tree' "$PASS")" ]] || die "source tree mismatch"
  lock="$(jq -r '.source.lockfile_path' "$PASS")"
  manifest="$(jq -r '.source.manifest_path' "$PASS")"
  [[ -f "$ROOT/$lock" && -f "$ROOT/$manifest" ]] || die "source manifest/lockfile missing"
  [[ "$(sha "$ROOT/$lock")" == "$(jq -r '.source.cargo_lock_sha256' "$PASS")" ]] || die "Cargo.lock digest mismatch"
  [[ "$(sha "$ROOT/$manifest")" == "$(jq -r '.source.manifest_sha256' "$PASS")" ]] || die "manifest digest mismatch"
fi
echo "PASS_ARTIFACT_PASSPORT_INTEGRITY_ONLY"
echo "compiler_artifact_records=$actual"
echo "artifact_files=$(jq -r '.artifacts.file_count' "$PASS")"
