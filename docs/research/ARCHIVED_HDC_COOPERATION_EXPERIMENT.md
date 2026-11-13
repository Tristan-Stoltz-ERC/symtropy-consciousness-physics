# Archived experiment: HDC vs scalar cooperation

**Disposition:** archived source; not an executable experiment in this repository revision.

## Why this example was retired from Cargo's supported target surface

The prior path `examples/hdc_cooperation.rs` was discovered automatically by Cargo and therefore made `cargo check --all-targets` fail under Rust 1.99.1, Rust 1.100.0 and Rust 1.101.0-beta.1.

The source imports:

```rust
symtropy_consciousness_physics::hdc_context::{self, HdcConsciousnessContext}
```

but the current crate root has no `hdc_context` module. Its run comment also references a `consciousness-hdc` feature that is not declared in the package manifest. There is no implementation in this repository that satisfies the imported API contract. This is a source/API completeness defect, not a compiler regression.

A placeholder module or a fake feature would make the target superficially compile while misrepresenting the experiment. That would be worse than an explicit archive disposition.

## Preservation

The full historical source was moved to:

`research/archived-examples/hdc_cooperation.rs`

The original source content is preserved after a short archive notice. Its pre-move Git blob SHA-1 was:

`2464da8339ee1d8cf77e9806fce91f4832033493`

The source is no longer under Cargo's auto-discovered `examples/` directory. It is therefore not built, tested, benchmarked or presented as an executable result.

No experimental result, seed, data file or published conclusion was changed by this move.

## Requirements before restoration

A future implementation must be a separate, reviewable research change and should:

1. Implement the real `HdcConsciousnessContext` API or compose an already existing qualified owner. Do not copy a second HDC implementation.
2. Specify and test `inputs_from_state`, stepping, thought-vector update and `thought_resonance` semantics.
3. Declare the correct feature and actual dependency graph, and make the `cargo run` command match it.
4. Use deterministic, versioned seeds and fixed scalar/HDC initial states.
5. Report all 10 seeds for each condition, not only means; add confidence intervals and predeclared primary outcomes.
6. Compare against scalar, random-walk, and resource-only controls. Include ablations for HDC state, temporal dynamics and coupling.
7. Preserve all raw per-seed outputs and define numerical equivalence/tolerance before comparing compiler versions.
8. Qualify the source on Rust 1.99.1 and the current stable toolchain before returning it to `examples/`.

## Claim ceiling

This archive is not evidence that HDC improves cooperation, survival, collective Phi, or thermodynamic efficiency. No HDC run was produced by this source in the current repository state. The source is retained as a research artifact only.

The all-target CI gate remains enabled. A future example may re-enter the supported target surface only with a complete implementation and reproducible tests.
