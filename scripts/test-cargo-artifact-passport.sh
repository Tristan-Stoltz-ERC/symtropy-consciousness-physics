#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(mktemp -d)"
OUT="$(mktemp -d)"
trap 'rm -rf "$ROOT" "$OUT"' EXIT
mkdir -p "$OUT/target/debug/deps" "$OUT/build/out"
printf 'lock-v1\n' > "$ROOT/Cargo.lock"
printf '[package]\nname="fixture"\nversion="0.1.0"\n' > "$ROOT/Cargo.toml"
git -C "$ROOT" init -q
git -C "$ROOT" add Cargo.lock Cargo.toml
git -C "$ROOT" -c user.name=PassportTest -c user.email=passport@example.invalid commit -qm fixture

printf 'target-file\n' > "$OUT/target/debug/deps/libfixture.rlib"
printf 'build-file\n' > "$OUT/build/out/build-output.bin"
printf '{"packages":[],"workspace_members":[]}\n' > "$OUT/cargo-metadata.json"
printf '{"reason":"compiler-artifact"}\n' > "$OUT/cargo-messages.jsonl"
printf 'diagnostics\n' > "$OUT/diagnostics.log"
printf 'rustc 1.100.0\nhost: x86_64-unknown-linux-gnu\ncommit-hash: fixture\n' > "$OUT/rustc-version.txt"
printf 'cargo 1.100.0\n' > "$OUT/cargo-version.txt"
sha(){ sha256sum -- "$1" | awk '{print $1}'; }

t_sha="$(sha "$OUT/target/debug/deps/libfixture.rlib")"
t_size="$(stat -c '%s' "$OUT/target/debug/deps/libfixture.rlib")"
b_sha="$(sha "$OUT/build/out/build-output.bin")"
b_size="$(stat -c '%s' "$OUT/build/out/build-output.bin")"
jq -n -S --arg ts "$t_sha" --argjson tz "$t_size" --arg bs "$b_sha" --argjson bz "$b_size" '[
 {package_id:"fixture@0.1.0",target:{name:"fixture",kind:["lib"]},profile:{opt_level:"0"},fresh:false,has_executable:false,
  files:[{root:"target",path:"debug/deps/libfixture.rlib",sha256:$ts,size_bytes:$tz},{root:"build",path:"out/build-output.bin",sha256:$bs,size_bytes:$bz}]}
]' > "$OUT/compiler-artifacts.json"
commit="$(git -C "$ROOT" rev-parse HEAD)"
tree="$(git -C "$ROOT" rev-parse 'HEAD^{tree}')"
jq -n -S \
 --arg commit "$commit" --arg tree "$tree" \
 --arg lock "$(sha "$ROOT/Cargo.lock")" --arg manifest "$(sha "$ROOT/Cargo.toml")" \
 --arg meta "$(sha "$OUT/cargo-metadata.json")" --arg msg "$(sha "$OUT/cargo-messages.jsonl")" \
 --arg diag "$(sha "$OUT/diagnostics.log")" --arg rustc "$(sha "$OUT/rustc-version.txt")" \
 --arg cargo "$(sha "$OUT/cargo-version.txt")" --arg art "$(sha "$OUT/compiler-artifacts.json")" \
 --slurpfile records "$OUT/compiler-artifacts.json" \
 '{
  schema:"luminous.cargo-artifact-passport.v0.1",
  source:{git_commit:$commit,git_tree:$tree,manifest_path:"Cargo.toml",manifest_sha256:$manifest,lockfile_path:"Cargo.lock",cargo_lock_sha256:$lock},
  inputs:{cargo_metadata_sha256:$meta,cargo_messages_sha256:$msg,diagnostics_sha256:$diag,rustc_version_file_sha256:$rustc,cargo_version_file_sha256:$cargo,compiler_artifacts_file_sha256:$art},
  artifacts:{compiler_artifact_count:($records[0]|length),file_count:([$records[0][]|.files[]]|length),compiler_artifacts:$records[0]}
 }' > "$OUT/passport.json"
sha256sum "$OUT/passport.json" > "$OUT/passport.sha256"

HERE="$(cd "$(dirname "$0")" && pwd)"
bash "$HERE/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT"
printf 'tamper\n' >> "$OUT/target/debug/deps/libfixture.rlib"
if bash "$HERE/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT" >/dev/null 2>&1; then echo "target tamper accepted" >&2; exit 1; fi
printf 'target-file\n' > "$OUT/target/debug/deps/libfixture.rlib"
printf 'tamper\n' >> "$OUT/build/out/build-output.bin"
if bash "$HERE/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT" >/dev/null 2>&1; then echo "build tamper accepted" >&2; exit 1; fi
printf 'build-file\n' > "$OUT/build/out/build-output.bin"
printf 'tamper\n' >> "$OUT/cargo-metadata.json"
if bash "$HERE/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT" >/dev/null 2>&1; then echo "metadata tamper accepted" >&2; exit 1; fi
printf '{"packages":[],"workspace_members":[]}\n' > "$OUT/cargo-metadata.json"
printf 'lock-v2\n' > "$ROOT/Cargo.lock"
if bash "$HERE/verify-cargo-artifact-passport.sh" "$OUT" "$ROOT" >/dev/null 2>&1; then echo "lockfile tamper accepted" >&2; exit 1; fi
echo "PASS_ARTIFACT_PASSPORT_SELF_TEST"
