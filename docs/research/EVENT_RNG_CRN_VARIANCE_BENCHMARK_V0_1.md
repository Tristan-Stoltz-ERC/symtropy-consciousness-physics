# Event-keyed CRN variance benchmark v0.1

## Purpose

This benchmark tests a scientific consequence of event-addressed randomness, not merely generator repeatability: whether matched exogenous event identities change the variance of a counterfactual treatment-effect estimator.

It uses a known structural causal model and reports both a positive-covariance case and a negative-covariance counterexample. The counterexample is important: common random numbers (CRN) do not always reduce variance.

## Structural causal model

Let \(U \sim \operatorname{Uniform}(0,1)\).

\[
Y_0 = U^2
\]
\[
Y_1 = U^2 + 0.25 + \beta U
\]

The exact average treatment effect is:

\[
ATE = E[Y_1-Y_0] = 0.25 + \frac{\beta}{2}
\]

The paired CRN estimator uses the same event address for control and treatment. The independent-draw estimator uses a separate event namespace for the treatment-side exogenous value.

## Two preregistered cases

### Positive covariance: \(\beta=0.5\)

The exact effect is 0.5. The matched per-subject difference is \(0.25 + 0.5U\), with variance \(0.5^2/12\). Independent draws add the variation of the baseline and treatment outcomes.

The expected theoretical matched/independent estimator-variance ratio is approximately 0.074. The executable uses a deliberately looser software gate of less than 0.20 to avoid confusing a fixture with a precision benchmark.

### Negative covariance control: \(\beta=-1.5\)

The exact effect is -0.5. For this case the covariance between the control and treatment potential outcomes is negative. Matched CRN is expected to *increase* estimator variance, with theoretical matched/independent ratio greater than 1.

This control prevents an invalid universal claim that event matching is always statistically beneficial.

## Design

- 2,048 stable subject IDs per independent replicate.
- 512 independent outer replicate seeds.
- 2,000 deterministic bootstrap resamples over outer replicate estimates.
- Matched and independent estimators are computed for each outer replicate.
- A 95% percentile bootstrap interval is reported for the ratio of sample variances.
- The executable checks that both estimators recover each analytical ATE within 0.01.
- The positive case must have empirical matched/independent variance ratio below 0.20.
- The negative control must have empirical ratio above 1.20.

The bootstrap RNG is analysis-only resampling machinery; it does not feed the structural model.

## Interpretation and limits

This is a deterministic software qualification experiment for the event-address mapping and a specific CRN estimator. It is not a result from a physical, social, or historical Symtropy experiment. It does not validate event semantics in any real simulation and does not compare Philox statistical quality against TestU01/BigCrush/PractRand.

The correct research conclusion is conditional: event identity matching can reduce estimator variance when common exogenous inputs induce positive covariance, and can increase it when covariance is negative. Therefore every application should report matched-event coverage, unmatched/conditional event counts, and paired-versus-independent estimator variance rather than assuming CRN is beneficial.
