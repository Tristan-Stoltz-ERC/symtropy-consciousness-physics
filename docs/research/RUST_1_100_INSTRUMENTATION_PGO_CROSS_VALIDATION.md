# Symtropy instrumentation PGO cross-validation v0.1

**Purpose:** determine whether Rust 1.100.0 instrumentation-based profile-guided optimization (PGO) improves release runtime for the deterministic Symtropy workloads without changing experiment outputs.

This is a research benchmark only. It does not change Cargo profiles, source code, dependencies, default toolchain pin or scientific inputs.

## Paradigm shift: optimize for held-out physical behavior, not the profile's training traces

A common PGO failure mode is to profile the same workload on which the final binary is judged. That measures how well the binary fits its profile, not how well it generalizes.

The pilot uses three-fold leave-one-example-out validation across:
- `gradient_replacement`;
- `cooperation_decomposition`;
- `jphi_convergence`.

For each fold:
1. Two workloads generate the instrumentation profile.
2. The third is never executed while collecting that fold's profile.
3. The candidate binary is rebuilt with `-Cprofile-use`.
4. The held-out workload is measured.
5. Its stdout must match the non-PGO binary's exact digest across all repeats.

The held-out role rotates through all three workloads. A profile therefore cannot claim success based only on its training workloads.

## Controls

- Exact Rust/Cargo 1.100.0 stable.
- Same Git commit and unchanged Cargo.lock for baseline, instrumented and PGO builds.
- Fresh, isolated Cargo target directories for baseline and every fold.
- `--locked`, network fetch before measured builds, offline compilation after fetch.
- `CARGO_INCREMENTAL=0`; sccache wrappers unset.
- Same release profile, target triple and source for baseline and PGO builds.
- Profile digest and exact compiler metadata retained.
- Repeated runtime measurements; raw stdout/stderr retained.
- Exact output digest match required; a mismatch fails the run.
- Lockfile SHA-256 before/after must match.

## Metrics

For baseline and each PGO fold:
- build wall time and peak RSS;
- release artifact size;
- held-out runtime per repeat;
- repeated output digest;
- compiler metadata, Cargo lock identity and host metadata.

The first pass defaults to 10 repeats per workload and is exploratory. It must not produce p99 claims. Extend to 30+ paired independent runs for ordinary latency claims and 10,000+ observations for p99. A performance result must be reported with medians, dispersion and confidence intervals, not a best-run comparison.

## PGO variants and future research

The first executable pilot covers instrumentation PGO using `-Cprofile-generate` / `-Cprofile-use`. Rust 1.100.0 also stabilizes sample-profile use. Sample-based PGO is deliberately separate: hosted runners may lack stable access to hardware sampling counters, so a valid sample-profile run requires an explicit host/counter availability check and must not silently fall back to instrumentation PGO.

A later sample-profile study should:
- pin the exact perf toolchain and profile format;
- record sampling frequency, permissions, kernel and PMU availability;
- bind profile digest to exact compiler, target, flags and source;
- use the same three-fold held-out design;
- report unsupported counters as NOT RUN, not as a PGO result.

## Interpretation

- Faster build is not faster runtime.
- Faster training workloads do not establish generalization.
- Changed numerical output is a blocker until explained, even if the binary is faster.
- Exact stdout identity is necessary but not sufficient: follow-up scientific analysis must compare numeric invariants and the reported experiment conclusions.
- The result is a Symtropy-specific PGO experiment, not evidence that Rust PGO beats general compiler SOTA.

## References

- Rust 1.100.0 release: https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/
- Rust profile-guided optimization: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- rustc performance suite: https://rustc-dev-guide.rust-lang.org/tests/perf.html
