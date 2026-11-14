#![forbid(unsafe_code)]

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic Philox corpus for comparison with Random123's C++ API.
//!
//! Each stdout row contains counter[4], key[2], and Rust output[4] as 16-digit
//! hexadecimal u64 values. Diagnostics go to stderr so stdout stays machine-readable.

use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use symtropy_consciousness_physics::event_rng::{
    philox4x64_10, registry, DrawPurpose, EventAddress, EventKind, SubjectId,
};

const CORPUS_SEED: u64 = 0x5241_4e44_4f4d_3132;
const EXPERIMENT_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

fn emit_row(
    output: &mut String,
    counter: [u64; 4],
    key: [u64; 2],
) -> Result<(), std::fmt::Error> {
    let result = philox4x64_10(counter, key);
    writeln!(
        output,
        "{:016x} {:016x} {:016x} {:016x} {:016x} {:016x} {:016x} {:016x} {:016x} {:016x}",
        counter[0],
        counter[1],
        counter[2],
        counter[3],
        key[0],
        key[1],
        result[0],
        result[1],
        result[2],
        result[3]
    )
}

fn parse_count() -> usize {
    let count = env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()
        .expect("corpus count must be a positive integer")
        .unwrap_or(4096);
    assert!((1..=1_000_000).contains(&count), "corpus count must be in 1..=1_000_000");
    count
}

fn main() {
    let count = parse_count();
    let mut output = String::with_capacity((count * 2 + 3) * 180);

    let fixed_vectors = [
        ([0_u64; 4], [0_u64; 2]),
        ([u64::MAX; 4], [u64::MAX; 2]),
        (
            [
                0x243f_6a88_85a3_08d3,
                0x1319_8a2e_0370_7344,
                0xa409_3822_299f_31d0,
                0x082e_fa98_ec4e_6c89,
            ],
            [0x4528_21e6_38d0_1377, 0xbe54_66cf_34e9_0c6c],
        ),
    ];
    for (counter, key) in fixed_vectors {
        emit_row(&mut output, counter, key).expect("format fixed vector");
    }

    let mut raw_rng = SplitMix64::new(CORPUS_SEED);
    for _ in 0..count {
        let counter = [
            raw_rng.next_u64(),
            raw_rng.next_u64(),
            raw_rng.next_u64(),
            raw_rng.next_u64(),
        ];
        let key = [raw_rng.next_u64(), raw_rng.next_u64()];
        emit_row(&mut output, counter, key).expect("format raw vector");
    }

    let mut address_rng = SplitMix64::new(CORPUS_SEED ^ 0xa5a5_5a5a_d3c3_b4b4);
    for index in 0..count {
        let subject_id = u32::try_from(index + 1).expect("bounded corpus fits u32 subject IDs");
        let event_kind = EventKind::new((index % u16::MAX as usize + 1) as u16)
            .expect("generated event-kind tag is nonzero");
        let purpose = DrawPurpose::new(((index.wrapping_mul(7)) % u16::MAX as usize + 1) as u16)
            .expect("generated purpose tag is nonzero");
        let address = EventAddress::new(
            address_rng.next_u64(),
            address_rng.next_u64(),
            SubjectId::agent(subject_id).expect("subject ID is nonzero"),
            address_rng.next_u64(),
            event_kind,
            purpose,
            address_rng.next_u64() as u32,
        );
        let block_index = address_rng.next_u64();
        let counter = address.counter_words(block_index);
        let key = address.key_words();
        emit_row(&mut output, counter, key).expect("format event-address vector");
    }

    io::stdout()
        .write_all(output.as_bytes())
        .expect("write reference corpus to stdout");
    eprintln!(
        "reference_corpus_status=EMITTED fixed=3 raw={} event_addressed={}",
        count, count
    );
    let _ = registry::RANDOM_WALK;
}
