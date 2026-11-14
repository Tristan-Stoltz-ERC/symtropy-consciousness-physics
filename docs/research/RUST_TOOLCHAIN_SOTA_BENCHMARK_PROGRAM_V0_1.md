# Rust toolchain qualification and SOTA benchmark programme v0.1

## Purpose

This programme compares compiler releases as controlled engineering dependencies. It does not attempt to outperform the Rust compiler team's compiler implementation. The goal is to outperform weak industry practice in the quality of toolchain selection evidence: exact source and dependency identity, matched workload design, separate cache conditions, application-specific outcomes, numerical behavior, and verifiable artifact provenance.

Rust compiler-wide performance must be compared with the upstream \`rustc-perf\` corpus. The Luminous corpus complements it; it is not a substitute.

## Separate benchmark families

### 1. Compiler-wide performance

- Run the same pinned \`rustc-perf\` revision on both toolchains.
- Use the same runner image, host architecture, benchmark corpus and configuration.
- Retain raw measurements and upstream harness metadata.
- Report compiler-wide results separately from application build results.
- Do not claim a compiler-wide SOTA improvement from Symtropy-specific workloads.

### 2. Cargo build efficiency

Treat these as separate experiments, not one aggregate score:

| Condition | Input/output treatment | Question |
|---|---|---|
| Fresh target build | Fresh target and build directories; warmed registry; offline build | What is clean compilation cost? |
| No-op build | Reuse exactly the same target/build directories | What is Cargo's no-op overhead? |
| Incremental build | Reuse one target directory and apply a predefined source edit | What is edit-to-build latency? |
| Dependency resolution | Fresh Cargo home versus prewarmed Cargo home, measured separately | What is network/resolver cost? |
| sccache | Explicit cache-off and cache-on conditions with separate cache namespaces | What is attributable compiler-cache benefit? |
| Linking | Record Cargo timings and linker/version/configuration | Is link time dominating the observed result? |

A fresh-target result is not a cold-disk or cold-registry result. The current baseline is fresh output directories with a warmed registry and a page cache that is not forcibly dropped.

### 3. Luminous application behavior

Use source/lockfile-identical project workloads. Keep each project suite separately reportable.

- **Symtropy:** deterministic simulation throughput, event-key generation overhead, HDC/field operations where the standalone benchmark is correctly implemented, numerical invariants, output determinism and causal fixture recovery.
- **Symthaea:** HDC bind/bundle/query, CfC/LTC sequence processing, FEP inference, memory footprint and numerical behavior.
- **Prism:** startup, request throughput, concurrency, p50/p95/p99 latency, peak resident memory and error rates.
- **Mycelix:** zome compilation, Wasm artifact size, serialization round trips, cryptographic operations and conductor integration.
- **NixOS/Nixward:** evaluation time, derivation closure size, build/rebuild time, cache behavior and rollback correctness.

Do not compare unlike workloads or use one project's application result as evidence about another project's runtime.

### 4. Runtime and scientific behavior

A faster build is not a faster application. A faster application is not necessarily numerically or scientifically equivalent.

- Preserve exact source, lockfile, binary identity and workload inputs.
- Use deterministic fixtures and compare output bytes where bit identity is expected.
- Use domain-specific tolerances only when preregistered and justified.
- Keep compiler correctness, numerical regression, runtime, and scientific inference as separate gates.
- For counterfactual simulation, keep event-key alignment and unmatched/conditional event counts in the same run receipt.

## Trial design and sample sizes

### Exploratory phase

- At least 5 independent paired fresh-build trials per workload.
- Rotate toolchain order within blocks (AB/BA or a balanced schedule).
- Treat 5-run results as exploratory; do not call them confirmatory.

### Confirmatory phase

- At least 30 independent paired build trials for ordinary build-latency claims.
- Keep all runs, failures, retries, and exclusions in the raw record with reasons.
- Use paired trial-level log ratios; report median, MAD and a deterministic bootstrap interval.
- Bootstrap independent build trials, not each runtime repetition as if it were an independent compiler build.
- For multi-workload claims, preregister the comparison family and use Holm adjustment.

### Tail latency

- Do not publish p99 from a handful of runs.
- Aim for at least 10,000 runtime observations per condition, with an explicit plan for dependence between observations.
- Where samples are nested within binaries/build trials, use a hierarchical or block bootstrap. A large count of repeated requests against one binary is not 10,000 independent compiler-build observations.
- Report p50/p95/p99, error rate, and confidence intervals; state the observation unit.

## Host and environment controls

Record at minimum:

- exact \`rustc -Vv\`, \`cargo -Vv\`, compiler commit and target triple;
- exact Git commit/tree, manifest digest and Cargo.lock digest;
- OS image, kernel, CPU model/count, memory, filesystem/mount and governor when available;
- \`RUSTFLAGS\`, \`CARGO_ENCODED_RUSTFLAGS\`, Cargo build flags, linker and wrapper policy;
- fresh/reused target and build-directory state;
- Cargo registry/cache state and network mode;
- raw Cargo JSON messages, Cargo timing reports, stdout/stderr and artifact digests.

Do not mix measurements from different hosts or images into one paired comparison without an explicit blocked design. Do not silently discard failed runs or outliers.

## Initial acceptance targets

These are proposed research gates, not measured achievements:

| Dimension | Initial target |
|---|---|
| Correctness | Zero unexpected test, ABI, serialization or deterministic-output regressions |
| Dependency identity | Zero unapproved Cargo.lock changes |
| Artifact provenance | 100% of Cargo-reported artifacts represented by content digest and byte size |
| Fresh build trials | 30+ paired trials for confirmatory claims |
| Runtime p99 | 10,000+ observations per condition, with dependence accounted for |
| Cache correctness | Zero wrong-key or tampered-artifact acceptance in negative controls |
| Performance win claim | At least 5% paired median improvement with interval/uncertainty supporting the claim |
| Regression budget | Any >=5% median regression in a critical workload blocks an unqualified performance claim and requires investigation |
| Reproducibility | Independent clean builds compared by artifact digests; no bit-reproducibility claim until demonstrated |

A compiler release may pass compatibility and promotion without being faster. The 5% target applies to a performance-improvement claim, not to the decision to qualify a stable release.

## Cache qualification: from cache hit to verifiable cache reuse

A build cache should be treated as a distribution of previously produced artifacts with an explicit identity key. A cache key should include at least:

- compiler release and commit;
- target triple and target features;
- package/target identity and dependency graph identity;
- profile, codegen options and relevant environment flags;
- source identity and build-script inputs;
- linker identity/configuration where it affects the output.

Cache experiments should include:
1. known-good artifact/key pair accepted;
2. one-bit artifact mutation rejected;
3. compiler version change invalidates reuse;
4. target/profile/flags change invalidates reuse;
5. source/dependency change invalidates reuse;
6. cache miss falls back to a locked clean build;
7. cache restore does not modify Cargo.lock or source state.

The artifact passport is a prerequisite for this programme, not proof that cache reuse is safe. A subsequent cache-reuse design must verify the cache's declared key against the requested build and verify every returned artifact digest independently.

## Promotion rule

1. Verify stable distribution and exact component provenance.
2. Qualify compile/test/Clippy/rustdoc/ABI/target compatibility.
3. Compare source, lockfile, numerical behavior and deterministic outputs.
4. Run the application-specific paired benchmark programme.
5. Review all regressions and limitations.
6. Publish a promotion receipt bound to the exact source and toolchain.
7. Change default pins only through a separate reviewable promotion decision.

## Industry comparison

| Common capability | Evidence gap frequently left open | Luminous qualification target |
|---|---|---|
| rustup toolchain pin | Intended version may differ from the compiler actually invoked | Capture actual rustc/Cargo identity in each receipt |
| Cargo.lock | Graph identity alone does not identify emitted binaries | Bind lockfile/source/toolchain to artifact hashes |
| cargo-audit/advisories | Vulnerability scans do not address release maturity or resolver freshness | Pair advisory review with Cargo publication-age policy |
| sccache | Cache hit does not by itself prove correct artifact identity | Explicit cache key, content hashes and tamper controls |
| CI build/test | A green test run may not establish cross-version behavioral parity | Differential behavior and exact artifact provenance |
| compiler microbenchmarks | Compiler benchmark speed may not help target workloads | Run upstream rustc-perf and matched Luminous workload suite |
| performance dashboards | Aggregation can hide host/order/sample bias | Paired, blocked trials with uncertainty and preserved raw data |

The intended differentiator is not a new compiler or cache algorithm alone. It is the evidence chain from requested toolchain through actual compiler identity, source/dependencies, build execution, output artifacts, behavior, and release decision.
