# Rust toolchain qualification v0.4: stable 1.99.1 baseline, 1.100.0 release candidate, 1.101 beta canary

**Status:** Rust/Cargo 1.99.1 is the previously verified project baseline. Rust 1.100.0 is now officially published (2026-11-12) and is undergoing exact-head qualification. The project default remains unchanged until the candidate clears all gates. Rust 1.101.0-beta.1 is a separate, non-blocking forward-compatibility canary.

## Release-day chronology and evidence

1. Earlier on 2026-11-12, GitHub Actions verified Rust/Cargo 1.99.1 as the published stable channel. At that observation, an exact `rustup toolchain install 1.100.0` attempt returned HTTP 404 and beta resolved to 1.101.0-beta.1. We correctly skipped candidate checks then.
2. Later on 2026-11-12, the official Rust release notes published **Version 1.100.0 (2026-11-12)**. The Rust Forge schedule also placed 1.100.0's stable date on November 12.
3. The release gate in this PR discovers the actual stable channel. On this updated head it should now identify 1.100.0 and execute the exact candidate checks; the run result is the authority for install/build availability. Do not transfer the earlier 1.99.1 or beta smoke results to 1.100.0.

The 1.100.0 release notes are available at https://dev-doc.rust-lang.org/releases.html. The release announcement index at https://blog.rust-lang.org/releases/ may lag the compiler release notes on release day; do not use its stale list alone to infer that the distribution is unavailable.

## 1.100 capabilities to qualify

The 1.100 release notes introduce:
1. Cargo's own lint framework and the `cargo::unused_dependencies` lint configured through `[lints.cargo]`.
2. Minimum publish-age dependency cooldown configuration. The resolver ignores versions newer than the configured minimum publish age unless the version is already present in Cargo.lock.
3. A new intermediate build-artifact directory layout. It is internal, not a stable interface; tooling should consume Cargo's `compiler-artifact` JSON messages.
4. Stabilization of the never type and related inference changes.
5. Stabilized `extern "custom"` and other compiler/library updates listed in the official release notes.

The isolated unused-dependency fixture declares a local path dependency not referenced by the fixture source and requires Cargo to emit the expected warning. This test already passed under Rust/Cargo 1.101.0-beta.1; the exact 1.100.0 run remains separately required.

## Cargo publication-age policy

The opt-in canary configures a 14-day minimum publish age and `resolver.incompatible-publish-age = "deny"`. Existing locked versions remain usable, while newly resolved too-recent versions should be ignored. The current workspace fetch exercises the locked-graph path. A deterministic local sparse-registry negative control with a deliberately too-new `pubtime` is still required before claiming the resolver rejects such a version in our own test corpus.

Do not treat a green locked fetch as proof that the negative-control behavior works.

## Build artifact interface

Cargo's intermediate build directory is internal and may change. Tooling should consume `compiler-artifact` JSON messages emitted by:

```sh
cargo build --message-format=json-render-diagnostics
```

The manifest captures package identity, target metadata, final artifact filenames, executable path and freshness. It does not guess `target/debug/deps`, `target/debug/build`, or other private intermediate paths.

## ABI regression test

Rust 1.99.1 fixed an x86_64 FFI `bool` return-value miscompilation where upper return-register bits could be unspecified. A conventional C `_Bool` check can normalize and hide the defect. The new x86_64 assembly observer calls the Rust function and exposes raw EAX as `uint32_t`, requiring exactly 0 or 1.

Hosted evidence:
- Rust 1.99.1 baseline raw-register probe: PASS.
- Rust 1.101.0-beta.1 isolated raw-register probe: PASS.
- Rust 1.100.0 exact stable probe: pending the current release-gated run.

## Qualification gates

- Discover and record exact published stable rustc/Cargo identity.
- Run Rust 1.100.0 only when both rustc and Cargo identify as 1.100.0.
- Keep default `rust-toolchain.toml`, Cargo.toml and Cargo.lock unchanged.
- Require locked metadata, all-target check, tests, Clippy and rustdoc.
- Require raw x86_64 ABI probe.
- Require Cargo-native unused-dependency fixture.
- Produce artifact manifest through Cargo JSON messages.
- Verify the locked workspace remains compatible with the 14-day publish-age policy.
- Keep 1.101.0-beta.1 as a separate non-blocking forward-compatibility canary.
- Record every skipped or failed stage explicitly; no implicit promotion.

## Workspace blocker discovered by the forward canary

Rust/Cargo 1.101.0-beta.1 installed and fetched the locked dependency graph successfully, but all-target compilation exposed a repository source/API mismatch:

`examples/hdc_cooperation.rs` imports `symtropy_consciousness_physics::hdc_context`, which is not exported by the crate root. This is tracked separately in issue #3. It is not attributed to the compiler, and the all-target check has not been weakened. The isolated beta ABI/Cargo-lint/artifact smoke job passed independently.

## Performance research: compare both the compiler and the application

The exact 1.99.1 versus 1.100.0 benchmark is available only after the release distribution is present:

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup toolchain install 1.100.0 --profile minimal
BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash scripts/benchmark-rust-1.99.1-vs-1.100.0.sh
```

The wrapper refuses to run unless rustc and Cargo both identify as exact 1.100.0. It also requires a clean worktree, unchanged lockfile, fresh target dirs, offline measured builds, no sccache wrapper, rotated compiler order and raw outputs.

Compare:
- upstream rustc-perf fresh/incremental workloads for compiler-wide performance;
- Cargo `--timings` for build-graph and critical-path diagnosis;
- compile wall time, CPU time, peak RSS, build dir size and final binary size;
- deterministic Symtropy release simulation runtime and numerical outcomes;
- fixed-seed determinism and independent-build reproducibility;
- cache hit/miss separately from uncached compiler performance.

Five trials are exploratory only. Do not claim p99 from five measurements. No performance gain is assumed from the version number.

## SOTA and promotion standard

We do not claim superiority over rustc-perf using unrelated application examples. Fair comparison uses rustc-perf for compiler-wide behavior, then Symtropy workloads for real application behavior. Qualification requires correctness and reproducibility first; speed is an independent measurement.

Keep the 1.99.1 default until exact stable 1.100.0 checks pass and any candidate-only result differences are explained. Any pin promotion requires a separate reviewed change.

## References

- Official Rust 1.100 release notes: https://dev-doc.rust-lang.org/releases.html
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- rustc-perf profiling: https://rustc-dev-guide.rust-lang.org/profiling/with-rustc-perf.html
- Cargo lints: https://doc.rust-lang.org/cargo/reference/lints.html
- Cargo publish-age configuration: https://doc.rust-lang.org/nightly/cargo/reference/config.html
- Minimum publish age RFC: https://rust-lang.github.io/rfcs/3923-cargo-min-publish-age.html
