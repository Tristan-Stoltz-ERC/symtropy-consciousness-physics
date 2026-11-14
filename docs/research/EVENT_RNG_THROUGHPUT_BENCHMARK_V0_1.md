# Event-keyed RNG throughput benchmark v0.1

## Purpose

This benchmark separates three costs that should not be blended into one performance number:

1. the raw Philox4x64-10 block permutation;
2. construction of a complete EventAddress plus a random draw;
3. use of an already constructed EventAddress.

It also measures the historical LCG helper used by the gradient-replacement example for context. The LCG and Philox have different semantics and output quality; their speed is not a quality-equivalence claim.

## Workloads

| Label | Work performed | Scalar words reported |
|---|---|---:|
| legacy_lcg_top53 | historical example LCG step and top-53 extraction | 1 per iteration |
| philox4x64_core_block | raw four-word Philox permutation | 4 per iteration |
| event_address_construct_plus_word | stable subject ID + EventAddress + one scalar word | 1 per iteration |
| event_address_construct_plus_block | stable subject ID + EventAddress + one four-word block | 4 per iteration |
| event_address_prebuilt_scalar_word | prebuilt address + varying scalar index | 1 per iteration |

The benchmark consumes outputs into checksums to prevent dead-code elimination. It rotates workload order across five rounds and verifies stable checksums.

## How to run

Run:

    cargo run --release --locked --example event_rng_throughput -- 1000000

Run under the exact toolchain whose performance is being evaluated and preserve:

- rustc -Vv and cargo -Vv;
- host CPU, architecture, kernel, governor, memory and filesystem;
- exact Git commit and Cargo.lock hash;
- raw per-round rows, not only medians;
- workload order, iteration count and release profile.

Run on an otherwise idle, pinned host. Repeat on each target CPU architecture. Do not aggregate different hosts into one median without a blocked design.

## Interpretation

- Raw Philox throughput is not a direct Random123 comparison; this measures the Rust implementation only.
- The historical LCG is the specific formula in examples/gradient_replacement.rs, not a claim that every experiment uses this same RNG.
- EventAddress construction is reported separately from prebuilt-address draws.
- A scalar word(i) call computes the containing Philox block; callers consuming four adjacent words should benchmark block(i) and reuse its lanes.
- CI results are smoke measurements on shared hosted runners and declare no winner.
- A genuine SOTA comparison still requires same-host comparison against the Random123 C reference and its Philox/Threefry implementations, together with statistical batteries.
