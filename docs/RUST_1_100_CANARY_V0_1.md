# Rust toolchain qualification v0.6 — stable 1.100.0, 1.101 beta canary

**Current status (2026-11-13):** Rust/Cargo 1.100.0 was officially released on 2026-11-12. The project qualification branch now installs the exact release and runs a supported-library suite, a separate full-target build, and isolated Cargo/language/ABI smoke tests. The exact stable suite was not run in the earlier 2026-11-12 observations that still returned HTTP 404; the new workflow commit is the first post-release exact-version qualification attempt. The default project pin remains 1.99.1.

## What changed in Rust 1.100.0

The official release announcement and release notes identify these relevant changes:

1. The never type `!` is stable.
2. Never-type fallback changes from `()` to `!` on every edition, including 2015/2018/2021 as well as 2024.
3. Cargo's `unused_dependencies` lint is stable and configurable via `[lints.cargo]`.
4. Cargo minimum publish age / dependency cooldown controls are stable.
5. Cargo changes the internal intermediate build-artifact directory layout and reduces build-cache collisions.
6. Sample-profile use through `-Cprofile-sample-use` is stabilized.

Sources:
- Rust 1.100.0 release announcement (2026-11-12): https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/
- Rust 1.100.0 release notes: https://doc.rust-lang.org/releases.html
- Never-type fallback migration note: https://doc.rust-lang.org/beta/edition-guide/rust-2024/never-type-fallback.html

The default pin and package MSRV are separate decisions. This qualification does not change either one.

## Release publication versus previous hosted evidence

The release is now officially published. The previous exact Rust 1.100.0 attempt in run #43267488582 happened on 2026-11-12 at 17:15 UTC and returned HTTP 404; that observation preceded the public release announcement. It is historical evidence, not today's availability status.

The current workflow now directly installs exact Rust/Cargo 1.100.0 and asserts both identities before running qualification. A new hosted result is required before saying that the Symtropy workspace or its examples pass on 1.100.0.

## Qualification lanes

### Exact Rust 1.100.0 stable — supported library

- Exact toolchain identity (`rustc -Vv`, `cargo -Vv`).
- Locked registry fetch using the 14-day minimum publish-age policy.
- `cargo metadata --locked --no-deps`.
- `cargo check --locked --lib`.
- `cargo test --locked --lib`.
- `cargo clippy --locked --lib`.
- `cargo doc --locked --no-deps --lib`.
- Raw x86_64 C ABI `bool` return-register probe.
- Cargo.lock immutability.

### Exact Rust 1.100.0 stable — all targets

A separate job runs `cargo check --locked --all-targets`. This is deliberately not folded into the library lane: stale examples must remain visible as source/API defects, not block the collection of useful library evidence or be silently excluded.

Known example issue:
- `examples/hdc_cooperation.rs` imports `symtropy_consciousness_physics::hdc_context`, which is not exported by the current crate root. This is tracked in issue #3. The example either needs its real HDC context implementation and tests, or an explicit retirement/archive decision. A placeholder module is not an acceptable fix.
- The curvature feature contract is checked separately against the exact feature and remains source-level work where its declared API and example usage disagree.

### Isolated feature and language probes

The branch runs isolated tests for:
- Cargo's native `unused_dependencies` lint using a deliberately unused local path dependency.
- Cargo JSON `compiler-artifact` events, capturing public artifact metadata instead of guessing private build directories.
- Never-type syntax and `Result<T, !>` under editions 2018, 2021 and 2024.
- A differential never-type fallback fixture comparing Rust 1.99.1 against 1.100.0.

The fallback differential expects:
- Rust 1.99.1: the legacy unit fallback still compiles the probe in editions 2018 and 2021; edition 2024 already uses `!` and rejects it.
- Rust 1.100.0: all three editions use `!` and reject the historical unit-fallback assumption.

The negative control is intentional. A compile failure is a pass only when it is the expected fallback-related type error; unrelated failures fail the test.

### Cargo minimum publish-age controls

The local-registry test creates its own sparse registry on loopback and adds exact `pubtime` values. It runs with stable Cargo 1.100.0 and verifies three independent properties:

1. **Old-version fallback:** if a semver-compatible 30-day-old release and a one-hour-old release exist, a 14-day policy chooses the older release.
2. **Fresh-only rejection:** if the only compatible release is one hour old, the resolver rejects it under `deny`.
3. **Lockfile exception:** an explicitly allowed fresh version can be recorded in Cargo.lock; a later `deny --locked` resolution preserves that already-pinned version.

