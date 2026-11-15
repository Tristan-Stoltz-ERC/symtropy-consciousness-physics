# GeoBio Foundry — State-of-the-Art Gap Analysis (15 November 2026)

**Purpose:** Update the programme against public evidence available on 2026-11-15 and turn research gaps into benchmarkable work.  
**Status:** Literature-informed analysis, not a systematic review or technology qualification.

## 1. Executive update

The competitive landscape is more mature than a generic “space habitats are unsolved” framing suggests. NASA and industry already demonstrate important ISRU components in simulated lunar environments; real-scene lava-tube geometry has been used in finite-element studies; BioAsteroid has demonstrated microgravity bioleaching of meteorite material; and small-body robotics reviews already emphasize integrated mobility, sampling, and anchoring.

GeoBio Foundry should not claim novelty in those component ideas. The defensible opportunity is integration of uncertainty-preserving geological models, co-designed pressure/energy/thermal/resource networks, matched biological-versus-abiotic comparisons, whole-sequence robotic recovery, and evidence-qualified autonomy.

A key strategic change follows: **start lunar resource engineering with prospecting and regolith oxygen/metal pathways, not an assumed water-fed biological economy.** NASA's summaries describe oxygen/metal extraction as more mature, while polar volatile extraction remains strongly dependent on deposit form, concentration, and distribution that are not sufficiently characterized.

## 2. Evidence-based industry baselines

### 2.1 Lunar ISRU: oxygen and metals are ahead of water mining

NASA reports that its Molten Regolith Electrolysis (MRE) effort, completed in 2025, demonstrated oxygen extraction from lunar regolith simulant in vacuum. A NASA technical report for a large-scale test describes 25 kg of simulant processed, an average oxygen production rate of 0.07 kg/hour, and nine hours of sustained molten-regolith electrolysis. This is a meaningful component-level throughput baseline, not a full lunar plant demonstration. The public summary does not provide the complete energy intensity needed for a fair kg-O2/kWh comparison, so GeoBioBench must not invent one.

NASA's ISRU overview says analog field demonstrations have so far been short duration and conducted in Earth conditions. A NASA progress review reports oxygen/metal extraction approaches at approximately TRL 5/6 in simulated lunar conditions, while emphasizing that polar water/volatile extraction viability depends on form, concentration, and distribution that remain insufficiently known.

**GeoBio response**
- Record 25 kg / 9 h / 0.07 kg/h as a published throughput reference with its test conditions.
- Compare processes only after matching feedstock, purity, pressure/temperature regime, operating duration, and system boundary.
- Require kg product/kg feed, kg product/kWh, thermal rejection, consumables, uptime, and maintenance.
- Treat polar water as a prospecting claim until spatially representative measurements support accessible reserve estimates.

Sources:
- NASA, Lunar Surface Technology: https://www.nasa.gov/lunar-surface-technology/
- NASA NTRS, Large-Scale Demonstration of Molten Regolith Electrolysis for Oxygen Production (2026): https://ntrs.nasa.gov/citations/20260006167
- NASA, Overview: In-Situ Resource Utilization: https://www.nasa.gov/overview-in-situ-resource-utilization/
- NASA NTRS, Progress Review of NASA Lunar ISRU (2025): https://ntrs.nasa.gov/api/citations/20250003730/downloads/Progress%20Review%20NASA%20Lunar%20ISRU_Sanders.pdf?attachment=true

### 2.2 NASA's 2026 capability priorities

NASA's September 2026 Lunar Enabling Infrastructure Accelerator call names five capability areas: vertical solar arrays and power management/storage; oxygen extraction from regolith; radioisotope Stirling generation; in-space advanced manufacturing; and nanomaterials production.

This reinforces that power, energy storage, oxygen, materials and manufacturing are near-term priorities. Biological manufacturing can complement these systems, but should not displace foundational energy and ISRU engineering in our first demonstrator.

Source: https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/

### 2.3 BioAsteroid: important proof of principle, not a self-sufficient factory

