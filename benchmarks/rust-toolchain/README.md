# Paired Rust toolchain benchmark v0.2

This harness compares exact Rust compiler/Cargo toolchains on the same clean source revision and unchanged Cargo.lock. The primary comparison is Rust 1.99.1 versus Rust 1.100.0 stable.

To compare them:

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup toolchain install 1.100.0 --profile minimal
BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash scripts/benchmark-rust-1.99.1-vs-1.100.0.sh
```

The wrapper refuses to run unless both `rustc` and Cargo identify as exactly 1.100.0 for the candidate.

## Controls and evidence

The harness:
- requires a clean worktree and records the exact Git commit;
- records the Cargo.lock SHA-256 before and after the run;
- records complete `rustc -Vv` and `cargo -Vv` for each toolchain;
- warms the Cargo registry before measurements;
- runs measured Cargo work offline and with `--locked`;
- uses a fresh target directory for each toolchain/trial/example;
- disables incremental compilation for the primary compiler comparison;
- disables `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` so sccache is measured separately;
- rotates toolchain order between trials;
- records compile time, peak RSS where GNU time is available, target-directory size, binary size, release-example runtime and stdout SHA-256;
- records kernel, CPU, memory, filesystem/mount and CPU governor metadata.

The page cache is not dropped. Results are therefore **fresh-target / warmed-registry** measurements, not cold-machine or cold-disk measurements. Missing GNU time on a NixOS host leaves RSS as NA; it does not invalidate the other measurements.

## Workloads

The initial corpus uses the repository's documented deterministic examples:
- `gradient_replacement`
- `cooperation_decomposition`
- `jphi_convergence`

These exercise materially different parts of the thermodynamic multi-agent simulation. Add workloads only when their input seeds, environment and expected outputs are explicit.

## Industry and SOTA comparison

This project-specific corpus measures behavior that matters to Symtropy. It does not replace the Rust team's compiler benchmark suite.

For compiler-level comparison, use the official `rustc-perf` workloads where practical. The Rust Compiler Development Guide describes fresh and incremental compilation comparisons across a collection of real crates, associated with exact compiler commit hashes. Use those results as the compiler ecosystem baseline, then compare the Symtropy simulation workloads as the application baseline.

Use Cargo's `--timings` reports to identify critical-path crates, build-script bottlenecks and parallelism rather than attributing total build time to the compiler alone. Measure Cargo test orchestration separately from test binary runtime. A cargo-nextest comparison is a test-runner experiment, not a compiler-performance result.

Measure sccache hit rates as a separate profile. Do not mix cache hits into the primary no-cache compiler comparison; sccache has distinct caching constraints, including incremental compilation and some linker/proc-macro cases.

## Interpretation

Compare paired trial distributions, not one best run. Report median, interquartile range and confidence intervals. Keep compile time and runtime separate. A compiler that produces a faster build is not necessarily producing a faster simulation.

Compare stdout digests across repeated runs within each toolchain. Cross-toolchain digest changes require investigation: distinguish harmless floating-point ordering/formatting changes from changed experiment conclusions. Preserve raw stdout/stderr and never silently normalize results.

Five paired runs are exploratory, not a robust tail-latency campaign. Do not report p99 from five repetitions. p99 claims require a much larger observation set or a justified statistical design. No performance target is enforced until a stable workstation baseline exists.

This benchmark is opt-in and hardware-dependent. The CI qualification workflow does not claim to have executed a workstation performance campaign.

## References

- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- rustc-perf profiling: https://rustc-dev-guide.rust-lang.org/profiling/with-rustc-perf.html
- Cargo build timings: https://doc.rust-lang.org/stable/cargo/reference/timings.html
- Cargo build performance guide: https://doc.rust-lang.org/cargo/guide/build-performance.html
- sccache Rust caching constraints: https://github.com/mozilla/sccache/blob/main/docs/Rust.md
- cargo-nextest benchmark considerations: https://nexte.st/docs/benchmarks/
