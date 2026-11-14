# Dynamic entity lineage identity v0.1

## Why this exists

Counter-based random addresses require stable subject identity. Static agents and canonical pair identities are not enough for dynamic entities that are born, spawned, split, merged or otherwise created during a simulation.

Allocating IDs from "the next integer" is order-sensitive. Inserting an unrelated birth, changing thread scheduling, or traversing events in a different order can shift later IDs and therefore change every downstream random address. That defeats counterfactual alignment even when the RNG itself is stateless.

## Canonical birth identity

The implementation represents a birth with:

**BirthKey = (world_namespace, sorted_parent_subject_ids, semantic_birth_event_id, offspring_slot)**

- world_namespace is a model-owned identity domain shared by alternatives that should preserve the same entity lineage. It is not a treatment or controller label.
- sorted_parent_subject_ids are the complete stable identities of the parents. Order is canonicalized; duplicates are rejected.
- semantic_birth_event_id is a model-owned nonzero SemanticEventId, not a runtime event counter or a hash of a display name.
- offspring_slot is a domain-defined semantic slot, not the child's position in a runtime vector.

The key intentionally excludes thread ID, wall clock, processing order, allocator sequence and branch traversal path.

## Deterministic derivation

The BirthKey is mixed through the already-qualified Philox4x64-10 function using a separate lineage domain. The high bit is set as a diagnostic marker for derived IDs. This marker does not create a disjoint numeric namespace: static pair IDs can occupy the same range, so explicit reservation and collision checks remain mandatory.

The derivation is **not cryptographic**. It is a deterministic identity mixer. Its output is never trusted without the collision registry.

## Collision registry

LineageIdRegistry maintains both directions:

- BirthKey -> SubjectId
- SubjectId -> BirthKey

It also accepts a reservation set for pre-existing static, global and pair identities.

Required use:
1. Reserve every static/global/pair subject identity before resolving dynamic births.
2. Use one run-scoped registry across the counterfactual alternatives whose entity identities are meant to align.
3. Resolve a BirthKey. Repeated resolution of the same key is idempotent.
4. Treat a collision or late static reservation as a hard qualification failure.
5. Preserve the full BirthKey in the run evidence ledger; do not retain only the derived u64.

A collision is not repaired by assigning a new ID because that would make identity dependent on whichever event was processed first. Qualification fails closed.

## Tests

The module tests:
- parent-order canonicalization;
- deterministic IDs across independent registries;
- idempotent resolution;
- insertion/order invariance when an unrelated birth is registered first;
- identity changes when namespace, parent, birth-event or offspring slot changes;
- collision against reserved static IDs;
- dynamic-to-dynamic hash collision rejection (forced candidate injection);
- rejection of a static ID registered after a dynamic identity has claimed it;
- duplicate-parent and zero-namespace rejection.

## Boundaries

- A successful resolution means the key is internally stable and collision-free in the registry used. It does not prove that the modeler chose the scientifically correct birth event or parent set.
- The world namespace must be shared across alternatives for entities that should correspond. Giving alternatives distinct world namespaces intentionally produces distinct lineages.
- A run-scoped registry must be shared or its receipts reconciled across scenario executions. Independent registries alone cannot detect a cross-scenario hash collision.
- There is no automatic instrumentation of existing Symtropy birth sites yet.
- There is no automatic causal matching of child entities; model-owned semantic event claims and event-alignment receipts still apply.
- This is not a cryptographic identity, authentication, anti-forgery or Sybil-resistance mechanism.

## Next integration gate

Before dynamic identities enter a real model:
1. identify each dynamic entity class and its canonical parent set;
2. define birth-event IDs and offspring slots for each class;
3. reserve the model's static identities;
4. emit complete BirthKey/SubjectId receipts;
5. run order/thread permutation tests;
6. reconcile lineages across matched counterfactual scenarios;
7. keep unmatched/conditional births visible in the event-alignment ledger.
