# Event-keyed RNG implementation v0.1

## Status

This is a pure, separately versioned Philox4x64-10 implementation and an address-to-counter mapping. It is not wired into Symtropy simulations. The historical LCG, seed schedule, call sites, CSVs and published results remain unchanged.

## Exact implementation

- Algorithm: Random123 Philox4x64-10.
- Key: \`[replicate_seed, experiment_namespace]\`.
- Counter: \`[subject_id, simulation_tick, packed_event_code, block_index]\`.
- Event code: \`event_kind:u16 | purpose:u16 | occurrence_ordinal:u32\`.
- Each Philox block returns four u64 words. Scalar index i maps to block \`i / 4\`, lane \`i % 4\`.
- No mutable stream, execution-order index, treatment label, thread ID or controller branch is accepted by the API.
- Subject ID 0 is reserved for environment-global events. Stable agent IDs are nonzero u32 values; pair IDs use canonical ordered packing.
- Event-kind and purpose tags are fixed numeric registry entries, not hashes or enum declaration order.

## Floating-point mapping correction

The RFC suggested a 53-bit midpoint construction. The implementation uses the top 52 bits instead:

\`(top52 + 0.5) * 2^-52\`

At the upper end of the 53-bit integer range, adding 0.5 can round to exactly 2^53 in binary64, yielding 1.0. The 52-bit midpoint form is exactly representable and stays strictly inside (0, 1). This is a deliberate correctness refinement before any output is published.

## Reference qualification

The pure Philox core is tested against Random123's published \`examples/kat_vectors\` known-answer cases: all-zero input, all-ones input and a mixed counter/key input. The implementation constants and round ordering follow Random123's \`philox.h\` reference definition.

These are implementation known-answer tests, not a statistical battery. No TestU01, BigCrush, PractRand, distribution-transform, GPU, or cross-language campaign has run.

## Adversarial properties covered by software tests

- Same address and draw index are bit-identical.
- Intervening unrelated event queries cannot shift a later event's value.
- Scenario/controller labels are absent from the event address.
- All four block lanes and the next block are mapped deterministically.
- Subject identity is canonical for unordered pairs.
- Zero agent IDs and self-pairs are rejected.
- Every tested open-unit output is strictly between 0 and 1.
- Serial and parallel address evaluation agree.

## Explicit limits

- No simulation event semantics are chosen or integrated by this module.
- Stable event identities are a model-owner responsibility. A hash cannot infer whether two events in different interventions are causally the same event.
- Dynamic-entity genealogical IDs, pair-event occurrence semantics and per-model tag registries still require domain review.
- Philox is not a cryptographic random-number generator.
- Execution-order invariance is necessary for coherent CRN comparisons, but it does not prove structural causal validity.
- No performance or SOTA claim is made.

## Next qualification steps

1. Add model-reviewed event registry entries and deterministic identity allocation.
2. Port initialization and RandomWalk angle sampling behind an opt-in feature/experiment flag.
3. Build a toy structural causal model with an analytically known treatment effect.
4. Record matched, unmatched and duplicate event identities across controller alternatives.
5. Run TestU01/PractRand on raw words and the exact uniform conversion.
6. Benchmark event-key construction and end-to-end simulation overhead against the unchanged LCG.
7. Run cross-platform and cross-thread-count bit-identity tests before any publication.
