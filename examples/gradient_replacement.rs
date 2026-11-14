// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Gradient Replacement Study — is cooperation controller-invariant?
//!
//! The most important experiment for the book's thesis. Tests whether
//! cooperative clustering emerges under DIFFERENT movement controllers,
//! not just the handcrafted FEP gradient.
//!
//! If cooperation appears across multiple controllers → universal result.
//! If only under our FEP gradient → the gradient IS the mechanism, not thermodynamics.
//!
//! 6 controllers × 20 seeds × 8,000 ticks:
//! 1. FEP_GRADIENT: current handcrafted gradient (baseline)
//! 2. WELL_ONLY: seek nearest well, ignore other agents entirely
//! 3. PARTNER_ONLY: seek nearest resonant partner, ignore wells
//! 4. GREEDY_ENERGY: move toward whatever maximizes energy gain next tick
//! 5. RANDOM_WALK: uniform random direction each tick
//! 6. STATIONARY: don't move at all
//!
//! All controllers share identical thermodynamics (maintenance, resonance
//! regen, well regen, energy budget). Only the movement rule changes.
//!
//! Run: cargo run --example gradient_replacement --release

use nalgebra::SVector;
use symthaea_consciousness_equation::ConsciousnessInputs;
use symtropy_consciousness_physics::event_rng::{
    registry, EventAddress, SubjectId,
};
use symtropy_consciousness_physics::experiment_seed::ExperimentSeedSchedule;
use symtropy_consciousness_physics::fep_gradient;
use symtropy_consciousness_physics::harmony_field::HarmonyField;
use symtropy_consciousness_physics::paired_inference::{
    bootstrap_mean_interval, pair_by_seed, summarize_paired, BootstrapConfig, SeededValue,
};
use symtropy_consciousness_physics::{ConsciousnessField, ThermodynamicConstants};
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const AGENTS: usize = 20;
const TICKS: usize = 8_000;
const DT: f64 = 1.0 / 64.0;
const SEEDS: usize = 20;

const HARMONY_PROFILES: [[f64; 8]; 4] = [
    [0.7, 0.4, 0.2, 0.1, 0.3, 0.3, 0.2, 0.6],
    [0.3, 0.6, 0.3, 0.2, 0.2, 0.4, 0.6, 0.3],
    [0.2, 0.2, 0.7, 0.4, 0.6, 0.2, 0.3, 0.2],
    [0.4, 0.3, 0.3, 0.6, 0.4, 0.6, 0.3, 0.4],
];

#[derive(Clone, Copy)]
enum Controller {
    FepGradient,
    WellOnly,
    PartnerOnly,
    GreedyEnergy,
    RandomWalk,
    Stationary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RandomWalkRngMode {
    LegacyLcg,
    EventAddressed,
}

impl RandomWalkRngMode {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value {
            Some("event") => Ok(Self::EventAddressed),
            Some("legacy") | None => Ok(Self::LegacyLcg),
            Some(value) => Err(format!(
                "SYMTROPY_GRADIENT_REPLACEMENT_RNG must be 'legacy' or 'event', got {value:?}"
            )),
        }
    }

    fn from_env() -> Result<Self, String> {
        match std::env::var("SYMTROPY_GRADIENT_REPLACEMENT_RNG") {
            Ok(value) => Self::parse(Some(&value)),
            Err(std::env::VarError::NotPresent) => Self::parse(None),
            Err(error) => Err(format!(
                "could not read SYMTROPY_GRADIENT_REPLACEMENT_RNG: {error}"
            )),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::LegacyLcg => "legacy_lcg",
            Self::EventAddressed => "event_addressed_philox4x64_10",
        }
    }
}

const GRADIENT_REPLACEMENT_NAMESPACE: u64 = 0x4752_4144_5245_5031;

