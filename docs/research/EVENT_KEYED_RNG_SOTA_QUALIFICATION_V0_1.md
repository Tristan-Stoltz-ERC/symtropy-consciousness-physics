# Event-keyed RNG SOTA qualification programme v0.1

## Position relative to the state of the art

The Philox4x64-10 core is not a new RNG algorithm. It follows the Random123 Philox construction. Random123 provides counter-based generators for CPU and GPU use, documents Philox and Threefry families, and reports SmallCrush, Crush and BigCrush qualification for its library implementation.

The current Rust prototype has passed three known-answer vectors and deterministic mapping tests. That is implementation correctness evidence, not inherited TestU01 qualification. A correct port still needs its own test campaign, including its float conversion and event-address mapping.

The differentiated research contribution is the layer above the generator:

1. semantic event addresses rather than mutable call-order streams;
2. explicit model-owned event-sharing policy;
3. auditable matched, conditional and unmatched counterfactual events;
4. causal known-effect fixtures;
5. empirical measurement of when common random numbers help and when they hurt.

Do not claim a better RNG than Random123 on statistical quality or throughput until matched measurements exist.

## Baseline families

### Counter-based generator baselines

- Random123 Philox4x64-10 reference implementation.
- Random123 Threefry4x64 at the reference round count.
- The current Symtropy LCG as a historical baseline, only where the same output width and workload semantics can be compared.
- A separate high-quality general-purpose generator such as PCG64 or ChaCha20 may be included for context, but cryptographic and simulation RNGs must not be ranked on one undifferentiated scale.

Random123 explicitly describes these generators as non-cryptographic. The event RNG must never be used for secrets, authentication, tokens or adversarial unpredictability.

### Statistical batteries

1. TestU01 SmallCrush as an early smoke battery.
2. TestU01 Crush as a larger screening battery.
3. TestU01 BigCrush as the full high-cost empirical battery.
4. PractRand at staged byte-volume checkpoints, with complete test logs.
5. Tests of the exact top-52-bit open-interval conversion, not only raw u64 words.

Test both:
- the raw Philox4x64-10 output words;
- the event-addressed stream families produced by varying subject IDs, ticks, event kinds, purposes, namespaces, replicate seeds and block indices.

The test harness must preserve the exact key/counter schedule. A statistical pass on one sequential counter stream does not test every event-address mapping or prove event identity is causally correct.

## Benchmark decomposition

Keep three costs separate:

| Layer | Measurement | Comparison |
|---|---|---|
| Generator core | Philox blocks/second, cycles/word, latency distribution | Rust implementation vs Random123 reference |
| Address mapping | complete EventAddress to word, including packing and subject-key construction | event-keyed RNG vs raw Philox |
| Application | end-to-end Symtropy model throughput and paired estimator variance | unchanged LCG baseline vs opt-in event-keyed run |

For each layer, record exact compiler, CPU, target features, OS/kernel, thread count, input corpus and source SHA. Measure serial and parallel schedules separately. Do not compare CPU and GPU throughput as if they were the same execution environment.

## Throughput and latency targets

These are qualification targets, not claims about current performance:

- report raw words/second and event-addressed draws/second;
- measure 1-thread, physical-core and oversubscribed-thread configurations;
- report p50/p95/p99 latency only with an adequate observation count;
- profile address packing separately from the 10-round Philox permutation;
- use an unchanged baseline and a reference implementation on the same host;
- any optimization must preserve all known-answer, order-invariance and cross-thread bit-identity tests.

A proposed optimization gate is no more than 10% overhead for address mapping relative to the same Rust Philox core in an optimized build. This is an initial engineering target and must be revised after representative workload measurements; it is not a performance result.

## Cross-platform and implementation parity

Required before broad integration:

- x86_64 and aarch64 raw-word identity;
- little-endian and big-endian serialization fixtures where supported;
- Rust/C reference parity for complete event addresses;
- GPU parity only if a GPU implementation is actually introduced;
- thread-count and task-order invariance;
- integer-only mapping parity before testing the f64 conversion.

No GPU support is currently implemented. A CPU implementation that is deterministic on one architecture must not be described as cross-platform qualified.

## Event-address schedule testing

A separate suite must exercise:
- sequential subjects and sparse/random subject IDs;
- consecutive ticks and skipped ticks;
- all registered event-kind and draw-purpose values;
- occurrence ordinals 0, 1, 2, boundary values and collision checks;
- multiple experiment namespaces;
- adjacent and widely separated replicate seeds;
- block and scalar draw indices around 0, 3, 4 and block boundaries;
- reordered and parallel evaluation.

Report coverage for each key dimension and retain raw fixtures. KAT vectors validate the permutation implementation; schedule tests validate the mapping into that permutation.

## Causal quality and CRN evaluation

A high-quality random generator does not make a counterfactual comparison valid. Every real model integration must report:
- matched event count and fraction;
- reference-only and treatment-only event counts;
- conditional event counts by registered event family;
- semantic alias and duplicate-key counts;
- paired versus independent estimator variance;
- analytical known-effect fixture recovery where available.

CRN can reduce variance when common exogenous inputs induce positive covariance, and increase variance when covariance is negative. The benchmark in this PR is intentionally two-sided; a single favorable test case is not adequate evidence.

## Acceptance gates

| Gate | Minimum evidence before promotion |
|---|---|
| Reference correctness | all published Random123 KAT vectors for the supported Philox configuration |
| TestU01 | SmallCrush, Crush and BigCrush logs retained; failures investigated, never filtered silently |
| PractRand | staged long-stream runs with raw command/output receipts |
| Mapping | full event-address dimension and block-boundary tests |
| Parallelism | bit-identical words across tested thread counts and task orders |
| Cross-language | Rust/reference C parity before claiming portability |
| Performance | same-host, same-compiler paired benchmarks; report core and mapping costs separately |
| Causal integration | model-reviewed event registry, lineage policy and matched/unmatched event ledger |
| Security | explicit non-cryptographic classification; no use for adversarial secrets |

A statistical test battery is evidence about tested output streams, not a proof of randomness. Passing these tests does not replace model validation, event-semantics review, independent replicate design or causal identification.

## References

- Random123 project and published generator qualifications: https://random123.com/releases/1.06/docs/
- TestU01 project: https://simul.iro.umontreal.ca/testu01/tu01.html
- TestU01 paper: https://www.gerad.ca/en/papers/G-2006-69
- PractRand documentation and test batteries: https://pracrand.sourceforge.net/
- Parallel Random Numbers: As Easy as 1, 2, 3 (SC11), linked from the Random123 documentation.

## Current status

Completed:
- Philox4x64-10 implementation with three Random123 known-answer vectors;
- event-address mapping and serial/parallel invariance tests;
- offline event-alignment ledger;
- analytical positive- and negative-covariance CRN fixture;
- semantic event/lineage policy draft.

Not yet completed:
- TestU01 or PractRand campaign;
- direct Rust-vs-C throughput comparison;
- event-address throughput benchmark;
- GPU and cross-language port;
- dynamic entity lineage implementation;
- real Symtropy call-site migration.
