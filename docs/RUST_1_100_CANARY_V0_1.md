# Rust toolchain qualification v0.5: stable 1.99.1, scheduled 1.100.0 release, 1.101 beta canary

**Status at the latest hosted observation on 2026-11-12:** Rust/Cargo 1.99.1 remains the only verified published stable toolchain available to this workflow. Rust 1.100.0 has a scheduled stable date of November 12, but the official release-notes tracker is still marked as a draft and the exact distribution install returns HTTP 404. Rust/Cargo 1.101.0-beta.1 is a separate, non-blocking forward-compatibility canary. The project default remains unchanged.

## Release schedule versus distribution availability

The evidence must be kept distinct:

1. The official Rust Forge release table lists Stable 1.99 and Beta 1.100, with 1.100 scheduled to become stable on November 12, 2026.
2. The Rust 1.100.0 release-notes tracking issue is explicitly titled "Draft release notes for 1.100.0". It contains the expected version/date and proposed Cargo/compiler changes, but is not an official stable distribution announcement.
3. On the latest hosted qualification run, the stable channel resolves to rustc 1.99.1 / Cargo 1.99.1.
4. A direct `rustup toolchain install 1.100.0` attempt, independent of the moving stable alias, returns HTTP 404 for the official static distribution checksum.
5. Therefore, exact 1.100.0 qualification is **NOT AVAILABLE / NOT RUN** at that observation. The scheduled release date must not be represented as proof of publication, installation or compatibility.

Sources:
- Rust Forge release schedule: https://forge.rust-lang.org/
- Draft Rust 1.100 release-notes tracker: https://github.com/rust-lang/rust/issues/163811
- Official release announcement index: https://blog.rust-lang.org/releases/

The workflow separately records the live stable-channel identity and directly attempts exact 1.100.0 installation. When the exact distribution becomes available, the job must assert both rustc and Cargo are exactly 1.100.0 before any candidate checks run. A stale stable alias can no longer prevent the exact candidate from being attempted.

## 1.100 capabilities to qualify when published

The draft release-notes tracker identifies these planned changes:
1. Cargo's native lint framework, including `cargo::unused_dependencies` under `[lints.cargo]`.
2. Minimum publish-age/dependency cooldown configuration; the resolver ignores versions that are too recent unless already present in Cargo.lock.
3. A new intermediate build-artifact directory layout. This is an internal layout; consumers should use Cargo JSON `compiler-artifact` messages instead of guessing private directories.
4. Never-type stabilization and associated compiler/library changes.
5. Stabilized `extern "custom"` and other language/library updates listed in the tracker.

These are draft release-note entries until stable publication is verified. The isolated Cargo lint fixture has already passed under Rust/Cargo 1.101.0-beta.1; this is beta evidence only and does not imply 1.100.0 availability.

## Cargo publication-age policy

The opt-in canary configures a 14-day minimum publish age and `resolver.incompatible-publish-age = "deny"`. Existing locked versions remain usable, while newly resolved too-recent versions should be ignored. The current workspace fetch exercises the locked-graph path. A deterministic local sparse-registry negative control with a deliberately too-new `pubtime` remains necessary before claiming the resolver rejects such a version in our own test corpus.

A green locked fetch is not proof that the negative-control behavior works.

### Executable local-registry negative control

The branch now includes `scripts/test-cargo-publish-age.sh`. It creates a disposable sparse registry served only on loopback, and writes explicit `pubtime` values into its index entries. No crates.io traffic is required.

The pinned `nightly-2026-11-11` job runs Cargo's `-Zmin-publish-age` experiment and tests three independent outcomes:

1. **Old-version fallback:** a 30-day-old release and a one-hour-old release both satisfy the semver requirement; a 14-day deny policy must select the older release.
2. **Fresh-only rejection:** when the only compatible version is one hour old, deny must fail specifically because the version is too new.
3. **Existing-lock exception:** the fresh-only release is first selected using the explicit `allow` override and written to Cargo.lock; a later `--locked` resolution under deny must preserve it.

