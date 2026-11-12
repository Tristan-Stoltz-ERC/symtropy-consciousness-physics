# Paired Rust toolchain benchmark v0.2

This harness compares exact Rust compiler/Cargo toolchains on the same clean source revision and unchanged Cargo.lock. The default comparison remains Rust 1.96.1 vs 1.99.1. To evaluate the 1.100.0 beta while it is the current beta release, run:

```sh
TOOLCHAINS="1.99.1 beta" BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash benchmarks/rust-toolchain/run-paired.sh
```

Install both toolchains first. The candidate lane must not silently compare a later beta:

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup toolchain install beta --profile minimal
rustup run beta rustc --version
```

The canary workflow separately asserts that beta resolves to Rust 1.100.0 and skips explicitly if the beta channel moves on.

## Controls and evidence

The harness:
- requires a clean worktree and records the exact Git commit;
- records the Cargo.lock SHA-256 before and after the run;
- records complete rustc -Vv and cargo -Vv for each toolchain;
- warms the Cargo registry before measurements;
- runs measured Cargo work offline and with --locked;
- uses a fresh target directory for each toolchain/trial/example;
- disables incremental compilation for the primary compiler comparison;
- disables RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER so sccache is measured separately;
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

## Interpretation

Compare paired trial distributions, not one best run. Report median, interquartile range and confidence intervals. Keep compile time and runtime separate. A compiler that produces a faster build is not necessarily producing a faster simulation.

Compare stdout digests across repeated runs within each toolchain. Cross-toolchain digest changes require investigation: distinguish harmless floating-point ordering/formatting changes from changed experiment conclusions. Preserve raw stdout/stderr and never silently normalize results.

Five paired runs are exploratory, not a robust tail-latency campaign. Do not report p99 from five repetitions. p99 claims require a much larger observation set or a justified statistical design. No performance target is enforced until a stable workstation baseline exists.

This benchmark is opt-in and hardware-dependent. The CI qualification workflow does not claim to have executed a workstation performance campaign.
