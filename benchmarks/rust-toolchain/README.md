# Paired Rust toolchain benchmark v0.1

This track compares Rust 1.96.1 (the workstation's known compatibility baseline) with Rust 1.99.1 on the same Symtropy source and unchanged Cargo.lock.

## Run

Install both exact toolchains first:

```sh
rustup toolchain install 1.96.1 --profile minimal
rustup toolchain install 1.99.1 --profile minimal
```

Then run:

```sh
BUILD_REPEATS=5 RUNTIME_REPEATS=5 \
  bash benchmarks/rust-toolchain/run-paired.sh
```

The harness:
- warms the Cargo registry cache before timing;
- runs measured Cargo work offline and with `--locked`;
- uses a fresh target directory for each toolchain/trial/example;
- disables incremental compilation for every measured build;
- rotates toolchain order between trials;
- records compile time, release-example runtime and stdout SHA-256;
- records source revision, dirty-tree state, kernel, CPU, memory, filesystem and CPU governor.

The page cache is not dropped. The results are therefore **fresh-target / warmed-registry** measurements, not cold-machine or cold-disk measurements. Background workload and thermal state should be controlled and recorded.

## Workloads

The initial corpus uses the repository's documented deterministic examples:

- `gradient_replacement`
- `cooperation_decomposition`
- `jphi_convergence`

These exercise materially different parts of the thermodynamic multi-agent simulation. Add workloads only when their input seeds, environment and expected outputs are explicit.

## Interpretation

Compare paired trial distributions, not one best run. Report median, interquartile range and confidence intervals. Keep compile time and runtime separate. A compiler that produces a faster build is not necessarily producing a faster simulation.

Compare stdout digests across repeated runs within each toolchain. Cross-toolchain digest changes require investigation: distinguish harmless floating-point ordering/formatting changes from changed experiment conclusions. Preserve raw stdout/stderr and never silently normalize results.

Do not report p99 from five repetitions. p99 claims require a much larger observation set or a justified statistical design. No performance target is enforced until a stable workstation baseline exists.
