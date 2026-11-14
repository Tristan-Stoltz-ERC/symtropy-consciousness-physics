#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layered throughput microbenchmark for the event-addressed Philox prototype.
//!
//! This reports candidate costs, not a winner: the historical LCG and Philox
//! do not provide equivalent stream semantics. CI is a build/smoke environment;
//! performance comparisons must be repeated on a pinned, controlled host.

use std::env;
use std::hint::black_box;
use std::time::Instant;

use symtropy_consciousness_physics::event_rng::{philox4x64_10, registry, EventAddress, SubjectId};

const DEFAULT_ITERATIONS: usize = 100_000;
const ROUNDS: usize = 5;
const ROOT_SEED: u64 = 0x4556_454e_545f_524e;
const EXPERIMENT_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const LCG_INCREMENT: u64 = 1_442_695_040_888_963_407;

#[derive(Clone, Copy, Debug)]
enum BenchmarkKind {
    LegacyLcg53,
    PhiloxCoreBlock,
    EventConstructWord,
    EventConstructBlock,
    EventPrebuiltWord,
    EventPrebuiltCachedStream,
    EventPrebuiltCachedUniformStream,
}

impl BenchmarkKind {
    const ALL: [Self; 7] = [
        Self::LegacyLcg53,
        Self::PhiloxCoreBlock,
        Self::EventConstructWord,
        Self::EventConstructBlock,
        Self::EventPrebuiltWord,
        Self::EventPrebuiltCachedStream,
        Self::EventPrebuiltCachedUniformStream,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::LegacyLcg53 => "legacy_lcg_top53",
            Self::PhiloxCoreBlock => "philox4x64_core_block",
            Self::EventConstructWord => "event_address_construct_plus_word",
            Self::EventConstructBlock => "event_address_construct_plus_block",
            Self::EventPrebuiltWord => "event_address_prebuilt_scalar_word",
            Self::EventPrebuiltCachedStream => "event_address_cached_scalar_stream",
            Self::EventPrebuiltCachedUniformStream => "event_address_cached_uniform_stream",
        }
    }

    const fn scalar_draws_per_iteration(self) -> u128 {
        match self {
            Self::LegacyLcg53
            | Self::EventConstructWord
            | Self::EventPrebuiltWord
            | Self::EventPrebuiltCachedStream
            | Self::EventPrebuiltCachedUniformStream => 1,
            Self::PhiloxCoreBlock | Self::EventConstructBlock => 4,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    kind: BenchmarkKind,
    round: usize,
    iterations: usize,
    scalar_draws: u128,
    elapsed_ns: u128,
    checksum: u64,
}

impl Sample {
    fn draws_per_second(self) -> f64 {
        if self.elapsed_ns == 0 {
            return f64::INFINITY;
        }
        self.scalar_draws as f64 * 1_000_000_000.0 / self.elapsed_ns as f64
    }

    fn nanoseconds_per_draw(self) -> f64 {
        self.elapsed_ns as f64 / self.scalar_draws as f64
    }
}

fn address_for(event_index: u64) -> EventAddress {
    let agent_id = u32::try_from(event_index + 1)
        .expect("benchmark iteration count is constrained to fit nonzero u32 IDs");
    EventAddress::new(
        ROOT_SEED,
        EXPERIMENT_NAMESPACE,
        SubjectId::agent(agent_id).expect("agent ID is nonzero"),
        event_index,
        registry::RANDOM_WALK,
        registry::RANDOM_WALK_ANGLE,
        0,
    )
}

fn lcg_top53(iterations: usize) -> u64 {
    let mut state = black_box(ROOT_SEED);
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        state = state
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        let top53 = state >> 11;
        checksum = checksum.wrapping_add(black_box(top53));
    }
    black_box(checksum)
}

fn philox_core_blocks(iterations: usize) -> u64 {
    let key = [ROOT_SEED, EXPERIMENT_NAMESPACE];
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let counter_index = index as u64;
        let counter = [
            counter_index + 1,
            counter_index.rotate_left(17),
            ((registry::RANDOM_WALK.tag() as u64) << 48)
                | ((registry::RANDOM_WALK_ANGLE.tag() as u64) << 32),
            0,
        ];
        let output = philox4x64_10(black_box(counter), black_box(key));
        for word in output {
            checksum = checksum.wrapping_add(black_box(word));
        }
    }
    black_box(checksum)
}

fn event_construct_word(iterations: usize) -> u64 {
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let address = address_for(index as u64);
        checksum = checksum.wrapping_add(black_box(address.word(0)));
    }
    black_box(checksum)
}

fn event_construct_block(iterations: usize) -> u64 {
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let address = address_for(index as u64);
        for word in address.block(0) {
            checksum = checksum.wrapping_add(black_box(word));
        }
    }
    black_box(checksum)
}