fn random_walk_uniform(
    mode: RandomWalkRngMode,
    replicate_seed: u64,
    simulation_tick: u64,
    agent_index: usize,
    legacy_state: &mut u64,
) -> f64 {
    match mode {
        RandomWalkRngMode::LegacyLcg => rng_f64(legacy_state),
        RandomWalkRngMode::EventAddressed => {
            let stable_id = u32::try_from(agent_index + 1)
                .expect("agent index must fit the nonzero u32 subject ID");
            let subject = SubjectId::agent(stable_id)
                .expect("one-based agent identity is nonzero");
            EventAddress::new(
                replicate_seed,
                GRADIENT_REPLACEMENT_NAMESPACE,
                subject,
                simulation_tick,
                registry::RANDOM_WALK,
                registry::RANDOM_WALK_ANGLE,
                0,
            )
            .uniform_open01(0)
        }
    }
}

struct ReplaceResult {
    controller: &'static str,
    seed: u64,
    alive: f64,
    clustering: f64,
    cooperation: f64,
    energy: f64,
}

fn run_experiment(ctrl: Controller, seed: u64, rng_mode: RandomWalkRngMode) -> ReplaceResult {
    let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, 0.0]));
    let mut consciousness = ConsciousnessField::<2>::new();
    consciousness.constants = ThermodynamicConstants::research();

    let wells = vec![SVector::from([30.0, 0.0]), SVector::from([-30.0, 0.0])];
    let mut well_remaining = vec![2500.0f64; 2];
    let mut rng = seed;
    let mut handles = Vec::new();

    for i in 0..AGENTS {
        let x = (rng_f64(&mut rng) - 0.5) * 100.0;
        let y = (rng_f64(&mut rng) - 0.5) * 100.0;
        let h = world.add_sphere(Point::new([x, y]), 1.0, 1.0);
        if let Some(b) = world.body_mut(h) {
            b.linear_damping = 0.05;
        }
        consciousness.register(
            h,
            consciousness.constants.initial_energy,
            consciousness.constants.harmony_range,
        );
        if let Some(e) = consciousness.entities.get_mut(&h) {
            e.harmony_activations = HARMONY_PROFILES[i % 4];
        }
        handles.push(h);
    }

    let name = match ctrl {
        Controller::FepGradient => "FEP_GRADIENT",
        Controller::WellOnly => "WELL_ONLY",
        Controller::PartnerOnly => "PARTNER_ONLY",
        Controller::GreedyEnergy => "GREEDY_ENERGY",
        Controller::RandomWalk => "RANDOM_WALK",
        Controller::Stationary => "STATIONARY",
    };

    let mut coop = 0u64;

    for tick in 0..TICKS {
        // Consciousness update (same for all controllers)
        for &h in &handles {
            let e = consciousness.entities.get(&h);
            let ef = e.map(|e| e.energy.fraction_remaining()).unwrap_or(0.0);
            let pe = e.map(|e| e.prediction_error).unwrap_or(0.0);
            let ht = e.map(|e| e.total_harmony_energy()).unwrap_or(0.0);
            let collapsed = e.map(|e| e.energy.is_collapsed()).unwrap_or(true);
            let inputs = if collapsed {
                ConsciousnessInputs {
                    phi: 0.0,
                    broadcast: 0.0,
                    working_memory: 0.0,
                    attention: 0.0,
                    recurrence: 0.0,
                    embodiment: 0.0,
                    knowledge: 0.0,
                    synchrony: 0.0,
                }
            } else {
                ConsciousnessInputs {
                    phi: ef,
                    broadcast: 0.5,
                    working_memory: (1.0 - pe).max(0.0),
                    attention: 0.5,
                    recurrence: 1.0,
                    embodiment: 0.7,
                    knowledge: (ht / 8.0).min(1.0),
                    synchrony: consciousness.collective_phi.max(0.5),
                }
            };
            consciousness.update_entity(h, &inputs, Point::origin());
        }

        // Movement — THE VARIABLE UNDER TEST
        let adata: Vec<_> = handles
            .iter()
            .filter_map(|&h| {
                Some((
                    world.body(h)?.position().0,
                    consciousness.entities.get(&h)?.harmony_activations,
                ))
            })
            .collect();
        let wdata: Vec<_> = wells
            .iter()
            .zip(well_remaining.iter())
            .filter(|(_, &r)| r > 0.0)
            .map(|(&p, &r)| (p, (r / 2500.0).min(1.0)))
            .collect();

        for (idx, &h) in handles.iter().enumerate() {
            let Some(b) = world.body(h) else { continue };
            let Some(e) = consciousness.entities.get(&h) else {
                continue;
            };
            if e.energy.is_collapsed() {
                continue;
            }
            let pos = b.position().0;

            let vel: SVector<f64, 2> = match ctrl {
                Controller::FepGradient => {
                    let near: Vec<_> = adata
                        .iter()
                        .filter(|(p, _)| {
                            let d = (p - pos).norm();
                            d > 2.0 && d < consciousness.constants.harmony_range
                        })
                        .cloned()
                        .collect();
                    fep_gradient::free_energy_gradient(
                        &pos,
                        e.energy.fraction_remaining(),
                        &e.harmony_activations,
                        &near,
                        &wdata,
                        None,
                        0.0,
                    ) * 20.0
                }

                Controller::WellOnly => {
                    // Move toward nearest available well
                    let mut best_dir = SVector::zeros();
                    let mut best_dist = f64::MAX;
                    for (wp, wr) in &wdata {
                        if *wr < 0.01 {
                            continue;
                        }
                        let delta = wp - pos;
                        let d = delta.norm();
                        if d < best_dist && d > 1.0 {
                            best_dist = d;
                            best_dir = delta / d;
                        }
                    }
                    best_dir * 20.0
                }

                Controller::PartnerOnly => {
                    // Move toward nearest resonant partner, ignore wells
                    let mut best_dir = SVector::zeros();
                    let mut best_score = 0.0f64;
                    for (ap, ah) in &adata {
                        let delta = ap - pos;
                        let d = delta.norm();
                        if d < 2.0 || d > consciousness.constants.harmony_range {
                            continue;
                        }
                        let res = HarmonyField::<2>::resonance(&e.harmony_activations, ah);
                        if res > best_score {
                            best_score = res;
                            best_dir = delta / d;
                        }
                    }
                    best_dir * 20.0
                }

                Controller::GreedyEnergy => {
                    // Sample 8 directions, pick the one that would maximize energy gain
                    let mut best_dir = SVector::zeros();
                    let mut best_gain = f64::NEG_INFINITY;
                    for angle_idx in 0..8 {
                        let angle = angle_idx as f64 * std::f64::consts::TAU / 8.0;
                        let test_dir = SVector::from([angle.cos(), angle.sin()]);
                        let test_pos = pos + test_dir * 5.0; // look 5 units ahead

                        // Estimate energy gain at test position
                        let mut gain = 0.0;
                        // Well proximity bonus
                        for (wp, wr) in &wdata {
                            if *wr < 0.01 {
                                continue;
                            }
                            let d = (test_pos - wp).norm();
                            if d < 35.0 {
                                gain += consciousness.constants.energy_well_regen_rate * wr;
                            }
                        }
                        // Resonance proximity bonus
                        for (ap, ah) in &adata {
                            let d = (test_pos - ap).norm();
                            if d < 2.0 || d > consciousness.constants.harmony_range {
                                continue;
                            }
                            let res = HarmonyField::<2>::resonance(&e.harmony_activations, ah);
                            if res > 0.5 {
                                gain += consciousness.constants.harmony_resonance_regen_rate
                                    * (res - 0.5)
                                    * 2.0;
                            }
                        }
                        if gain > best_gain {
                            best_gain = gain;
                            best_dir = test_dir;
                        }
                    }
                    best_dir * 20.0
                }

                Controller::RandomWalk => {
                    let uniform = random_walk_uniform(
                        rng_mode,
                        seed,
                        tick as u64,
                        idx,
                        &mut rng,
                    );
                    let angle = uniform * std::f64::consts::TAU;
                    SVector::from([angle.cos() * 20.0, angle.sin() * 20.0])
                }

                Controller::Stationary => SVector::zeros(),
            };

            if let Some(b) = world.body_mut(h) {
                b.linear_velocity = vel;
            }
        }

        // Maintenance + regen (IDENTICAL for all controllers)
        let rm = consciousness.resource_regeneration_multiplier();
        let mr = consciousness.constants.consciousness_maintenance_per_tick;
        let ar = consciousness.constants.ambient_regen_rate;
        let wr = consciousness.constants.energy_well_regen_rate;
        for &h in handles.iter() {
            if let Some(e) = consciousness.entities.get_mut(&h) {
                e.energy.tick_reset();
            }
            consciousness.consume_energy(h, mr * (1.0 + consciousness.phi(h) * 0.5));
            if let Some(e) = consciousness.entities.get_mut(&h) {
                e.energy.regenerate(ar * rm);
                if let Some(b) = world.body(h) {
                    let pos = b.position().0;
                    for (wi, &w) in wells.iter().enumerate() {
                        if (pos - w).norm() < 35.0 && well_remaining[wi] > 0.0 {
                            let d = wr.min(well_remaining[wi]);
                            e.energy.regenerate(d);
                            well_remaining[wi] -= d;
                            break;
                        }
                    }
                }
            }
        }

        // Cooperation (IDENTICAL for all controllers — passive, not chosen)
        for i in 0..handles.len() {
            for j in (i + 1)..handles.len() {
                let (ha, hb) = (handles[i], handles[j]);
                let in_range = match (world.body(ha), world.body(hb)) {
                    (Some(a), Some(b)) => {
                        a.position().distance(b.position()) < consciousness.constants.harmony_range
                    }
                    _ => false,
                };
                if !in_range {
                    continue;
                }
                let (a, b) = match (
                    consciousness.entities.get(&ha),
                    consciousness.entities.get(&hb),
                ) {
                    (Some(a), Some(b)) => (a.harmony_activations, b.harmony_activations),
                    _ => continue,
                };
                let res = HarmonyField::<2>::resonance(&a, &b);
                if res > 0.5 {
                    let rg =
                        consciousness.constants.harmony_resonance_regen_rate * (res - 0.5) * 2.0;
                    if let Some(e) = consciousness.entities.get_mut(&ha) {
                        e.energy.regenerate(rg);
                    }
                    if let Some(e) = consciousness.entities.get_mut(&hb) {
                        e.energy.regenerate(rg);
                    }
                    coop += 1;
                }
            }
        }

        consciousness.tick_prediction_errors();
        world.step_with_callback(DT, &mut consciousness);
        consciousness.tick_thermodynamics();
    }

    let alive = handles
        .iter()
        .filter(|h| {
            consciousness
                .entities
                .get(h)
                .map(|e| !e.energy.is_collapsed())
                .unwrap_or(false)
        })
        .count() as f64;
    let energy = handles
        .iter()
        .filter_map(|h| consciousness.entities.get(h).map(|e| e.energy.available))
        .sum::<f64>()
        / AGENTS as f64;
    let pos: Vec<SVector<f64, 2>> = handles
        .iter()
        .filter_map(|h| world.body(*h).map(|b| b.position().0))
        .collect();
    let clustering = if pos.len() >= 2 {
        pos.iter()
            .enumerate()
            .map(|(i, p)| {
                pos.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, q)| (p - q).norm())
                    .fold(f64::MAX, f64::min)
            })
            .sum::<f64>()
            / pos.len() as f64
    } else {
        f64::MAX
    };

    ReplaceResult {
        controller: name,
        seed,
        alive,
        energy,
        clustering: if clustering.is_finite() {
            clustering
        } else {
            0.0
        },
        cooperation: coop as f64,
    }
}