**Hosted exact-head result: PASS** on run [#43267753894](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/actions/runs/43267753894), job `Cargo publish-age deterministic negative controls (dated nightly)`.

The dated selector `nightly-2026-11-11` installed:
- rustc `1.102.0-nightly (9c5648833 2026-11-10)`;
- Cargo `1.102.0-nightly (c07ac2c4d 2026-11-06)`.

All three controls passed:
- `publish_age_old_version_fallback=PASS`: Cargo chose `age-choice 0.1.0` over `0.2.0` published one hour earlier.
- `publish_age_fresh_only_rejected=PASS`: Cargo rejected the only available version and reported that it was too new (published one hour ago; minimum age 14 days).
- `publish_age_existing_lock_exception=PASS`: after the explicit `allow` resolution wrote `fresh-only 0.1.0` to Cargo.lock, a subsequent `--locked` resolution under `deny` preserved it.

The test uses an exact dated nightly because the full `min-publish-age` feature is still documented under Cargo's unstable-feature reference. This is research evidence for Cargo's resolver semantics, not a statement that the feature is available on stable Rust 1.100.0 or that a stable 1.100.0 distribution exists.

The test registry is a disposable sparse registry served only on `127.0.0.1`; it publishes explicit `pubtime` values and does not contact crates.io. The RFC notes that this check applies to registries that publish `pubtime`; it does not apply to path/git dependencies or registries without publish-time metadata.

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
- Rust 1.100.0 exact stable probe: NOT RUN because exact distribution installation returned 404.

## Qualification gates

- Record the live stable-channel rustc/Cargo identity.
- Attempt exact Rust 1.100.0 distribution installation independently of the stable alias.
- Run Rust 1.100.0 only when both rustc and Cargo identify as exactly 1.100.0.
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

## Reproducible paired-result analysis

The benchmark output is not just a set of fastest-run numbers. The dependency-free Rust analyzer is compiled and self-tested in the isolated 1.101.0-beta.1 smoke lane:

```sh
RUSTC_TOOLCHAIN=1.99.1 bash scripts/analyze-rust-toolchain-benchmark.sh --self-test
```

For a completed run:

```sh
RUSTC_TOOLCHAIN=1.99.1 bash scripts/analyze-rust-toolchain-benchmark.sh \
  benchmark-results/rust-toolchain/run-<id>/results/measurements.csv \
  1.99.1 1.100.0
```

It pairs trials by exact trial/example key, de-duplicates build-level measurements, verifies contiguous runtime repeats, rejects unstable within-trial stdout digests and inconsistent build metadata, and calculates median/MAD and paired speedup estimates with a deterministic bootstrap interval. It includes peak RSS and artifact-size comparisons. p99 is withheld until both toolchains have at least 10,000 runtime observations for the workload.

The analyzer is standard-library-only and does not alter Cargo.toml or Cargo.lock. Its output is still an engineering signal, not a scientific correctness proof: matching stdout digests do not replace numerical tolerance and invariant checks.

## Performance research: compare compiler and application separately

The exact 1.99.1 versus 1.100.0 benchmark must wait until the 1.100.0 distribution exists:

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
- compile wall time, CPU time, peak RSS, build directory size and final binary size;
- deterministic Symtropy release simulation runtime and numerical outcomes;
- fixed-seed determinism and independent-build reproducibility;
- cache hit/miss separately from uncached compiler performance.

Five trials are exploratory only. Do not claim p99 from five measurements. Use 30+ independent paired runs for ordinary latency-distribution claims and at least 10,000 observations per condition (or a justified design) for p99. No performance gain is assumed from the version number.

## SOTA and promotion standard

We do not claim superiority over rustc-perf using unrelated application examples. Fair comparison uses rustc-perf for compiler-wide behavior, then Symtropy workloads for real application behavior. Qualification requires correctness and reproducibility first; speed is an independent measurement.

Keep the 1.99.1 default until exact stable 1.100.0 checks pass and any candidate-only result differences are explained. Any pin promotion requires a separate reviewed change.

## References

- Rust Forge release schedule: https://forge.rust-lang.org/
- Rust 1.100 draft release-notes tracker: https://github.com/rust-lang/rust/issues/163811
- Official Rust release announcements: https://blog.rust-lang.org/releases/
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- rustc-perf profiling: https://rustc-dev-guide.rust-lang.org/profiling/with-rustc-perf.html
- Cargo lints: https://doc.rust-lang.org/cargo/reference/lints.html
- Cargo publish-age configuration: https://doc.rust-lang.org/nightly/cargo/reference/config.html
- Minimum publish age RFC: https://rust-lang.github.io/rfcs/3923-cargo-min-publish-age.html
