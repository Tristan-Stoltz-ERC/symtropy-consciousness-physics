# Event-keyed RNG qualification

This is an opt-in research track. It does not replace Symtropy's historical LCG or alter existing seed schedules, CSVs or published experiment outputs.

## Implemented layers

1. **Generator:** pure Philox4x64-10 implementation with three Random123 known-answer tests.
2. **Event mapping:** stable key/counter address, event-kind/purpose registry, subject and pair IDs, deterministic block/scalar access.
3. **Trace alignment:** matched, reference-only, counterfactual-only and mismatched event/draw accounting.
4. **Semantic identity audit:** model-owned event IDs and MustShare / Conditional / Distinct policy; fail closed on alias, split identity, duplicates, policy drift and missing MustShare claims.
5. **Causal benchmark:** analytical counterfactual model with a positive-covariance CRN case and a negative-covariance control.
6. **Throughput benchmark:** raw Philox, address construction, random-access scalar, cached sequential scalar, cached open-uniform and historical LCG helper measured separately.
7. **Independent reference parity:** pinned Random123 C++ differential comparison across raw and event-address-derived counter/key pairs.
8. **Dynamic lineage prototype:** canonical BirthKey derivation, static/dynamic collision registry and deterministic full-provenance assignment export.

## Evidence currently available

### Generator and identity software checks

The exact-toolchain qualification workflow covers Rust 1.99.1, Rust 1.100.0 and Rust 1.101.0-beta.1. It runs the Philox known-answer tests, event-address adversarial tests, semantic identity audit tests, CRN fixture, formatting and library compilation.

The audited event-identity branch passed this workflow on commit c9ea5fdbf023996db8b1b31ac933645ad2df9dfa.

### Analytical CRN result

The fixture uses:

- U distributed Uniform(0,1)
- Y0 = U²
- Y1 = U² + 0.25 + beta U
- exact ATE = 0.25 + beta/2
- 2,048 subjects per replicate, 512 outer replicates, 2,000 bootstrap resamples

The hosted qualification run measured:

| Case | Empirical matched/independent variance ratio | Theoretical ratio | 95% bootstrap interval |
|---|---:|---:|---:|
| Positive covariance, beta = 0.5 | 0.077000 | 0.073892 | [0.066638, 0.088522] |
| Negative covariance control, beta = -1.5 | 1.644712 | 1.626506 | [1.514869, 1.793229] |

Both estimators recovered the analytical ATE within the fixture tolerance. The result illustrates the conditional nature of common random numbers: they can strongly reduce variance under positive covariance and increase variance under negative covariance.

This is a deterministic software fixture, not a real Symtropy physical/social experiment or a causal-validity result.

### Independent implementation parity

The pinned Random123 differential workflow passed the exact Rust 1.99.1, 1.100.0 and 1.101.0-beta.1 lanes on the parity PR head. It compared **4,099 cases / 16,396 output words** with zero mismatches against Random123 commit `ff4443af61194bb57a042cd4945f2c73bdf1d0dd`.

The corpus comprised three fixed vectors, 2,048 raw counter/key pairs and 2,048 event-address-derived pairs. This is finite differential parity, not TestU01/PractRand qualification or a proof over the entire input space.

### Cached sequential-word throughput

A five-round Rust 1.100 hosted-runner smoke measured the cached u64 iterator at **5.284 ns/word**, versus **11.061 ns/word** for repeated random-access scalar calls on a prebuilt address, with stable checksums. The block API measured **3.023 ns/word** in the same run. This indicates that the iterator removes repeated Philox block calculations but block consumption remains the lowest-cost supported path in that sample.

These are shared-runner microbenchmarks, not same-host Random123 performance comparisons or SOTA claims.

## Contracts and specifications

- [Philox implementation contract](implementation-contract-v0.1.json)
- [Event alignment contract](alignment-contract-v0.1.json)
- [CRN variance contract](crn-variance-contract-v0.1.json)
- [Event identity/lineage contract](event-identity-lineage-contract-v0.1.json)
- [Semantic identity claim-audit contract](event-identity-claim-contract-v0.1.json)
- [SOTA qualification contract](sota-qualification-contract-v0.1.json)
- [Throughput benchmark contract](throughput-contract-v0.1.json)
- [Random123 reference parity contract](reference-parity-contract-v0.1.json)
- [Dynamic entity lineage contract](dynamic-entity-lineage-contract-v0.1.json)

Research documents are in docs/research:
- EVENT_KEYED_RNG_IMPLEMENTATION_V0_1.md
- EVENT_ALIGNMENT_LEDGER_V0_1.md
- EVENT_RNG_CRN_VARIANCE_BENCHMARK_V0_1.md
- EVENT_IDENTITY_AND_LINEAGE_POLICY_V0_1.md
- EVENT_IDENTITY_CLAIM_AUDIT_V0_1.md
- EVENT_KEYED_RNG_SOTA_QUALIFICATION_V0_1.md
- EVENT_RNG_THROUGHPUT_BENCHMARK_V0_1.md
- EVENT_RNG_RANDOM123_REFERENCE_PARITY_V0_1.md
- EVENT_DYNAMIC_ENTITY_LINEAGE_V0_1.md

## Not yet qualified

- TestU01 SmallCrush, Crush and BigCrush.
- PractRand staged stream tests.
- Rust-versus-Random123 C reference throughput (parity is now qualified on the finite corpus; throughput comparison remains unrun).
- Aarch64 and cross-language parity.
- GPU implementation or qualification.
- Integration of the dynamic-entity lineage prototype into real model birth sites; automatic instrumentation remains unimplemented.
- Population of a model-owned event registry.
- Integration into actual Symtropy simulation call sites.
- Same-host application performance or end-to-end estimator improvement in a real model.
- TestU01/PractRand statistical batteries for this Rust port.

Do not treat the fact that Random123's published implementation passed statistical batteries as inherited qualification of this Rust port. Do not claim a new RNG statistical-quality or throughput SOTA from the current evidence.

## Execution

The current modules are opt-in and independently testable. A prototype lineage registry can derive and collision-check dynamic IDs from canonical birth provenance, but no historical Symtropy birth sites are instrumented. Historical RNG call sites remain unchanged until model-owned sharing policies, persistent lineage receipts, real trace capture and application-specific validation are reviewed.
