# Random123 differential parity qualification v0.1

## Purpose

The Rust Philox4x64-10 implementation already has three fixed known-answer tests. This qualification expands that evidence by comparing a deterministic corpus against the upstream Random123 C++ implementation at a pinned source commit.

The differential check includes:
- the all-zero counter/key vector;
- the all-ones counter/key vector;
- the published mixed counter/key vector;
- thousands of deterministic raw counter/key pairs;
- thousands of event-address-derived counter/key pairs that exercise subject, tick, event-kind, purpose, occurrence, replicate-seed and experiment-namespace fields.

All four output words are compared for every row.

## Reference pin

- Repository: https://github.com/lemire/random123
- Commit: ff4443af61194bb57a042cd4945f2c73bdf1d0dd
- API: r123::Philox4x64 (the upstream ten-round default)

The workflow fetches this exact commit and verifies the checked-out SHA before compiling the C++ reference harness. Random123 is used only in the test harness; it is not a production Cargo dependency.

## Running locally

The script requires git, g++, rustup and an installed Rust toolchain:

    bash scripts/test-philox-random123-parity.sh 1.100.0 4096

The generated corpus and compiler identities are kept in a temporary directory for the duration of the check. The script prints the Random123 commit, rustc/Cargo/C++ versions and corpus digest on success.

## Interpretation

A pass establishes parity of the Rust permutation implementation with the pinned Random123 reference for the tested corpus. It does not qualify the generator with TestU01 or PractRand, prove statistical independence across application event schedules, establish causal validity, or compare performance.

The corpus tests raw Philox outputs and event-address-derived counter/key packing. It is still a finite deterministic differential test, not a proof over all 2^256 counter/key inputs.