The 2026 BioAsteroid study tested bacterial and fungal bioleaching of 44 elements from L-chondrite meteorite material aboard the ISS under microgravity. It reports that Penicillium simplicissimum enhanced palladium release to 549.3 ± 234.4% of the non-biological control in microgravity (about 5.5 times the control under the tested conditions).

The methods matter as much as the headline:
- meteorite aliquots were approximately 0.79 ± 0.14 g;
- fragments were approximately 1–3.5 mm;
- each sample used 5 mL of 50% R2A nutrient medium;
- the medium included yeast extract, peptone, casamino acids, glucose, starch, sodium pyruvate and mineral salts;
- the study reports substantial variability, plausibly associated with microbial growth and heterogeneous meteorite samples;
- for some elements and comparisons, abiotic leaching was more effective than biological leaching.

This is a valuable spaceflight proof of principle for microbe–mineral interaction. It does not demonstrate asteroid-scale throughput, closed nutrient supply, economical product separation, or a nutrient-independent biofactory.

**GeoBio response**
- Reproduce the published sample-level comparison before extending it.
- Preserve the abiotic control and nutrient-medium mass in the denominator.
- Add medium-only, mineral-only, killed-biomass, and organism-specific controls where scientifically appropriate.
- Report product recovery after separation and purification, not merely element concentration in leachate.
- Measure yield per kg meteorite, kg imported nutrient medium, litre liquid, kWh, and reactor-day.
- Test multiple mineralogies and independent batches before claiming transferability.

Source: https://www.nature.com/articles/s41526-026-00567-3

### 2.4 Bioreactor scale-up: gradients may be useful information

A September 2026 review on bioreactor heterogeneity describes scale-up effects caused by spatial and temporal gradients coupling fluid dynamics and cell physiology. It proposes multiscale modelling and intelligent optimization, including deliberate use of environmental histories rather than assuming that all gradients must be eliminated.

This challenges simplistic perfectly mixed tank models.

**GeoBio response**
- Make spatial and temporal gradients explicit in later BioProcessBench versions.
- Couple residence-time distributions and CFD-derived fields to physiological response models.
- Measure whether a process benefits from or is harmed by heterogeneity.
- Do not claim that a single bulk sensor reading represents the whole culture.

Source: https://www.sciencedirect.com/science/article/pii/S0734975026001059

### 2.5 Lava tubes: real-scene geometry and finite-element analysis already exist

A 2025 study reconstructed a terrestrial lava tube from point-cloud data and used finite-element analysis to examine how real cross-sectional geometry affects stability under a simulated surface pressure associated with meteoroid impacts.

A 2026 study in Advances in Space Research proposes reshaping the roof of lunar lava tubes by removing selected rock mass, using PLAXIS 2D finite-element analysis. It reports improvements in selected numerical stability metrics under its model assumptions.

These are useful baselines. Neither demonstrates that a particular lunar tube is safe, that its rock-mass properties are known, or that a roof-reshaping procedure is operationally validated. The 2026 roof-modification proposal is a model-specific hypothesis, not a generic construction recommendation.

**GeoBio response**
- Benchmark against real-scene point-cloud reconstruction plus conventional FEM.
- Add fracture networks, uncertain material properties, spatial heterogeneity, thermal cycling, seismic/impact loads, and multiple model classes.
- Use blind tests against independent physical analogue measurements.
- Report probability of failure and false-safe rate, not only a deterministic factor of safety.
- Keep support/reshaping optimization separate from the decision to occupy or pressurize a void.

Sources:
- https://www.mdpi.com/2079-9292/14/15/3062
- https://www.sciencedirect.com/science/article/pii/S0273117725012955

### 2.6 Small-body robotics: test the whole contact sequence

The March 2026 review of small-body sampling robots emphasizes the coupling of sampling, mobility, and anchoring. Contact impulses, attitude disturbance, uncertain surface mechanics, and retreat/recovery interact. The review identifies short-contact and rapid-retreat approaches as having relevant flight heritage, while deeper sampling and sustained throughput remain challenging.

**GeoBio response**
- Benchmark full sequences: approach → contact → anchor/stabilize → sample → disengage → retreat → verify state.
- Score momentum transfer, attitude error, detachment/rebound risk, sample yield, and recovery probability together.
- Make reversible engagement and safe retreat first-class design objectives.
- Avoid optimizing peak anchor holding force in isolation.

