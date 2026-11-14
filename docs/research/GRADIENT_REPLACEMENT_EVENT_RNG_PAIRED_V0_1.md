# Gradient replacement: event-addressed random walk and paired inference v0.1

## Purpose

The gradient-replacement experiment compares six movement controllers under shared thermodynamics. The prior RANDOM_WALK controller consumed one mutable LCG stream inside the per-agent movement loop. That stream was independent of the other controllers' control flow only by accident, and its identity was not explicitly bound to an agent/tick event.

This revision adds an opt-in event-addressed random-walk mode and seed-keyed paired analysis without changing the legacy default.

## RNG modes

Set `SYMTROPY_GRADIENT_REPLACEMENT_RNG=event` to use event-addressed Philox. The default, including an unset variable, remains `legacy`.

### Legacy mode

- Uses the existing LCG and `rng_f64` function.
- Initial position draws are unchanged.
- The random-walk controller continues consuming the same mutable stream after initialization.
- This path is retained for historical reproduction.

### Event mode

The random walk angle is generated from one semantic event address:

- replicate key: actual simulation seed;
- experiment namespace: `0x4752_4144_5245_5031`;
- subject: one-based stable agent ID;
- tick: exact simulation tick;
- event kind: `RANDOM_WALK`;
- purpose: `RANDOM_WALK_ANGLE`;
- occurrence ordinal: zero;
- scalar draw index: zero, mapped through `uniform_open01`.

No mutable PRNG draw is consumed in event mode. A random-walk agent's angle is therefore independent of whether other agents/controllers performed unrelated draws earlier in the tick.

The initial placement LCG is deliberately unchanged. This preserves matched starting positions across controller conditions and keeps the new random-address behavior scoped to the movement variable under study.

## Paired analysis

Every controller result carries its actual seed value. The experiment now joins each controller to FEP_GRADIENT by seed key, not vector position, and emits four outcomes:

- alive agents;
- final energy;
- final clustering;
- cooperation events.

The estimand is controller minus FEP_GRADIENT. Receipts report paired mean and median difference, sample SD, SE, Cohen's d_z, and a deterministic 95% percentile bootstrap interval using 20,000 resamples of complete seed pairs.

Agents and ticks are nested observations; the replicate seed remains the independent unit. The current seed count is 20 and is exploratory, below the proposed 30-pair minimum for confirmatory build/statistical claims.

No p-value is emitted. Pairing by seed does not establish the sign-exchangeability needed for a sign-flip test, and no randomized controller-assignment mechanism is claimed. No equivalence test is emitted because no outcome-specific margin has been preregistered.

## Synthetic receipt smoke test

The `--receipt-smoke` option creates deterministic synthetic outputs for all six controllers and emits the same 20 paired JSON receipts (five comparator controllers × four outcomes) without running the 8,000-tick simulation.

CI parses the receipts, checks exact count, schema, seed count, bootstrap parameters, RNG mode, and the not-evaluated status of p-values/equivalence. This validates the output path independently from a long physical-model run.

## Qualification boundaries

- Existing runs remain on the legacy LCG path unless the environment variable is explicitly set to `event`.
- No prior results are re-labeled or overwritten.
- The event-addressed mode is a controller-specific randomness change; it does not assert that controller outcomes are physically or scientifically valid.
- TestU01/PractRand remain unrun for this Rust generator.
- No p-values, equivalence verdicts, or SOTA claims are made.
- The 20 replicate seeds are exploratory; confirmatory analysis needs a preregistered design and at least 30 matched seed replicates (with more if power calculations require).
