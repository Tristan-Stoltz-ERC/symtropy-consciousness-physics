# Paired Rust toolchain benchmark v0.6

Compares exact Rust/Cargo 1.99.1 and 1.100.0 on the same clean source revision and unchanged Cargo.lock. Rust 1.100.0 was officially released on 2026-11-12; the new qualification workflow now targets the exact stable distribution, not a scheduled release date or mutable beta alias.

Run locally after installing both exact toolchains:

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup toolchain install 1.100.0 --profile minimal
BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash scripts/benchmark-rust-1.99.1-vs-1.100.0.sh
```

The exact wrapper refuses to run unless Rust/Cargo 1.100.0 are installed and identified exactly.

## Automated paired benchmark

The opt-in workflow is `.github/workflows/rust-1-99-1-vs-1-100-0-performance.yml`. It is triggered by changes to the benchmark harness on a pull request or manual dispatch. It:
- installs exact Rust/Cargo 1.99.1 and 1.100.0;
- performs five paired build trials for each fixed simulation example;
- uses fresh target directories, offline/locked compilation and a warmed Cargo registry;
- rotates toolchain order between trials;
- measures compile time, peak RSS when available, target bytes, executable bytes and runtime;
- retains stdout/stderr and compiler output digests;
- runs the dependency-free Rust analysis tool;
- uploads metadata, logs, raw CSV and analysis as a 30-day artifact.

## Controls and evidence

- Clean worktree and exact Git commit.
- Cargo.lock SHA-256 before and after.
- Full `rustc -Vv` and `cargo -Vv` for both compiler versions.
- Cargo registry warmed before measurements; measured commands run offline and with `--locked`.
- Fresh target directory per toolchain/trial/example.
- Incremental compilation disabled for the primary compiler comparison.
- `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` unset; sccache measured separately.
- Rotating toolchain order across trials.
- Kernel, CPU, memory, filesystem/mount and CPU governor recorded.

The page cache is not dropped. Results are **fresh-target / warmed-registry**, not cold-machine or cold-disk. Missing GNU time leaves RSS as NA; it is not fabricated.

## Reproducible analysis

The raw CSV is analyzed by a dependency-free Rust program:

```sh
RUSTC_TOOLCHAIN=1.100.0 bash scripts/analyze-rust-toolchain-benchmark.sh \
  benchmark-results/rust-toolchain/run-<id>/results/measurements.csv \
  1.99.1 1.100.0
```

The analyzer pairs baseline and candidate by exact trial/example key, de-duplicates build-level metrics, verifies contiguous runtime repeats, rejects inconsistent build metadata or digest instability, and calculates medians, MAD and paired speedup estimates with a deterministic bootstrap interval. p99 is withheld until both versions have at least 10,000 observations for that workload.

## Workloads and comparison

The initial corpus uses:
- `gradient_replacement`
- `cooperation_decomposition`
- `jphi_convergence`

These represent distinct parts of the thermodynamic multi-agent simulation. They are application-specific workloads, not substitutes for compiler-wide benchmark corpora.

Use upstream `rustc-perf` for compiler-wide comparison and Cargo `--timings` to distinguish compiler work from build graph, build scripts, proc macros and linking. Keep compiler build time and application runtime separate. Measure sccache independently from the no-cache build profile.

## Next frontier: PGO

Rust 1.100 stabilizes `-Cprofile-sample-use`. A separate research branch now contains two opt-in PGO pilots and an evidence passport:

- `scripts/run-rust-pgo-instrumented.sh`: exact-source `profile-generate` → `llvm-profdata` → `profile-use`, with no-PGO versus PGO paired runtime measurements.
- `scripts/run-rust-sample-pgo.sh`: Linux `perf` branch-stack collection → `llvm-profgen` → `profile-sample-use`; this requires a PMU-capable host and an LLVM-matched `llvm-profgen`.
- `docs/research/RUST_110_PGO_EVIDENCE_PASSPORT_V0_1.md`: profile provenance, cross-example transfer probes, correctness gates and proposed acceptance thresholds.

The first training workload is `jphi_convergence`; `gradient_replacement` and `cooperation_decomposition` are explicitly treated as cross-example transfer probes, not an IID hold-out set. No PGO speedup or scientific result is claimed until the physical workload tournament runs.

## Statistical interpretation

Five paired trials are exploratory only. Use 30+ independent paired runs for ordinary latency-distribution claims and 10,000+ observations per condition for p99. Keep failed/skipped runs visible. Do not infer a compiler improvement from one best run or from a faster compilation alone.

## References

- Rust 1.100.0 release announcement: https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- Rust PGO guide: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- Cargo build timings: https://doc.rust-lang.org/cargo/reference/timings.html
- Criterion.rs: https://bheisler.github.io/criterion.rs/book/
