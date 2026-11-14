# Event-keyed randomness for counterfactual simulation v0.1

**Status:** research specification only. No simulator RNG has been changed.

## 1. Problem and scientific motivation

A shared mutable PRNG seed produces the same sequence of random numbers, but not necessarily the same random inputs for the same modeled events when two controller/intervention variants execute different paths. An extra draw in one branch shifts later draws in that scenario. The result can be a misleading common-random-number (CRN) pairing even though the initial seed is equal.

A recent treatment of this problem frames it as execution invariance: a modeled stochastic event that exists in both counterfactual scenarios must receive the same exogenous random value in both, independent of the order in which the program reaches it. It also makes an important distinction: event identity is part of the scientific model, not an implementation detail that a random-number library can infer.

This RFC proposes a separately versioned event-keyed RNG mode. The historical LCG and its outputs remain the reference baseline.

## 2. Design principles

1. **Randomness is addressed, not consumed.** A random value is a pure function of a root replicate seed and a canonical event/draw identity.
2. **Scenario-independent keys.** Controller, intervention, treatment label, branch path, thread ID, loop position, wall-clock time, and mutable population summaries must not enter the event key.
3. **Stable modeled identity.** Entity/event IDs must be stable across counterfactual worlds. Mutable global birth counters and execution-order indices are prohibited.
4. **Fine enough to preserve independence.** Event kind, purpose, tick, subject and deterministic occurrence must distinguish events that the model considers independent.
5. **Explicit multi-draw semantics.** Each draw within an event has an explicit index. Rejection attempts and variable-length samplers use event-local draw indices rather than consuming a global stream.
6. **No silent migration.** Historical stateful streams and published outputs remain immutable; event-keyed mode is a new experiment condition and RNG schema.
7. **Determinism is not cryptographic security.** Philox is used for reproducible simulation, not secrets, tokens, authentication, or adversarial randomness.

## 3. Proposed v1 identity mapping

Use the counter-based **Philox4x64-10** construction as the initial candidate. It accepts a 256-bit counter (four u64 words) and a 128-bit key (two u64 words), and returns four u64 words.

### Key

- key[0] = replicate_seed, preserving the exact existing seed schedule (currently 42 + 997 * logical_index).
- key[1] = experiment_namespace, a fixed, reviewed u64 that is constant across controller/intervention variants intended to share exogenous noise.

The scenario/condition name is deliberately excluded. A distinct experiment namespace represents a deliberately independent random world, not a different treatment arm.

### Counter

- counter[0] = subject_id
- counter[1] = simulation_tick
- counter[2] = packed_event_code
- counter[3] = block_index

The packed event code is a fixed-width schema:

- bits 63..48: event-kind tag (u16)
- bits 47..32: purpose tag (u16)
- bits 31..0: deterministic occurrence ordinal (u32)

Event-kind and purpose tags are explicit numeric constants in a versioned registry; do not hash ad-hoc strings or use enum declaration order as a persistent code. Exhaustion of a field fails closed and requires an explicit schema version change.

### Subject identity

The simulator assigns stable u32 agent IDs at model creation, independently of physics-engine handles or iteration order.

- Agent-local events use the agent ID widened to u64.
- Environment-global events reserve subject ID 0.
- Pair events encode the ordered pair of u32 agent IDs into one u64: (min_id << 32) | max_id.
- Dynamic entities require a stable genealogical ID derived from a stable parent identity and a deterministic within-parent birth ordinal; a process-global creation counter is not valid across branches.

The model owner must define what constitutes the same subject/event across counterfactual worlds. If a future model needs identities wider than this schema, introduce a new event-key schema rather than truncating or silently hashing IDs.

### Multiple draws for one event

Philox emits four u64 words for each counter block. For logical draw_index:

- block_index = draw_index / 4
- lane = draw_index % 4
- evaluate Philox once for the key and counter above, then select the specified lane.

A repeated query of the same event and draw index returns the same value. Call sites should normally sample once and retain the result rather than repeatedly querying the same semantic event.

For f64 open-interval uniforms, use a documented conversion from the top 53 bits, e.g. (top53 + 0.5) * 2^-53; never rely on platform-specific float casts from wider integer ranges. Distribution samplers must state how they allocate event-local draw indices. Rejection sampling increments an attempt/draw index within that event and cannot shift any other event.

## 4. First Symtropy application candidates

The first prototype should be deliberately narrow:

1. **Initialization:** agent.initial_position.x and .y receive distinct purpose tags for each stable agent ID. All controller variants in a replicate therefore start from the same exogenous initial conditions.
2. **Random-walk controller:** the angle at (agent_id, tick, random_walk, angle) receives its own key. A random-walk branch cannot perturb later agents' randomness.
3. **Environment stochasticity:** add only when the model has an actual stochastic environmental event and its event semantics are specified.
4. **Pair/inter-agent events:** add only after canonical subject identity and event occurrence semantics have adversarial coverage.

The current J/Phi and cooperation comparisons reportedly reuse initial positions, with no later random draws found; gradient replacement additionally has RandomWalk angle draws. Those observations motivate this staged rollout, but the repository call sites must be re-audited at implementation time.

