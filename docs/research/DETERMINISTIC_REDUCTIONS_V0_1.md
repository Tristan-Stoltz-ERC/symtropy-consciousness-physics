# Deterministic floating-point reductions v0.1

## Finding

The ConsciousnessField keeps entity and sanctuary state in HashMaps. Two operations consume those maps in iteration order:

1. recompute_collective sums each entity's Phi value and stores collective_phi. The cooperation-decomposition experiment feeds this value back into every agent's synchrony input on the next tick.
2. PhysicsCallback::modulate_impulse multiplies sanctuary impulse factors. Floating-point multiplication is not associative, so changing iteration order can change the last bits of a collision impulse.

The first operation is a particularly credible source of trajectory divergence: a one-bit change in a value that feeds back into an iterative simulation can change later branches and produce materially different long-run output. This is a source-level mechanism, not yet a proven attribution for the existing benchmark failure.

## Candidate implementation

The deterministic-reductions Cargo feature is opt-in and has no default activation.

When enabled:
- entities are ordered by stable BodyHandle before collective reduction;
- the collective Phi total uses compensated summation;
- sanctuaries are ordered by stable BodyHandle before their impulse multipliers are applied.

When the feature is disabled, the historical HashMap traversal is retained. This avoids silently changing previously published experiment runs or verdicts.

The feature is a determinism candidate, not automatically a production default. It adds sorting and temporary allocations to the relevant operations; that overhead must be measured separately.

## Qualification protocol

The dedicated workflow:
- runs unit tests both with and without the feature;
- runs the adversarial registration-order regression tests with the feature;
- builds cooperation_decomposition once with the feature enabled;
- executes the exact same binary five times in fresh processes;
- captures stdout and stderr separately;
- requires byte-identical stdout across all five executions;
- uploads raw outputs and checksums even when the determinism assertion fails.

This establishes same-host, same-binary process repeatability for this one executable. It does not prove cross-architecture bitwise reproducibility, physical model validity, or compiler-wide determinism.

## Follow-up measurement

Compare default and feature-enabled builds across the same compiler, source, lockfile, host, seed corpus, and target layout. Report:
- exact output equality (only where the behavior contract requires it);
- simulation runtime p50/p95/p99;
- total runtime overhead of sorting and compensation;
- allocation count and peak RSS;
- per-tick collective_phi drift;
- compiler and target fingerprints.

A deterministic mode should be promoted only if output repeatability is established and its performance cost is acceptable or explicitly justified.
