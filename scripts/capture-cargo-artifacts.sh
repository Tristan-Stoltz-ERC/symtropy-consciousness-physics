#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.100.0}"
OUTPUT="${2:?usage: $0 TOOLCHAIN OUTPUT_DIRECTORY [MANIFEST_PATH]}"
MANIFEST_REL="${3:-Cargo.toml}"
ROOT="$(git rev-parse --show-toplevel)"
ROOT="$(realpath -e "$ROOT")"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
sha256_file() { sha256sum -- "$1" | awk '{print $1}'; }

case "$MANIFEST_REL" in
  ""|/*|../*|*/../*) die "manifest must be repository-relative and traversal-free" ;;
esac
MANIFEST="$(realpath -e "$ROOT/$MANIFEST_REL")"
case "$MANIFEST" in "$ROOT"/*) ;; *) die "manifest escapes repository" ;; esac
[[ -f "$MANIFEST" ]] || die "manifest does not exist"
command -v jq >/dev/null 2>&1 || die "jq is required"

LOCK_DIR="$(dirname "$MANIFEST")"
LOCK_FILE=""
while :; do
  if [[ -f "$LOCK_DIR/Cargo.lock" ]]; then LOCK_FILE="$LOCK_DIR/Cargo.lock"; break; fi
  [[ "$LOCK_DIR" != "$ROOT" ]] || break
  case "$LOCK_DIR" in "$ROOT"/*) ;; *) break ;; esac
  LOCK_DIR="$(dirname "$LOCK_DIR")"
done
[[ -n "$LOCK_FILE" ]] || die "no Cargo.lock found for selected manifest"

assert_source_clean() {
  if ! git -C "$ROOT" diff --quiet || ! git -C "$ROOT" diff --cached --quiet; then
    git -C "$ROOT" status --short >&2
    die "tracked source changes detected"
  fi
  local unexpected
  unexpected="$(git -C "$ROOT" status --porcelain --untracked-files=all | grep -vE '^\?\? target(/|$)' || true)"
  if [[ -n "$unexpected" ]]; then
    printf '%s\n' "$unexpected" >&2
    die "unexpected untracked source files detected"
  fi
}
assert_source_clean

[[ ! -e "$OUTPUT" ]] || {
  [[ -d "$OUTPUT" ]] || die "output path exists and is not a directory"
  [[ -z "$(find "$OUTPUT" -mindepth 1 -maxdepth 1 -print -quit)" ]] || die "output directory must be empty"
}
mkdir -p "$OUTPUT"
OUTPUT="$(realpath -e "$OUTPUT")"
case "$OUTPUT/" in "$ROOT/"*) die "output directory must be outside the source worktree" ;; esac

TARGET_DIR="$OUTPUT/target"
BUILD_DIR="$OUTPUT/build"
mkdir -p "$TARGET_DIR" "$BUILD_DIR"

RAW="$OUTPUT/cargo-messages.jsonl"
DIAGNOSTICS="$OUTPUT/diagnostics.log"
METADATA="$OUTPUT/cargo-metadata.json"
ARTIFACTS_TMP="$OUTPUT/compiler-artifacts.raw.jsonl"
ARTIFACTS="$OUTPUT/compiler-artifacts.json"
PASSPORT="$OUTPUT/passport.json"

LOCK_BEFORE="$(sha256_file "$LOCK_FILE")"

rustup run "$TOOLCHAIN" rustc -Vv > "$OUTPUT/rustc-version.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$OUTPUT/cargo-version.txt"
RUSTC_RELEASE="$(head -n 1 "$OUTPUT/rustc-version.txt")"
CARGO_RELEASE="$(head -n 1 "$OUTPUT/cargo-version.txt")"
[[ "$RUSTC_RELEASE" == "rustc $TOOLCHAIN "* ]] || die "requested rustc $TOOLCHAIN but observed $RUSTC_RELEASE"
[[ "$CARGO_RELEASE" == "cargo $TOOLCHAIN "* ]] || die "requested Cargo $TOOLCHAIN but observed $CARGO_RELEASE"
HOST="$(awk -F': ' '$1=="host" {print $2; exit}' "$OUTPUT/rustc-version.txt")"
[[ -n "$HOST" ]] || die "rustc -Vv omitted host triple"

rustup run "$TOOLCHAIN" cargo metadata \
  --manifest-path "$MANIFEST" --locked --format-version 1 --no-deps \
  > "$METADATA"

# Resolve exact locked dependencies before switching the build itself offline.
rustup run "$TOOLCHAIN" cargo fetch --manifest-path "$MANIFEST" --locked

# Final compiler artifacts and intermediates go to separate fresh output roots.
# Cargo's wrappers are explicitly disabled; no host sccache wrapper is in scope.
env \
  RUSTC_WRAPPER="" \
  RUSTC_WORKSPACE_WRAPPER="" \
  CARGO_TARGET_DIR="$TARGET_DIR" \
  CARGO_BUILD_BUILD_DIR="$BUILD_DIR" \
  CARGO_INCREMENTAL=0 \
  rustup run "$TOOLCHAIN" cargo build \
    --manifest-path "$MANIFEST" \
    --locked --all-targets --offline --target "$HOST" \
    --message-format=json-render-diagnostics \
    > "$RAW" 2> "$DIAGNOSTICS"

[[ "$(sha256_file "$LOCK_FILE")" == "$LOCK_BEFORE" ]] || die "selected Cargo.lock changed despite --locked"
assert_source_clean

: > "$ARTIFACTS_TMP"
while IFS= read -r artifact; do
  package_id="$(jq -r '.package_id' <<< "$artifact")"
  target_json="$(jq -c '.target' <<< "$artifact")"
  profile_json="$(jq -c '.profile // null' <<< "$artifact")"
  fresh_json="$(jq -c '.fresh // null' <<< "$artifact")"
  executable_json="$(jq -c '(.executable != null)' <<< "$artifact")"
  files_json='[]'

  mapfile -t filenames < <(
    jq -r '.filenames[]?, (.executable // empty)' <<< "$artifact" | LC_ALL=C sort -u
  )
  (( ${#filenames[@]} > 0 )) || die "Cargo compiler-artifact record has no filenames: $package_id"

  for filename in "${filenames[@]}"; do
    [[ -f "$filename" ]] || die "Cargo-reported artifact is missing: $filename"
    resolved="$(realpath -e "$filename")"
    case "$resolved" in
      "$TARGET_DIR"/*)
        artifact_root="target"
        relative="${resolved#"$TARGET_DIR"/}"
        ;;
      "$BUILD_DIR"/*)
        artifact_root="build"
        relative="${resolved#"$BUILD_DIR"/}"
        ;;
      *)
        die "Cargo-reported artifact escaped both isolated output roots: $resolved"
        ;;
    esac
    [[ "$relative" != *$'\n'* && "$relative" != *$'\t'* ]] || die "artifact path contains an unsupported control character"

    file_record="$(jq -cn \
      --arg root "$artifact_root" \
      --arg path "$relative" \
      --arg sha256 "$(sha256_file "$resolved")" \
      --argjson size "$(stat -c '%s' -- "$resolved")" \
      '{root:$root,path:$path,sha256:$sha256,size_bytes:$size}')"
    files_json="$(jq -cn --argjson current "$files_json" --argjson next "$file_record" '$current + [$next]')"
  done

  jq -cn \
    --arg package_id "$package_id" \
    --argjson target "$target_json" \
    --argjson profile "$profile_json" \
    --argjson fresh "$fresh_json" \
    --argjson executable "$executable_json" \
    --argjson files "$files_json" \
    '{package_id:$package_id,target:$target,profile:$profile,fresh:$fresh,has_executable:$executable,files:$files}' \
    >> "$ARTIFACTS_TMP"
done < <(jq -c 'select(.reason == "compiler-artifact")' "$RAW")

[[ -s "$ARTIFACTS_TMP" ]] || die "Cargo emitted no compiler-artifact records"
jq -s 'sort_by(.package_id,.target.name)' "$ARTIFACTS_TMP" > "$ARTIFACTS"
rm -f "$ARTIFACTS_TMP"

RUSTC_RAW="$(cat "$OUTPUT/rustc-version.txt")"
CARGO_RAW="$(cat "$OUTPUT/cargo-version.txt")"
CARGO_CONFIG_SHA=""
if [[ -f "$ROOT/.cargo/config.toml" ]]; then CARGO_CONFIG_SHA="$(sha256_file "$ROOT/.cargo/config.toml")"
elif [[ -f "$ROOT/.cargo/config" ]]; then CARGO_CONFIG_SHA="$(sha256_file "$ROOT/.cargo/config")"
fi
CARGO_HOME_EFFECTIVE="${CARGO_HOME:-$HOME/.cargo}"
CARGO_HOME_CONFIG_SHA=""
if [[ -f "$CARGO_HOME_EFFECTIVE/config.toml" ]]; then CARGO_HOME_CONFIG_SHA="$(sha256_file "$CARGO_HOME_EFFECTIVE/config.toml")"
elif [[ -f "$CARGO_HOME_EFFECTIVE/config" ]]; then CARGO_HOME_CONFIG_SHA="$(sha256_file "$CARGO_HOME_EFFECTIVE/config")"
fi

jq -n -S \
  --arg schema "luminous.cargo-artifact-passport.v0.1" \
  --arg requested_toolchain "$TOOLCHAIN" \
  --arg rustc_release "$RUSTC_RELEASE" \
  --arg rustc_verbose "$RUSTC_RAW" \
  --arg rustc_commit "$(awk -F': ' '$1=="commit-hash" {print $2; exit}' "$OUTPUT/rustc-version.txt")" \
  --arg cargo_release "$CARGO_RELEASE" \
  --arg cargo_verbose "$CARGO_RAW" \
  --arg target "$HOST" \
  --arg source_commit "$(git -C "$ROOT" rev-parse HEAD)" \
  --arg source_tree "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" \
  --arg manifest_path "${MANIFEST#"$ROOT"/}" \
  --arg manifest_sha "$(sha256_file "$MANIFEST")" \
  --arg lock_path "${LOCK_FILE#"$ROOT"/}" \
  --arg lock_sha "$LOCK_BEFORE" \
  --arg metadata_sha "$(sha256_file "$METADATA")" \
  --arg cargo_config_sha "$CARGO_CONFIG_SHA" \
  --arg cargo_home_config_sha "$CARGO_HOME_CONFIG_SHA" \
  --arg messages_sha "$(sha256_file "$RAW")" \
  --arg diagnostics_sha "$(sha256_file "$DIAGNOSTICS")" \
  --arg rustc_file_sha "$(sha256_file "$OUTPUT/rustc-version.txt")" \
  --arg cargo_file_sha "$(sha256_file "$OUTPUT/cargo-version.txt")" \
  --arg artifacts_sha "$(sha256_file "$ARTIFACTS")" \
  --arg rustflags "${RUSTFLAGS-}" \
  --arg encoded_rustflags "${CARGO_ENCODED_RUSTFLAGS-}" \
  --arg cargo_build_rustflags "${CARGO_BUILD_RUSTFLAGS-}" \
  --slurpfile compiler_artifacts "$ARTIFACTS" \
  '{
    schema:$schema,
    toolchain:{
      requested:$requested_toolchain,
      rustc_release:$rustc_release,
      rustc_verbose:$rustc_verbose,
      rustc_commit:$rustc_commit,
      cargo_release:$cargo_release,
      cargo_verbose:$cargo_verbose,
      target_triple:$target
    },
    source:{
      git_commit:$source_commit,
      git_tree:$source_tree,
      manifest_path:$manifest_path,
      manifest_sha256:$manifest_sha,
      lockfile_path:$lock_path,
      cargo_lock_sha256:$lock_sha,
      cargo_metadata_sha256:$metadata_sha,
      workspace_cargo_config_sha256:(if $cargo_config_sha=="" then null else $cargo_config_sha end),
      cargo_home_config_sha256:(if $cargo_home_config_sha=="" then null else $cargo_home_config_sha end)
    },
    build:{
      command:["cargo","build","--locked","--all-targets","--offline","--target",$target],
      profile:"dev",
      incremental:false,
      rustflags:$rustflags,
      cargo_encoded_rustflags:$encoded_rustflags,
      cargo_build_rustflags:$cargo_build_rustflags,
      rustc_wrapper:"disabled_by_empty_environment_override",
      rustc_workspace_wrapper:"disabled_by_empty_environment_override",
      target_directory:"target",
      build_directory:"build",
      layout_policy:"cargo_json_messages_only"
    },
    inputs:{
      cargo_metadata_sha256:$metadata_sha,
      cargo_messages_sha256:$messages_sha,
      diagnostics_sha256:$diagnostics_sha,
      rustc_version_file_sha256:$rustc_file_sha,
      cargo_version_file_sha256:$cargo_file_sha,
      compiler_artifacts_file_sha256:$artifacts_sha
    },
    artifacts:{
      compiler_artifact_count:($compiler_artifacts[0]|length),
      file_count:([$compiler_artifacts[0][]|.files[]]|length),
      compiler_artifacts:$compiler_artifacts[0]
    }
  }' > "$PASSPORT"

sha256sum "$PASSPORT" > "$OUTPUT/passport.sha256"
bash "$ROOT/scripts/verify-cargo-artifact-passport.sh" "$OUTPUT" "$ROOT"

printf 'artifact_passport=%s\n' "$PASSPORT"
printf 'artifact_passport_sha256=%s\n' "$(sha256_file "$PASSPORT")"
printf 'compiler_artifact_records=%s\n' "$(jq -r '.artifacts.compiler_artifact_count' "$PASSPORT")"
printf 'artifact_files=%s\n' "$(jq -r '.artifacts.file_count' "$PASSPORT")"
printf 'cargo_layout_policy=JSON_MESSAGES_AND_CONTENT_HASHES_ONLY\n'
