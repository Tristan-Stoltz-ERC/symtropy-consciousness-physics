#!/usr/bin/env bash
set -Eeuo pipefail
root="$(mktemp -d)"
out="$(mktemp -d)"
trap 'rm -rf "$root" "$out"' EXIT
mkdir -p "$root" "$out/target/debug/deps" "$out/build/out"
printf 'lock-v1\n' > "$root/Cargo.lock"
printf '[package]\nname="fixture"\nversion="0.1.0"\n' > "$root/Cargo.toml"
git -C "$root" init -q
git -C "$root" add Cargo.lock Cargo.toml
git -C "$root" -c user.name=PassportTest -c user.email=passport@example.invalid commit -qm fixture
printf 'target-byte-stream\n' > "$out/target/debug/deps/libfixture-target.rlib"
printf 'build-byte-stream\n' > "$out/build/out/libfixture-build.rlib"
printf '{"packages":[],"workspace_members":[]}\n' > "$out/cargo-metadata.json"
printf 'rustc 1.100.0\nhost: x86_64-unknown-linux-gnu\ncommit-hash: fixture\n' > "$out/rustc-version.txt"
printf 'cargo 1.100.0\n' > "$out/cargo-version.txt"
printf 'empty diagnostics\n' > "$out/diagnostics.log"
jq -cn --arg target "$out/target/debug/deps/libfixture-target.rlib" --arg build "$out/build/out/libfixture-build.rlib" '{
 reason:"compiler-artifact",package_id:"path+file:///fixture#fixture@0.1.0",
 target:{name:"fixture",kind:["lib"],crate_types:["lib"],src_path:"src/lib.rs",edition:"2021",doc:true,doctest:true,test:true},
 profile:{opt_level:"0"},filenames:[$target,$build],executable:null,fresh:false
}' > "$out/cargo-messages.jsonl"
sha() { sha256sum -- "$1" | awk '{print $1}'; }
jq -cn --arg th "$(sha "$out/target/debug/deps/libfixture-target.rlib")" --argjson ts "$(stat -c '%s' "$out/target/debug/deps/libfixture-target.rlib")" \
 --arg bh "$(sha "$out/build/out/libfixture-build.rlib")" --argjson bs "$(stat -c '%s' "$out/build/out/libfixture-build.rlib")" '[
 {package_id:"path+file:///fixture#fixture@0.1.0",target:{name:"fixture",kind:["lib"],crate_types:["lib"],src_path:"src/lib.rs",edition:"2021",doc:true,doctest:true,test:true},profile:{opt_level:"0"},fresh:false,has_executable:false,
 files:[{root:"target",path:"debug/deps/libfixture-target.rlib",sha256:$th,size_bytes:$ts},{root:"build",path:"out/libfixture-build.rlib",sha256:$bh,size_bytes:$bs}]}
]' > "$out/compiler-artifacts.json"
jq -n -S --arg commit "$(git -C "$root" rev-parse HEAD)" --arg tree "$(git -C "$root" rev-parse 'HEAD^{tree}')" \
 --arg lock "$(sha "$root/Cargo.lock")" --arg manifest "$(sha "$root/Cargo.toml")" --arg metadata "$(sha "$out/cargo-metadata.json")" \
 --arg messages "$(sha "$out/cargo-messages.jsonl")" --arg diag "$(sha "$out/diagnostics.log")" \
 --arg rustc "$(sha "$out/rustc-version.txt")" --arg cargo "$(sha "$out/cargo-version.txt")" --arg artifacts "$(sha "$out/compiler-artifacts.json")" \
 --slurpfile records "$out/compiler-artifacts.json" '{
 schema:"luminous.cargo-artifact-passport.v0.1",
 source:{git_commit:$commit,git_tree:$tree,manifest_path:"Cargo.toml",manifest_sha256:$manifest,lockfile_path:"Cargo.lock",cargo_lock_sha256:$lock},
 inputs:{cargo_metadata_sha256:$metadata,cargo_messages_sha256:$messages,diagnostics_sha256:$diag,rustc_version_file_sha256:$rustc,cargo_version_file_sha256:$cargo,compiler_artifacts_file_sha256:$artifacts},
 artifacts:{compiler_artifact_count:($records[0]|length),file_count:([$records[0][]|.files[]]|length),compiler_artifacts:$records[0]}
}' > "$out/passport.json"
sha256sum "$out/passport.json" > "$out/passport.sha256"
script_dir="$(cd "$(dirname "$0")" && pwd)"
bash "$script_dir/verify-cargo-artifact-passport.sh" "$out" "$root"

printf 'tamper\n' >> "$out/target/debug/deps/libfixture-target.rlib"
if bash "$script_dir/verify-cargo-artifact-passport.sh" "$out" "$root" >/dev/null 2>&1; then echo "accepted artifact tamper" >&2; exit 1; fi
printf 'target-byte-stream\n' > "$out/target/debug/deps/libfixture-target.rlib"
printf 'tamper\n' >> "$out/cargo-metadata.json"
if bash "$script_dir/verify-cargo-artifact-passport.sh" "$out" "$root" >/dev/null 2>&1; then echo "accepted metadata tamper" >&2; exit 1; fi
printf '{"packages":[],"workspace_members":[]}\n' > "$out/cargo-metadata.json"
printf 'lock-v2\n' > "$root/Cargo.lock"
if bash "$script_dir/verify-cargo-artifact-passport.sh" "$out" "$root" >/dev/null 2>&1; then echo "accepted lockfile tamper" >&2; exit 1; fi
echo "PASS_ARTIFACT_PASSPORT_SELF_TEST"
