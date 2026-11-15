# Coverage-gated CRN selector v0.1

## Research question

Can a coupling that has much lower true estimator variance still produce unreliable finite-sample confidence intervals?

Yes. This fixture constructs a simple event-addressed counterexample and adds a coverage constraint to pilot-based coupling selection.

The central change is from:

> Select the coupling with the smallest observed variance.

to:

> First require calibrated inferential coverage on independent training studies; then minimize variance among eligible couplings. Freeze the policy before touching holdout studies.

## Exact data-generating mechanism

Let U,V be independent Uniform(0,1) random variables. The control draw is U.

The independent treatment draw is U', also Uniform(0,1), independent of U. Its paired contrast is:

\[
\Delta_{\mathrm{ind}}=U'-U.
\]

The cyclic-shift treatment draw is:

\[
T_\epsilon=(U+\epsilon V)\bmod 1,\qquad 0<\epsilon\leq 1.
\]

For every fixed V, adding the shift \(\epsilon V\) modulo one maps a uniform U to another uniform variable. Thus T_epsilon has the same Uniform(0,1) marginal as U, while its dependence on U is different. Its paired contrast is:

\[
\Delta_\epsilon=T_\epsilon-U.
\]

Both policies estimate the same zero estimand:

\[
E[\Delta_{\mathrm{ind}}]=E[\Delta_\epsilon]=0.
\]

The exact variances are:

\[
\operatorname{Var}(\Delta_{\mathrm{ind}})=\frac{1}{6}
\]

and

\[
\operatorname{Var}(\Delta_\epsilon)
=\frac{\epsilon}{2}-\frac{\epsilon^2}{3}.
\]

For epsilon = 0.01, the cyclic shift has only about 2.98% of the independent policy's true variance. That sounds excellent until finite-sample interval coverage is examined.

The cyclic shift produces many small positive differences and rare large negative wrap-around differences. The population mean remains zero, but the single-observation distribution is highly skewed. A normal interval based on the sample standard deviation can severely undercover when the sample is too small to represent the rare wrap events.

## Preregistered study design

- 5,000 independent training meta-studies.
- 5,000 disjoint holdout meta-studies.
- 200 observations per meta-study.
- Event-addressed Philox streams for control U, independent treatment U', and jitter V.
- Nominal 95% normal intervals using the fixed z critical value 1.959963984540054.
- A 95% Wilson interval for each empirical coverage rate.
- Candidate epsilon values 0.01, 0.025, 0.1, and 0.25, plus the independent baseline.

The training studies select the policy. Holdout studies are not read by the selector and are used once for confirmation.

## Two selectors

### Variance-only selector

Chooses the candidate with the smallest average within-study sample variance. It is expected to choose epsilon = 0.01.

### Coverage-gated selector

A candidate is eligible only when the lower bound of its training 95% Wilson interval for 95% interval coverage is above 93%. Among eligible candidates, it chooses the lowest average within-study sample variance.

The 93% floor is a synthetic engineering tolerance: two percentage points below the nominal coverage. It is not a universal scientific standard and must be replaced with a domain-justified tolerance in real experiments.

## Why this is important

Variance reduction is not equivalent to improved inference. A policy can lower mean squared error asymptotically while making a commonly used finite-sample interval unreliable at the actual sample size.

This creates an important new CRN benchmark dimension:

- point-estimator variance;
- confidence-interval coverage;
- interval width;
- estimator bias;
- tail shape and rare-event frequency;
- selection uncertainty;
- holdout performance.

A coupling policy should not be selected solely because its sample variance is smallest.

## Machine-readable receipts

For each policy and partition, the benchmark emits:
- empirical interval coverage;
- Wilson 95% coverage interval;
- average within-study sample variance;
- exact theoretical variance;
- empirical variance ratio against the independent policy;
- mean interval width;
- absolute bias of the mean study estimate.

The final selection receipt reports the variance-only choice, coverage-gated choice, and disjoint holdout outcomes.

## Qualification boundaries

This fixture establishes a known-truth counterexample for the specified normal interval and sample size. It does not establish that normal intervals are always inappropriate, that cyclic shifts are useful in real Symtropy models, or that the 93% gate should be adopted universally.

It does not evaluate a Student-t interval, bootstrap interval, sequential interval, or robust skewness-aware interval. Those are next candidates for the same preregistered study design.

## Proposed SOTA comparison programme

The next benchmark should compare, on exactly matched event addresses and study partitions:

1. independent draws;
2. point-estimate CRN selection;
3. uncertainty-bounded CRN selection;
4. coverage-gated selection;
5. antithetic coupling when a valid measure-preserving transform exists;
6. a constrained joint portfolio;
7. an analytic oracle for known-truth fixtures only.

For each method, report coverage at nominal 90%, 95%, and 99%; interval width; absolute bias; RMSE; variance; type-I error; false-selection rate; abstention rate; and wall-clock cost.

Run sample sizes 50, 200, 1,000 and 10,000, and at least 10,000 meta-studies for confirmatory coverage claims when the Monte Carlo precision target requires it. Report Monte Carlo standard errors for the measured coverage, not only the point estimate.

A future policy objective should be constrained and multi-objective:

\[
\min_{\pi\in\mathcal A}
\quad \operatorname{Var}(\widehat\Delta_\pi)
\quad\text{subject to}\quad
\operatorname{LCB}_{95\%}(\operatorname{Coverage}_\pi)\geq c_{\min},
\]

where \(\mathcal A\) contains only couplings with validated marginal and semantic admissibility. A separate extension can optimize work-normalized variance after measuring actual execution cost.

The SOTA target is not merely lower variance. It is lower work-normalized error **without sacrificing calibration, semantic validity, or reproducibility**.


## Exact-head observations

The qualified run used the same deterministic event-address schedule on Rust 1.99.1, 1.100.0 stable and 1.101.0-beta.1. All three produced byte-identical machine-readable receipts.

| Holdout selector | Selected policy | 95% interval coverage | 95% Wilson interval | Variance ratio vs independent |
|---|---|---:|---:|---:|
| Variance-only | cyclic shift epsilon = 0.01 | 64.76% | [63.42%, 66.07%] | 0.0306 |
| Coverage-gated | cyclic shift epsilon = 0.25 | 94.28% | [93.60%, 94.89%] | 0.6262 |

The independent baseline's holdout coverage was 94.42%, with a Wilson interval [93.75%, 95.02%]. Its empirical variance is the denominator for both variance ratios.

The variance-only policy's 3.06% variance ratio looks spectacular but its interval coverage is unacceptable for the nominal 95% procedure. The coverage-gated selector accepts a more moderate variance reduction while preserving the preregistered coverage floor on disjoint holdout studies.

These results apply to this exact cyclic-shift distribution and normal interval at n=200. They do not establish that 93% is an appropriate coverage floor for real Symtropy work, nor that normal intervals should be the production estimator. Student-t, bootstrap, and skewness-aware procedures remain to be compared.
