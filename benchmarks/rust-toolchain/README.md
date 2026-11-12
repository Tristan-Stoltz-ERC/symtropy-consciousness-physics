# Paired Rust toolchain benchmark v0.4

Compares exact Rust/Cargo 1.99.1 and 1.100.0 on the same clean source revision and unchanged Cargo.lock. Rust 1.100.0 was officially published on 2026-11-12. The default project toolchain is not changed by this benchmark.

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup toolchain install 1.100.0 --profile minimal
BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash scripts/benchmark-rust-1.99.1-vs-1.100.0.sh
```

The wrapper refuses to run unless both `rustc` and Cargo identify as exactly 1.100.0 for the candidate. It does not follow moving `stable` or `beta` aliases during measurements.

## Controls and evidence

- Clean worktree; exact Git commit recorded.
- Cargo.lock SHA-256 before and after.
- Full `rustc -Vv` and `cargo -Vv` for both compiler versions.
- Cargo registry warmed before measurements; measured commands run offline and with `--locked`.
- Fresh target directory per toolchain/trial/example.
- Incremental compilation disabled for the primary compiler comparison.
- `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` unset; sccache measured separately.
- Rotating toolchain order across trials.
- Compile wall time, peak RSS where GNU time is available, target directory size, executable size, simulation runtime and stdout SHA-256.
- Kernel, CPU, memory, filesystem/mount and CPU governor recorded.

The page cache is not dropped. Results are fresh-target / warmed-registry, not cold-machine or cold-disk. If GNU time is missing, RSS is NA and is not fabricated.

## Workloads

- `gradient_replacement`
- `cooperation_decomposition`
- `jphi_convergence`

These existing examples represent different parts of the thermodynamic multi-agent simulation. Do not change experiment seeds, model parameters, or scientific source while comparing compilers.

## Compiler ecosystem comparison

Use upstream `rustc-perf` for compiler-wide fresh and incremental build workloads. Its hosted performance service compares compiler revisions across popular crates; the published tracking scope is currently x86_64 Linux. This project-specific corpus does not replace rustc-perf.

Use Cargo `--timings` to identify dependency graph, build-script and critical-path bottlenecks. Keep compilation, tests, docs and simulation runtime as distinct metrics. Measure sccache hit/miss as a separate profile.

## Interpretation

Compare paired distributions rather than best-of-run. Report median, IQR and confidence intervals. Preserve all raw stdout/stderr. Compare outputs within each toolchain for repeatability and across toolchains for numerical drift. A digest mismatch requires an explanation; do not silently normalize it.

Five runs are exploratory. No p99 claim is justified from five observations. Use 30+ independent paired runs for ordinary latency distribution claims and at least 10,000 observations per condition (or a justified design) for p99. No performance advantage is claimed before a completed matched campaign.

## References

- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- rustc-perf profiling: https://rustc-dev-guide.rust-lang.org/profiling/with-rustc-perf.html
- Cargo build timings: https://doc.rust-lang.org/stable/cargo/reference/timings.html
- Cargo build performance: https://doc.rust-lang.org/cargo/guide/build-performance.html
- sccache Rust caching constraints: https://github.com/mozilla/sccache/blob/main/docs/Rust.md
