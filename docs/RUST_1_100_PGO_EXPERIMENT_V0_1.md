# Rust 1.100 sample-based PGO experiment v0.1

**Status: protocol only; no profile or PGO speedup is claimed.** This is an opt-in research experiment for the exact Rust/Cargo 1.100.0 stable toolchain. It does not change the default build flags, Cargo manifests, lockfile, scientific source, inputs or reported findings.

## Why this experiment exists

Rust 1.100 stabilizes `-Cprofile-sample-use`. Sample-based profile-guided optimization (PGO) can use observed hot paths to inform inlining, code layout and register allocation. The sample profile is produced externally, then consumed by rustc.

This is distinct from instrumentation PGO:
- instrumentation mode: `-Cprofile-generate` -> `.profraw` -> `llvm-profdata merge` -> `-Cprofile-use`;
- sample mode: DWARF line tables -> external sampling (Linux `perf`) -> `llvm-profgen` -> `-Cprofile-sample-use`.

These profile types are not interchangeable. The experiment must retain the mode and profile format in its receipt. This first contract investigates sample mode specifically because that capability is newly stable in Rust 1.100.

## Frozen experiment corpus

The corpus uses three existing, deterministic examples, each with a source blob recorded in `RUST_1_100_SAMPLE_PGO_CORPUS_V0_1.json`.

### Training workload

`gradient_replacement`:
- 20 agents;
- 6 movement controllers;
- 8,000 ticks per run;
- 20 deterministic seeds: `42 + 997*s`, `s = 0..19`;
- the profile-training workload only.

### Held-out workloads

`cooperation_decomposition`:
- 20 agents;
- 6 controller × resonance conditions;
- 8,000 ticks per run;
- 20 deterministic seeds: `42 + 997*s`, `s = 0..19`.

`jphi_convergence`:
- 12 agents;
- 3 conditions;
- up to 10,000 ticks per run;
- 10 deterministic seeds: `42 + 997*s`, `s = 0..9`;
- includes its existing second statistical comparison pass.

Neither held-out executable may be run while collecting the sample profile. The experiment therefore tests profile generalization from one simulation program to two different simulation programs and code paths. It is a **cross-workload holdout**, not a claim of independent seed populations: the programs use the same deterministic seed formula.

No source file or simulation input is changed to produce this split.

## Protocol

1. Freeze exact source commit and record the three example Git blob IDs from the corpus file.
2. Install exact Rust/Cargo 1.100.0; record full `rustc -Vv`, `cargo -Vv`, host triple, LLVM version, kernel, CPU and profiling permissions.
3. Build the training example in release mode with line-table debug information and the exact target triple. Do not add `-Cprofile-generate` in sample mode.
4. Run the training binary under Linux `perf` with branch-stack sampling:
   `perf record -b -e branches:u`.
5. Convert the collected `perf.data` to an LLVM sample profile with `llvm-profgen --binary=<training-binary> --output=<profile> --perfdata=<perf.data>`.
6. Hash and preserve the raw `perf.data`, profile output, executable, source commit, binary build flags and profile receipt.
7. Build the two held-out examples with `-Cprofile-sample-use=<absolute-profile-path>` and `-Cllvm-args=-pgo-warn-missing-function`. Keep the source, target, release flags, features and dependency lock unchanged.
8. Build the same two held-out examples again without PGO under the same toolchain and environment.
9. Run baseline and PGO executables on the exact same deterministic inputs. Preserve stdout/stderr, hashes, runtime samples, peak RSS and binary sizes.
10. Analyze the two held-out workloads separately. Do not tune the profile after inspecting held-out results and then call those results validation.

Use absolute profile paths: Cargo can invoke rustc from different working directories. Pass an explicit target triple so profile flags do not leak into build scripts. The PGO experiment is release-only.

## Environment and preflight

Sample PGO is only evaluable when:
- host is `x86_64-unknown-linux-gnu` for this initial corpus;
- `perf` is installed and can record branch stacks on the runner;
- the CPU/kernel expose the selected branch event;
- `llvm-profgen` is available from the exact Rust 1.100.0 `llvm-tools-preview` component or a compatible LLVM installation;
- the resulting sample profile is non-empty and consumed by rustc.

A denied `perf_event` permission, unavailable branch-stack hardware or missing profiler is `NOT_EVALUABLE`, not a failed PGO compiler test. Do not silently switch to instrumentation mode. If instrumentation PGO is tested later, it must have its own mode and profile receipt.

## Correctness and rejection gates

- Baseline and PGO binaries must run the same exact input corpus.
- Every output remains archived. Compare stdout digests and parse numeric result rows into a domain-aware comparison.
- Exact digest equality is preferred for these deterministic examples. Any difference blocks promotion until explained at the field/metric level; do not normalize away a difference.
- PGO must not change experiment conclusion labels, condition ordering, seed coverage, convergence classification or any statistical result.
- All existing unit/integration/supported-target tests remain required.
- The PGO profile must bind exact source, compiler, Cargo, target, flags, training corpus, training binary, perf input and profile digest.
- A profile generated from another compiler, source revision, target triple or set of compiler flags is rejected.

## Metrics and aspirational targets

Measure separately per held-out workload:
- median runtime and MAD over paired runs;
- throughput / simulated ticks per second;
- p50/p95 latency for repeated complete runs;
- peak RSS and binary size;
- compile time and profile generation overhead;
- exact output digest and domain metric deltas.

Initial exploratory target: at least 5% median held-out runtime improvement on both held-out workloads, with no correctness change and no more than 5% binary-size increase. This is an aspiration, not a release gate until a baseline distribution and variance are established. No p99 claim from a small number of complete experiment executions; 10,000 observations per condition or a justified alternative are required for p99.

If training performance improves but either held-out workload regresses, label it **overfit / no promotion**.

## Relationship to SOTA

The fair comparison has three levels:
1. upstream `rustc-perf` for general compiler performance;
2. Cargo `--timings` for dependency/build critical-path analysis;
3. these frozen Symtropy workloads for application-specific PGO generalization.

We do not claim to outperform upstream rustc-perf using unrelated examples. PGO success means a measured gain on both held-out Symtropy workloads without semantic drift, not an industry-wide compiler result.

## Safety and scope

- Research-only; no PGO flags are added to the default Cargo config or release profile.
- No dependency or lockfile changes.
- No scientific input/source changes.
- No workstation-wide configuration or NixOS changes.
- No Crater execution: Crater runs untrusted third-party crate code and requires sandboxed infrastructure. This project-specific PGO run only executes the three reviewed repository examples.

## References

- Rust 1.100 release announcement: https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/
- rustc PGO modes and sampling workflow: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- Rust compiler performance suite: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- Cargo timing reports: https://doc.rust-lang.org/cargo/reference/timings.html
- Crater safety and experiment model: https://github.com/rust-lang/crater
