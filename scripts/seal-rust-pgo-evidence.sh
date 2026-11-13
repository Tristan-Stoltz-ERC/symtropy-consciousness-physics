#!/usr/bin/env bash
set -Eeuo pipefail

RESULT_DIR="${1:?usage: seal-rust-pgo-evidence.sh RESULT_DIRECTORY}"
RUN_MANIFEST_REL="${2:-metadata/run-manifest.txt}"
RESULT_DIR="$(realpath "$RESULT_DIR")"
[[ -d "$RESULT_DIR" ]] || { echo "result directory does not exist: $RESULT_DIR" >&2; exit 2; }
[[ -f "$RESULT_DIR/$RUN_MANIFEST_REL" ]] || { echo "run manifest missing: $RUN_MANIFEST_REL" >&2; exit 2; }
[[ -f "$RESULT_DIR/STATUS.txt" ]] || { echo "status receipt missing" >&2; exit 2; }
grep -q '^PASS_MECHANICAL_' "$RESULT_DIR/STATUS.txt" || {
  echo "only a completed mechanical status can be sealed" >&2
  exit 2
}
command -v sha256sum >/dev/null 2>&1 || { echo "sha256sum is required" >&2; exit 2; }
command -v sort >/dev/null 2>&1 || { echo "sort is required" >&2; exit 2; }
command -v xargs >/dev/null 2>&1 || { echo "xargs is required" >&2; exit 2; }

cd "$RESULT_DIR"
rm -f evidence-files.sha256 integrity-manifest.txt
find . -type f \
  ! -name 'evidence-files.sha256' \
  ! -name 'integrity-manifest.txt' \
  -print0 | sort -z | xargs -0 sha256sum > evidence-files.sha256
[[ -s evidence-files.sha256 ]] || { echo "no evidence files were found" >&2; exit 1; }

RUN_MANIFEST_SHA="$(sha256sum "$RUN_MANIFEST_REL" | awk '{print $1}')"
EVIDENCE_LIST_SHA="$(sha256sum evidence-files.sha256 | awk '{print $1}')"
{
  echo "schema=luminous.rust-pgo-integrity-seal.v0.1"
  echo "run_manifest_path=$RUN_MANIFEST_REL"
  echo "run_manifest_sha256=$RUN_MANIFEST_SHA"
  echo "evidence_files_path=evidence-files.sha256"
  echo "evidence_files_sha256=$EVIDENCE_LIST_SHA"
  echo "seal_semantics=content_integrity_only_not_signature_or_attestation"
  echo "claim_ceiling=PASS_EVIDENCE_INTEGRITY_ONLY"
} > integrity-manifest.txt

printf 'evidence_file_count=%s\n' "$(wc -l < evidence-files.sha256 | tr -d ' ')"
printf 'run_manifest_sha256=%s\n' "$RUN_MANIFEST_SHA"
printf 'evidence_files_sha256=%s\n' "$EVIDENCE_LIST_SHA"
printf 'PASS_EVIDENCE_SEALED\n'
