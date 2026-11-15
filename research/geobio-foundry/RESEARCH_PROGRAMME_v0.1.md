# GeoBio Foundry: Research Programme and Engineering RFC v0.1

**Status:** Research proposal; not a flight-qualified design  
**Owner:** Luminous Dynamics  
**Initial implementation home:** Symtropy research prototype  
**Date:** 2026-11-15  
**Scope:** Geological infrastructure, pressure containment, resource processing, biological manufacturing, autonomous operations, and independent assurance

## 1. Executive thesis

GeoBio Foundry investigates how geological environments can become productive infrastructure rather than passive locations for habitats. The programme develops reusable methods to characterize a site, compile its measured properties and constraints into candidate infrastructure designs, evaluate those designs against physical and operational models, and qualify claims using independent observations.

The near-term objective is not to design a complete extraterrestrial settlement. It is to build an Earth-testable, evidence-carrying engineering workflow whose assumptions, uncertainty, failure modes, and resource balances remain explicit.

**Core principle:** a geological opportunity, an inferred resource, an engineering prediction, a physical demonstration, and an operationally qualified capability are different evidence states. Software must not silently promote one state into another.

## 2. Research questions

1. Can heterogeneous geological observations be fused into a versioned site model that preserves source provenance, spatial resolution, uncertainty, and unknown regions?
2. Can a design compiler generate multiple feasible pressure, utility, access, and industrial layouts while rejecting infeasible candidates?
3. Can coupled geomechanical, thermal, pressure, fluid, and resource-flow models predict failure envelopes with calibrated uncertainty?
4. Can biological and abiotic processing routes be compared on matched feedstock, product specification, energy, water, consumables, waste, and maintenance boundaries?
5. Can autonomous operations remain inside explicit safety envelopes under sensor drift, model disagreement, communication loss, and injected faults?
6. Can independent reviewers reproduce every qualification claim from frozen inputs, model versions, test procedures, and raw observations?

## 3. Paradigm shifts

### 3.1 Geological opportunity compiler

Replace site-first habitat templates with a compiler pipeline:

**observations → site model → constraints → candidate designs → simulations → tests → qualification**

The compiler may return **infeasible**, **insufficient evidence**, or **requires measurement**. It must never invent missing geological properties to complete a design.

### 3.2 Pressure boundaries are engineered assets

Natural rock is not assumed to be a pressure vessel. The initial architecture uses independently designed and tested pressure boundaries, with geology providing shielding, thermal buffering, anchoring opportunities, and non-pressure-bearing spatial enclosure where verified. Differential pressure loads, penetrations, seals, fatigue, fire, leak propagation, and emergency isolation are explicit.

### 3.3 Optimize the complete resource service

Compare process routes using functional units such as one kilogram of on-spec product delivered to a defined point, or one day of a specified life-support service. Include upstream feedstock preparation, imported mass, power generation, heat rejection, water, nutrient make-up, spares, downtime, waste treatment, and end-of-life handling.

### 3.4 Biology is a contained process, not an environmental release

Initial biological work stays inside closed, monitored, sterilizable systems. Biological outputs are not assumed to be suitable for construction, food, or medical use without independent quality, biosafety, and product-specific validation. Planetary-protection constraints are first-class site constraints.

### 3.5 Evidence is part of the product

Every important claim carries provenance, method, calibration, uncertainty, model and dataset versions, acceptance criteria, independent witness, and validity period. Model output is not ground truth; simulation is not physical demonstration; a successful test is not automatically flight qualification.

## 4. System architecture and responsibilities

