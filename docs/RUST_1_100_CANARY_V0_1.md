# Rust 1.100.0 stable qualification v0.2

**Status:** exact stable-toolchain qualification lane; not the default toolchain and not a promotion decision.

## Baseline and candidate

- Stable baseline: Rust/Cargo 1.99.1, pinned by `rust-toolchain.toml` in the underlying qualification branch.
- Candidate: exact Rust/Cargo 1.100.0 stable, installed by version rather than a mutable channel alias.
- The workflow asserts both compiler and Cargo versions. It fails rather than silently substituting a different release.
- The candidate lane does not edit the default toolchain pin, Cargo.toml, Cargo.lock, edition, MSRV, simulation source, seeds or experiment parameters.

Rust 1.100.0 release notes are dated 2026-11-12. The exact distribution availability is verified by the hosted `rustup toolchain install 1.100.0` gate; a release-note page alone is not proof of installation availability.

## 1.100 capabilities under test

1. Cargo-native lint configuration, including `unused_dependencies`.
2. Cargo minimum publication-age configuration, with a 14-day cooldown and deny behavior.
3. Lockfile compatibility while the cooldown is enabled. Cargo's RFC explicitly allows versions already present in Cargo.lock; new resolution should reject versions whose published time is too recent.
4. Cargo 1.100's changed intermediate build-directory layout.
5. Rust language/compiler changes in the 1.100 release notes, including never-type stabilization and updated inference/compatibility behavior.

The unused-dependency fixture deliberately declares a local path dependency that no Rust source uses. The canary requires Cargo to emit its own unused-dependency warning. This validates that the Cargo-native lint is active without changing the scientific package's dependencies.

The publish-age setting is applied only to the qualification commands via an explicitly named config file. It does not affect stable local builds. The current workspace check exercises existing locked dependencies under the cooldown. A fully deterministic negative-control registry fixture is still required before claiming that a newly published dependency is rejected; this lane does not claim that stronger result yet.

## 1.100 build artifact interface

Cargo's intermediate build directory is internal and may change. Tooling should consume `compiler-artifact` JSON messages emitted by:

```sh
cargo build --message-format=json-render-diagnostics
```

The canary captures package identity, target metadata, final artifact filenames, executable path and freshness from those structured messages. It does not guess `target/debug/deps`, `target/debug/build`, or other private intermediate paths.

The audit target is not "make the directory tree stable"; it is "remove downstream dependence on an unstable implementation detail."

## ABI regression test

The original C smoke test checked only C's normalized `_Bool` value. This can hide an invalid upper portion of the integer return register. The new x86_64 assembly observer calls the Rust function and returns its raw EAX value as `uint32_t`, requiring exactly 0 or 1. This directly tests the ABI property fixed in Rust 1.99.1.

## Qualification gates

- Candidate must install and identify as exactly Rust 1.100.0 stable.
- Cargo must come from the same rustup toolchain and identify as exactly 1.100.0.
- Existing Cargo.lock remains byte-identical.
- Locked metadata, all-target check, test, Clippy and rustdoc must succeed.
- ABI raw-register probe passes on x86_64 Linux.
- Cargo-native unused-dependency fixture emits the expected warning.
- Build artifact manifest is produced from Cargo JSON messages.
- Publication-age configuration is parsed and existing locked dependencies remain usable.

## Remaining research

- Build a deterministic local sparse-registry test for a too-new pubtime negative control.
- Measure 1.99.1 versus 1.100.0 compiler throughput, peak RSS, incremental work and release simulation behavior on the same host.
- Compare numerical outputs and experiment conclusions, not only stdout digests.
- Test the candidate on any other actual product target (for example, WASM/Holochain) only after inspecting the project's real target requirements.
- Evaluate cache hit-rate and binary reproducibility separately from the no-cache compiler benchmark.

## References

- Rust 1.100 release notes: https://dev-doc.rust-lang.org/releases.html
- Cargo lints: https://doc.rust-lang.org/nightly/cargo/reference/lints.html
- Cargo build cache: https://doc.rust-lang.org/nightly/cargo/reference/build-cache.html
- Minimum publish age RFC: https://rust-lang.github.io/rfcs/3923-cargo-min-publish-age.html