The test is hermetic at the dependency-registry boundary: the local registry is served only on 127.0.0.1 and uses explicit `pubtime`. This tests Cargo resolver semantics, not the authenticity or completeness of the public crates.io index.

Cargo cooldowns are a risk-reduction measure, not a complete supply-chain defence:
- registries must publish `pubtime` for the version to be filtered;
- path and git dependencies are not covered by registry publish age;
- `cargo install` does not apply this control;
- a lockfile pin can preserve a version that is now younger than the configured cooldown.

We still require checksum verification, dependency review, vulnerability scanning and explicit review of lockfile updates.

## Benchmark programme

The branch now has an opt-in paired benchmark workflow:
`.github/workflows/rust-1-99-1-vs-1-100-0-performance.yml`.

It compares exact Rust/Cargo 1.99.1 and 1.100.0 on the same Git commit and unchanged Cargo.lock. It runs five paired fresh-target trials over the fixed deterministic simulation examples, captures raw logs and measurements, runs a dependency-free Rust analysis program, and uploads the evidence bundle.

Metrics:
- build wall time;
- peak RSS where GNU time is available;
- target-directory size and final executable size;
- release simulation runtime per repeated execution;
- stdout digest stability;
- exact compiler and Cargo metadata;
- source commit and Cargo.lock SHA-256;
- kernel, CPU model/count, memory, filesystem/mount and CPU governor.

The measured profile is fresh target directories with a warmed Cargo registry and warm host page cache. It is not a cold disk or cold machine benchmark. sccache is disabled in the primary comparison and must be measured in a separate profile.

The analyzer:
- pairs measurements by exact example and trial;
- checks duplicate build records and complete runtime repetition indices;
- rejects unstable per-trial output digests or inconsistent build metadata;
- reports medians, median absolute deviations and paired speedup estimates;
- computes a deterministic bootstrap interval for median paired runtime speedup;
- reports RSS and artifact-size comparisons;
- withholds p99 until both toolchains have at least 10,000 runtime observations for the relevant workload.

Five trials are exploratory. Do not infer p99 from five runs. Use 30+ independent paired runs for ordinary latency distribution claims and at least 10,000 observations per condition for p99 unless a justified statistical design supports another approach.

### Fair SOTA and industry comparison

Do not compare our three application examples directly to an unrelated compiler benchmark and claim to beat the Rust ecosystem.

Use three comparison layers:
1. **Compiler-wide:** upstream `rustc-perf` fresh and incremental workloads.
2. **Build graph:** Cargo `--timings` to distinguish compiler time from dependency graph, proc-macro, build-script, linker and cache effects.
3. **Application:** Symtropy's deterministic simulations plus direct Criterion microbenchmarks where available.

Keep compilation performance separate from application runtime. A compiler can build faster without producing faster code; runtime can improve while binaries grow or peak memory increases. Report a Pareto profile instead of hiding these trade-offs inside one weighted score.

### Profile-guided optimization research

Rust 1.100's stable `-Cprofile-sample-use` makes sample-profile evaluation a useful new research track. Do not assume PGO is free or always beneficial.

A fair PGO experiment should:
- freeze the workload corpus before collecting profiles;
- generate the profile only from a declared training set of workloads/seeds;
- evaluate on held-out workloads/seeds not used to build the profile;
- compare non-PGO, instrumentation-PGO and sample-PGO variants where the profile format/tooling is available;
- bind the profile digest to the exact compiler, flags, target and source revision;
- measure startup, throughput, tail latency, binary size and peak RSS;
- validate numerical outputs and scientific conclusions against the non-PGO baseline.

Reject a profile that improves only the training corpus while harming held-out scenarios. No PGO win is claimed until this experiment is executed.

References:
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- Cargo build timings: https://doc.rust-lang.org/cargo/reference/timings.html
- Rust PGO guide: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- Criterion.rs: https://bheisler.github.io/criterion.rs/book/

## Cargo build-directory compatibility

Cargo's intermediate build layout is internal and may change. The new branch captures artifact records from Cargo's `compiler-artifact` JSON messages. Scripts should not infer crate artifact locations by scanning `target/debug/deps`, `target/debug/build` or the new internal build directory.

Final artifact paths returned by Cargo are valid evidence; private intermediate cache paths are not stable interfaces.

## Promotion rule

