# GeoBioBench Evaluation Protocol v0.1

**Date:** 2026-11-15  
**Status:** Proposed benchmark protocol; thresholds are research targets, not achieved results or universal standards.

## 1. Purpose

GeoBioBench evaluates whether a geological-industrial design is measurable, safe within a declared envelope, resource-efficient, recoverable, and independently reproducible. It is not one leaderboard score. A system may be strong in mapping and still fail pressure isolation or resource accounting.

## 2. Evidence tiers are separate scoreboards

| Tier | Evidence source | What it can support |
|---|---|---|
| S0 | Synthetic deterministic scenario | Algorithm correctness and controlled fault coverage |
| S1 | Analytical solution or independently implemented reference | Numerical verification within stated assumptions |
| S2 | Bench or laboratory experiment | Physical model validation for tested conditions |
| S3 | Terrestrial field analogue | Environmental and operational validation for analogue conditions |
| S4 | Orbital / spaceflight experiment | Validation under the tested spaceflight conditions |
| S5 | Planetary surface demonstration | Validation under the actual destination conditions |

Scores must never be pooled across tiers to imply a higher evidence tier. Simulation and laboratory performance do not establish flight readiness.

## 3. Preregistered benchmark targets

All targets apply only to the frozen benchmark envelope. They do not constitute universal safety guarantees.

| Benchmark | Initial target | Required reporting |
|---|---|---|
| GeoSurveyBench | 90% prediction-interval empirical coverage between 85% and 95% on held-out regions; no confident fill-in of masked regions | Spatial error by scale, coverage, interval width, calibration by geology and sensor modality |
| GeoStabilityBench | Zero false-safe classifications in the finite high-consequence challenge suite; calibrated failure probabilities on blind cases | Brier score, reliability plot, false-safe count, false-alarm count, uncertainty sensitivity |
| PressureBench | Detect every seeded leak above the pre-calibrated detection threshold in the finite test matrix; no missed critical isolation command | Detection/false-alarm rates, leak-rate threshold, isolation latency, pressure residual, thermal-cycle history |
| ISRUProcessBench | Demonstrate a statistically supported improvement over the best matched baseline before claiming superiority; proposed go/no-go effect size: at least 10% improvement in the pre-registered primary resource-efficiency metric | kg product/kg feed, kg product/kWh, purity, throughput, heat rejection, consumables, uptime, uncertainty |
| BioAsteroidRepBench | Reproduce direction and effect-size uncertainty under published control conditions before modifying organisms, feedstock, or medium | Element-specific yield, confidence intervals, sample size, mineralogy, medium mass, abiotic and biological controls |
| MultiScaleBioBench | Predict spatial and temporal response on held-out reactor scales/conditions better than a well-mixed baseline | Local gradients, residence-time distribution, sensor placement, physiology proxy, scale-up error |
| BioLoopBench | Every measured elemental balance residual must be consistent with propagated measurement uncertainty; no unmeasured stream silently classified as closed | C/H/O/N/P and water inventories, residual, uncertainty, detection limits, make-up mass, purge losses |
| ContactSequenceBench | No uncontrolled rebound or unrecoverable state in the finite challenge suite; at least 95% mission-sequence success in the preregistered nominal/stochastic test distribution | Contact impulse, attitude error, anchor engagement, sample recovery, contact duration, retreat margin, failure recovery |
| FailureCascadeBench | Zero hard safety-invariant violations in the frozen fault-injection suite; safe degradation or abstention is acceptable | Availability of each critical service, unsafe-state duration, isolation/recovery time, fault detection delay |
| ValueOfInformationBench | At equal observation budget, reduce decision regret by at least 20% versus a declared passive sampling baseline, or reach equal regret with 20% fewer observations | Expected regret, posterior uncertainty, mission mass/energy/time cost, calibration and sensitivity |
| EvidenceAdversaryBench | Reject every known forged, stale, replayed, incomplete, or unauthorized claim in the finite adversarial suite | False acceptance, false rejection, provenance chain, time validity, negative-control results |
| EconomyBench | Show lifecycle advantage only when uncertainty intervals and sensitivity analysis support it; no single-point break-even claim | Cost per delivered functional unit, imported mass, energy, water, spares, maintenance, downtime, disposal, discount assumptions |