| System | Responsibility | Must not do |
|---|---|---|
| Sol-Atlas | Spatial knowledge, geological feature records, source provenance, preservation and access constraints | Overwrite source observations with generated interpretations |
| Symtropy | Physical models, uncertainty propagation, scenario execution, hardware/field test comparison | Treat a simulation pass as physical qualification |
| Symthaea | Hypothesis generation, causal diagnosis, experiment planning, bounded operational recommendations | Self-certify its own predictions or bypass hard safety constraints |
| Mycelix | Resource provenance, capacity coordination, claims, attestations, settlement and governance | Convert reported availability into verified inventory or self-authorize physical facts |
| Independent instruments / reviewers | Ground truth, calibration, test witnessing, external reproduction | Depend on the same model or sensor path being qualified |

The integration boundary is an evidence-qualified exchange contract, not a shared mutable source of truth.

## 5. GeoBio Resource and Constraint Graph

The common typed graph should represent:

- Geological bodies, layers, voids, fractures, surfaces, and spatial regions.
- Material composition, inferred deposits, measured samples, accessible reserves, and extracted inventory as distinct entity types.
- Pressure boundaries, seals, penetrations, compartments, and isolation devices.
- Energy sources, storage, conversion, loads, and thermal rejection.
- Water, carbon, nitrogen, phosphorus, trace nutrients, and other material flows.
- Biological strains/process identities, contained reactors, feedstocks, batches, products, and waste streams.
- Robots, tools, anchors, mobility modes, payloads, and operational envelopes.
- Evidence objects, calibration records, uncertainty, validity intervals, and qualification decisions.

Every quantity must have units. Every edge must identify its basis (measured, inferred, modelled, assumed, or planned), uncertainty, provenance, spatial and temporal applicability, and transformation history. Unknown is a valid value; it is not zero.

## 6. Work packages

### WP1 — GeoBioAtlas (Sol-Atlas-led)

Build a site model that can ingest terrain meshes, point clouds, imagery, radar/seismic observations, sample assays, thermal observations, and derived products. Preserve native resolution and data lineage. Represent coverage gaps and confidence explicitly.

**First deliverable:** a schema and one synthetic basalt-cavern site fixture with known ground truth, masked regions, fractures, and conflicting sensor observations.

### WP2 — PressureForge (Symtropy-led)

Model modular pressure compartments, interfaces, penetrations, isolation, leakage, thermal cycling, and fault propagation. Couple the pressure boundary to surrounding geology only where a validated mechanical interface model exists.

**First deliverable:** a small deterministic compartment network model with leak injection, isolation logic, mass-balance checks, and explicit assumptions. Structural analysis must distinguish thin-wall pressure-vessel calculations from rock-mass stability analysis; neither substitutes for certified design.

### WP3 — ResourceForge (Symtropy + Mycelix)

Build auditable material and energy flow accounting for extraction and processing routes. Compare abiotic and biological routes on matched functional units and system boundaries.

**First deliverable:** a resource-flow ledger and a small set of synthetic process recipes, including uncertainty and missing-feedstock handling.

### WP4 — BioFoundry (Symthaea + Symtropy)

Develop a process-model interface for contained microbial/fungal production, bioleaching, polymer precursors, enzymes, and waste conversion. Start with published data and benchtop-compatible models; do not infer that microgravity performance transfers to lunar gravity or asteroid conditions.

**First deliverable:** a batch record schema and matched biological-versus-abiotic comparison harness. Any live biological experiment requires suitable laboratory biosafety, containment, and specialist oversight.

### WP5 — Autonomous Geological Construction

Develop inspection, mapping, contact, anchoring, excavation, and deployment task models. Treat asteroid contact mechanics separately from lunar/Martian geotechnics: weak gravity, escape velocity, reaction impulses, and anchoring uncertainty change the governing problem.

**First deliverable:** fault-injected task scenarios for inspection and contact operations, with no physical actuation.

### WP6 — Assurance and governance

Define claim states, evidence bundles, independent test interfaces, resource attestations, and change control. Unknown outcomes block new mutations until reconciliation. Every action is authorized separately from its recommendation.

**First deliverable:** machine-readable qualification states and an evidence manifest with negative controls.

## 7. GeoBioBench v0.1

