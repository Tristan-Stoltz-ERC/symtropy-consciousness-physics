# Joint event-coupling portfolio v0.1

## Why this is the next step

PRs #34–#36 move from unconditional common random numbers (CRN) to cross-fitted, event-family-specific coupling with uncertainty-aware abstention. That is a substantial improvement, but independent sign decisions still search a restricted policy space: same-family shared draws versus independent draws.

The general counterfactual estimator variance is

$$
\operatorname{Var}(Y_1-Y_0)
=\operatorname{Var}(Y_1)+\operatorname{Var}(Y_0)
-2\operatorname{Cov}(Y_0,Y_1).
$$

The covariance term depends on the joint coupling of the full exogenous event vector, not only the diagonal same-family terms. Antithetic transformations and admissible cross-family permutations can create useful covariance that a diagonal sign selector never examines.

## Analytic counterexample

Let U,V be independent Uniform(0,1) event draws in both arms:

$$
Y_0=U-V,\qquad Y_1=-U'+V'.
$$

Every admissible coupling preserves the joint marginal law Uniform(0,1)^2 in each arm.

- Independent treatment draws: Var(Y1-Y0)=1/3.
- Same-event identity U'=U,V'=V: Var(Y1-Y0)=2/3. Reusing the same streams doubles estimator variance.
- Antithetic U'=1-U,V'=1-V: Y1=Y0, giving theoretical variance zero.
- Cross-family permutation U'=V,V'=U: also gives theoretical variance zero, but only when the two semantic event families have an explicit exchangeability declaration. Even in the non-exchangeable case, the coordinate permutation still preserves the toy Uniform(0,1)^2 marginal distribution; it is rejected because the semantic coupling is not admissible, not because it breaks the marginal law.

The diagonal same-family identity covariances are both negative:

$$
\operatorname{Cov}(U,-U)=-1/12,\qquad
\operatorname{Cov}(-V,V)=-1/12.
$$

A same-family sign selector therefore chooses independent draws. That is safer than harmful identity CRN, but it misses the admissible antithetic and (in the exchangeable fixture) cross-family candidates.

This is a constructed counterexample, not a real-world Symtropy result or a claim that event families may generally be swapped.

## Pilot / holdout design

- 8,192 deterministic replicate seeds for policy selection.
- 8,192 disjoint replicate seeds for held-out evaluation.
- Each policy is evaluated on the same pilot partition and the same holdout partition.
- Selection minimizes pilot variance over the explicitly admissible candidate set.
- The final receipt reports the selected candidate and holdout variance ratio to all-independent.
- The second run disables cross-family exchangeability; the swap candidate must be absent from the feasible set.

The fixture uses Philox event-addressed values. The underlying event identities remain distinct; the coupling policy explicitly describes how the treatment arm maps to baseline event values. It does not rewrite or alias the event registry.

## Admissibility is a hard constraint

A covariance optimizer must never be allowed to propose a statistically attractive but semantically invalid coupling. For real models, each candidate transformation needs a contract covering:

1. marginal preservation for every event family;
2. independence/dependence relationships required within each arm;
3. semantic equivalence or justified exchangeability of mapped events;
4. causal ancestry and conditional-event constraints;
5. deterministic replay and an auditable mapping receipt.

The feasible set is defined by that contract before training. Optimization occurs only inside it. If no nontrivial candidate is justified, independent sampling remains the conservative fallback.

The antithetic complement is valid here because the fixture's primitive inputs are Uniform(0,1). For other distributions, the inverse-CDF reflection or an exact distribution-specific measure-preserving transform must be derived and tested; the transform u -> 1-u is not universally valid.

## What this establishes and what it does not

The software fixture tests:
- a concrete counterexample to same-family identity CRN;
- marginal-preserving antithetic and permutation couplings;
- pilot/holdout seed separation;
- explicit refusal of a cross-family map without exchangeability;
- an observable comparison between local sign selection and a broader coupling portfolio.

It does not establish:
- that the same transformations are admissible in the physical Symtropy model;
- general optimality of the selected coupling;
- universal variance reduction;
- statistical optimality beyond this known-ATE fixture;
- RNG cryptographic security or statistical-battery qualification;
- any physical, social, or consciousness-theory result.

## Next research programme

Move from a finite coupling menu to constrained coupling design:

- represent admissible transformations as a versioned coupling graph;
- estimate the full cross-family covariance matrix on training clusters;
- select a coupling map under marginal and semantic constraints;
- include identity, independent, antithetic, permutation, block, and transport/copula candidates where mathematically valid;
- evaluate selection regret and held-out variance on disjoint seeds;
- compare against the constrained analytic oracle and the unconstrained oracle as an unattainable upper bound.

Report work-normalized variance Var(Delta-hat) times runtime, not variance reduction alone. Candidate-selection overhead can outweigh a reduction in estimator variance.

## Proposed qualification targets

For a real-model follow-up, preregister at least 1,000 meta-replicated pilot/holdout studies per scenario. Report marginal-preservation failures (target zero), inadmissible-map proposals (target zero), selector regret, abstention rate, held-out coverage, work-normalized variance, and p50/p95/p99 overhead. These are proposed gates, not achieved results.