fn event_prebuilt_word(iterations: usize) -> u64 {
    let address = address_for(1);
    let mut checksum = 0_u64;
    for index in 0..iterations {
        checksum = checksum.wrapping_add(black_box(address.word(index as u64)));
    }
    black_box(checksum)
}

fn event_prebuilt_cached_stream(iterations: usize) -> u64 {
    let mut words = address_for(1).words();
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        let word = words.next().expect("benchmark draw count is finite");
        checksum = checksum.wrapping_add(black_box(word));
    }
    black_box(checksum)
}

fn event_prebuilt_cached_uniform_stream(iterations: usize) -> u64 {
    let mut uniforms = address_for(1).uniforms_open01();
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        let value = uniforms.next().expect("benchmark draw count is finite");
        checksum = checksum.wrapping_add(black_box(value.to_bits()));
    }
    black_box(checksum)
}

fn run(kind: BenchmarkKind, round: usize, iterations: usize) -> Sample {
    let started = Instant::now();
    let checksum = match kind {
        BenchmarkKind::LegacyLcg53 => lcg_top53(iterations),
        BenchmarkKind::PhiloxCoreBlock => philox_core_blocks(iterations),
        BenchmarkKind::EventConstructWord => event_construct_word(iterations),
        BenchmarkKind::EventConstructBlock => event_construct_block(iterations),
        BenchmarkKind::EventPrebuiltWord => event_prebuilt_word(iterations),
        BenchmarkKind::EventPrebuiltCachedStream => event_prebuilt_cached_stream(iterations),
        BenchmarkKind::EventPrebuiltCachedUniformStream => {
            event_prebuilt_cached_uniform_stream(iterations)
        }
    };
    let elapsed_ns = started.elapsed().as_nanos();
    Sample {
        kind,
        round,
        iterations,
        scalar_draws: (iterations as u128) * kind.scalar_draws_per_iteration(),
        elapsed_ns,
        checksum,
    }
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn parse_iterations() -> usize {
    let parsed = env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()
        .expect("iteration count must be a positive integer")
        .unwrap_or(DEFAULT_ITERATIONS);
    assert!(
        (1..=(u32::MAX as usize - 1)).contains(&parsed),
        "iteration count must fit in 1..=u32::MAX-1"
    );
    parsed
}

fn main() {
    let iterations = parse_iterations();
    let mut samples = Vec::with_capacity(ROUNDS * BenchmarkKind::ALL.len());

    println!("benchmark_schema=luminous.symtropy.event-rng-throughput.v0.1");
    println!("target_arch={}", env::consts::ARCH);
    println!("target_os={}", env::consts::OS);
    println!("iterations_per_workload={iterations}");
    println!("rounds={ROUNDS}");
    println!("legacy_lcg_constants=multiplier:{LCG_MULTIPLIER};increment:{LCG_INCREMENT}");
    println!(
        "measurement_policy=rotated_workload_order;release_mode_recommended;no_performance_gate"
    );
    println!(
        "workload,round,iterations,scalar_draws,elapsed_ns,draws_per_second,ns_per_draw,checksum"
    );

    for round in 0..ROUNDS {
        for offset in 0..BenchmarkKind::ALL.len() {
            let index = (round + offset) % BenchmarkKind::ALL.len();
            let kind = BenchmarkKind::ALL[index];
            let sample = run(kind, round + 1, iterations);
            println!(
                "{},{},{},{},{},{:.3},{:.6},{:016x}",
                kind.label(),
                sample.round,
                sample.iterations,
                sample.scalar_draws,
                sample.elapsed_ns,
                sample.draws_per_second(),
                sample.nanoseconds_per_draw(),
                sample.checksum
            );
            samples.push(sample);
        }
    }

    println!("median_summary");
    for kind in BenchmarkKind::ALL {
        let matching: Vec<_> = samples
            .iter()
            .copied()
            .filter(|sample| std::mem::discriminant(&sample.kind) == std::mem::discriminant(&kind))
            .collect();
        let rates: Vec<_> = matching
            .iter()
            .map(|sample| sample.draws_per_second())
            .collect();
        let costs: Vec<_> = matching
            .iter()
            .map(|sample| sample.nanoseconds_per_draw())
            .collect();
        let checksums: Vec<_> = matching.iter().map(|sample| sample.checksum).collect();
        let checksum_stable = checksums.windows(2).all(|pair| pair[0] == pair[1]);
        println!(
            "{},median_draws_per_second={:.3},median_ns_per_draw={:.6},checksum_stable={}",
            kind.label(),
            median(&rates),
            median(&costs),
            checksum_stable
        );
        assert!(checksum_stable, "workload checksum changed between rounds");
    }

    println!("benchmark_status=MEASURED_NO_WINNER_DECLARED");
    println!("interpretation=CI_smoke_measurements_are_not_SOTA_performance_claims");
}