The benchmark suite evaluates components and the integrated workflow. All targets below are proposed research targets, not claims of current performance or universal industry standards.

| Benchmark | Measurement | Initial acceptance target |
|---|---|---|
| GeoSurveyBench | Geometry and property reconstruction against withheld ground truth | Report spatial error by scale and region; no hidden-region certainty; calibrated uncertainty |
| GeoStabilityBench | Failure/deformation prediction under blind scenarios | Beat declared simple baselines on held-out cases; report false-safe rate separately |
| PressureBench | Leak detection, isolation, pressure/mass balance | Detect all seeded faults in the defined test matrix; isolate within declared limits; zero unreported mass-balance violations |
| ResourceBench | On-spec product per energy, water, imported mass, and waste | Matched-boundary comparison; disclose all make-up inputs and uncertainty |
| BioProcessBench | Yield, purity, productivity, stability, contamination, batch variance | Reproducible independent batches; controls; full feedstock and waste accounting |
| BioLoopBench | Recovery and make-up rates for C, H, O, N, P, water and trace elements | Close elemental balances within measurement uncertainty; never label a partial loop closed |
| ContactBench | Robot stability under contact, drilling, sampling, anchor pullout | Measure impulse, momentum, detachment probability and recovery across stochastic material properties |
| AutonomyBench | Fault detection, safe recovery, communications loss, sensor drift | No safety-envelope violation in the declared fault suite; abstention is a valid success |
| AssuranceBench | Reproducibility and evidence integrity | Exact input/model hashes; independent replay; negative controls; no self-attestation |
| EconomyBench | Cost and mass per delivered service | Lifecycle comparison including transport, power, spares, maintenance, downtime and disposal |

### Benchmark rules

1. Freeze datasets, seeds, model versions, and scoring code before blind evaluation.
2. Publish simple baselines: analytic models, conventional control, abiotic processing, and non-learning planners.
3. Use train/validation/test separation by site or material batch, not random rows from the same source.
4. Report median, tails, uncertainty intervals, failure rates, and abstention rates.
5. Penalize false-safe predictions more heavily than conservative abstention.
6. Include adversarial cases: sensor bias, missing modalities, conflicting assays, unmodelled fractures, blocked vents, contamination alarms, and unavailable resources.
7. Keep simulated, laboratory, field-analogue, orbital, and flight evidence in separate scoreboards.
8. Do not change a benchmark after seeing test results without versioning it as a new benchmark.

## 8. First demonstrator: terrestrial basalt-cavern analogue

The first demonstrator should be a non-habitable, uncrewed test rig. It should not begin with a human-occupied pressurized cave.

**Configuration**
- A surveyed rock/void analogue or instrumented test chamber.
- Two independently sealed pressure compartments or representative pressure vessels.
- A leak/isolation test loop using safe inert test gas and appropriate engineering controls.
- Independent pressure, temperature, flow, and deformation instrumentation.
- A contained, non-pathogenic process demonstrator or abiotic surrogate for the first integration pass.
- A robotic inspection payload or recorded sensor dataset.
- A reproducible digital twin and evidence bundle.

**Sequence**
1. Establish baseline survey and calibration.
2. Freeze geometry, material assumptions, and test inputs.
3. Run blind prediction before physical test.
4. Execute controlled pressure/leak/isolation tests under qualified procedures.
5. Compare predictions to independent observations.
6. Inject sensor faults and communications loss in the control test harness.
7. Reconcile discrepancies and update the model under a new version.
8. Publish a qualification report with limitations and unresolved risks.

## 9. Research baselines and state of the art

The programme should benchmark against existing demonstrations and literature rather than claim a blanket advantage.

