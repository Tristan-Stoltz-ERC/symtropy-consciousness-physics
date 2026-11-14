#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.100.0}"
OUTPUT="${2:?usage: $0 TOOLCHAIN OUTPUT_DIRECTORY [MANIFEST_PATH]}"
MANIFEST_PATH="${3:-Cargo.toml}"
ROOT="$(git rev-parse --show-toplevel)"

case "$MANIFEST_PATH" in
  ""|/*|../*|*/../*) echo "manifest path must be repository-relative and traversal-free" >&2; exit 2 ;;
esac
MANIFEST_ABS="$(realpath -e "$ROOT/$MANIFEST_PATH")"
case "$MANIFEST_ABS" in "$ROOT"/*) ;; *) echo "manifest path escapes repository" >&2; exit 2 ;; esac
[[ -f "$MANIFEST_ABS" ]] || { echo "manifest is missing" >&2; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 2; }

# Select the lockfile Cargo will actually use for this manifest.
LOCK_DIR="$(dirname "$MANIFEST_ABS")"
LOCK_FILE=""
while [[ "$LOCK_DIR" == "$ROOT"/* || "$LOCK_DIR" == "$ROOT" ]]; do
  if [[ -f "$LOCK_DIR/Cargo.lock" ]]; then LOCK_FILE="$LOCK_DIR/Cargo.lock"; break; fi
  [[ "$LOCK_DIR" != "$ROOT" ]] || break
  LOCK_DIR="$(dirname "$LOCK_DIR")"
done
[[ -n "$LOCK_FILE" ]] || { echo "no Cargo.lock found for manifest" >&2; exit 2; }

if [[ -n "$(git -C "$ROOT" status --porcelain --untracked-files=all)" ]]; then
  echo "refusing to capture from a dirty worktree" >&2
  git -C "$ROOT" status --short >&2
  exit 1
fi

mkdir -p "$OUTPUT"
OUTPUT="$(realpath "$OUTPUT")"
case "$OUTPUT/" in "$ROOT/"*) echo "output must be outside source worktree" >&2; exit 2 ;; esac
TARGET_DIR="$OUTPUT/target"
BUILD_DIR="$OUTPUT/build"
mkdir -p "$TARGET_DIR" "$BUILD_DIR"

RAW="$OUTPUT/cargo-messages.jsonl"
DIAGNOSTICS="$OUTPUT/diagnostics.log"
METADATA="$OUTPUT/cargo-metadata.json"
ARTIFACTS_TMP="$OUTPUT/compiler-artifacts.raw.jsonl"
ARTIFACTS="$OUTPUT/compiler-artifacts.json"
PASSPORT="$OUTPUT/passport.json"

LOCK_BEFORE="$(sha256sum "$LOCK_FILE" | awk '{print $1}')"
rustup run "$TOOLCHAIN" rustc -Vv > "$OUTPUT/rustc-version.txt"
rustup run "$TOOLCHAIN" cargo -Vv > "$OUTPUT/cargo-version.txt"
HOST="$(awk -F': ' '$1 == "host" {print $2; exit}' "$OUTPUT/rustc-version.txt")"
[[ -n "$HOST" ]] || { echo "rustc host triple missing" >&2; exit 1; }

rustup run "$TOOLCHAIN" cargo metadata \
  --manifest-path "$MANIFEST_ABS" --locked --format-version 1 --no-deps \
  > "$METADATA"

# Warm the exact locked graph online, then make measured artifact production offline.
rustup run "$TOOLCHAIN" cargo fetch --manifest-path "$MANIFEST_ABS" --locked
env \
  -u RUSTC_WRAPPER \
  -u RUSTC_WORKSPACE_WRAPPER \
  CARGO_TARGET_DIR="$TARGET_DIR" \
  CARGO_BUILD_BUILD_DIR="$BUILD_DIR" \
  CARGO_INCREMENTAL=0 \
  rustup run "$TOOLCHAIN" cargo build \
    --manifest-path "$MANIFEST_ABS" \
    --locked --all-targets --offline --target "$HOST" \
    --config 'build.rustc-wrapper=""' \
    --config 'build.rustc-workspace-wrapper=""' \
    --message-format=json-render-diagnostics \
    > "$RAW" 2> "$DIAGNOSTICS"

LOCK_AFTER="$(sha256sum "$LOCK_FILE" | awk '{print $1}')"
[[ "$LOCK_BEFORE" == "$LOCK_AFTER" ]] || { echo "lockfile changed despite --locked" >&2; exit 1; }
if [[ -n "$(git -C "$ROOT" status --porcelain --untracked-files=all)" ]]; then
  echo "build modified source worktree; refusing passport" >&2
  git -C "$ROOT" status --short >&2
  exit 1
fi

sha256_file() { sha256sum -- "$1" | awk '{print $1}'; }
: > "$ARTIFACTS_TMP"
while IFS= read -r artifact; do
  package_id="$(jq -r '.package_id' <<< "$artifact")"
  target_json="$(jq -c '.target' <<< "$artifact")"
  profile_json="$(jq -c '.profile // null' <<< "$artifact")"
  fresh_json="$(jq -c '.fresh // null' <<< "$artifact")"
  executable_json="$(jq -c '(.executable != null)' <<< "$artifact")"
  files_json='[]'
  mapfile -t paths < <(jq -r '.filenames[]?, (.executable // empty)' <<< "$artifact" | LC_ALL=C sort -u)
  (( ${#paths[@]} > 0 )) || { echo "Cargo compiler-artifact had no filenames: $package_id" >&2; exit 1; }

  for path in "${paths[@]}"; do
    [[ -f "$path" ]] || { echo "Cargo-reported artifact missing: $path" >&2; exit 1; }
    resolved="$(realpath -e "$path")"
    case "$resolved" in "$TARGET_DIR"/*) ;; *) echo "Cargo artifact escaped target dir: $resolved" >&2; exit 1 ;; esac
    relative="${resolved#"$TARGET_DIR"/}"
    [[ "$relative" != *$'\n'* && "$relative" != *$'\t'* ]] || { echo "unsupported artifact path control character" >&2; exit 1; }
    file_record="$(jq -cn \
      --arg path "$relative" \
      --arg sha256 "$(sha256_file "$resolved")" \
      --argjson size "$(stat -c '%s' -- "$resolved")" \
      '{path:$path,sha256:$sha256,size_bytes:$size}')"
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

[[ -s "$ARTIFACTS_TMP" ]] || { echo "Cargo emitted no compiler-artifact records" >&2; exit 1; }
jq -s 'sort_by(.package_id,.target.name)' "$ARTIFACTS_TMP" > "$ARTIFACTS"
rm -f "$ARTIFACTS_TMP"

RUSTC_RAW="$(cat "$OUTPUT/rustc-version.txt")"
CARGO_RAW="$(cat "$OUTPUT/cargo-version.txt")"
CARGO_CONFIG_SHA=""
if [[ -f "$ROOT/.cargo/config.toml" ]]; then
  CARGO_CONFIG_SHA="$(sha256_file "$ROOT/.cargo/config.toml")"
elif [[ -f "$ROOT/.cargo/config" ]]; then
  CARGO_CONFIG_SHA="$(sha256_file "$ROOT/.cargo/config")"
fi
CARGO_HOME_CONFIG_SHA=""
if [[ -n "${CARGO_HOME-}" ]]; then
  if [[ -f "$CARGO_HOME/config.toml" ]]; then CARGO_HOME_CONFIG_SHA="$(sha256_file "$CARGO_HOME/config.toml")"
  elif [[ -f "$CARGO_HOME/config" ]]; then CARGO_HOME_CONFIG_SHA="$(sha256_file "$CARGO_HOME/config")"
  fi
fi

jq -n -S \
  --arg schema "luminous.cargo-artifact-passport.v0.1" \
  --arg requested_toolchain "$TOOLCHAIN" \
  --arg rustc_release "$(head -n 1 "$OUTPUT/rustc-version.txt")" \
  --arg rustc_verbose "$RUSTC_RAW" \
  --arg rustc_commit "$(awk -F': ' '$1=="commit-hash" {print $2; exit}' "$OUTPUT/rustc-version.txt")" \
  --arg cargo_release "$(head -n 1 "$OUTPUT/cargo-version.txt")" \
  --arg cargo_verbose "$CARGO_RAW" \
  --arg target "$HOST" \
  --arg source_commit "$(git -C "$ROOT" rev-parse HEAD)" \
  --arg source_tree "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" \
  --arg manifest_path "${MANIFEST_ABS#"$ROOT"/}" \
  --arg manifest_sha "$(sha256_file "$MANIFEST_ABS")" \
  --arg lock_path "${LOCK_FILE#"$ROOT"/}" \
  --arg lock_sha "$(sha256_file "$LOCK_FILE")" \
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
      rustc_wrapper:"disabled_by_cli_override",
      rustc_workspace_wrapper:"disabled_by_cli_override",
      target_directory:"target",
      build_directory:"build",
      layout_policy:"cargo_json_messages_only"
    },
    inputs:{
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
