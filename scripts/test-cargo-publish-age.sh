#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-1.100.0}"
EXPECTED_VERSION="${2:-$TOOLCHAIN}"
ROOT="$(git rev-parse --show-toplevel)"
WORK="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

command -v rustup >/dev/null 2>&1 || { echo "rustup is required" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 is required for the loopback-only static index server" >&2; exit 2; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 2; }
command -v tar >/dev/null 2>&1 || { echo "tar is required" >&2; exit 2; }

RUSTC_VERSION="$(rustup run "$TOOLCHAIN" rustc --version)"
CARGO_VERSION="$(rustup run "$TOOLCHAIN" cargo --version)"
[[ "$RUSTC_VERSION" == "rustc $EXPECTED_VERSION "* ]] || {
  echo "Expected exact compiler rustc $EXPECTED_VERSION; got: $RUSTC_VERSION" >&2
  exit 2
}
[[ "$CARGO_VERSION" == "cargo $EXPECTED_VERSION "* ]] || {
  echo "Expected Cargo $EXPECTED_VERSION paired with the compiler; got: $CARGO_VERSION" >&2
  exit 2
}

REGISTRY="$WORK/registry"
BUILD="$WORK/crates"
CARGO_HOME="$WORK/cargo-home"
mkdir -p "$REGISTRY/index" "$REGISTRY/api/v1/crates" "$BUILD" "$CARGO_HOME"
export CARGO_HOME
export CARGO_TERM_COLOR=never
export CARGO_NET_RETRY=0
export CARGO_HTTP_TIMEOUT=10

PORT="$(python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

index_path() {
  local name="$1"
  local n="${#name}"
  if (( n == 1 )); then
    printf '1/%s' "$name"
  elif (( n == 2 )); then
    printf '2/%s' "$name"
  elif (( n == 3 )); then
    printf '3/%s/%s' "${name:0:1}" "$name"
  else
    printf '%s/%s/%s' "${name:0:2}" "${name:2:2}" "$name"
  fi
}

add_version() {
  local name="$1" version="$2" pubtime="$3" symbol="$4"
  local pkg="$BUILD/$name-$version"
  local rel idx crate_file checksum
  mkdir -p "$pkg/src"
  cat > "$pkg/Cargo.toml" <<EOF
[package]
name = "$name"
version = "$version"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"
EOF
  cat > "$pkg/src/lib.rs" <<EOF
pub const FIXTURE_VALUE: &str = "$symbol";
EOF
  rel="$(index_path "$name")"
  idx="$REGISTRY/index/$rel"
  mkdir -p "$(dirname "$idx")"
  crate_file="$REGISTRY/api/v1/crates/$name/$version/download"
  mkdir -p "$(dirname "$crate_file")"
  tar --sort=name --mtime='UTC 2026-01-01' --owner=0 --group=0 --numeric-owner \
    -czf "$crate_file" -C "$BUILD" "$name-$version"
  checksum="$(sha256sum "$crate_file" | awk '{print $1}')"
  printf '{"name":"%s","vers":"%s","deps":[],"cksum":"%s","features":{},"yanked":false,"pubtime":"%s"}\n' \
    "$name" "$version" "$checksum" "$pubtime" >> "$idx"
}

OLD_PUBTIME="$(date -u -d '30 days ago' +%Y-%m-%dT%H:%M:%SZ)"
FRESH_PUBTIME="$(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%SZ)"

# One old and one too-new version: deny must choose the older compatible release.
add_version age-choice 0.1.0 "$OLD_PUBTIME" old
add_version age-choice 0.2.0 "$FRESH_PUBTIME" fresh

# A package whose only version is too new: deny must produce a resolution error.
add_version fresh-only 0.1.0 "$FRESH_PUBTIME" fresh-only

cat > "$REGISTRY/index/config.json" <<EOF
{"dl":"http://127.0.0.1:$PORT/api/v1/crates/{crate}/{version}/download"}
EOF

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$REGISTRY" \
  > "$WORK/registry-http.log" 2>&1 &
SERVER_PID=$!

READY=false
for _ in $(seq 1 50); do
  if curl -fsS "http://127.0.0.1:$PORT/index/config.json" > /dev/null; then
    READY=true
    break
  fi
  sleep 0.1
done
[[ "$READY" == true ]] || {
  cat "$WORK/registry-http.log" >&2 || true
  echo "local sparse registry did not start" >&2
  exit 1
}

DENY_CONFIG="$WORK/cargo-deny.toml"
ALLOW_CONFIG="$WORK/cargo-allow.toml"
cat > "$DENY_CONFIG" <<EOF
[registries.local-age]
index = "sparse+http://127.0.0.1:$PORT/index/"

[registry]
global-min-publish-age = "14 days"

[resolver]
incompatible-publish-age = "deny"
EOF
sed 's/incompatible-publish-age = "deny"/incompatible-publish-age = "allow"/' \
  "$DENY_CONFIG" > "$ALLOW_CONFIG"

new_project() {
  local dir="$1" package="$2" dep="$3" requirement="$4"
  mkdir -p "$dir/src"
  cat > "$dir/Cargo.toml" <<EOF
[package]
name = "$package"
version = "0.1.0"
edition = "2021"

[dependencies]
$dep = { version = "$requirement", registry = "local-age" }
EOF
  cat > "$dir/src/main.rs" <<EOF
fn main() {
    println!("cargo-publish-age-fixture");
}
EOF
}

cargo_check() {
  local project="$1" config="$2" lockflag="${3:-}"
  local args=(check --manifest-path "$project/Cargo.toml" --config "$config")
  if [[ -n "$lockflag" ]]; then args+=("$lockflag"); fi
  (cd "$WORK" && rustup run "$TOOLCHAIN" cargo "${args[@]}")
}

# Control A: deny must choose the 30-day-old compatible release.
CHOICE="$WORK/choice"
new_project "$CHOICE" age-choice-app age-choice ">=0.1.0, <0.3.0"
cargo_check "$CHOICE" "$DENY_CONFIG"
grep -A2 '^name = "age-choice"$' "$CHOICE/Cargo.lock" | grep -F 'version = "0.1.0"' > /dev/null || {
  echo "FAIL: resolver did not choose the 30-day-old version under a 14-day deny policy" >&2
  exit 1
}
echo "publish_age_old_version_fallback=PASS"

# Control B: when the only compatible version is too new, deny must fail for age.
FRESH="$WORK/fresh-only"
new_project "$FRESH" fresh-only-app fresh-only "0.1"
set +e
cargo_check "$FRESH" "$DENY_CONFIG" > "$WORK/fresh-deny.log" 2>&1
status=$?
set -e
cat "$WORK/fresh-deny.log"
if [[ $status -eq 0 ]]; then
  echo "FAIL: Cargo accepted a fresh-only dependency while publish-age deny was enabled" >&2
  exit 1
fi
grep -Eiq 'too new|minimum age|minimum publish age' "$WORK/fresh-deny.log" || {
  echo "FAIL: fresh-only rejection was not diagnosed as a publish-age decision" >&2
  exit 1
}
echo "publish_age_fresh_only_rejected=PASS"

# Control C: an explicit allow resolution may enter Cargo.lock; deny --locked
# then preserves that already-pinned version.
cargo_check "$FRESH" "$ALLOW_CONFIG"
grep -A2 '^name = "fresh-only"$' "$FRESH/Cargo.lock" | grep -F 'version = "0.1.0"' > /dev/null || {
  echo "FAIL: allow control did not lock the fresh version" >&2
  exit 1
}
cargo_check "$FRESH" "$DENY_CONFIG" --locked
echo "publish_age_existing_lock_exception=PASS"

{
  echo "publish_age_toolchain=$RUSTC_VERSION"
  echo "publish_age_cargo=$CARGO_VERSION"
  echo "publish_age_minimum=14 days"
  echo "publish_age_registry=isolated loopback sparse registry with explicit pubtime"
  echo "publish_age_old_version_fallback=PASS"
  echo "publish_age_fresh_only_rejected=PASS"
  echo "publish_age_existing_lock_exception=PASS"
  echo "publish_age_network_scope=127.0.0.1 only"
}