The 10%, 20%, 90%, 85–95%, and 95% values are proposed programme targets. They must be reviewed and preregistered with domain experts before formal benchmark release. Safety-critical acceptance criteria may be stricter depending on the physical system and applicable standards.

## 4. Baseline matrix

Every benchmark should compare against the strongest relevant baseline available for its test tier.

| Problem | Minimum baseline set |
|---|---|
| Geological reconstruction | Classical photogrammetry/LiDAR/GPR processing; point-cloud registration; conventional Bayesian mapping; NASA CADRE cooperative mapping/GPR where comparable |
| Geomechanics | Analytical rock mechanics where applicable; conventional deterministic FEM; stochastic/reliability analysis where available |
| Pressure systems | Standard pressure-vessel design methods; conventional leak detection and safety instrumented logic |
| Regolith processing | NASA MRE and CaRD; Blue Alchemist's reported integrated regolith-to-products prototype; matched abiotic processing; measured process energy and product purity |
| Bioleaching | Matched abiotic leach; organism-only and medium controls; published BioAsteroid conditions where reproducible |
| Bioreactor control | Well-mixed process model; conventional PID/MPC; multiscale CFD/physiology model where data permit |
| Asteroid robotics | Mission-heritage contact concepts; short-contact/rapid-retreat baseline; integrated mobility/sampling/anchoring review; independent contact-dynamics model |
| Autonomy | Deterministic state machine; conventional fault detection and recovery; learning-enabled planner with safety shield |
| Robotic construction / excavation | NASA ARMADAS modular voxel assembly and IPEx excavation design target as architectural/throughput references; match mass, geometry and test tier |
| Resource allocation | Greedy and linear-programming baselines; robust optimization under uncertainty |
| Evidence verification | Cryptographic provenance checks plus deliberately forged/stale/replayed negative controls |

Use the best baseline that can be run under the same data, hardware, information and compute constraints. If a baseline cannot be reproduced, document the limitation and do not claim superiority over it.

## 5. Experimental design

1. Publish the benchmark specification, expected interfaces, units, data provenance, and scoring functions before running candidate systems.
2. Partition by geological site, mineralogical batch, reactor scale, environmental regime, and mission sequence. Avoid random splits that leak adjacent samples or repeated runs into train and test.
3. Freeze blind test labels and hashes. Candidate systems must not read hidden ground truth.
4. Preregister primary endpoints, secondary endpoints, exclusions, sample sizes, stopping criteria, uncertainty methods, and multiplicity correction.
5. Include ablations for Symtropy physics, Symthaea planning, Sol-Atlas observations, Mycelix provenance/authorization, and independent sensing.
6. Repeat stochastic tests with disclosed seed sets and confidence intervals. Report tails and catastrophic outcomes, not only averages.
7. Include negative controls: corrupted sensors, impossible mass balances, missing units, stale calibration, forged provenance, out-of-domain geometry and unmeasured streams.
8. Publish raw observations when permitted, derived data, code version, environment, input digests, and a one-command reproduction path.

## 6. Decision rule for a superiority claim

A candidate may claim benchmark superiority only when:
- the baseline and candidate use the same test information and resource boundaries;
- the primary metric and threshold were preregistered;
- improvement exceeds measurement uncertainty and is statistically supported;
- no critical safety, purity, reliability, or evidence metric regresses beyond its declared margin;
- the result is reproduced by an independent implementation or reviewer;
- the evidence tier is stated explicitly.

A result at S0 or S1 may establish algorithmic or numerical advantage, but not physical or flight superiority.

## 7. Failure accounting

Every run must produce a machine-readable disposition:
- PASS: all declared criteria met;
- FAIL: one or more criteria violated;
- UNQUANTIFIED: required measurement, uncertainty, or provenance missing;
- ABSTAIN: system correctly refused to make a claim or action;
- INVALID: test fixture or instrumentation violated its own protocol.

Do not convert UNQUANTIFIED or INVALID to PASS. Do not count ABSTAIN as failure when abstention is the correct safety response.

## 8. Initial benchmark release gate

GeoBioBench v0.1 is ready for external review only when:
- at least one deterministic synthetic dataset exists for each software-only benchmark;
- analytical reference cases exist for elemental balance and pressure compartment mass balance;
- scoring code has unit tests and negative controls;
- benchmark dataset/model hashes are frozen;
- at least one independent reviewer can reproduce a scorecard;
- all targets are labelled as proposed and all limitations are documented.