Source: https://onlinelibrary.wiley.com/doi/full/10.1002/smb2.70032

### 2.7 Planetary protection is a versioned design constraint

COSPAR's 2026 planetary-protection policy introduced a unified policy for icy worlds and updated the policy structure. GeoBio sites must carry body-, destination-, mission- and activity-specific planetary-protection constraints, including preservation of locations relevant to life-detection science.

**GeoBio response**
- Model preservation and contamination constraints as typed, versioned site constraints.
- Keep biological production closed and contained in initial demonstrations.
- Never treat an atlas-level industrial-potential score as permission to disturb or inoculate a site.
- Require mission-specific planetary-protection review before any extraterrestrial biological deployment.

Source: https://cosparhq.cnes.fr/cospar-policy-on-planetary-protection/

## 3. Updated paradigm shifts

### A. Prospecting is an economic decision, not merely a mapping task

The first question is not “how much material is there?” It is “which observation most reduces the risk of a bad infrastructure decision?” GeoBio should optimize measurement campaigns using expected value of information, measurement cost, mission risk, and downstream design sensitivity.

### B. Co-design energy, heat, materials and biology

Do not optimize each factory independently. A biological process may have modest electric demand but require cooling, water purification, pressure control, nutrient imports and waste treatment. The system optimizer must use a coupled mass, energy, and thermal network. Energy recovery must remain separately measured rather than being silently netted against gross consumption.

### C. Design around process histories, not ideal operating points

Bioreactors, rocks, pressure systems and robots experience histories: gradients, thermal cycling, fatigue, contamination, transient contact and sensor drift. Benchmark transient behaviour and recovery, not only steady-state efficiency.

### D. Make reversibility and retreat the default for uncertain environments

For asteroid contact, cave exploration and robotic construction, prioritize operations that can disengage, retreat, isolate or revert safely. An action that cannot be reversed requires a higher evidence threshold.

### E. Make abstention a positive capability

The system should return states such as insufficient evidence, out of domain, unsafe, or requires measurement when it lacks evidence. Conservative abstention should score better than a confident false-safe prediction.

### F. Preserve the distinction between local resource, accessible reserve and product

Use explicit states:
observed occurrence → inferred deposit → characterized resource → accessible reserve → extracted feedstock → qualified product.

A material estimate cannot be booked as inventory or used as a guaranteed bioreactor feedstock.

## 4. GeoBioBench v0.2 additions

| Benchmark | Required test | Primary scores |
|---|---|---|
| GeoSurveyBench | Multimodal reconstruction with withheld regions and conflicting sensors | Spatial error by scale, coverage, uncertainty calibration, false certainty in masked zones |
| GeoStabilityBench | Blind fracture/material/geometry cases plus physical analogue tests | Failure probability calibration, Brier score, false-safe rate at fixed risk threshold |
| PressureBench | Seeded leaks, blocked vents, sensor drift, thermal cycles, isolation failures | Detection probability, false alarms, isolation time, pressure/mass residuals |
| ISRUProcessBench | Regolith processing against published and laboratory baselines | kg product/kg feed, kg product/kWh, purity, throughput, uptime, thermal rejection, consumables |
| BioAsteroidRepBench | Reproduce BioAsteroid test conditions and controls | Element-specific recovery, effect size, confidence intervals, batch variability, media burden |
| BioProcessBench | Process performance across batch and reactor conditions | Product yield, productivity, purity, batch-to-batch variance, contamination and downtime |
| MultiScaleBioBench | Spatial/temporal gradient and physiology coupling | Prediction error across locations and time; performance under scale-up |
| BioLoopBench | Elemental inventories and waste/recovery flows | C/H/O/N/P/water balance residuals, make-up mass, unresolved material |
| ContactSequenceBench | Full asteroid approach/contact/sample/retreat sequence | Momentum impulse, attitude error, sample mass, contact duration, recovery probability |
| FailureCascadeBench | Concurrent power, cooling, pressure, sensor, communications and contamination faults | Critical-service continuity, unsafe-state duration, recovery time |
| ValueOfInformationBench | Competing measurement plans under a fixed mass/energy/time budget | Expected decision-regret reduction per kg, kWh and unit time |
| EvidenceAdversaryBench | Stale, forged, correlated, missing or replayed observations | Detection rate, false acceptance, provenance preservation, safe abstention |
| EconomyBench | Lifecycle comparison per delivered functional unit | Cost, imported mass, energy, water, spares, waste, downtime and uncertainty |

