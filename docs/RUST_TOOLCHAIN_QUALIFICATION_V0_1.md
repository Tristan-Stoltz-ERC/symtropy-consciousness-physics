# Rust toolchain qualification v0.1

**Status:** candidate policy and CI checks; performance and host qualification remain pending.

## Selected toolchain

The repository pins Rust 1.99.1 in `rust-toolchain.toml`, with rustfmt and Clippy. Cargo is taken from the same Rust distribution rather than installed independently.

Rust 1.99.1 is the latest stable release as of 2026-11-12. Rust 1.100 is beta and is not the production toolchain.

## Why this release

The 1.99.1 patch release fixes:
- a missing `wcslen` symbol affecting UEFI and embedded targets;
- an x86_64 FFI `bool` return-value miscompilation where bits 1–7 of the return register could be unspecified;
- an `rust-lld` macOS library-loading issue.

The x86_64 C ABI probe in `tests/toolchain` exercises Rust bool returns from a C caller. It is a focused regression smoke test, not a substitute for a complete ABI test suite.

Cargo 1.99 also includes manifest and registry-index parsing performance improvements. It disables incremental compilation by default in CI; this repository makes that policy explicit with `CARGO_INCREMENTAL=0` in the qualification workflow.

## Compatibility policy

Do not confuse three different versions:

1. **Pinned development toolchain:** the exact Rust/Cargo version developers and the stable CI lane use (1.99.1).
2. **MSRV:** the minimum compiler version the project promises to support. It is not declared yet and must not be inferred from the development pin.
3. **Canary toolchain:** beta/nightly used in a separate, non-blocking research lane.

The upgrade does not change Cargo.lock, dependency versions, package edition or declared MSRV. A future MSRV should be chosen only after dependency metadata and source compatibility have been checked.

## Required qualification

The pinned stable lane runs:
- rustc/Cargo/rustfmt/Clippy version checks;
- formatting and locked dependency metadata checks;
- all-target compilation and tests;
- Clippy and rustdoc;
- the x86_64 C ABI bool smoke test.

A green CI result proves these checks passed on the exact workflow run and runner image. It does not prove the workstation host was upgraded or that release-mode physics performance improved.

## Performance and correctness benchmark programme

The paired harness compares 1.96.1 with 1.99.1 using the same source revision, Cargo.lock, build flags and workload definitions. It isolates target directories and explicitly controls Cargo incremental behavior.

Measure separately:
- clean-target debug and release build time;
- `cargo check`, test, Clippy and rustdoc duration;
- peak RSS, target-directory size and binary size;
- release runtime of deterministic simulation examples;
- output digests and numerical result deltas;
- repeated-build reproducibility;
- FFI/unsafe and cross-target compatibility.

The current harness starts with fresh-target release builds and three deterministic examples. Memory, binary size, sanitizer/Miri and broader cross-target qualification are follow-up lanes.

## Industry/SOTA comparison

A meaningful compiler comparison is a paired, controlled experiment—not a claim based on release notes or a single local build. Use at least five independent trials for exploratory decisions, rotate toolchain order, preserve raw logs and report medians/IQR/confidence intervals. Use substantially more observations for tail-latency claims.

Do not compare a warmed incremental build on one compiler with a clean build on another. Do not compare different dependency graphs or quietly update Cargo.lock. Do not attribute runtime changes to Rust without matched inputs and exact artifact metadata.

## Next steps

1. Run CI on this exact PR head and repair any compatibility failures without dependency churn.
2. Execute paired benchmarks on the target workstation and one independent builder.
3. Determine MSRV from source and dependency evidence.
4. Add a separate beta canary after stable qualification, without making beta a required release gate.
5. Extend qualification to WASM/Holochain targets only where a real product target requires it.