fn synthetic_controller_results() -> Vec<(&'static str, Vec<ReplaceResult>)> {
    const CONTROLLERS: [&str; 6] = [
        "FEP_GRADIENT",
        "WELL_ONLY",
        "PARTNER_ONLY",
        "GREEDY_ENERGY",
        "RANDOM_WALK",
        "STATIONARY",
    ];

    CONTROLLERS
        .iter()
        .copied()
        .enumerate()
        .map(|(controller_index, name)| {
            let offset = controller_index as f64 * 0.2;
            let results = (0..SEEDS)
                .map(|seed_index| {
                    let seed = 42 + seed_index as u64 * 997;
                    let replicate = seed_index as f64;
                    ReplaceResult {
                        controller: name,
                        seed,
                        alive: 8.0 + (seed_index % 5) as f64 + offset,
                        clustering: 2.0 + replicate * 0.05 + offset,
                        cooperation: 100.0 + replicate * 3.0 + offset * 10.0,
                        energy: 200.0 + replicate * 0.75 + offset * 2.0,
                    }
                })
                .collect();
            (name, results)
        })
        .collect()
}

fn main() {
    let rng_mode = match RandomWalkRngMode::from_env() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    eprintln!("random_walk_rng_mode={}", rng_mode.label());

    if std::env::args().any(|argument| argument == "--receipt-smoke") {
        let synthetic = synthetic_controller_results();
        emit_paired_controller_receipts(&synthetic, rng_mode);
        return;
    }

    let seed_schedule = match ExperimentSeedSchedule::from_env(SEEDS) {
        Ok(schedule) => schedule,
        Err(error) => {
            eprintln!("Invalid experiment seed schedule: {error}");
            std::process::exit(2);
        }
    };
    if seed_schedule.is_overridden() {
        eprintln!("Seed schedule: {}", seed_schedule.describe());
    }
    eprintln!("=== Gradient Replacement Study ===");
    eprintln!("Is cooperation controller-invariant?");
    eprintln!(
        "{AGENTS} agents, {TICKS} ticks, {} seeds, 6 controllers",
        seed_schedule.len()
    );

    println!("controller,seed,alive,energy,clustering,cooperation");

    let controllers = [
        (Controller::FepGradient, "FEP_GRADIENT"),
        (Controller::WellOnly, "WELL_ONLY"),
        (Controller::PartnerOnly, "PARTNER_ONLY"),
        (Controller::GreedyEnergy, "GREEDY_ENERGY"),
        (Controller::RandomWalk, "RANDOM_WALK"),
        (Controller::Stationary, "STATIONARY"),
    ];
    let mut all: Vec<(&str, Vec<ReplaceResult>)> = Vec::new();

    for &(ctrl, name) in &controllers {
        let mut results = Vec::new();
        for &seed in seed_schedule.seeds() {
            eprintln!("  {name} seed={seed}...");
            let r = run_experiment(ctrl, seed, rng_mode);
            println!(
                "{},{seed},{:.1},{:.1},{:.2},{:.0}",
                r.controller, r.alive, r.energy, r.clustering, r.cooperation
            );
            results.push(r);
        }
        let n = results.len() as f64;
        eprintln!(
            "  → {name}: alive={:.1}, cluster={:.2}, coop={:.0}",
            results.iter().map(|r| r.alive).sum::<f64>() / n,
            results.iter().map(|r| r.clustering).sum::<f64>() / n,
            results.iter().map(|r| r.cooperation).sum::<f64>() / n
        );
        all.push((name, results));
    }

    // Summary
    eprintln!("\n── Controller Comparison ──");
    eprintln!("  Controller       Alive  Cluster  Cooperation  Energy");
    for (name, results) in &all {
        let n = results.len() as f64;
        eprintln!(
            "  {:15} {:5.1}  {:6.2}   {:9.0}    {:5.1}J",
            name,
            results.iter().map(|r| r.alive).sum::<f64>() / n,
            results.iter().map(|r| r.clustering).sum::<f64>() / n,
            results.iter().map(|r| r.cooperation).sum::<f64>() / n,
            results.iter().map(|r| r.energy).sum::<f64>() / n
        );
    }

    emit_paired_controller_receipts(&all, rng_mode);

    // THE KEY QUESTION
    let controllers_with_coop: Vec<&str> = all
        .iter()
        .filter(|(_, results)| {
            let mean_coop =
                results.iter().map(|r| r.cooperation).sum::<f64>() / results.len() as f64;
            mean_coop > 50_000.0 // substantial cooperation
        })
        .map(|(name, _)| *name)
        .collect();

    eprintln!("\n── VERDICT ──");
    eprintln!(
        "  Controllers producing substantial cooperation: {:?}",
        controllers_with_coop
    );
    if controllers_with_coop.len() >= 4 {
        eprintln!(
            "  CONTROLLER-INVARIANT: Cooperation emerges across {} of 6 controllers.",
            controllers_with_coop.len()
        );
        eprintln!(
            "  The thesis is UNIVERSAL — cooperation depends on thermodynamics, not the gradient."
        );
    } else if controllers_with_coop.len() >= 2 {
        eprintln!(
            "  PARTIALLY INVARIANT: Cooperation emerges under {} controllers.",
            controllers_with_coop.len()
        );
        eprintln!("  The gradient matters but is not uniquely privileged.");
    } else {
        eprintln!("  CONTROLLER-DEPENDENT: Only the FEP gradient produces cooperation.");
        eprintln!("  The thesis is NARROW — the gradient IS the mechanism, not thermodynamics.");
    }

    eprintln!("\n=== Complete ===");
}

