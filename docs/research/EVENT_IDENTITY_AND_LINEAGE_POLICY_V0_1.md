# Event identity and lineage policy v0.1

## Why this is a separate contract

Counter-based RNGs remove dependence on mutable stream position. They do not decide whether two random draws in different counterfactual worlds represent the same exogenous event.

That semantic decision belongs to the model. A false match can be more damaging than an unmatched event because it silently couples two different causes. Event identity therefore needs an explicit, reviewable lifecycle contract.

## Identity layers

### Experiment identity

**experiment_namespace** names a fixed experiment and random-variable domain. Keep it stable across paired alternatives. Use a different domain namespace when intentionally drawing an independent exogenous variable. Do not reuse a namespace for unrelated experiments.

### Replicate identity

**replicate_seed** identifies an independent outer replicate. It is the unit used for statistical replication; agent IDs, event counts and simulation ticks are not independent replicates.

### Subject identity

- Static subjects use model-assigned, nonzero IDs that remain stable across counterfactual alternatives.
- The global environment reserves subject ID zero.
- Unordered pair subjects use canonical ordering.
- IDs must not be allocated from intervention-dependent iteration order or thread scheduling.

### Dynamic entity identity

Dynamic entities (births, particles, vehicles, households, buildings, jobs, spawned agents) need a stable lineage identity, not merely "the next integer allocated".

The recommended conceptual birth key is:

**BirthKey = (world_namespace, canonical_parent_ids, birth_event_key, offspring_slot)**

The offspring slot is a domain-defined semantic slot, not the position in a runtime vector. The same birth event and lineage inputs must yield the same child identity after changes to thread count, event processing order, or unrelated births.

A later implementation may hash this canonical tuple into the current u64 subject field, but it must also retain the full birth provenance in an audit ledger and reject any hash collision. A hash alone is not a proof of identity.

### Event identity

The current v1 address is:

**(replicate_seed, experiment_namespace, subject_id, simulation_tick, event_kind, draw_purpose, occurrence_ordinal)**

- **event_kind** identifies the modeled event family.
- **draw_purpose** identifies the exogenous variable being sampled.
- **occurrence_ordinal** is a semantic occurrence slot within that event family, not a mutable draw counter.
- Draw index selects output words from one address; it is not a new causal event identity.

Never put controller name, treatment label, thread ID, wall clock, execution order or branch traversal path into a shared exogenous event address.

## Sharing policy

Every model event family must declare one of three policies in its model-owned registry:

| Policy | Meaning | Alignment interpretation |
|---|---|---|
| must_share | Same exogenous cause exists in both alternatives | Same semantic event key must match when present in both |
| conditional | Cause exists only when a model condition is met | Missing events remain unmatched and are reported, never synthesized |
| distinct | Events are genuinely different causes | Different semantic keys; do not force a common random draw |

This policy is metadata for the model and auditor. It is not automatically inferred from identical output values.

## Failure modes to detect

1. **Alias:** two different semantic causes accidentally map to the same RNG address.
2. **Split identity:** the same modeled cause receives different addresses in two alternatives.
3. **Order leakage:** an address changes when unrelated events are inserted or processing order changes.
4. **Generation reuse:** a dead entity's ID is reused for a new entity.
5. **Ordinal drift:** an occurrence ordinal is derived from traversal order rather than semantic slot.
6. **Namespace leakage:** independent experiments accidentally reuse an event namespace.
7. **False completeness:** a high matched-event percentage hides a small but scientifically important unmatched event family.

## Required evidence before real simulation integration

- A model-owned event registry with named causal roles and sharing policies.
- A stable subject/lineage policy for every dynamic entity class.
- Collision and alias checks over complete semantic provenance, not just sampled words.
- Permutation tests that reorder unrelated births/events and vary thread count.
- Per-event-family matched, unmatched and duplicate/alias counts.
- Known-effect counterfactual fixtures and a negative-covariance CRN control.
- Reconciliation of every event claim emitted by a simulation trace to the registry.

## Acceptance targets

These are proposed gates, not measured achievements:

- Zero alias collisions.
- Zero duplicate event/draw keys within a scenario trace.
- 100% bit identity for shared event addresses across supported execution schedules.
- 100% reporting of conditional and distinct event coverage; no fabricated matches.
- 100% of dynamic entity types have a documented birth/lineage key before they use shared exogenous noise.
- Any unknown/ambiguous identity blocks qualification; it is not silently assigned a convenient common key.

## Current boundary

The current Philox module supports static nonzero u32 agents, global environment ID zero, canonical unordered pairs and explicit u32 occurrence ordinals. It does not implement dynamic genealogical IDs, semantic alias auditing or a model-owned sharing registry. Those are prerequisites for moving from a toy fixture to actual Symtropy simulation call sites.
