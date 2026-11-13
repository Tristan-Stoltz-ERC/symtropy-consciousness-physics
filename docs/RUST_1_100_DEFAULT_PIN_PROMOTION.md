# Rust 1.100.0 default toolchain promotion proposal

**Status:** draft proposal; no merge or production promotion has occurred.

## Decision basis

The default compiler pin is a developer/build-tool choice, not the package's MSRV and not a dependency upgrade. Exact hosted qualification has already passed for the Symtropy repository's supported build/test surface on Rust 1.100.0, with Rust 1.99.1 as the previous stable comparison.

Relevant evidence, all against the qualification PR's exact merge tree:

- Run [#43428281223](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/actions/runs/43428281223): supported targets and tests on Rust 1.99.1 and 1.100.0 passed; Cargo.lock immutability passed.
- Run [#43428281524](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/actions/runs/43428281524): complete Rust 1.100.0 target/test surface and Rust 1.101.0-beta.1 forward canary passed; the 2018/2021/2024 never-type and ABI tests passed; the Cargo 1.100 unused-dependency lint and artifact interface tests passed; three local sparse-registry publish-age controls passed.
- Supported target/test surface: 113 passed, 0 failed per compiler run. The obsolete HDC example was preserved as archived research, not presented as an executable target; the curvature example feature metadata was repaired without changing scientific source.

This evidence supports a compatibility pin proposal for this repository's currently supported target surface. It does not establish whole-ecosystem compatibility for every downstream consumer or hardware target.

## Proposed change

- Update `rust-toolchain.toml` from exact Rust 1.99.1 to exact Rust 1.100.0.
- Keep profile `minimal`.
- Keep `clippy` and `rustfmt` components.
- Do not change Cargo.toml, Cargo.lock, edition, dependency versions or MSRV.
- Do not change the experiment sources, seeds, constants or result files.

## Why performance is a separate decision

The 1.99.1 vs 1.100.0 paired application benchmark is still a separate, opt-in measurement. No compiler-speedup, runtime-speedup or binary-size benefit is asserted by this pin PR.

A compiler can be selected for compatibility, maintenance, security and language/Cargo capabilities even if its workload speed is unchanged. Any performance claim still requires the exact paired benchmark, raw artifacts, numerical-output review and independent target-host measurements.

## Promotion gate

The promotion should merge only after:
1. The exact-head stable qualification workflow passes on the PR merge tree with the new pin.
2. The stable release and forward-compatibility jobs preserve their distinct result labels.
3. Cargo.lock remains byte-identical.
4. The PR reviewers agree that the supported target surface is the intended scope.

The default pin is not the MSRV. If older toolchains remain supported by package consumers, they should be covered by a separately declared `rust-version` policy and CI matrix.

## Claim ceiling

A passing PR establishes that this repository's supported target/test surface works under the new pinned toolchain. It does not mean:
- every Luminous Dynamics repository is now qualified for Rust 1.100.0;
- Rust 1.100.0 is faster than 1.99.1;
- PGO has been run or has delivered a gain;
- archived research examples have been restored or scientifically validated;
- a production deployment or hardware qualification has occurred.