## 5. Execution-invariance and scientific contract

For a base seed and a declared set of interventions, if event e is modeled as the same event in two scenarios, then its exogenous value must be identical:

U(e, seed, experiment_namespace, schema)

must not depend on the event execution order or on unrelated events that occurred earlier.

An event that only exists in one scenario may be queried only in that scenario; its absence must not shift other keys. Do not manufacture a counterpart event where the model says none exists.

Event identity must not contain endogenous values changed by the intervention (for example, current population size, treatment-dependent incidence, or an intervention-dependent state hash). Those values belong in the structural mechanism that transforms U(e) into an outcome, not in the identity of U(e) itself.

## 6. Adversarial qualification plan

### Correctness and invariance

- Same key/counter returns bit-identical words on repeated calls.
- Reordering event execution leaves every event's output unchanged.
- Insert/delete arbitrary draws in an unrelated controller branch; matched events retain the same outputs.
- Add a placebo/no-op path that consumes extra draws; all pre-existing event values remain unchanged.
- Execute the same event set serially and with several thread schedules; compare per-event outputs and final deterministic summaries.
- Detect duplicate semantic keys in debug/test instrumentation.
- Reject invalid IDs, tag overflow, occurrence overflow and unsupported schema versions.

### Counterfactual alignment

- For each scenario pair, report the set of matched event IDs, unmatched conditional events, and exact exogenous-value agreement among matched events.
- Test known toy structural causal models with analytically known treatment effects.
- Check monotonicity properties where the model supplies a monotonic intervention invariant.
- Keep independent-seed and legacy stateful runs as comparators; event-keying is not proof that the structural model itself is correct.

### Distribution quality

- Verify implementation against published Philox test vectors.
- Run TestU01/BigCrush and/or PractRand on raw output streams and the exact uniform conversion; record versions and input digests.
- Test each distribution transform separately, including tail behaviour and rejection-sampler edge cases.
- These tests qualify the generator/mapping, not the scientific event model.

### Performance and parallelism

- Measure raw words/second, event-key construction cost, end-to-end simulation throughput, p50/p95/p99 latency where applicable, and memory footprint.
- Compare stateful LCG, event-keyed Philox and (as a separate candidate) Threefry under identical event counts and hardware.
- Use at least 30 randomized paired A/B blocks for confirmatory throughput claims; pin compiler, flags, source, CPU governor and affinity.
- Report the complete overhead as well as the RNG kernel alone. Do not hide canonical event-key construction costs.
- Measure worker-count scaling and serial/parallel determinism independently.

## 7. Metrics and promotion gates

| Dimension | Required evidence |
|---|---|
| Execution invariance | 100% identical values for all matched event-key tuples across tested orderings/branches |
| Key integrity | Zero duplicate semantic keys in qualified fixtures; explicit collision/overflow negative controls |
| Reproducibility | Bit-identical outputs for fixed key, counter and algorithm version |
| Counterfactual alignment | Explicit matched/unmatched event accounting; no claim based solely on equal base seeds |
| Distribution | Published reference vectors plus independent statistical battery |
| Scientific validity | Analytically known toy SCMs pass; model-specific identity assumptions documented |
| Runtime | Measured end-to-end overhead and parallel scaling; no universal speedup target |
| Compatibility | Historical LCG mode and published output hashes unchanged |

An execution-invariance pass is a necessary property of the RNG mapping, not sufficient evidence that a counterfactual simulation is scientifically valid.

## 8. Receipt schema

Every run should record:

- algorithm identifier and round count (philox4x64-10/v1)
- event-key schema and tag-registry version
- exact compiler and target
- replicate seed and experiment namespace
- source, lockfile and build-input digests
- event counts by type/purpose and matched/unmatched counts
- duplicate-key and overflow counters
- output digest and execution mode (serial/parallel)

No receipt should include a treatment label in the event key itself.

## 9. Nonclaims and migration policy

- This RFC is not an implementation and reports no RNG benchmark or statistical pass.
- Philox output is not cryptographically secure.
- Stable event keys do not automatically establish causal validity; event matching is a model assumption.
- Shared randomness does not guarantee lower variance for every estimand.
- The current LCG, seed schedule, experiment CSVs, and published conclusions remain unchanged.
- Adoption requires a separate implementation PR, independent review of event semantics, reference-vector tests, statistical qualification, and matched performance results.

## Related work

- Buffalo, Pearson and Klein, *Realizing Common Random Numbers: Event-Keyed Hashing for Causally Valid Stochastic Models* (2026), arXiv:2603.11084: https://arxiv.org/abs/2603.11084
- Salmon, Moraes, Dror and Shaw, *Parallel Random Numbers: As Easy as 1, 2, 3* (2011), which introduced Random123 counter-based generators: https://www.thesalmons.org/john/random123/papers/random123sc11.pdf

The first reference motivates execution-invariant event addressing for counterfactual comparisons. The second is the underlying counter-based RNG family reference; it does not establish the simulator's event-identity assumptions or scientific validity.
