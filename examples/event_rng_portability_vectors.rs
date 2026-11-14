#![forbid(unsafe_code)]

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits fixed portability vectors for native x86-64 and AArch64 comparison.
//! The output intentionally contains no host metadata so the bytes are identical.

use symtropy_consciousness_physics::event_identity::SemanticEventId;
use symtropy_consciousness_physics::event_lineage::{BirthKey, LineageIdRegistry};
use symtropy_consciousness_physics::event_lineage_receipt::LineageReceipt;
use symtropy_consciousness_physics::event_rng::{
    philox4x64_10, registry, DrawPurpose, EventAddress, EventKind, SubjectId,
};

const NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;

fn words_as_hex(words: [u64; 4]) -> String {
    words
        .iter()
        .map(|word| format!("{word:016x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    println!("schema=luminous.symtropy.event-rng-portability.v1");

    let zero = philox4x64_10([0; 4], [0; 2]);
    println!("philox.zero={}", words_as_hex(zero));

    let mixed = philox4x64_10(
        [
            0x243f_6a88_85a3_08d3,
            0x1319_8a2e_0370_7344,
            0xa409_3822_299f_31d0,
            0x082e_fa98_ec4e_6c89,
        ],
        [0x4528_21e6_38d0_1377, 0xbe54_66cf_34e9_0c6c],
    );
    println!("philox.mixed={}", words_as_hex(mixed));

    let event = EventAddress::new(
        42,
        NAMESPACE,
        SubjectId::agent(7).expect("fixed subject ID is nonzero"),
        99,
        EventKind::new(2).expect("fixed event tag is nonzero"),
        DrawPurpose::new(3).expect("fixed purpose tag is nonzero"),
        3,
    );
    println!("event.address.block5={}", words_as_hex(event.block(5)));
    println!(
        "event.address.uniform_draw20_bits={:016x}",
        event.uniform_open01(20).to_bits()
    );

    let birth = BirthKey::new(
        7,
        [
            SubjectId::agent(3).expect("fixed parent ID is nonzero"),
            SubjectId::agent(11).expect("fixed parent ID is nonzero"),
        ],
        SemanticEventId::new(0xabc).expect("fixed event ID is nonzero"),
        2,
    )
    .expect("fixed BirthKey is valid");
    let mut lineage_registry = LineageIdRegistry::default();
    let dynamic_subject = lineage_registry
        .resolve(birth)
        .expect("fixed dynamic birth has no collision");
    println!("dynamic.birth_subject={:016x}", dynamic_subject.raw());

    let receipt = LineageReceipt::from_registry(&lineage_registry)
        .expect("single assignment receipt is canonical");
    println!("receipt.begin");
    print!("{}", receipt.encode());
    println!("receipt.end");
}
