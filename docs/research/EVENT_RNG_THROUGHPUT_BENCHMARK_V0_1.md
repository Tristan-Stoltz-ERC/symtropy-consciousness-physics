# Event-keyed RNG throughput benchmark v0.1

## Purpose

This benchmark separates the costs that should not be blended into one performance number:

1. the raw Philox4x64-10 block permutation;
2. construction of a complete EventAddress plus a random draw;
3. scalar random-access versus cached sequential word consumption;
4. cached sequential conversion into open-interval f64 uniforms.

It also measures the historical LCG helper used by the gradient-replacement example for context. The LCG and Philox have different semantics and output quality; their speed is not a quality-equivalence claim.

## Workloads

| Label | Work performed | Scalar draws reported |
|---|---|---:|
| legacy_lcg_top53 | historical example LCG step and top-53 extraction | 1 per iteration |
| philox4x64_core_block | raw four-word Philox permutation | 4 per iteration |
| event_address_construct_plus_word | stable subject ID + EventAddress + one scalar word | 1 per iteration |
| event_address_construct_plus_block | stable subject ID + EventAddress + one four-word block | 4 per iteration |
| event_address_prebuilt_scalar_word | prebuilt address + random-access scalar index; recomputes its block | 1 per iteration |
| event_address_cached_scalar_stream | prebuilt address + sequential block-cached u64 iterator | 1 per iteration |
| event_address_cached_uniform_stream | prebuilt address + sequential block-cached open-interval f64 iterator | 1 per iteration |

The benchmark consumes outputs into checksums to prevent dead-code elimination. The uniform stream checksums the f64 bit patterns, preserving evidence of exact conversion behavior. Workload order rotates across five rounds.

## How to run

Run:

    cargo run --release --locked --example event_rng_throughput -- 1000000

Run under the exact toolchain being evaluated and preserve:
- rustc -Vv and cargo -Vv;
- host CPU, architecture, kernel, governor, memory and filesystem;
- exact Git commit and Cargo.lock hash;
- raw per-round rows, not only medians;
- workload order, iteration count and release profile.

Run on an otherwise idle, pinned host. Repeat on each target CPU architecture. Do not aggregate different hosts into one median without a blocked design.

## Interpretation

- Raw Philox throughput is not a direct Random123 comparison; this measures the Rust implementation only.
- The historical LCG is the specific formula in `examples/gradient_replacement.rs`, not a claim that every experiment uses the same RNG.
- EventAddress construction is reported separately from prebuilt-address draws.
- A scalar `word(i)` call recomputes the containing Philox block. `EventAddress::words()` caches each block and reuses its four lanes.
- `EventAddress::uniforms_open01()` uses the same cached stream and the same top-52 midpoint conversion as `uniform_open01(i)`. It must remain bit-identical to random-access values and strictly inside (0,1).
- CI results are smoke measurements on shared hosted runners and declare no winner.
- A genuine SOTA comparison still requires same-host comparison against Random123 C++ Philox/Threefry implementations, plus TestU01 and PractRand qualification.
