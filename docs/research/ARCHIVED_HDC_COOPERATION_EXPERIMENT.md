# Archived HDC-vs-scalar cooperation example

**Disposition:** archived / not executable / not scientific evidence.

On 2026-11-13, the source at `examples/hdc_cooperation.rs` was found to import the absent `symtropy_consciousness_physics::hdc_context` module and types/functions `HdcConsciousnessContext`, `inputs_from_state`, and `thought_resonance`. The repository contains no corresponding module or HDC crate dependency. The example's run instruction also refers to a `consciousness-hdc` Cargo feature that the package does not declare.

The complete source has been preserved at:

`research/archived-examples/hdc_cooperation.rs`

It has been removed from the Cargo example directory so that `cargo check --all-targets` does not imply that an unavailable HDC implementation exists. This is a scope correction, not a successful HDC experiment and not a scientific finding.

## Claim ceiling

This archived source does **not** demonstrate:
- a working 16,384-dimensional HDC thought-vector implementation in this crate;
- an `HdcLtcUnifiedNetwork` or CfC/LTC recurrent model;
- valid HDC thought-diversity or resonance calculations;
- a matched HDC-vs-scalar comparison;
- any measured improvement in survival, clustering, Φ, J/Φ, or cooperation.

It must not be counted among the repository's published 55 findings or 65 experiment files.

## Requirements before restoration

A future implementation must be a separate reviewed change that:

1. Identifies the exact source repository/revision and licence for the actual HDC/LTC implementation, or implements and documents the full algorithm locally. A placeholder vector update is not sufficient.
2. Defines a typed API for context construction, seeded initialization, state-to-HDC encoding, time-step updates, Φ derivation, and pairwise thought similarity.
3. Declares the Cargo feature and all required target/dependency constraints.
4. Adds unit tests for vector dimension, finite values, normalization, deterministic seed replay, update stability, and similarity symmetry/bounds.
5. Adds a small exact known-answer fixture for `inputs_from_state`, `step`, and `thought_resonance`.
6. Runs scalar and HDC conditions under identical physics, initial positions, seeds, timestep, agent population, resource fields, and termination rules.
7. Prespecifies the primary outcome, secondary outcomes, exclusions, uncertainty method and minimum seed count before inspecting result differences.
8. Reports per-seed raw results and a machine-readable evidence receipt; uses paired statistics across seeds and does not infer causality from one trajectory.
9. Reproduces the result independently on the pinned stable toolchain before re-entering the supported example directory.

The original source is retained to make this future work recoverable. Its presence in the archive is not a qualification pass.
