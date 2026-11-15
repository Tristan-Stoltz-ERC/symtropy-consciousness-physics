# Uncertainty-aware event-family CRN guard v0.1

## Objective

Add an explicit uncertainty and abstention layer to the point-estimate coupling selectors in PRs #34 and #35. The selector operates independently for each semantic event family.

For family j, estimate the training covariance:

\[
\widehat C_j = \operatorname{Cov}(Y_{0,j},Y_{1,j}).
\]

The synthetic analytic model uses U ~ Uniform(0,1), Y0,j = U², and Y1,j = U² + beta_j U, so

\[
C_j = \operatorname{Var}(U^2)+\beta_j\operatorname{Cov}(U^2,U)
    = \frac{4}{45}+\frac{\beta_j}{12}.
\]

## Frozen decision rule

The synthetic benchmark preregisters a covariance deadband delta = 0.001, in the model's covariance units, and a 99% percentile cluster-bootstrap interval [L_j, U_j].

- SHARED only when L_j > delta.
- INDEPENDENT when U_j < -delta.
- ABSTAIN_TO_INDEPENDENT otherwise.

The deadband is an action threshold for this synthetic benchmark. It is not an equivalence margin, does not establish equivalence, and must not be copied to a physical/social model without a domain-based justification. Abstention is explicit in the receipt even though the operational fallback is independent randomness.

The point-estimate selector (share when the estimated covariance mean is positive) and analytic oracle (share when the known model covariance is positive) are retained as comparators.

## Statistical unit and leakage control

Each training outer replicate produces one covariance estimate per event family. The 99% bootstrap resamples those 256 replicate-level estimates as clusters. The 2,048 subjects inside one replicate are not counted as 2,048 independent outer replicates.

Training seeds occupy indices [0, 256). Holdout seeds occupy [256, 512). The selector is frozen after training and before any holdout outcome is computed. Holdout variance-ratio intervals resample whole paired outer-replicate estimates.

## Benchmark conditions

Four fixed two-family cases cover:
- strong positive / strong negative covariance;
- the mirrored family ordering;
- a small positive covariance near the predeclared deadband;
- the analytic zero-covariance boundary beta = -16/15, paired with a strongly positive family.

The machine-readable output compares all-independent, all-shared, point-estimate per-family, uncertainty-guarded per-family, and analytic-oracle policies. It records 99% training covariance intervals, decisions, held-out ATE estimates, variance ratios to all-independent, and a 95% paired bootstrap interval for the guarded variance ratio.

## What this demonstrates

This is a software/method benchmark with a known analytic estimand. It tests deterministic event addressing, replicate-level uncertainty estimation, explicit abstention, disjoint training/holdout, and reproducible machine-readable receipts.

It does not establish:
- causal correctness of event identities in a real Symtropy model;
- universal superiority of guarded CRN;
- calibrated 99% interval coverage across arbitrary data-generating processes;
- RNG statistical-battery qualification;
- physical, social, or consciousness-model validity;
- SOTA performance.

## Next confirmatory benchmark targets

Before treating the guard as a production selector, add a preregistered calibration tournament over at least 1,000 independent meta-replicated training studies per scenario. Measure:

1. 99% interval coverage and false-share rate at zero and negative covariance;
2. false-independent rate at clearly positive covariance;
3. abstention rate and selection regret against the analytic oracle;
4. held-out variance ratios and confidence-interval coverage under skewed, heavy-tailed, heteroskedastic, autocorrelated, and nonstationary cluster structures;
5. sensitivity to cluster count, subjects per cluster, and unequal cluster sizes.

Targets for the synthetic calibration suite are at least 98% empirical coverage for nominal 99% intervals (with Monte Carlo uncertainty reported), no more than 1% false-share decisions under non-positive covariance, and no more than 5% false-independent decisions when the true covariance is at least 3 delta. These are proposed gates, not current results.

For the real application suite, compare:
- all-independent and all-shared CRN;
- global sign selector;
- per-event point selector;
- per-event uncertainty guard;
- antithetic and control-variate couplings where the model admits them;
- oracle policy as a lower-bound comparator, never as an attainable production baseline.

Report work-normalized variance reduction (variance times wall time), overhead per event, p50/p95/p99 runtime, memory, and independent seed-level replications. A coupling policy that reduces variance but costs more than it saves should not be called an efficiency improvement.

## Research context

Event-addressed random inputs remove call-order dependence from common-random-number counterfactual comparisons. Classical CRN theory also makes clear that the sign and magnitude of induced covariance determine whether sharing helps. Recent work on joint antithetic/control-variate covariance design suggests a broader future coupling portfolio, but any such policy must be selected on training clusters and evaluated on disjoint holdout clusters.

References:
- Buffalo, Pearson & Klein (2026), Realizing Common Random Numbers: Event-Keyed Hashing for Causally Valid Stochastic Models, https://arxiv.org/abs/2603.11084
- Kleijnen (1975), Antithetic Variates, Common Random Numbers and Optimal Computer Time Allocation in Simulation, https://doi.org/10.1287/mnsc.21.10.1176
- Kawai (2026), Controlling antithetic variates, https://doi.org/10.1016/j.ejor.2025.08.027
- Morris et al. (2019), Using simulation studies to evaluate statistical methods, https://doi.org/10.1002/sim.8086
- MacKinnon, Nielsen & Webb (2023), Cluster-robust inference: A guide to empirical practice, https://doi.org/10.1016/j.jeconom.2022.04.001
