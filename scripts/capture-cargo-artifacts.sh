#!/usr/bin/env bash
set -Eeuo pipefail
TOOLCHAIN="${1:-1.100.0}"
OUT="${2:?usage: $0 TOOLCHAIN OUTPUT_DIR [MANIFEST]}"
MANIFEST="${3:-Cargo.toml}"
ROOT="$(git rev-parse --show-toplevel)"
die(){ echo "ERROR: $*" >&2; exit 2; }
sha(){ sha256sum -- "$1" | awk '{print $1}'; }

case "$MANIFEST" in ""|/*|../*|*/../*|*/..) die "manifest must be a safe repository-relative path";; esac
MANIFEST_ABS="$(realpath -e "$ROOT/$MANIFEST")"
case "$MANIFEST_ABS" in "$ROOT"/*) ;; *) die "manifest escapes repository";; esac
[[ -f "$MANIFEST_ABS" ]] || die "manifest missing"
LOCK_DIR="$(dirname "$MANIFEST_ABS")"
LOCK=""
while :; do
  if [[ -f "$LOCK_DIR/Cargo.lock" ]]; then LOCK="$LOCK_DIR/Cargo.lock"; break; fi
  [[ "$LOCK_DIR" != "$ROOT" ]] || break
  LOCK_DIR="$(dirname "$LOCK_DIR")"
done
[[ -n "$LOCK" ]] || die "applicable Cargo.lock not found"
git -C "$ROOT" diff --quiet && git -C "$ROOT" diff --cached --quiet || die "tracked source tree is dirty"
UNTRACKED="$(git -C "$ROOT" status --porcelain --untracked-files=all | grep -vE '^\?\? target(/|$)' || true)"
[[ -z "$UNTRACKED" ]] || die "unexpected untracked source files: $UNTRACKED"
command -v jq >/dev/null || die "jq required"
[[ ! -d "$OUT" || -z "$(find "$OUT" -mindepth 1 -maxdepth 1 -print -quit)" ]] || die "output directory must be empty"
mkdir -p "$OUT"
OUT="$(realpath "$OUT")"
case "$OUT/" in "$ROOT/"*) die "output must be outside source worktree";; esac
TARGET="$OUT/target"
BUILD="$OUT/build"
mkdir -p "$TARGET" "$BUILD"

RUSTC="$OUT/rustc-version.txt"
CARGO="$OUT/cargo-version.txt"
META="$OUT/cargo-metadata.json"
MSG="$OUT/cargo-messages.jsonl"
DIAG="$OUT/diagnostics.log"
RAW="$OUT/compiler-artifacts.raw.jsonl"
ART="$OUT/compiler-artifacts.json"
PASS="$OUT/passport.json"
LOCK_BEFORE="$(sha "$LOCK")"

rustup run "$TOOLCHAIN" rustc -Vv > "$RUSTC"
rustup run "$TOOLCHAIN" cargo -Vv > "$CARGO"
[[ "$(head -n1 "$RUSTC")" == "rustc $TOOLCHAIN "* ]] || die "rustc version mismatch"
[[ "$(head -n1 "$CARGO")" == "cargo $TOOLCHAIN "* ]] || die "Cargo version mismatch"
HOST="$(awk -F': ' '$1=="host"{print $2;exit}' "$RUSTC")"
[[ -n "$HOST" ]] || die "rustc host triple missing"

rustup run "$TOOLCHAIN" cargo metadata --manifest-path "$MANIFEST_ABS" --locked --format-version 1 --no-deps > "$META"
rustup run "$TOOLCHAIN" cargo fetch --manifest-path "$MANIFEST_ABS" --locked

env -u RUSTC -u CARGO_BUILD_RUSTC \
  CARGO_TARGET_DIR="$TARGET" CARGO_BUILD_BUILD_DIR="$BUILD" CARGO_INCREMENTAL=0 \
  RUSTC_WRAPPER="" RUSTC_WORKSPACE_WRAPPER="" \
  CARGO_BUILD_RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER="" \
  rustup run "$TOOLCHAIN" cargo build \
    --manifest-path "$MANIFEST_ABS" --locked --all-targets --offline --target "$HOST" \
    --config 'build.rustc="rustc"' \
    --config 'build.rustc-wrapper=""' \
    --config 'build.rustc-workspace-wrapper=""' \
    --message-format=json-render-diagnostics > "$MSG" 2> "$DIAG"

[[ "$LOCK_BEFORE" == "$(sha "$LOCK")" ]] || die "Cargo.lock changed"
git -C "$ROOT" diff --quiet && git -C "$ROOT" diff --cached --quiet || die "build modified tracked source"

: > "$RAW"
while IFS= read -r item; do
  package="$(jq -r '.package_id' <<<"$item")"
  target_json="$(jq -c '.target' <<<"$item")"
  profile_json="$(jq -c '.profile // null' <<<"$item")"
  fresh_json="$(jq -c '.fresh // null' <<<"$item")"
  executable_json="$(jq -c '(.executable != null)' <<<"$item")"
  files='[]'
  mapfile -t paths < <(jq -r '.filenames[]?, (.executable // empty)' <<<"$item" | LC_ALL=C sort -u)
  (( ${#paths[@]} > 0 )) || die "Cargo artifact has no filenames: $package"
  for path in "${paths[@]}"; do
    [[ -f "$path" ]] || die "Cargo-reported artifact missing: $path"
    resolved="$(realpath -e "$path")"
    case "$resolved" in
      "$TARGET"/*) root="target"; rel="${resolved#"$TARGET"/}";;
      "$BUILD"/*) root="build"; rel="${resolved#"$BUILD"/}";;
      *) die "Cargo-reported file outside isolated output roots: $resolved";;
    esac
    [[ "$rel" != *$'\n'* && "$rel" != *$'\t'* ]] || die "unsupported control character in path"
    rec="$(jq -cn --arg root "$root" --arg path "$rel" --arg digest "$(sha "$resolved")" --argjson size "$(stat -c '%s' "$resolved")" '{root:$root,path:$path,sha256:$digest,size_bytes:$size}')"
    files="$(jq -cn --argjson a "$files" --argjson b "$rec" '$a+[$b]')"
  done
  jq -cn --arg package "$package" --argjson target "$target_json" --argjson profile "$profile_json" --argjson fresh "$fresh_json" --argjson executable "$executable_json" --argjson files "$files" \
    '{package_id:$package,target:$target,profile:$profile,fresh:$fresh,has_executable:$executable,files:$files}' >> "$RAW"
done < <(jq -c 'select(.reason=="compiler-artifact")' "$MSG")
[[ -s "$RAW" ]] || die "no Cargo compiler-artifact JSON records"
jq -s 'sort_by(.package_id,.target.name)' "$RAW" > "$ART"
rm "$RAW"

PROJECT_CONFIG=""
[[ -f "$ROOT/.cargo/config.toml" ]] && PROJECT_CONFIG="$ROOT/.cargo/config.toml"
[[ -z "$PROJECT_CONFIG" && -f "$ROOT/.cargo/config" ]] && PROJECT_CONFIG="$ROOT/.cargo/config"
HOME_CARGO="${CARGO_HOME:-$HOME/.cargo}"
HOME_CONFIG=""
[[ -f "$HOME_CARGO/config.toml" ]] && HOME_CONFIG="$HOME_CARGO/config.toml"
[[ -z "$HOME_CONFIG" && -f "$HOME_CARGO/config" ]] && HOME_CONFIG="$HOME_CARGO/config"

jq -n -S \
  --arg schema "luminous.cargo-artifact-passport.v0.1" \
  --arg requested "$TOOLCHAIN" --arg rustc_release "$(head -n1 "$RUSTC")" --arg rustc_verbose "$(cat "$RUSTC")" \
  --arg rustc_commit "$(awk -F': ' '$1=="commit-hash"{print $2;exit}' "$RUSTC")" \
  --arg cargo_release "$(head -n1 "$CARGO")" --arg cargo_verbose "$(cat "$CARGO")" --arg host "$HOST" \
  --arg commit "$(git -C "$ROOT" rev-parse HEAD)" --arg tree "$(git -C "$ROOT" rev-parse 'HEAD^{tree}')" \
  --arg manifest "${MANIFEST_ABS#"$ROOT"/}" --arg manifest_sha "$(sha "$MANIFEST_ABS")" \
  --arg lockfile "${LOCK#"$ROOT"/}" --arg lock_sha "$(sha "$LOCK")" \
  --arg meta_sha "$(sha "$META")" --arg msg_sha "$(sha "$MSG")" --arg diag_sha "$(sha "$DIAG")" \
  --arg rustc_sha "$(sha "$RUSTC")" --arg cargo_sha "$(sha "$CARGO")" --arg art_sha "$(sha "$ART")" \
  --arg project_config_sha "$( [[ -n "$PROJECT_CONFIG" ]] && sha "$PROJECT_CONFIG" || true )" \
  --arg home_config_sha "$( [[ -n "$HOME_CONFIG" ]] && sha "$HOME_CONFIG" || true )" \
  --arg rustflags "${RUSTFLAGS-}" --arg encoded_rustflags "${CARGO_ENCODED_RUSTFLAGS-}" \
  --arg build_rustflags "${CARGO_BUILD_RUSTFLAGS-}" --arg cargo_home "$HOME_CARGO" \
  --slurpfile records "$ART" \
  '{
    schema:$schema,
    toolchain:{requested:$requested,rustc_release:$rustc_release,rustc_verbose:$rustc_verbose,rustc_commit:$rustc_commit,cargo_release:$cargo_release,cargo_verbose:$cargo_verbose,target_triple:$host},
    source:{git_commit:$commit,git_tree:$tree,manifest_path:$manifest,manifest_sha256:$manifest_sha,lockfile_path:$lockfile,cargo_lock_sha256:$lock_sha,cargo_metadata_sha256:$meta_sha,project_cargo_config_sha256:(if $project_config_sha=="" then null else $project_config_sha end),cargo_home_config_sha256:(if $home_config_sha=="" then null else $home_config_sha end)},
    build:{command:["cargo","build","--locked","--all-targets","--offline","--target",$host],profile:"dev",incremental:false,rustflags:$rustflags,cargo_encoded_rustflags:$encoded_rustflags,cargo_build_rustflags:$build_rustflags,rustc:"rustup-selected-toolchain",rustc_wrapper:"disabled",rustc_workspace_wrapper:"disabled",cargo_home:$cargo_home,target_directory:"target",build_directory:"build",artifact_discovery:"Cargo compiler-artifact JSON only"},
    inputs:{cargo_metadata_sha256:$meta_sha,cargo_messages_sha256:$msg_sha,diagnostics_sha256:$diag_sha,rustc_version_file_sha256:$rustc_sha,cargo_version_file_sha256:$cargo_sha,compiler_artifacts_file_sha256:$art_sha},
    artifacts:{compiler_artifact_count:($records[0]|length),file_count:([$records[0][]|.files[]]|length),compiler_artifacts:$records[0]}
  }' > "$PASS"
sha256sum "$PASS" > "$OUT/passport.sha256"
bash "$ROOT/scripts/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT"
echo "PASS_ARTIFACT_PASSPORT_CAPTURE"
echo "passport=$PASS"
