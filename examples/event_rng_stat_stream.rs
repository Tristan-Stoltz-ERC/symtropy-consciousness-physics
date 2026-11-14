#![forbid(unsafe_code)]

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits deterministic little-endian Philox words for external statistical suites.
//!
//! Stdout is a raw byte stream with no header. All metadata is written to stderr.
//! Modes:
//! - raw: sequential Philox4x64-10 blocks with a fixed key and increasing counter.
//! - event: one EventAddress per block using a stable, deterministic key schedule.

use std::env;
use std::error::Error;
use std::io::{self, BufWriter, Write};

use symtropy_consciousness_physics::event_rng::{
    philox4x64_10, DrawPurpose, EventAddress, EventKind, SubjectId,
};

const DEFAULT_BLOCKS: u64 = 1_000_000;
const DEFAULT_SEED: u64 = 0x5354_4154_5354_524d;
const DEFAULT_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const WORDS_PER_BLOCK: usize = 4;
const BYTES_PER_WORD: usize = 8;
const BLOCKS_PER_FLUSH: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamMode {
    Raw,
    EventAddressed,
}

impl StreamMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "raw" => Ok(Self::Raw),
            "event" => Ok(Self::EventAddressed),
            other => Err(format!("unknown stream mode {other:?}; expected raw or event")),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Raw => "raw_philox_counter_stream",
            Self::EventAddressed => "event_addressed_philox_stream",
        }
    }
}

fn parse_u64(value: Option<String>, default: u64, name: &str) -> Result<u64, String> {
    match value {
        Some(text) => text
            .parse::<u64>()
            .map_err(|_| format!("{name} must be an unsigned 64-bit integer")),
        None => Ok(default),
    }
}

fn raw_block(index: u64, seed: u64, namespace: u64) -> [u64; 4] {
    philox4x64_10([index, 0, 0, 0], [seed, namespace])
}

fn event_block(index: u64, seed: u64, namespace: u64) -> [u64; 4] {
    let agent_id = u32::try_from(index + 1)
        .expect("event stream block count must not exceed u32::MAX");
    let event_kind_tag = (index % 3 + 1) as u16;
    let purpose_tag = (index % 4 + 1) as u16;
    let event_kind = EventKind::new(event_kind_tag).expect("event kind tag is nonzero");
    let purpose = DrawPurpose::new(purpose_tag).expect("draw purpose tag is nonzero");

    let address = EventAddress::new(
        seed,
        namespace,
        SubjectId::agent(agent_id).expect("generated agent ID is nonzero"),
        index,
        event_kind,
        purpose,
        (index % (u32::MAX as u64 + 1)) as u32,
    );
    address.block(0)
}

fn emit_stream(
    mode: StreamMode,
    blocks: u64,
    seed: u64,
    namespace: u64,
) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();
    let mut output = BufWriter::with_capacity(
        (BLOCKS_PER_FLUSH as usize) * WORDS_PER_BLOCK * BYTES_PER_WORD,
        stdout.lock(),
    );
    let mut buffer =
        Vec::with_capacity((BLOCKS_PER_FLUSH as usize) * WORDS_PER_BLOCK * BYTES_PER_WORD);

    for start in (0..blocks).step_by(BLOCKS_PER_FLUSH as usize) {
        let end = (start + BLOCKS_PER_FLUSH).min(blocks);
        buffer.clear();
        for index in start..end {
            let words = match mode {
                StreamMode::Raw => raw_block(index, seed, namespace),
                StreamMode::EventAddressed => event_block(index, seed, namespace),
            };
            for word in words {
                buffer.extend_from_slice(&word.to_le_bytes());
            }
        }
        output.write_all(&buffer)?;
    }
    output.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mode = StreamMode::parse(&args.next().unwrap_or_else(|| "raw".to_string()))?;
    let blocks = parse_u64(args.next(), DEFAULT_BLOCKS, "block count")?;
    let seed = parse_u64(args.next(), DEFAULT_SEED, "replicate seed")?;
    let namespace = parse_u64(args.next(), DEFAULT_NAMESPACE, "experiment namespace")?;
    if args.next().is_some() {
        return Err("usage: event_rng_stat_stream [raw|event] [blocks] [seed] [namespace]".into());
    }
    if blocks == 0 {
        return Err("block count must be nonzero".into());
    }
    if blocks > u32::MAX as u64 {
        return Err("block count must be <= u32::MAX for the event-addressed mode".into());
    }

    eprintln!("stream_schema=luminous.symtropy.event-rng-stat-stream.v0.1");
    eprintln!("mode={}", mode.label());
    eprintln!("blocks={blocks}");
    eprintln!("words={}", blocks * WORDS_PER_BLOCK as u64);
    eprintln!("bytes={}", blocks * WORDS_PER_BLOCK as u64 * BYTES_PER_WORD as u64);
    eprintln!("seed={seed:#018x}");
    eprintln!("experiment_namespace={namespace:#018x}");
    eprintln!("serialization=little_endian_u64");
    eprintln!("stdout=raw_binary_only");

    emit_stream(mode, blocks, seed, namespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_stream_blocks_are_deterministic_and_block_indexed() {
        let first = raw_block(0, DEFAULT_SEED, DEFAULT_NAMESPACE);
        let second = raw_block(1, DEFAULT_SEED, DEFAULT_NAMESPACE);
        assert_eq!(first, raw_block(0, DEFAULT_SEED, DEFAULT_NAMESPACE));
        assert_ne!(first, second);
    }

    #[test]
    fn event_addressed_blocks_are_deterministic_and_order_independent() {
        let first = event_block(7, DEFAULT_SEED, DEFAULT_NAMESPACE);
        let _unrelated = event_block(99, DEFAULT_SEED, DEFAULT_NAMESPACE);
        assert_eq!(first, event_block(7, DEFAULT_SEED, DEFAULT_NAMESPACE));
    }

    #[test]
    fn raw_and_event_addressed_schedules_are_separate() {
        assert_ne!(
            raw_block(0, DEFAULT_SEED, DEFAULT_NAMESPACE),
            event_block(0, DEFAULT_SEED, DEFAULT_NAMESPACE)
        );
    }

    #[test]
    fn event_addressed_schedule_rejects_unrepresentable_subject_count() {
        assert!(std::panic::catch_unwind(|| {
            event_block(u32::MAX as u64, DEFAULT_SEED, DEFAULT_NAMESPACE)
        })
        .is_err());
    }

    #[test]
    fn little_endian_serialization_is_explicit() {
        let word = 0x0123_4567_89ab_cdef_u64;
        assert_eq!(
            word.to_le_bytes(),
            [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
        );
    }
}
