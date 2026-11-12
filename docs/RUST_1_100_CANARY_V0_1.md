# Rust toolchain qualification v0.3: stable 1.99.1, 1.100 release gate, 1.101 beta canary

**Status:** Rust 1.99.1 is the verified baseline. Rust 1.100.0 stable is a release gate, not yet a production pin. Rust 1.101.0-beta.1 is a separate, non-blocking forward-compatibility canary.

## Verified release status

At the observation point recorded in PR #2:

- The official Rust release-announcement index lists Rust 1.99.1 (October 15, 2026) as its latest published release.
- The Rust 1.100.0 release notes are present in development documentation, and the associated GitHub issue is a draft release-notes tracker.
- GitHub Actions tried to install `1.100.0-x86_64-unknown-linux-gnu` from `static.rust-lang.org` and received HTTP 404.
- Rustup's mutable beta alias resolved to 1.101.0-beta.1. We did not substitute it for 1.100.0.

Therefore, the default project toolchain remains pinned at Rust/Cargo 1.99.1. The exact stable workflow discovers the published stable channel and only runs the 1.100.0 suite when both rustc and Cargo identify as 1.100.0. It records a clear NOT YET AVAILABLE disposition otherwise.

The separate 1.101.0-beta.1 lane is explicitly non-blocking and is never treated as evidence that 1.100.0 stable is available.

## 1.100 Cargo capabilities to qualify when published

The 1.100 development release notes document:

1. Cargo's native lint framework and the `cargo::unused_dependencies` lint, configured through `[lints.cargo]`.
2. Minimum publication-age configuration / dependency cooldowns. The resolver ignores recently published versions unless they already exist in Cargo.lock.
3. A new intermediate build-artifact directory layout. The layout is internal and not a supported interface; tools should consume Cargo's `compiler-artifact` JSON messages instead of hard-coding private paths.
4. Rust language/compiler changes including never-type stabilization and the associated fallback change across editions.

The unused-dependency fixture deliberately declares a local path dependency that no Rust source uses. The canary requires Cargo to emit its unused-dependency warning without changing the scientific workspace's dependencies.

The publish-age configuration is applied only to the opt-in canary/qualification commands. It does not affect default local builds. The current workspace check exercises existing locked dependencies under the cooldown. A deterministic local-registry too-new-pubtime negative control remains necessary before claiming that a newly published version is rejected in our environment.

## Build artifact interface

Cargo's intermediate build directory is internal and may change. Tooling should consume `compiler-artifact` JSON messages emitted by:

```sh
cargo build --message-format=json-render-diagnostics
```

The manifest captures package identity, target metadata, final artifact filenames, executable path and freshness. It does not guess `target/debug/deps`, `target/debug/build`, or other private intermediate paths.

## ABI regression test

Rust 1.99.1 fixed an x86_64 FFI `bool` return-value miscompilation: upper bits of the return register could be unspecified. The original C `_Bool` smoke test normalized the return value and could hide that defect. The new x86_64 assembly observer calls the Rust function and returns the raw EAX value as `uint32_t`, requiring exactly 0 or 1.

The stable 1.99.1 baseline raw-register probe passed on GitHub Actions. The Rust 1.100 stable probe remains gated on actual distribution availability; the 1.101 beta probe is separate.

## Qualification gates

- Preserve Rust/Cargo 1.99.1 as the default until an exact stable candidate qualifies.
- Discover the published stable channel and record exact rustc/Cargo versions.
- Only run 1.100 tests when stable and Cargo both identify as exactly 1.100.0.
- Keep Cargo.lock byte-identical.
- Require locked metadata, all-target check, tests, Clippy and rustdoc.
- Require the raw x86_64 ABI probe.
- Require the Cargo-native unused-dependency fixture to emit the expected warning.
- Produce the compiler-artifact manifest from Cargo JSON messages.
- Verify existing locked dependencies remain usable under the publication-age policy.
- Keep 1.101.0-beta.1 as a non-blocking forward-compatibility lane.

## Performance research

Measure 1.99.1 vs 1.100.0 on the same source and lockfile after the stable candidate is available:

- clean-target debug and release build time;
- cargo check, tests, Clippy and rustdoc duration;
- peak RSS, target-directory size and binary size;
- release runtime of deterministic simulation examples;
- output digests and numerical result deltas;
- repeated-build reproducibility;
- FFI/unsafe and cross-target compatibility.

Use rustc-perf as the compiler ecosystem baseline, Cargo `--timings` to distinguish compiler and build-graph bottlenecks, and the Symtropy deterministic simulation examples as the application-specific benchmark. Measure sccache hit/miss behavior separately from cold compiler work.

Five paired runs are exploratory. Do not claim p99 from five repetitions. Tail claims require a much larger observation set or a justified statistical design.

## References

- Official Rust release announcements: https://blog.rust-lang.org/releases/
- Rust 1.100 development release notes: https://dev-doc.rust-lang.org/releases.html
- Rust 1.100 draft release-notes tracker: https://github.com/rust-lang/rust/issues/163811
- Cargo lints: https://doc.rust-lang.org/nightly/cargo/reference/lints.html
- Cargo build cache: https://doc.rust-lang.org/nightly/cargo/reference/build-cache.html
- Minimum publish age RFC: https://rust-lang.github.io/rfcs/3923-cargo-min-publish-age.html
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
