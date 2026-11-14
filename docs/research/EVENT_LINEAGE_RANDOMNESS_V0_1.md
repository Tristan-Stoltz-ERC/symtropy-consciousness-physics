# Hierarchical event-lineage randomness v0.1

## Paradigm shift

Event-keyed randomness removes mutable-stream call-order contamination only when a caller can name the event. Dynamic entities and conditional events make stable identity harder: global actor counters, array positions, branch-local draw counts, and thread schedules can accidentally pair different causal events.

This module adds explicit semantic ancestry. A model supplies a stable root subject and describes each child by a versioned event-kind tag, draw-purpose tag, and model-defined occurrence ordinal. Identity derives from the parent and semantic role, never from execution order, a treatment label, or a global counter.

## Identity construction

- Root ID: Philox4x64-10 over experiment namespace, stable root subject, a fixed root-domain tag, and schema version.
- Child ID: Philox4x64-10 over parent 128-bit ID, packed event-kind/purpose/occurrence, and a fixed child-domain tag.
- ID width: 128 bits.
- Lineage identity is independent of replicate seed; the random Philox key remains [replicate_seed, experiment_namespace].
- Random counter: [lineage_id_word_0, lineage_id_word_1, simulation_tick, block_index].

The semantic path is retained alongside its ID. A per-trace registry allows identical paths to be re-registered but rejects distinct paths that collide to the same 128-bit ID.

## Counterfactual semantics

A shared causal parent plus the same semantic child role yields the same child ID in both alternatives. A treatment-only or control-only child has no counterpart and remains unmatched. Sibling creation order does not change identity.

The model author must determine whether two events are causally the same event. The implementation does not infer equivalence from similar values, array positions, approximate timestamps, or state similarity.

## Qualification scope

Tests lock a schema-v1 root/child known-answer fixture, sibling-order independence, parent/role/occurrence sensitivity, shared-versus-conditional branch distinction, replicate-key separation, open-uniform bounds, registry idempotence, and a forced-collision negative control.

The forced collision is injected into the test; it is not evidence of a naturally occurring 128-bit collision.

## Limits

- Deterministic non-cryptographic 128-bit derivation is not mathematically collision-free.
- Collision detection is only as complete as the paths registered by the model's trace capture.
- The model must supply stable root IDs and semantically valid occurrence ordinals.
- This is additive; the existing v1 event-address API and historical LCG remain unchanged.
- This is not yet integrated into Symtropy simulation call sites.
- No statistical battery, GPU/cross-language identity, end-to-end performance, or physical-model causal validity is claimed.
