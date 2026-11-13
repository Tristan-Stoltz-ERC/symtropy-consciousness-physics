#!/usr/bin/env bash
set -Eeuo pipefail

RESULT_DIR="${1:?usage: verify-rust-pgo-evidence.sh RESULT_DIRECTORY [SOURCE_CHECKOUT]}"
SOURCE_ROOT="${2:-$(git rev-parse --show-toplevel)}"
RESULT_DIR="$(realpath "$RESULT_DIR")"
SOURCE_ROOT="$(realpath "$SOURCE_ROOT")"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
[[ -f "$RESULT_DIR/integrity-manifest.txt" ]] || die "integrity manifest missing"
[[ -f "$RESULT_DIR/evidence-files.sha256" ]] || die "evidence file digest list missing"
[[ -d "$SOURCE_ROOT/.git" || -f "$SOURCE_ROOT/.git" ]] || die "source checkout is not a Git worktree"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"

field() {
  local key="$1" file="$2"
  sed -n "s/^${key}=//p" "$file" | head -n1
}

SEAL="$RESULT_DIR/integrity-manifest.txt"
RUN_PATH="$(field run_manifest_path "$SEAL")"
RUN_SHA="$(field run_manifest_sha256 "$SEAL")"
LIST_PATH="$(field evidence_files_path "$SEAL")"
LIST_SHA="$(field evidence_files_sha256 "$SEAL")"
[[ -n "$RUN_PATH" && -n "$RUN_SHA" && -n "$LIST_PATH" && -n "$LIST_SHA" ]] || die "integrity manifest is incomplete"
[[ "$(field schema "$SEAL")" == "luminous.rust-pgo-integrity-seal.v0.1" ]] || die "unsupported seal schema"
[[ "$(field claim_ceiling "$SEAL")" == "PASS_EVIDENCE_INTEGRITY_ONLY" ]] || die "unexpected seal claim ceiling"

[[ -f "$RESULT_DIR/$RUN_PATH" ]] || die "run manifest path missing: $RUN_PATH"
[[ "$(sha256sum "$RESULT_DIR/$RUN_PATH" | awk '{print $1}')" == "$RUN_SHA" ]] || die "run manifest digest mismatch"
[[ -f "$RESULT_DIR/$LIST_PATH" ]] || die "evidence list path missing: $LIST_PATH"
[[ "$(sha256sum "$RESULT_DIR/$LIST_PATH" | awk '{print $1}')" == "$LIST_SHA" ]] || die "evidence list digest mismatch"

(
  cd "$RESULT_DIR"
  sha256sum -c "$LIST_PATH"
)

RUN_MANIFEST="$RESULT_DIR/$RUN_PATH"
SOURCE_COMMIT="$(field source_commit "$RUN_MANIFEST")"
SOURCE_TREE="$(field source_tree "$RUN_MANIFEST")"
LOCK_SHA="$(field cargo_lock_sha256 "$RUN_MANIFEST")"
LOCK_SHA_AFTER="$(field cargo_lock_sha256_after "$RUN_MANIFEST")"
TOOLCHAIN="$(field toolchain "$RUN_MANIFEST")"
RUSTC_VERSION="$(field rustc "$RUN_MANIFEST")"
CARGO_VERSION="$(field cargo "$RUN_MANIFEST")"
[[ -n "$SOURCE_COMMIT" && -n "$SOURCE_TREE" && -n "$LOCK_SHA" && -n "$LOCK_SHA_AFTER" ]] || die "run manifest lacks source/lock identity"
[[ "$LOCK_SHA" == "$LOCK_SHA_AFTER" ]] || die "run manifest shows lockfile drift"

ACTUAL_COMMIT="$(git -C "$SOURCE_ROOT" rev-parse HEAD)"
ACTUAL_TREE="$(git -C "$SOURCE_ROOT" rev-parse 'HEAD^{tree}')"
ACTUAL_LOCK="$(sha256sum "$SOURCE_ROOT/Cargo.lock" | awk '{print $1}')"
[[ "$ACTUAL_COMMIT" == "$SOURCE_COMMIT" ]] || die "source commit mismatch: expected $SOURCE_COMMIT, got $ACTUAL_COMMIT"
[[ "$ACTUAL_TREE" == "$SOURCE_TREE" ]] || die "source tree mismatch"
[[ "$ACTUAL_LOCK" == "$LOCK_SHA" ]] || die "Cargo.lock digest mismatch"

if [[ -n "$TOOLCHAIN" ]] && command -v rustup >/dev/null 2>&1; then
  [[ "$(rustup run "$TOOLCHAIN" rustc --version)" == "$RUSTC_VERSION" ]] || die "installed rustc identity differs from evidence"
  [[ "$(rustup run "$TOOLCHAIN" cargo --version)" == "$CARGO_VERSION" ]] || die "installed Cargo identity differs from evidence"
fi

grep -q '^PASS_MECHANICAL_' "$RESULT_DIR/STATUS.txt" || die "mechanical PGO status is not PASS"
printf 'PASS_EVIDENCE_INTEGRITY_ONLY\n'
printf 'source_commit=%s\n' "$SOURCE_COMMIT"
printf 'source_tree=%s\n' "$SOURCE_TREE"
printf 'cargo_lock_sha256=%s\n' "$LOCK_SHA"
printf 'seal_semantics=content_integrity_only_not_signature_or_attestation\n'
printf 'performance_claim=NOT_ESTABLISHED_BY_EVIDENCE_INTEGRITY_CHECK\n'
