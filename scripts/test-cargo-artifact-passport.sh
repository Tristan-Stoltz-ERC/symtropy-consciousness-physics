#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(mktemp -d)"
OUTPUT="$(mktemp -d)"
trap 'rm -rf "$ROOT" "$OUTPUT"' EXIT
mkdir -p "$ROOT" "$OUTPUT/target/debug/deps" "$OUTPUT/build"
printf 'lock-v1\n' > "$ROOT/Cargo.lock"
printf '[package]\nname="fixture"\nversion="0.1.0"\n' > "$ROOT/Cargo.toml"
git -C "$ROOT" init -q
git -C "$ROOT" add Cargo.lock Cargo.toml
git -C "$ROOT" -c user.name=PassportTest -c user.email=passport@example.invalid commit -qm fixture
printf 'rlib-content\n' > "$OUTPUT/target/debug/deps/libfixture-123.rlib"
printf '{"packages":[],"workspace_members":[]}\n' > "$OUTPUT/cargo-metadata.json"
printf 'rustc 1.100.0\nhost: x86_64-unknown-linux-gnu\ncommit-hash: fixture\n' > "$OUTPUT/rustc-version.txt"
printf 'cargo 1.100.0\n' > "$OUTPUT/cargo-version.txt"
printf 'diagnostics empty\n' > "$OUTPUT/diagnostics.log"

sha256_file() { sha256sum -- "$1" | awk '{print $1}'; }
artifact_sha="$(sha256_file "$OUTPUT/target/debug/deps/libfixture-123.rlib")"
artifact_size="$(stat -c '%s' "$OUTPUT/target/debug/deps/libfixture-123.rlib")"
jq -cn --arg sha "$artifact_sha" --argjson size "$artifact_size" '[
  {package_id:"path+file:///fixture#fixture@0.1.0",
   target:{name:"fixture",kind:["lib"],crate_types:["lib"],src_path:"src/lib.rs",edition:"2021",doc:true,doctest:true,test:true},
   profile:{opt_level:"0"},fresh:false,has_executable:false,
   files:[{root:"target",path:"debug/deps/libfixture-123.rlib",sha256:$sha,size_bytes:$size}]}
]' > "$OUTPUT/compiler-artifacts.json"

git_commit="$(git -C "$ROOT" rev-parse HEAD)"
git_tree="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
jq -n -S \
 --arg commit "$git_commit" \
 --arg tree "$git_tree" \
 --arg lock "$(sha256_file "$ROOT/Cargo.lock")" \
 --arg manifest "$(sha256_file "$ROOT/Cargo.toml")" \
 --arg metadata "$(sha256_file "$OUTPUT/cargo-metadata.json")" \
 --arg messages "$(sha256_file "$OUTPUT/cargo-messages.jsonl")" \
 --arg diagnostics "$(sha256_file "$OUTPUT/diagnostics.log")" \
 --arg rustc "$(sha256_file "$OUTPUT/rustc-version.txt")" \
 --arg cargo "$(sha256_file "$OUTPUT/cargo-version.txt")" \
 --arg artifacts "$(sha256_file "$OUTPUT/compiler-artifacts.json")" \
 --slurpfile records "$OUTPUT/compiler-artifacts.json" \
 '{
  schema:"luminous.cargo-artifact-passport.v0.1",
  source:{
    git_commit:$commit,
    git_tree:$tree,
    manifest_path:"Cargo.toml",
    manifest_sha256:$manifest,
    lockfile_path:"Cargo.lock",
    cargo_lock_sha256:$lock,
    cargo_metadata_sha256:$metadata
  },
  inputs:{
    cargo_metadata_sha256:$metadata,
    cargo_messages_sha256:$messages,
    diagnostics_sha256:$diagnostics,
    rustc_version_file_sha256:$rustc,
    cargo_version_file_sha256:$cargo,
    compiler_artifacts_file_sha256:$artifacts
  },
  artifacts:{
    compiler_artifact_count:($records[0]|length),
    file_count:([$records[0][]|.files[]]|length),
    compiler_artifacts:$records[0]
  }
 }' > "$OUTPUT/passport.json"
sha256sum "$OUTPUT/passport.json" > "$OUTPUT/passport.sha256"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bash "$SCRIPT_DIR/verify-cargo-artifact-passport.sh" "$OUTPUT" "$ROOT"

printf 'tampered\n' >> "$OUTPUT/target/debug/deps/libfixture-123.rlib"
if bash "$SCRIPT_DIR/verify-cargo-artifact-passport.sh" "$OUTPUT" "$ROOT" >/dev/null 2>&1; then
  echo "verifier accepted tampered artifact" >&2
  exit 1
fi
printf 'rlib-content\n' > "$OUTPUT/target/debug/deps/libfixture-123.rlib"
printf 'metadata-tamper\n' >> "$OUTPUT/cargo-metadata.json"
if bash "$SCRIPT_DIR/verify-cargo-artifact-passport.sh" "$OUTPUT" "$ROOT" >/dev/null 2>&1; then
  echo "verifier accepted changed Cargo metadata" >&2
  exit 1
fi
printf '{"packages":[],"workspace_members":[]}\n' > "$OUTPUT/cargo-metadata.json"
printf 'lock-v2\n' > "$ROOT/Cargo.lock"
if bash "$SCRIPT_DIR/verify-cargo-artifact-passport.sh" "$OUTPUT" "$ROOT" >/dev/null 2>&1; then
  echo "verifier accepted changed lockfile" >&2
  exit 1
fi
printf 'PASS_ARTIFACT_PASSPORT_SELF_TEST\n'
