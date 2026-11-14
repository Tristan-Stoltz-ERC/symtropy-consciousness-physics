# Paired scalar-ablation receipts v0.1

## Purpose

The scalar ablation runs each condition against the same 30 actual seed values. This is a matched design. Per-condition confidence intervals alone do not directly summarize the treatment-minus-control contrast, so this adds a separate paired analysis receipt.

## Preregistered contrasts

The fixed contrasts are:

1. RANDOM_LEGACY → RANDOM_EVENT (primary RNG-control comparison);
2. CONST_0.5 → RANDOM_EVENT;
3. CONST_0.5 → ENTROPY_PROXY.

Each contrast is evaluated for three outcomes: clustering, alive-agent count, and average final energy. That defines nine descriptive paired estimates. The comparison set is fixed in source before results are examined.

## Pairing and estimand

- Independent unit: one complete simulation seed replicate.
- Pairing key: the actual u64 seed value, not array position.
- Estimand: treatment minus control.
- The implementation rejects empty, duplicate, missing, non-finite or mismatched seed keys through the shared paired-inference module.
- All 30 seed IDs are included in every receipt.

Agents and ticks are nested observations. They do not increase n.

## Receipt contents

Each PAIRED_ANALYSIS_JSON line reports condition names and metric, the complete seed ID vector and n, paired mean and median difference, sample standard deviation, standard error and Cohen's d_z when defined, plus a deterministic 95% percentile bootstrap interval over whole seed-level paired differences.

The bootstrap uses 20,000 resamples and a unique fixed seed for each contrast/outcome pair. Whole seed pairs—not agents or ticks—are resampled.

## Why no p-value is emitted

This is a deterministic model comparison over seed-sampled initial conditions, not a randomized treatment-assignment experiment. Pairing by seed makes the contrasts comparable; it does not by itself establish the sign-exchangeability assumption required by the sign-flip procedure. The receipt therefore records p-values as not evaluated. A future analysis may add inferential p-values only after documenting a valid null/randomization argument.

Likewise, no equivalence margin has been preregistered. Equivalence is not evaluated, and no non-significance/equivalence language is used.

## Limits

- This is an exploratory model experiment, not confirmation of physical or social causal validity.
- The event RNG stream has not yet passed TestU01 or PractRand.
- The entropy condition is an energy-fraction proxy.
- No equation-level IIT-inspired Phi is measured by this harness.
- The previous PHI=0.5 placeholder was removed because it duplicated the constant condition.

## Reproduction

Run the example at the exact project toolchain with the locked dependency graph. Preserve stdout as the raw analysis artifact and bind it to the source commit, seed schedule, toolchain passport, and input configuration. The JSON lines can be extracted without reformatting the original descriptive table.
