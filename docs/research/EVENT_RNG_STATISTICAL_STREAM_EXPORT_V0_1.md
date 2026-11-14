# Reproducible statistical stream export v0.1

## Purpose

This adds a reproducible byte-stream boundary for external PRNG test suites. It deliberately separates stream generation from TestU01/PractRand execution so the exact test input can be hashed, archived and replayed independently.

The exporter supports two schedules:

1. **raw** — sequential Philox4x64-10 blocks using a fixed key and increasing counter block index;
2. **event** — one EventAddress per block, varying stable subject ID, tick, event kind, draw purpose and occurrence ordinal under a fixed replicate seed and experiment namespace.

## Binary format

- Four u64 words are emitted per block.
- Each word is serialized with u64::to_le_bytes().
- Stdout contains raw binary bytes only; metadata and diagnostics are written to stderr.
- The byte count is exactly blocks multiplied by four words multiplied by eight bytes.
- The exporter rejects a block count that would exceed the nonzero u32 subject-ID range required by event mode.

This format is deliberately explicit and host-endian independent.

## Repeatability qualification

The smoke script:
- checks the exact requested Rust/Cargo toolchain;
- warms the locked dependency graph;
- generates each mode twice with locked, offline, release builds;
- verifies exact expected byte counts;
- compares the two raw files byte-for-byte;
- reports SHA-256 digests;
- verifies raw and event-addressed schedules are not accidentally identical.

This is an input-integrity and repeatability test, not a statistical battery.

## External statistical qualification

The same binary files can be consumed by external statistical test harnesses without regenerating data. Preserve the file SHA-256, generator source commit, Rust/Cargo versions, seed, namespace, mode, block count and exact command line with every TestU01/PractRand run.

Required future campaigns:
- TestU01 SmallCrush, Crush and BigCrush on raw and event-addressed streams;
- PractRand at staged input volumes, with complete output logs;
- separate assessment of the top-52-bit midpoint conversion used for f64 uniforms;
- multiple independent seeds and namespaces;
- no silently filtered failed p-values or test diagnostics.

Passing a stream test suite would qualify only the tested output schedule and sample volume. It would not prove semantic event identity or causal validity.
