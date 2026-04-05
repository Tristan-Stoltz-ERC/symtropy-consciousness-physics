# symtropy-consciousness-physics

**By-product mutualism in energy-constrained multi-agent systems.**

> "Physics doesn't cooperate. It clusters at the wells. Everything we called society was resource geometry."

## What This Is

A thermodynamic multi-agent physics engine that produces the **appearance** of cooperation without any cooperation mechanism. 55 findings across 65 experiments demonstrate that social structure — clustering, cooperation events, group formation, survival patterns — emerges as **by-product mutualism** from individual resource-seeking, not from social drives.

## The Story

We built a physics engine expecting cooperation to emerge from thermodynamic necessity. It did — across 45 environmental variations. Then we replaced the behavioral controller (Finding 46) and discovered that **every controller** — including random walking and sitting still — produces cooperation. The social gradient we designed was a metabolic liability, not a survival mechanism.

Seven subsequent experiments (F47-55) systematically confirmed: no environmental modification makes social seeking beneficial. The cooperation was by-product mutualism — a free consequence of agents co-locating at shared resource attractors.

## Key Numbers

- **55 findings**, 65 experiment files
- **104 library tests** + 2 integration tests
- **6 controllers tested**: FEP gradient, well-seeking, greedy, partner-seeking, random walk, stationary
- **10 environmental variants**: standard, scarce, volatile, hidden, threshold, resonance-gated, complementary, resonance-only, large arena (1000×1000)
- **20-seed replication** with Holm-Bonferroni correction
- **Bitwise deterministic** (lock-in test verified)

## Quick Start

```bash
# Run the controller replacement study (the key experiment)
cargo run --example gradient_replacement --release

# Run the cooperation decomposition (wells vs resonance)
cargo run --example cooperation_decomposition --release

# Run the full original cooperation experiment
cargo run --example jphi_convergence --release

# Run all lib tests
cargo test --lib
```

## The 10 Falsification Experiments (F46-55)

| Finding | Environment | FEP Indispensable? |
|---------|-------------|--------------------|
| F46 | 6 controllers, standard wells | No — all controllers cooperate |
| F47 | ±resonance regeneration | No — wells alone sustain |
| F48 | Scarce wells (250-2500J) | No — even tiny wells suffice |
| F49 | Hidden wells | No — random exploration works |
| F50 | Volatile wells | Marginal (d=0.35, migrating only) |
| F51 | Threshold wells (≥4 agents) | No — natural co-location |
| F52 | Resonance-gated wells | No — dense clusters produce pairs |
| F53 | No wells (resonance only) | No — stationary agents survive best |
| F54 | Complementary hunt | No — types co-locate at shared prey |
| F55 | Large arena (1000×1000) | No — random walkers find wells |

## Architecture

```
src/
├── active_inference.rs    # Bayesian active inference agent
├── convergence.rs         # Mann-Whitney U, Cohen's d, Holm-Bonferroni
├── coupling.rs            # ConsciousnessField, EntityConsciousness, AgentMemory
├── energy.rs              # EnergyBudget (U, T, S, F = U-TS)
├── fep_gradient.rs        # Handcrafted FEP gradient + LearnedFepWeights
├── harmony_field.rs       # 8-channel harmony fields, resonance
├── prey.rs                # Complementary task system
├── spatial_hash.rs        # Grid-based O(n×k) neighbor queries
├── thermodynamics.rs      # ThermodynamicConstants, Landauer bound, J/Φ
└── ...
```

## Publications

- **Paper**: `paper.pdf` — "Social Structure Without Social Mechanisms" (6 pages)
- **Book**: `book.pdf` — "The Thermodynamics of Togetherness" (83 pages)
- **Thesis**: `THESIS.md` — The revised by-product mutualism thesis

## License

AGPL-3.0-or-later. Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics.

## Citation

If you use this engine or its findings, please cite:

```
Stoltz, T. (2026). Social Structure Without Social Mechanisms:
By-Product Mutualism in Energy-Constrained Multi-Agent Systems.
Luminous Dynamics. https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics
```