The baseline PR's default pin remains Rust 1.99.1. This PR does not change the default toolchain, package MSRV, edition, Cargo.toml, Cargo.lock, scientific source, seeds or experiment settings.

Promote only after:
1. exact stable 1.100 library qualification passes;
2. all-target findings are resolved or accurately scoped;
3. semantic and numerical differences are explained;
4. Cargo cooldown negative controls pass on stable 1.100;
5. matched benchmarks show the actual performance/resource trade-offs;
6. a separate reviewed change proposes updating the default pin.

No performance superiority or repository-wide compatibility claim is made by the presence of this workflow.


## Exact-hosted evidence update — 2026-11-13

The exact Rust 1.100.0 stable distribution is now installed and identified by hosted runners as:

- rustc `1.100.0 (0ef1bc386 2026-11-09)`
- Cargo `1.100.0 (3acc79c0c 2026-11-05)`

Run [#43426972144](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/actions/runs/43426972144) on the exact PR merge tree reported:

### Passed

- Rust 1.100.0 supported-library qualification: PASS.
- Library unit tests: **104 passed, 0 failed**.
- Locked graph under the 14-day minimum publish-age policy: PASS.
- Clippy, rustdoc, metadata and lockfile immutability: PASS.
- Raw x86_64 FFI bool ABI: `ffi_bool_abi=PASS`, `ffi_bool_raw_eax=PASS`.
- Cargo-native `unused_dependencies` isolated fixture: PASS.
- Cargo JSON compiler-artifact manifest: PASS.
- Dependency-free benchmark analyzer compile/self-test: PASS.
- Rust 1.100 language/edition differential and ABI suite: PASS.
- Stable Cargo publish-age controls: all three PASS (old-version fallback, fresh-only rejection, existing-lock exception).

The local registry emitted the expected diagnostics: it selected the 30-day-old version when a one-hour-old version was also available, rejected the one-hour-old version when it was the only candidate under deny, and preserved that fresh version when it was already in Cargo.lock and Cargo ran with `--locked`. The server was loopback-only.

### Supported-target failure and subsequent source repairs

The earlier `hdc_cooperation` source/API defect has now been handled as an explicit archive, not hidden behind a placeholder:

- Original source preserved at `research/archived-examples/hdc_cooperation.rs`.
- Original source blob SHA-1: `2464da8339ee1d8cf77e9806fce91f4832033493`.
- Unbuildable auto-discovered path `examples/hdc_cooperation.rs` removed.
- Archive/restoration contract documented at `docs/research/ARCHIVED_HDC_COOPERATION_EXPERIMENT.md`.
- The archive-disposition assertions passed on Rust 1.99.1 and 1.100.0.

The next `--all-targets` run progressed beyond HDC and then found a distinct feature-metadata defect: `examples/curvature_lensing.rs` consumes the feature-gated `curvature` module and `HarmonyField::sigma_gradient`, but Cargo had no `required-features` declaration for this auto-discovered example. The source's own run instructions already require `--features consciousness-curvature`; the method and module are correctly cfg-gated. The root manifest now explicitly declares:

```toml
[[example]]
name = "curvature_lensing"
path = "examples/curvature_lensing.rs"
required-features = ["consciousness-curvature"]
```

The curvature feature workflow now checks both `curvature_selforg` and `curvature_lensing` on 1.99.1, 1.100.0 and 1.101.0-beta.1. These latest checks and the next full supported-target build are pending; no all-target pass is claimed yet.

The HDC issue #3 is therefore resolved at the source-disposition level, while all-target qualification remains blocked pending the separate curvature metadata fix's exact-head run. This is intentionally tracked as two separate source/API decisions rather than a compiler regression.

### Current performance evidence boundary

The paired 1.99.1 versus 1.100.0 application benchmark remains separate from compatibility. It uses five paired fresh-target trials, a warmed registry, offline locked builds, no sccache wrapper, rotated compiler order, and records time/RSS/target size/binary size/runtime/output hashes. Its current hosted run is still in progress. Even when it completes, five runs are exploratory: no p99 claim, and no general compiler-wide SOTA claim. Upstream rustc-perf remains the compiler-wide reference; Symtropy's deterministic simulations are the application reference.

### Promotion

The repository default remains Rust 1.99.1 until the entire supported target surface and any candidate-only differences are resolved, with the matched performance and scientific-result evidence reviewed. Passing the 1.100.0 library and isolated feature suites is not equivalent to qualifying every example, target, Holochain guest, or runtime.
