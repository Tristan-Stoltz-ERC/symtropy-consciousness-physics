# Per-event-family CRN coupling portfolio v0.1

## Research question

A single global CRN choice can be suboptimal when different exogenous event families induce different covariance signs. A system can contain one event family for which sharing reduces counterfactual variance and another for which sharing increases it. Looking only at total covariance can conceal this structure.

This benchmark compares:
1. all event families independent;
2. all event families shared;
3. a single global policy selected from total training covariance;
4. a per-event-family policy selected from each family's training covariance.

## Structural model

Two independent event families, U and V, produce:

- Y0 = U² + V²
- Y1 = U² + βu U + V² + βv V + 0.25
- ATE = 0.25 + (βu + βv)/2
- Cov(U², U² + βU) = 4/45 + β/12

The two preregistered cases use opposite covariance signs but the same positive total covariance:

| Case | βu (environment noise) | βv (random-walk angle) | Cov(U) | Cov(V) | Total covariance |
|---|---:|---:|---|---|---|
| A | 0.5 | -1.5 | positive | negative | positive |
| B | -1.5 | 0.5 | negative | positive | positive |

A global positive-covariance rule selects SHARED_BOTH in both cases. The family-aware rule selects SHARED for the positive-covariance family and INDEPENDENT for the negative-covariance family.

## Cross-fitted selection

- Training replicate indices: [0, 256).
- Holdout replicate indices: [256, 512).
- Subjects per replicate: 2,048.
- The U event family is addressed with the environment/noise semantic tags.
- The V event family is addressed with the random-walk/angle semantic tags.
- Shared treatment events reuse the control's complete semantic address.
- Independent treatment events use distinct namespaces, while retaining the same replicate seed and subject identity.
- Each event family's policy is selected from its training covariance sign.
- The global policy is selected from the sum of the two training covariances.
- All policies are evaluated only on the disjoint holdout replicates.

## Theoretical target

For βu = 0.5 and βv = -1.5 (and the swapped case), the theoretical variance ratio relative to all-independent coupling is approximately:

- all-shared: 0.5245;
- per-family portfolio: 0.3427.

That corresponds to roughly 35% lower estimator variance than global all-shared. These are theoretical values from the specified structural model, not measured results. The executable reports exact theoretical ratios and empirical holdout ratios separately.

## Holdout measurements

For each policy, the executable reports the held-out ATE mean and sample variance across complete outer replicate estimates. It computes paired percentile bootstrap intervals for the variance ratio relative to all-independent coupling, resampling complete holdout replicates (2,000 draws).

The software gate requires:
- training covariance signs to select the expected policy for each event family;
- the total covariance to remain positive, so global policy chooses both shared;
- all four policy means to recover the known ATE within 0.01;
- the per-family policy to reduce the empirical variance ratio by at least 10% relative to global all-shared and below the all-independent baseline.

## Interpretation

This tests whether event-family-specific coupling can outperform one global CRN decision in a controlled structural model. It is a method demonstration, not a physical or social Symtropy experiment. A real simulator would need to define event families before inspecting holdout outcomes, retain policy receipts, estimate covariance by family on training seeds, and validate the complete policy on separate holdout seeds.

The portfolio can be extended to richer coupling choices (shared, independent, antithetic, stratified, or model-specific couplings), but each choice needs a correct event identity contract and a held-out evaluation.

## Limits

- The model is analytically specified and has a known ATE.
- The event-family partition and beta regimes are preregistered synthetic fixtures.
- No production adaptive-coupling validity is claimed.
- No TestU01/PractRand qualification is implied.
- No SOTA or physical-model performance claim is made.
