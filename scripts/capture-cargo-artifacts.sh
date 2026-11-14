#!/usr/bin/env bash
set -Eeuo pipefail
die() { echo "ERROR: $*" >&2; exit 2; }
toolchain="1.100.0"
[[ $# -ge 1 && -n "$1" ]] && toolchain="$1"
[[ $# -ge 2 && -n "$2" ]] || die "usage: $0 TOOLCHAIN OUTPUT [MANIFEST]"
output="$2"
manifest_rel="Cargo.toml"
[[ $# -ge 3 && -n "$3" ]] && manifest_rel="$3"
root="$(git rev-parse --show-toplevel)"
case "$manifest_rel" in /*|../*|*/../*|*/..) die "manifest path must be relative and traversal-free" ;; esac
manifest="$(realpath -e "$root/$manifest_rel")"
case "$manifest" in "$root"/*) ;; *) die "manifest escapes repository" ;; esac
command -v jq >/dev/null || die "jq required"
sha() { sha256sum -- "$1" | awk '{print $1}'; }

clean_source() {
  git -C "$root" diff --quiet && git -C "$root" diff --cached --quiet || {
    echo "tracked source changes present" >&2; git -C "$root" status --short >&2; return 1;
  }
  local unexpected
  unexpected="$(git -C "$root" status --porcelain --untracked-files=all | grep -vE '^\?\? target(/|$)' || true)"
  [[ -z "$unexpected" ]] || { echo "$unexpected" >&2; return 1; }
}
clean_source

lock_dir="$(dirname "$manifest")"
lock_file=""
while [[ "$lock_dir" == "$root" || "$lock_dir" == "$root/"* ]]; do
  if [[ -f "$lock_dir/Cargo.lock" ]]; then lock_file="$lock_dir/Cargo.lock"; break; fi
  [[ "$lock_dir" != "$root" ]] || break
  lock_dir="$(dirname "$lock_dir")"
done
[[ -n "$lock_file" ]] || die "no applicable Cargo.lock"

if [[ -d "$output" ]] && [[ -n "$(find "$output" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  die "output must be empty; stale artifact reuse is forbidden"
fi
mkdir -p "$output"
output="$(realpath "$output")"
case "$output/" in "$root/"*) die "output must be outside source worktree" ;; esac
target_dir="$output/target"
build_dir="$output/build"
mkdir -p "$target_dir" "$build_dir"

raw="$output/cargo-messages.jsonl"
diag="$output/diagnostics.log"
metadata="$output/cargo-metadata.json"
records_tmp="$output/compiler-artifacts.raw.jsonl"
records="$output/compiler-artifacts.json"
passport="$output/passport.json"
lock_before="$(sha "$lock_file")"

rustup run "$toolchain" rustc -Vv > "$output/rustc-version.txt"
rustup run "$toolchain" cargo -Vv > "$output/cargo-version.txt"
[[ "$(head -n1 "$output/rustc-version.txt")" == "rustc $toolchain "* ]] || die "rustc version mismatch"
[[ "$(head -n1 "$output/cargo-version.txt")" == "cargo $toolchain "* ]] || die "Cargo version mismatch"
host="$(awk -F': ' '$1=="host"{print $2;exit}' "$output/rustc-version.txt")"
[[ -n "$host" ]] || die "host triple missing"

rustup run "$toolchain" cargo metadata --manifest-path "$manifest" --locked --format-version 1 --no-deps > "$metadata"
rustup run "$toolchain" cargo fetch --manifest-path "$manifest" --locked

env -u RUSTC -u CARGO_BUILD_RUSTC -u CARGO_BUILD_RUSTC_WRAPPER -u CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER \
  RUSTC_WRAPPER="" RUSTC_WORKSPACE_WRAPPER="" \
  CARGO_TARGET_DIR="$target_dir" CARGO_BUILD_BUILD_DIR="$build_dir" CARGO_INCREMENTAL=0 \
  rustup run "$toolchain" cargo build --manifest-path "$manifest" --locked --all-targets --offline --target "$host" \
  --message-format=json-render-diagnostics > "$raw" 2> "$diag"

[[ "$lock_before" == "$(sha "$lock_file")" ]] || die "Cargo.lock changed"
clean_source

: > "$records_tmp"
while IFS= read -r message; do
  package_id="$(jq -r '.package_id' <<< "$message")"
  target_json="$(jq -c '.target' <<< "$message")"
  profile_json="$(jq -c '.profile // null' <<< "$message")"
  fresh_json="$(jq -c '.fresh // null' <<< "$message")"
  executable_json="$(jq -c '(.executable != null)' <<< "$message")"
  files='[]'
  count=0
  while IFS= read -r filename; do
    [[ -n "$filename" ]] || continue
    [[ -f "$filename" ]] || die "Cargo-reported file missing: $filename"
    resolved="$(realpath -e "$filename")"
    case "$resolved" in
      "$target_dir"/*) artifact_root="target"; base="$target_dir" ;;
      "$build_dir"/*) artifact_root="build"; base="$build_dir" ;;
      *) die "reported file outside isolated target/build roots: $resolved" ;;
    esac
    relative="$(realpath --relative-to="$base" "$resolved")"
    [[ "$relative" != *$'\n'* && "$relative" != *$'\t'* ]] || die "unsupported control character in artifact path"
    file_json="$(jq -cn --arg root "$artifact_root" --arg path "$relative" --arg hash "$(sha "$resolved")" \
      --argjson size "$(stat -c '%s' "$resolved")" '{root:$root,path:$path,sha256:$hash,size_bytes:$size}')"
    files="$(jq -cn --argjson current "$files" --argjson next "$file_json" '$current+[$next]')"
    count=$((count + 1))
  done < <(jq -r '(.filenames[]? , .executable? // empty)' <<< "$message" | LC_ALL=C sort -u)
  [[ "$count" -gt 0 ]] || die "compiler-artifact has no files: $package_id"
  jq -cn --arg id "$package_id" --argjson target "$target_json" --argjson profile "$profile_json" \
    --argjson fresh "$fresh_json" --argjson exe "$executable_json" --argjson files "$files" \
    '{package_id:$id,target:$target,profile:$profile,fresh:$fresh,has_executable:$exe,files:$files}' >> "$records_tmp"
done < <(jq -c 'select(.reason=="compiler-artifact")' "$raw")
[[ -s "$records_tmp" ]] || die "Cargo emitted no compiler-artifact records"
jq -s 'sort_by(.package_id,.target.name)' "$records_tmp" > "$records"
rm "$records_tmp"

cargo_cfg_path=""
cargo_cfg_sha=""
if [[ -f "$root/.cargo/config.toml" ]]; then cargo_cfg_path=".cargo/config.toml"; cargo_cfg_sha="$(sha "$root/$cargo_cfg_path")"
elif [[ -f "$root/.cargo/config" ]]; then cargo_cfg_path=".cargo/config"; cargo_cfg_sha="$(sha "$root/$cargo_cfg_path")"; fi
cargo_home="$(printenv CARGO_HOME || printf '%s' "$HOME/.cargo")"
cargo_home_cfg_sha=""
if [[ -f "$cargo_home/config.toml" ]]; then cargo_home_cfg_sha="$(sha "$cargo_home/config.toml")"
elif [[ -f "$cargo_home/config" ]]; then cargo_home_cfg_sha="$(sha "$cargo_home/config")"; fi
rustflags="$(printenv RUSTFLAGS || true)"
encoded_flags="$(printenv CARGO_ENCODED_RUSTFLAGS || true)"
build_flags="$(printenv CARGO_BUILD_RUSTFLAGS || true)"
rustc_raw="$(cat "$output/rustc-version.txt")"
cargo_raw="$(cat "$output/cargo-version.txt")"

jq -n -S \
  --arg schema "luminous.cargo-artifact-passport.v0.1" \
  --arg toolchain "$toolchain" --arg rustc_release "$(head -n1 "$output/rustc-version.txt")" \
  --arg rustc_raw "$rustc_raw" --arg rustc_commit "$(awk -F': ' '$1=="commit-hash"{print $2;exit}' "$output/rustc-version.txt")" \
  --arg cargo_release "$(head -n1 "$output/cargo-version.txt")" --arg cargo_raw "$cargo_raw" --arg host "$host" \
  --arg commit "$(git -C "$root" rev-parse HEAD)" --arg tree "$(git -C "$root" rev-parse 'HEAD^{tree}')" \
  --arg manifest_path "$(realpath --relative-to="$root" "$manifest")" --arg manifest_sha "$(sha "$manifest")" \
  --arg lock_path "$(realpath --relative-to="$root" "$lock_file")" --arg lock_sha "$(sha "$lock_file")" \
  --arg metadata_sha "$(sha "$metadata")" --arg cargo_cfg_path "$cargo_cfg_path" --arg cargo_cfg_sha "$cargo_cfg_sha" \
  --arg cargo_home_cfg_sha "$cargo_home_cfg_sha" --arg raw_sha "$(sha "$raw")" --arg diag_sha "$(sha "$diag")" \
  --arg rustc_sha "$(sha "$output/rustc-version.txt")" --arg cargo_sha "$(sha "$output/cargo-version.txt")" \
  --arg records_sha "$(sha "$records")" --arg rustflags "$rustflags" --arg encoded_flags "$encoded_flags" \
  --arg build_flags "$build_flags" --slurpfile artifacts "$records" \
  '{
    schema:$schema,
    toolchain:{requested:$toolchain,rustc_release:$rustc_release,rustc_verbose:$rustc_raw,rustc_commit:$rustc_commit,cargo_release:$cargo_release,cargo_verbose:$cargo_raw,target_triple:$host},
    source:{git_commit:$commit,git_tree:$tree,manifest_path:$manifest_path,manifest_sha256:$manifest_sha,lockfile_path:$lock_path,cargo_lock_sha256:$lock_sha,cargo_metadata_sha256:$metadata_sha,workspace_cargo_config_path:(if $cargo_cfg_path=="" then null else $cargo_cfg_path end),workspace_cargo_config_sha256:(if $cargo_cfg_sha=="" then null else $cargo_cfg_sha end),cargo_home_config_sha256:(if $cargo_home_cfg_sha=="" then null else $cargo_home_cfg_sha end)},
    build:{command:["cargo","build","--locked","--all-targets","--offline","--target",$host],incremental:false,rustflags:$rustflags,cargo_encoded_rustflags:$encoded_flags,cargo_build_rustflags:$build_flags,rustc_wrapper:"disabled",rustc_workspace_wrapper:"disabled",target_directory:"target",build_directory:"build",layout_policy:"cargo_json_messages_only"},
    inputs:{cargo_metadata_sha256:$metadata_sha,cargo_messages_sha256:$raw_sha,diagnostics_sha256:$diag_sha,rustc_version_file_sha256:$rustc_sha,cargo_version_file_sha256:$cargo_sha,compiler_artifacts_file_sha256:$records_sha},
    artifacts:{compiler_artifact_count:($artifacts[0]|length),file_count:([$artifacts[0][]|.files[]]|length),compiler_artifacts:$artifacts[0]}
  }' > "$passport"
sha256sum "$passport" > "$output/passport.sha256"
bash "$root/scripts/verify-cargo-artifact-passport.sh" "$output" "$root"
printf 'artifact_passport=%s\n' "$passport"
printf 'artifact_passport_sha256=%s\n' "$(sha "$passport")"
printf 'compiler_artifact_records=%s\n' "$(jq -r '.artifacts.compiler_artifact_count' "$passport")"
printf 'artifact_files=%s\n' "$(jq -r '.artifacts.file_count' "$passport")"
