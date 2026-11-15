# Deterministic reduction runtime and memory qualification v0.1

## Purpose

PR #39 establishes an opt-in deterministic reduction path and demonstrates byte-identical output across independent processes on Rust 1.99.1, 1.100.0 and 1.101.0 beta. That is correctness evidence for the tested workload, not a performance result.

This benchmark measures the cost of stable-handle sorting and compensated summation in the same Cooperation Decomposition executable.

## Paired design

- Exact Rust 1.100.0 stable compiler and Cargo.
- Identical source commit, Cargo.lock, release profile and seed partition.
- Historical default binary versus the deterministic-reductions feature binary.
- Separate fresh Cargo target directories for each build.
- Registry dependencies warmed before offline measured builds.
- Two warmup process executions per mode, excluded from results.
- Thirty paired runtime trials by default.
- Alternate run order by trial to reduce systematic thermal/order bias.
- Same historical seed partition, default [0,20), unless explicitly overridden.
- Byte-exact stdout/stderr SHA-256 per process; no normalization.
- Wall-clock runtime in milliseconds and maximum RSS in KiB for each process.

The analyzer reports paired median speedup, median overhead, deterministic-bootstrap 95% confidence intervals, peak-RSS ratio/delta and digest cardinality for each mode. Compilation time and binary size are recorded separately in metadata and are not mixed into runtime overhead.

## Proposed target

The research target is less than 3% median runtime overhead with no meaningful increase in peak memory. This is a target, not a measured guarantee or current promotion gate. A correct deterministic path should not be disabled merely because its overhead exceeds the target; the next engineering step should optimize the implementation while preserving the determinism receipt.

## Acceptance and interpretation

- The deterministic mode must produce one stdout and one stderr digest over all paired runs for the fixed seed partition.
- Historical-mode digest variation is diagnostic and does not make the benchmark fail.
- A thirty-pair result is useful for exploratory overhead estimation; confirmatory claims should use more independent runs, disclose host noise, and replicate on at least two hardware classes.
- The benchmark records CPU, RAM, kernel, governor and exact compiler/Cargo identity.
- Do not call the deterministic mode faster unless the paired speedup interval supports it.
- Do not claim cross-architecture determinism from this single-host experiment.

## Future optimization hypotheses

1. Replace repeated per-callback sorting with a stable index only if public mutation of the HashMaps can be safely constrained or detected.
2. Cache sorted BodyHandle keys behind a generation counter and invalidate on every structural mutation; reject stale cache use.
3. Consider ordered maps for private state only if API compatibility permits.
4. Use pairwise or reproducible accumulators for parallel reductions, not nondeterministic atomic floating-point accumulation.
5. Benchmark separate cost centres (collective Phi, sanctuary impulse, process-level experiment) so an optimization does not merely move cost between phases.

The current implementation deliberately prioritizes a verifiable stable order over clever caching. Any cache must fail closed when its source map changes.