fn seeded_outcome_values(results: &[ReplaceResult], outcome: &str) -> Vec<SeededValue> {
    results
        .iter()
        .map(|result| SeededValue {
            seed_id: result.seed,
            value: match outcome {
                "alive" => result.alive,
                "energy" => result.energy,
                "clustering" => result.clustering,
                "cooperation" => result.cooperation,
                _ => panic!("unknown gradient-replacement outcome: {outcome}"),
            },
        })
        .collect()
}

fn emit_paired_controller_receipts(
    all: &[(&str, Vec<ReplaceResult>)],
    rng_mode: RandomWalkRngMode,
) {
    const OUTCOMES: [&str; 4] = ["alive", "energy", "clustering", "cooperation"];
    const BOOTSTRAP_REPLICATES: usize = 20_000;
    const BOOTSTRAP_SEED_BASE: u64 = 0x4752_5041_4952_0001;

    let fep = &all[0].1;
    println!("paired_analysis_unit=complete_simulation_seed");
    println!("paired_analysis_key=actual_seed_value");
    println!("paired_estimand=controller_minus_FEP_GRADIENT");
    println!("paired_bootstrap=whole_seed_pair_percentile_20000");
    println!("paired_p_values=NOT_EVALUATED_SIGN_EXCHANGEABILITY_UNQUALIFIED");
    println!("paired_equivalence=NOT_EVALUATED_NO_PREREGISTERED_MARGIN");

    for (controller_index, (name, results)) in all.iter().enumerate().skip(1) {
        for (outcome_index, outcome) in OUTCOMES.iter().copied().enumerate() {
            let reference_values = seeded_outcome_values(fep, outcome);
            let candidate_values = seeded_outcome_values(results, outcome);
            let pairs = pair_by_seed(&reference_values, &candidate_values)
                .expect("all controllers must share the exact unique seed schedule");
            let summary = summarize_paired(&pairs)
                .expect("non-empty matched seed pairs must be summarizable");
            let rng_seed = BOOTSTRAP_SEED_BASE
                + ((controller_index - 1) * OUTCOMES.len() + outcome_index) as u64;
            let interval = bootstrap_mean_interval(
                &pairs,
                BootstrapConfig {
                    replicates: BOOTSTRAP_REPLICATES,
                    confidence_level: 0.95,
                    rng_seed,
                },
            )
            .expect("fixed bootstrap contract is valid");
            let seed_ids = pairs
                .iter()
                .map(|pair| pair.seed_id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let sd = summary
                .sample_standard_deviation
                .map(|value| format!("{value:.17e}"))
                .unwrap_or_else(|| "null".to_string());
            let se = summary
                .standard_error
                .map(|value| format!("{value:.17e}"))
                .unwrap_or_else(|| "null".to_string());
            let dz = summary
                .cohens_dz
                .map(|value| format!("{value:.17e}"))
                .unwrap_or_else(|| "null".to_string());

            println!(
                "PAIRED_CONTROLLER_JSON={{\"schema\":\"luminous.symtropy.gradient-replacement-paired-receipt.v0.1\",\"rng_mode\":\"{}\",\"reference\":\"FEP_GRADIENT\",\"controller\":\"{}\",\"outcome\":\"{}\",\"estimand\":\"controller_minus_FEP_GRADIENT\",\"pairing_key\":\"actual_seed_value\",\"seed_ids\":[{}],\"n\":{},\"mean_difference\":{:.17e},\"median_difference\":{:.17e},\"sample_standard_deviation\":{},\"standard_error\":{},\"cohens_dz\":{},\"bootstrap\":{{\"method\":\"percentile_whole_seed_pair\",\"confidence_level\":0.95,\"lower\":{:.17e},\"upper\":{:.17e},\"replicates\":{},\"rng\":\"SplitMix64\",\"rng_seed\":{}}},\"sign_flip\":{{\"status\":\"not_evaluated\",\"reason\":\"seed_pairing_does_not_establish_sign_exchangeability\"}},\"equivalence\":{{\"status\":\"not_evaluated\",\"reason\":\"no_preregistered_outcome_margin\"}}}}",
                rng_mode.label(),
                name,
                outcome,
                seed_ids,
                summary.n,
                summary.mean_difference,
                summary.median_difference,
                sd,
                se,
                dz,
                interval.lower,
                interval.upper,
                interval.replicates,
                interval.rng_seed,
            );
        }
    }
}

fn rng_f64(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod event_random_walk_tests {
    use super::*;

    #[test]
    fn event_addressed_random_walk_is_repeatable_for_seed_agent_and_tick() {
        let mut legacy_a = 42;
        let mut legacy_b = 42;
        let first = random_walk_uniform(
            RandomWalkRngMode::EventAddressed, 42, 17, 3, &mut legacy_a
        );
        let second = random_walk_uniform(
            RandomWalkRngMode::EventAddressed, 42, 17, 3, &mut legacy_b
        );
        assert_eq!(first.to_bits(), second.to_bits());
        assert_eq!(legacy_a, 42);
        assert_eq!(legacy_b, 42);
    }

    #[test]
    fn event_addressed_random_walk_separates_replicate_agent_and_tick() {
        let sample = |seed, tick, agent| {
            let mut legacy_state = seed;
            random_walk_uniform(
                RandomWalkRngMode::EventAddressed,
                seed,
                tick,
                agent,
                &mut legacy_state,
            )
        };
        let baseline = sample(42, 17, 3);
        assert_ne!(baseline.to_bits(), sample(1_039, 17, 3).to_bits());
        assert_ne!(baseline.to_bits(), sample(42, 18, 3).to_bits());
        assert_ne!(baseline.to_bits(), sample(42, 17, 4).to_bits());
    }

    #[test]
    fn event_addressed_random_walk_uniforms_are_strictly_open() {
        for seed in [42, 1_039, u64::MAX] {
            for tick in [0, 1, 17, (TICKS - 1) as u64] {
                for agent in 0..AGENTS {
                    let mut legacy_state = seed;
                    let value = random_walk_uniform(
                        RandomWalkRngMode::EventAddressed,
                        seed,
                        tick,
                        agent,
                        &mut legacy_state,
                    );
                    assert!(value > 0.0 && value < 1.0);
                    assert_eq!(legacy_state, seed);
                }
            }
        }
    }

    #[test]
    fn legacy_random_walk_still_consumes_the_historical_lcg_stream() {
        let mut actual_state = 42;
        let mut expected_state = 42;
        let actual = random_walk_uniform(
            RandomWalkRngMode::LegacyLcg, 42, 17, 3, &mut actual_state
        );
        let expected = rng_f64(&mut expected_state);
        assert_eq!(actual.to_bits(), expected.to_bits());
        assert_eq!(actual_state, expected_state);
    }

    #[test]
    fn missing_rng_mode_defaults_to_legacy_and_invalid_values_fail_closed() {
        assert_eq!(
            RandomWalkRngMode::parse(None).unwrap(),
            RandomWalkRngMode::LegacyLcg
        );
        assert_eq!(
            RandomWalkRngMode::parse(Some("event")).unwrap(),
            RandomWalkRngMode::EventAddressed
        );
        assert_eq!(
            RandomWalkRngMode::parse(Some("legacy")).unwrap(),
            RandomWalkRngMode::LegacyLcg
        );
        assert!(RandomWalkRngMode::parse(Some("anything-else")).is_err());
    }
}

