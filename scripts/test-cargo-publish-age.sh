#!/usr/bin/env bash
set -Eeuo pipefail

TOOLCHAIN="${1:-nightly-2026-11-11}"
EXPECTED_RUSTC_PREFIX="${2:-rustc 1.102.0-nightly }"
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
[[ "$RUSTC_VERSION" == "$EXPECTED_RUSTC_PREFIX"* ]] || {
  echo "Expected exact dated nightly compiler prefix '$EXPECTED_RUSTC_PREFIX'; got: $RUSTC_VERSION" >&2
  exit 2
}
[[ "$CARGO_VERSION" == cargo\ 1.102.0-nightly\ * ]] || {
  echo "Expected Cargo 1.102.0-nightly paired with the compiler; got: $CARGO_VERSION" >&2
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
min-publish-age = "14 days"

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

# Control A: when a fresh release and a 30-day-old compatible release both
# exist, deny must resolve the older version (not the latest version).
CHOICE="$WORK/choice"
new_project "$CHOICE" age-choice-app age-choice ">=0.1.0, <0.3.0"
(
  cd "$WORK"
  rustup run "$TOOLCHAIN" cargo -Zmin-publish-age check \
    --manifest-path "$CHOICE/Cargo.toml" \
    --config "$DENY_CONFIG"
)
grep -A2 '^name = "age-choice"$' "$CHOICE/Cargo.lock" | grep -F 'version = "0.1.0"' > /dev/null || {
  echo "FAIL: resolver did not choose the 30-day-old version under a 14-day deny policy" >&2
  exit 1
}
echo "publish_age_old_version_fallback=PASS"

# Control B: with no old version available, deny must fail specifically because
# the only compatible version is younger than the 14-day minimum.
FRESH="$WORK/fresh-only"
new_project "$FRESH" fresh-only-app fresh-only "0.1"
set +e
(
  cd "$WORK"
  rustup run "$TOOLCHAIN" cargo -Zmin-publish-age check \
    --manifest-path "$FRESH/Cargo.toml" \
    --config "$DENY_CONFIG"
) > "$WORK/fresh-deny.log" 2>&1
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

# Control C: allow is an explicit exception path. Once a fresh dependency is
# recorded in Cargo.lock, deny must preserve that existing lockfile selection.
(
  cd "$WORK"
  rustup run "$TOOLCHAIN" cargo -Zmin-publish-age check \
    --manifest-path "$FRESH/Cargo.toml" \
    --config "$ALLOW_CONFIG"
)
grep -A2 '^name = "fresh-only"$' "$FRESH/Cargo.lock" | grep -F 'version = "0.1.0"' > /dev/null || {
  echo "FAIL: allow control did not lock the fresh version" >&2
  exit 1
}
(
  cd "$WORK"
  rustup run "$TOOLCHAIN" cargo -Zmin-publish-age check --locked \
    --manifest-path "$FRESH/Cargo.toml" \
    --config "$DENY_CONFIG"
)
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
