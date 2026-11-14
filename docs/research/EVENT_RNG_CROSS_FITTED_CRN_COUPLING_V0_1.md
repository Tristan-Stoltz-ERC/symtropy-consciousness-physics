# Cross-fitted CRN coupling selection v0.1

## Research question

Common random numbers (CRN) are not uniformly beneficial. For a counterfactual effect estimator,

Var(Y1 - Y0) = Var(Y1) + Var(Y0) - 2 Cov(Y0, Y1).

Sharing event-addressed exogenous inputs can reduce estimator variance when the induced covariance is positive, but can increase it when covariance is negative. The existing fixed-coupling fixture demonstrates both cases. This experiment asks a further question: can a coupling policy be selected using a disjoint training seed set and then evaluated on held-out seeds?

## Cross-fitted policy

The policy is preregistered:

1. Use the training seed partition to estimate Cov(Y0(U), Y1(U)) under the shared event-address mapping.
2. Select SHARED when the estimated covariance is positive.
3. Select INDEPENDENT when the estimate is zero or negative.
4. Freeze the policy.
5. Evaluate shared, independent, and selected estimators only on a disjoint holdout seed partition.

No holdout outcome or variance is used to select the policy. This avoids choosing the coupling after seeing the same samples used to claim a variance reduction.

## Structural causal model

U is generated from stable Philox event addresses. The model is:

- Y0 = U²
- Y1 = U² + 0.25 + βU
- ATE = 0.25 + β/2
- Cov(Y0, Y1) = 4/45 + β/12

Three preregistered values exercise different covariance regimes:

| β | Exact ATE | Theoretical covariance | Expected policy | Purpose |
|---:|---:|---:|---|---|
| 0.5 | 0.5 | positive | SHARED | Strong positive covariance |
| -1.0 | -0.25 | weak positive | SHARED | Near-boundary positive covariance |
| -1.5 | -0.5 | negative | INDEPENDENT | CRN-harm counterexample |

## Split and address identity

- Training: logical replicate indices [0, 256).
- Holdout: logical replicate indices [256, 512).
- Subjects per replicate: 2,048.
- Replicate seeds derive from a fixed base and stride.
- Shared control/treatment potential outcomes use the same EventAddress: same replicate, namespace, subject, tick, event kind, purpose and occurrence.
- Independent treatment draws use a different experiment namespace; the control retains the common namespace.
- The bootstrap resamples whole holdout replicate estimates, keeping shared and independent estimates paired.

## Outputs and software gates

Each CROSSFIT_JSON record includes training covariance, theoretical covariance, selected policy, held-out mean effects, estimator variances, shared/independent and selected/independent variance ratios, and 95% percentile bootstrap intervals over 2,000 whole-replicate resamples.

The executable fails unless:
- the training-selected policy agrees with the preregistered theoretical covariance sign for each fixture;
- shared and independent held-out means recover the exact ATE within 0.01;
- the selected policy's held-out variance ratio is below 1.10;
- the negative-covariance fixture demonstrates shared CRN inflation above 1.20 while the selector chooses INDEPENDENT.

These are synthetic software-fixture gates, not empirical claims about Symtropy.

## Why the holdout matters

A policy that chooses shared coupling only after observing the complete dataset can look better because it has optimized to the noise in the evaluation sample. Splitting seed identities makes selection independent of evaluation. The holdout variance ratio is therefore a more honest test of whether the estimated covariance sign predicts coupling utility.

The current fixture evaluates three known structural regimes. It does not yet establish a universal policy for unknown real simulators, time-varying covariance, nonstationary event families, or model misspecification. A deployed simulator should estimate covariance by event family and context, use predeclared training partitions, record the selected policy and keep the evaluation seeds untouched.

## Limits

- This is a known-ATE structural model, not a physical/social Symtropy experiment.
- The three beta values are preregistered fixture conditions, not an empirical distribution of Luminous models.
- The 256/256 seed split is a software benchmark design, not a universal power recommendation.
- The generator has finite Random123 differential parity but no TestU01/PractRand qualification.
- No claim is made that event-addressed coupling always reduces variance.
- Cross-fitted selection itself needs further tests under misspecified covariance models and distribution shift before production use.
