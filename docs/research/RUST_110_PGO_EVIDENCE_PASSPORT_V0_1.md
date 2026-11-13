# Rust 1.100 profile-guided optimization: evidence passport v0.1

**Status:** research implementation. No PGO speedup has been measured or claimed.

## Why this is the next frontier

Rust 1.100.0 stabilizes sample-based profile-guided optimization through `-Cprofile-sample-use`. PGO feeds representative runtime behavior into LLVM's optimizer; it can affect inlining, code layout and register allocation. This makes the build a function of not only source, compiler and flags, but also of a workload-derived profile. That profile needs its own identity and qualification.

The paradigm shift is from **“build the source”** to **“build the source against a versioned, workload-qualified execution profile.”**

A profile should not be an anonymous `.profdata` or `.prof` file copied between machines. It is an evidence artifact bound to:
- exact source commit and tree;
- exact rustc and Cargo identities, including LLVM version;
- target triple, features, linker and codegen flags;
- training workload identity, parameters and seed schedule;
- profile-generation method and tool versions;
- raw profile digest and derived profile digest;
- collection environment and hardware-counter availability;
- validation workload identity and held-out status;
- output invariants and correctness results.

The profile's provenance is part of the artifact's trust boundary.

## Two distinct PGO lanes

Do not conflate instrumentation PGO and sample-based PGO. Their profiles are not interchangeable.

### Instrumentation PGO

The executable is built with `-Cprofile-generate`, run over the training workload, and the resulting `.profraw` data is merged by `llvm-profdata`. The application is rebuilt with `-Cprofile-use`.

Implemented pilot:
- `scripts/run-rust-pgo-instrumented.sh`
- exact Rust/Cargo 1.100.0 required by default;
- source commit, target, flags and Cargo.lock stay fixed;
- training example defaults to `jphi_convergence`;
- transfer probes default to `gradient_replacement` and `cooperation_decomposition`;
- separate build target directories for no-PGO, instrumented and PGO builds;
- build scripts do not receive the profiling flags because an explicit target is passed to Cargo;
- two training executions are merged;
- three or more matched runtime observations are retained by default;
- repeated stdout must be deterministic within each variant and exactly equal across no-PGO/PGO before the script returns a mechanical pass;
- raw binaries, profile data, logs and measurement CSV are retained outside the repository.

The current examples are **different workloads**, not an IID split of one task. The first experiment is therefore an explicit cross-example profile-transfer probe. It cannot establish generalization to every workload. A future experiment should refactor the simulation runner into a parameterized common executable with disjoint training/holdout seed manifests before making stronger generalization claims.

### Sample-based PGO

The application is built with DWARF line tables, run under Linux `perf` with branch-stack sampling, converted by `llvm-profgen`, and rebuilt with `-Cprofile-sample-use`.

Implemented pilot:
- `scripts/run-rust-sample-pgo.sh`
- exact toolchain/compiler/LLVM-major match checked;
- `perf` and `llvm-profgen` are explicit prerequisites;
- branch-stack collection failure returns `NOT_EVALUABLE`, not a false pass;
- profile, raw `perf.data`, compiler identity and exact workload are digested;
- exact stdout equality is required for a mechanical pass.

Sample PGO is initially scoped to x86_64 Linux, matching the Rust documentation's strongest validation area. Shared hosted CI may deny access to hardware performance counters. Run it on a controlled Linux/NixOS host with PMU access; absence of PMU permission is an infrastructure limitation, not an optimization result.

## Frozen pilot design

| Role | Workload | Purpose |
|---|---|---|
| Training | `jphi_convergence` | Capture common simulation-library execution paths |
| Transfer probe A | `gradient_replacement` | Different controller set and long multi-agent run |
| Transfer probe B | `cooperation_decomposition` | Different regeneration/control decomposition |

This is deliberately labelled a **transfer probe**, not a same-distribution holdout. These programs use their own fixed seed schedule, and the current examples do not accept external seeds or scenario files. Do not change those seeds or scientific source merely to make PGO appear faster.

## Benchmark tournament

