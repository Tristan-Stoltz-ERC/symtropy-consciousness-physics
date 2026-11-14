# Seed-keyed paired inference contract v0.1

## Status

This is an additive statistical implementation and protocol. It does not rewrite historical results, replace the existing unpaired summaries, or change any experiment verdict. The new functions are available for a separately reviewed analysis using raw, seed-keyed outputs.

## Independent unit and estimand

- Independent unit: one complete simulation replicate identified by its actual seed value.
- Pairing key: seed value, never vector position or CSV row order.
- Estimand: treatment minus control for each matched seed.
- Reject empty inputs, duplicate keys, missing/extra keys, and non-finite observations.
- Sort matched pairs by seed key before deterministic analysis.
- Agents, ticks, events and intra-run observations are nested measurements, not additional independent replicates.

## Inference contract

1. Report n, mean and median paired difference, sample standard deviation, standard error, and paired Cohen's d_z when defined.
2. Use a two-sided sign-flip test. Enumerate every 2^n sign assignment for n <= 20; above that use deterministic Monte Carlo with a recorded seed and plus-one p-value correction.
3. The sign-flip test is exact under within-pair randomized assignment or an appropriate sign-exchangeability/symmetry null. Matching seed IDs alone does not establish this assumption. Record the design rationale with each claim.
4. Bootstrap whole seed-level paired differences. The v0.1 implementation reports a percentile interval for the mean; record confidence level, replicate count, and PRNG seed. Never resample agents or ticks as independent replicates.
5. Equivalence is a separate interval-containment screen. The absolute margin must be scientifically justified and preregistered before examining the result. Only an interval wholly inside (-margin, +margin) yields equivalent_within_margin; otherwise the outcome is not_established. Not established is not evidence of difference. This is not advertised as a TOST p-value.
6. Apply Holm family-wise correction to a predeclared comparison family. Adjusted p-values must be monotonic in sorted raw-p order. Preserve exact family membership in the receipt.

## Adversarial coverage

Tests exercise order invariance, duplicate/missing keys, empty and non-finite samples, exact small-sample enumeration, deterministic Monte Carlo, deterministic bootstrap, invalid margins and confidence levels, and monotone Holm adjusted p-values.

## Explicit nonclaims

- No historical result, label, CSV, or verdict has been recomputed or changed.
- No random-stream implementation or simulation physics is changed.
- Statistical non-significance is not an equivalence result.
- A paired sign-flip p-value is not automatically exact merely because samples share seeds.
- Bootstrap resampling does not turn a small number of independent seeds into a large sample.
- Passing software tests is not scientific validation, SOTA performance, or external validity.

## Next qualification steps

1. Compile/test on exact Rust 1.99.1 and 1.100.0.
2. Review sign-flip exchangeability for each experiment design.
3. Preregister metric-specific equivalence margins and comparison families.
4. Export a new analysis artifact alongside, not over, the historical report.
5. Independently reproduce the analysis from raw seed-keyed CSVs.


## Existing-output analyzer

The opt-in cooperation_paired_analysis example reads the existing
cooperation_decomposition CSV and emits a separate JSON analysis receipt.
It selects two condition labels and one metric, pairs by the explicit seed
column, then reports paired summaries, a sign-flip result, and a deterministic
percentile interval. It never changes or rewrites the simulation's historical
CSV or terminal verdict.

Example:

    cargo run --locked --release --example cooperation_decomposition > run.csv
    cargo run --locked --release --example cooperation_paired_analysis -- \
      --input run.csv \
      --control WELL+NO_REGEN \
      --treatment WELL+REGEN \
      --metric alive \
      --input-sha256 "$(sha256sum run.csv | cut -d ' ' -f 1)" \
      --bootstrap-replicates 20000 \
      --bootstrap-seed 0x5041495245440001 \
      --equivalence-margin 1.0 \
      --margin-rationale "pre-registered domain-specific survival margin"

The optional input-sha256 field should be populated from sha256sum and kept
with the original CSV. The analyzer validates its 64-character hexadecimal
shape but does not recompute it; the evidence wrapper must verify the digest.

The example-output CSV is deliberately parsed as unquoted comma-separated
values, matching the current producer. It rejects quoted cells rather than
silently misparsing an expanded CSV format. The equivalence option requires a
rationale string, but the CLI cannot prove that rationale was preregistered;
the investigator must preserve the actual preregistration record separately.

The synthetic CI fixture checks schema, seed identity, paired mean, exact
small-sample sign-flip path, and interval-containment status. It is a software
contract test, not a scientific result.