### Benchmark governance

1. Freeze inputs, versions, random seeds, scoring code and acceptance thresholds before blind testing.
2. Split by geological site, mineralogy, reactor batch and environmental regime—not random rows from one source.
3. Publish simple analytical and conventional engineering baselines.
4. Publish failed cases and confidence intervals, including tails.
5. Separate simulation, bench, field-analogue, orbital and flight scoreboards.
6. Do not create a single aggregate score until weights are pre-registered and sensitivity analysis is published.
7. Use independent ground truth and negative controls.
8. Avoid leakage between training and blind evaluation.

## 5. Updated programme priorities

1. **ResourceForge + ISRUProcessBench:** closest route to measurable engineering value; develop resource and energy accounting before asserting process superiority.
2. **GeoSurveyBench + ValueOfInformationBench:** address upstream uncertainty that can invalidate an entire lunar site or resource business case.
3. **PressureForge + FailureCascadeBench:** use small uncrewed pressure compartments and independently instrumented test loops.
4. **BioAsteroidRepBench + MultiScaleBioBench:** reproduce carefully, then investigate whether biology adds net value under realistic reagent, energy, and separation burdens.
5. **ContactSequenceBench:** develop simulation and tabletop hardware-in-the-loop tests for asteroid robotics, without claiming flight-level validation.
6. **Cross-site compiler:** integrate only after individual reference kernels and their baselines are reproducible.

## 6. Economic and commercial framing

The near-term commercial hypothesis should be orbital and terrestrial manufacturing support, process monitoring, resource-accounting software, and autonomy/assurance services. A 2026 perspective argues for orbital biomanufacturing in logistics, servicing and in-space supply chains, but this remains a strategic thesis rather than proof of demand or profitable unit economics.

Asteroid platinum-group-element extraction should remain a scientific and engineering research topic until there is a credible target-specific assay, extraction/separation pathway, transport model, product market, and lifecycle cost comparison. High terrestrial spot prices do not establish an asteroid mining business case.

Source: https://www.nature.com/articles/s41526-026-00571-7

## 7. Explicit claims we should and should not make

**Supported by public evidence**
- Lunar regolith oxygen extraction has been demonstrated in relevant simulated environments.
- NASA reports a 25 kg lunar-simulant MRE demonstration at 0.07 kg/hour for nine hours.
- Microgravity bioleaching of L-chondrite meteorite material has been demonstrated on the ISS at small experimental scale.
- Real-scene lava-tube point-cloud geometry has been used in FEM research.
- Small-body sampling, mobility and anchoring require integrated design.
- Planetary-protection policies must be part of mission planning.

**Not demonstrated by those sources**
- A self-sustaining asteroid biofactory.
- Commercially viable asteroid PGE mining.
- A safe pressurized lunar lava tube.
- Long-duration autonomous underground industrial operation on the Moon or Mars.
- A closed biological nutrient loop using only local extraterrestrial resources.
- A universal pressure architecture for caves, craters, canyons and asteroids.
- GeoBio Foundry superiority over NASA, ESA, JAXA or commercial systems.

## 8. Decision

Proceed with an offline reference kernel and benchmark fixtures before connecting intelligent planners or physical simulation. The first software task is reproducible elemental and energy accounting with explicit uncertainty, missing-composition abstention, and evidence provenance. This creates a defensible foundation for later process comparison.

The first scientific milestone is replication and independent comparison—not novelty claims. The first competitive milestone is a benchmark suite with transparent baselines. The first engineering milestone is an uncrewed terrestrial analogue with independent instrumentation.