Compare on the same Rust version and exact source:
1. release build without PGO;
2. instrumentation PGO;
3. sample-based PGO;
4. combined or blended training profiles only after the first three lanes are understood;
5. optionally PGO with LTO/ThinLTO as a separate factor, never silently bundled into the first PGO comparison.

Measure independently:
- clean compilation and profile-generation/build overhead;
- steady-state executable runtime;
- peak RSS and final executable size;
- profile size and training duration;
- held-out/cross-example transfer;
- exact observable output and numerical/statistical invariants;
- run-to-run variance.

Do not compare PGO Rust 1.100 against non-PGO Rust 1.99 and call the difference a PGO gain. First compare PGO vs non-PGO under the same exact compiler. Compiler-version comparison remains a separate factor.

## Acceptance targets (proposed, not achieved)

A production PGO profile should be retained only if:
- at least 5% median runtime improvement on the representative target workload;
- at least 3% improvement on every required held-out workload, or a consciously documented workload-specific profile selection;
- 95% paired confidence interval excludes zero for the primary workload;
- no critical workload regresses by more than 2%;
- executable size growth stays within 5%, and peak RSS growth within 5%, unless an explicit deployment trade-off is accepted;
- deterministic output and scientific invariants pass;
- profile source/compiler/target/workload digests all match;
- profile expiry and workload-drift rules are defined.

These are candidate engineering thresholds. They are not universal industry SOTA scores and must be calibrated against a stable hardware baseline.

Statistical requirements:
- 5 paired runs are exploratory only;
- 30+ paired independent runs for normal runtime-improvement claims;
- p99 only with at least 10,000 observations per condition or a justified design;
- rotate trial order, fix CPU governor where possible, record thermal/power conditions, and retain raw outputs.

## Paradigm shifts beyond PGO

1. **Profile provenance as supply-chain evidence.** Sign and bind profile data like an artifact, because optimization behavior can be steered by the training corpus.
2. **Workload coverage, not average speed.** Use a workload portfolio and held-out transfer probes; reject profiles that overfit a single hot path.
3. **Correctness-constrained optimization.** Performance is a constrained objective: preserve outputs/invariants first, then optimize runtime/size/RSS.
4. **Optimization portfolios.** Keep separate CPU profiles for different deployment classes (laptop, server, WASM, embedded) instead of one global profile. The target triple and CPU baseline must be explicit.
5. **Profile drift detection.** Recollect when production workload mix diverges from the profile's training manifest. A stale profile is a versioned, expiring artifact—not an evergreen cache.
6. **Counterfactual performance diagnosis.** Compare profile hot paths with unoptimized and alternative algorithmic implementations; do not let PGO camouflage an O(n²) algorithm or allocation problem.

## Industry/SOTA comparison

- Use upstream `rustc-perf` for compiler-wide fresh/incremental build comparisons. It is not an application runtime benchmark.
- Use Cargo `--timings` to locate build-graph critical paths, not to infer simulation runtime.
- Use these Symtropy simulations for application-level execution and determinism.
- Compare PGO vs non-PGO on the same compiler/source; compare Rust versions separately.
- Use Criterion or another statistical harness for small kernel-level HDC/vector/physics routines if a larger reusable library benchmark becomes available.
- Do not claim SOTA superiority based on one host's absolute runtime. Publish source, profile, target, seed, hardware and analysis artifacts for reproduction.

## Current validation boundary

- Rust 1.100.0 stable has passed the compiler qualification and core supported-target suite in the parent PR.
- The instrumentation and sample-PGO scripts are new and have not yet been run on a physical PMU-capable target.
- No profile, speedup, binary-size result or scientific result is claimed by adding these scripts.
- The existing example outputs are not modified; a PGO output difference is a review blocker, not a result to normalize away.

## References

- Rust 1.100.0 release announcement: https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/
- Rust PGO guide: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- Rust compiler performance testing: https://rustc-dev-guide.rust-lang.org/tests/perf.html
- rustc-perf profiling: https://rustc-dev-guide.rust-lang.org/profiling/with-rustc-perf.html