- NASA's lunar surface technology overview reports vacuum testing of the Molten Regolith Electrolysis reactor with oxygen extraction from lunar regolith simulant and metal-rich slag as a byproduct. This is a relevant ISRU component baseline, not an integrated lunar factory demonstration: https://www.nasa.gov/lunar-surface-technology/
- NASA's ISRU overview describes excavation, drilling, processing, transport and storage pathways for water/volatile resources, while noting that analog field tests have been short-duration Earth-environment demonstrations: https://www.nasa.gov/overview-in-situ-resource-utilization/
- A 2026 review of yeast-driven space biomanufacturing surveys cellular agriculture, food and nutrient production, and remaining challenges: https://www.nature.com/articles/s41526-026-00576-2
- A 2026 review of small-body sampling robotics emphasizes coupled mobility, sampling and anchoring; it reports that general in-space anchoring remains without a successful demonstration: https://onlinelibrary.wiley.com/doi/10.1002/smb2.70032
- A 2026 review of autonomous planetary cave exploration surveys sensing, robotic platforms and subsurface access challenges: https://onlinelibrary.wiley.com/doi/full/10.1002/rob.70233
- A 2026 lunar lava-tube study evaluates structural reshaping with finite-element analysis. It is a useful numerical baseline, not proof that a specific natural tube is safe: https://www.sciencedirect.com/science/article/pii/S0273117725012955

These references are a starting bibliography, not an exhaustive systematic review. Their findings must be checked against full methods, boundary conditions, and source datasets before being used as benchmark ground truth.

## 10. Execution sequence independent of CI runners

### Increment 0 — specification and fixtures
- Create typed schemas for site, resource, pressure, process, robot, evidence, and qualification records.
- Add deterministic synthetic fixtures and explicit units.
- Define claim-state transitions and negative controls.
- No network dependency and no external runner required.

### Increment 1 — analytical kernels
- Pressure compartment mass balance and leak/isolation scenarios.
- Resource-flow and elemental-balance calculations.
- Geometry error metrics and uncertainty summaries.
- Contact impulse and momentum accounting for simplified small-body scenarios.

### Increment 2 — benchmark harness
- Deterministic seeds and frozen scenarios.
- Baseline implementations.
- Machine-readable scorecards.
- Tests for invalid units, missing data, provenance loss, false qualification, and non-reproducible outputs.

### Increment 3 — Symtropy integration
- Add model adapters only after the independent reference kernels exist.
- Compare predicted and observed results without allowing the model under test to generate its own ground truth.
- Record exact-head local validation separately from CI status.

### Increment 4 — Symthaea and Mycelix integration
- Symthaea proposes hypotheses and experiments with uncertainty and abstention.
- Mycelix records resource claims and evidence-qualified attestations.
- Authorization and execution remain separate; unknown outcome blocks further mutation pending reconciliation.

### Increment 5 — physical analogue
- Only after safety review, calibrated instrumentation, and test procedures exist.
- Begin with uncrewed, non-biological pressure and inspection tests.
- Add contained biological demonstrations as a separately qualified workstream.

## 11. Explicit non-goals and safety boundaries

- No claim of a universal pressure design across all geological environments.
- No assumption that caves, lava tubes, craters, or asteroid interiors are stable or accessible without survey.
- No assumption that lunar regolith, asteroid material, or Martian soil contains a biologically complete nutrient supply.
- No assumption that microgravity bioleaching results transfer directly to other gravity regimes.
- No release of engineered organisms into extraterrestrial environments.
- No human occupancy before independent structural, pressure, fire, life-support, radiation, and emergency qualification.
- No autonomous physical actuation based solely on Symthaea output.
- No use of a simulator score as evidence of flight readiness.

## 12. Definition of success

GeoBio Foundry v0.1 succeeds when an independent reviewer can take a frozen site fixture, regenerate candidate designs and resource balances, reproduce benchmark scores, trace each important claim to its evidence, and identify exactly which properties remain unknown.

The first meaningful breakthrough is not a simulated self-sustaining space city. It is a small, falsifiable, reproducible engineering pipeline that refuses to overstate what it knows—and becomes more capable as real measurements arrive.
