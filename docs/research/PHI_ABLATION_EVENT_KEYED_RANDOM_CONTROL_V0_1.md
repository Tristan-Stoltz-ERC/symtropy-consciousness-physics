# Scalar-control ablation with event-keyed random control v0.1

## What the prior code actually tested

The previous PHI branch returned the literal constant 0.5. The CONSTANT branch also returned 0.5, so these were identical conditions. The program did not compute equation-level IIT-inspired Phi. Keeping the row named PHI would have overstated the experiment's scope and duplicated a condition.

This version removes the duplicate placeholder and labels the programme accurately: it compares supplied scalar controls. No result from the prior source revision is rewritten or re-labelled.

## Five controls

| Control | Definition | Important property |
|---|---|---|
| ENTROPY_PROXY | Normalized binary entropy of clamped energy fraction | A scalar proxy, not full state entropy |
| RANDOM_LEGACY | Historical LCG-like formula using tick and agent index | Does not include the replicate seed; corresponding runs reuse the same noise value at each agent/tick |
| RANDOM_EVENT | Philox4x64-10 EventAddress uniform draw | Keyed by replicate seed and namespace; counter keyed by stable agent, tick, event kind, purpose and occurrence |
| CONST_0.5 | Constant 0.5 | Deterministic scalar baseline |
| ZERO | Constant 0 | No scalar modulation |

The old initial-state generator remains unchanged. Each condition receives the same seed schedule 42 + 997 * logical_seed_index, so initial positions and velocities are paired by replicate seed.

## Event-keyed address

The random event control uses:

- replicate_seed: current run seed;
- experiment_namespace: fixed PHI_ABLATION_NAMESPACE;
- subject_id: one-based stable agent identity (agent_idx + 1);
- simulation_tick: current tick;
- event kind: PHI_ABLATION_CONTROL;
- draw purpose: RANDOM_METRIC_SAMPLE;
- occurrence ordinal: zero;
- scalar draw index: zero, mapped with the generator's open-interval uniform_open01 method.

No mutable RNG stream is consumed. The same semantic event address is repeatable and independent of how many other events were evaluated.

## Software checks

The example adds tests for:
- exact repeatability at the same seed/agent/tick;
- separation when replicate seed, agent or tick changes;
- strict open-interval values;
- separation from the legacy control.

These checks validate event-address behavior, not the full simulation's scientific conclusions.

## Statistical interpretation and limits

The experiment runs 30 seed-matched replicates per condition and reports means with per-condition normal-approximation 95% intervals. It does not yet calculate paired treatment-minus-control intervals, a paired randomization test, equivalence bounds or multiplicity-adjusted p-values. Those should be added through the seed-paired inference layer before confirmatory claims.

The random event values use the already implemented Philox mapping. This experiment does not constitute TestU01 or PractRand qualification. The statistical stream export is a separate workflow.

Most importantly, this programme does not calculate the real IIT-inspired Phi equation. A future equation-level ablation must obtain Phi from the actual independently implemented equation and use preregistered matched controls. It must not relabel 0.5 as Phi.

## Historical boundary

- The legacy initial-state LCG and seed schedule remain unchanged.
- The RANDOM_LEGACY formula is preserved.
- The new RANDOM_EVENT mode is additive.
- The previous placeholder-PHI row is removed from the condition list because it exactly duplicated CONST_0.5.
- No published numerical result is updated by this code change.
