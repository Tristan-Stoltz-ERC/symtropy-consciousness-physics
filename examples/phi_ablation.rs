// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! SCALAR CONTROL ABLATION: How do alternative scalar inputs compare?
//!
//! Tests whether clustering depends on the supplied scalar input or whether
//! entropy, random, constant, and zero controls produce similar outcomes.
//!
//! 5 scalar controls × 30 matched seeds × 3000 ticks = 150 runs
//!
//! Conditions:
//! 1. ENTROPY:       Shannon entropy proxy of agent energy fraction
//! 2. RANDOM_LEGACY: Historical deterministic control, seed-independent by design
//! 3. RANDOM_EVENT:  Philox event-addressed control keyed by replicate/agent/tick
//! 4. CONSTANT:     Fixed value 0.5
//! 5. ZERO:          No modulation
//!
//! This is not a test of the actual IIT-inspired Φ equation. The former PHI
//! branch returned the constant 0.5 and duplicated CONSTANT exactly; it has
//! been removed rather than reported as an independent scientific condition.

use std::collections::BTreeMap;

use nalgebra::SVector;
use symtropy_consciousness_physics::fep_gradient;
use symtropy_consciousness_physics::event_rng::{
    registry, EventAddress, SubjectId,
};
use symtropy_consciousness_physics::harmony_field::HarmonyField;
use symtropy_consciousness_physics::paired_inference::{
    bootstrap_mean_interval, pair_by_seed, summarize_paired, BootstrapConfig, SeededValue,
};
use symtropy_consciousness_physics::{ConsciousnessField, ThermodynamicConstants};
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;
use symthaea_consciousness_equation::ConsciousnessInputs;

const AGENTS: usize = 12;
const TICKS: usize = 3000;
const SEEDS: usize = 30;

#[derive(Clone, Copy)]
enum ScalarControl {
    Entropy,
    RandomLegacy,
    RandomEvent,
    Constant,
    Zero,
}

impl ScalarControl {
    fn name(&self) -> &'static str {
        match self {
            Self::Entropy => "ENTROPY_PROXY",
            Self::RandomLegacy => "RANDOM_LEGACY",
            Self::RandomEvent => "RANDOM_EVENT",
            Self::Constant => "CONST_0.5",
            Self::Zero => "ZERO",
        }
    }

    fn compute(&self, energy_frac: f64, tick: u64, agent_idx: usize, seed: u64) -> f64 {
        match self {
            Self::Entropy => {
                // Shannon entropy proxy: H = -p*ln(p) where p = energy_fraction
                let p = energy_frac.clamp(0.01, 0.99);
                -(p * p.ln() + (1.0 - p) * (1.0 - p).ln()) / (2.0f64).ln()
            }
            Self::RandomLegacy => legacy_random_control(tick, agent_idx),
            Self::RandomEvent => event_random_control(seed, tick, agent_idx),
            Self::Constant => 0.5,
            Self::Zero => 0.0,
        }
    }
}

struct RunResult { clustering: f64, alive: usize, avg_energy: f64 }

#[derive(Default)]
struct ConditionSamples {
    clustering: Vec<SeededValue>,
    alive: Vec<SeededValue>,
    energy: Vec<SeededValue>,
}

impl ConditionSamples {
    fn values(&self, metric: &str) -> &[SeededValue] {
        match metric {
            "clustering" => &self.clustering,
            "alive" => &self.alive,
            "energy" => &self.energy,
            _ => panic!("unknown scalar-ablation metric: {metric}"),
        }
    }
}

const PHI_ABLATION_NAMESPACE: u64 = 0x5048_495f_4142_4c31;

fn legacy_random_control(tick: u64, agent_idx: usize) -> f64 {
    let mut state = tick
        .wrapping_mul(6364136223846793005)
        .wrapping_add(agent_idx as u64 * 1442695040888963407);
    state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    (state >> 11) as f64 / (1_u64 << 53) as f64
}

fn event_random_control(replicate_seed: u64, tick: u64, agent_idx: usize) -> f64 {
    let stable_agent_id = u32::try_from(agent_idx + 1)
        .expect("agent index must fit the stable nonzero u32 subject ID");
    let subject = SubjectId::agent(stable_agent_id)
        .expect("stable agent ID is nonzero");
    EventAddress::new(
        replicate_seed,
        PHI_ABLATION_NAMESPACE,
        subject,
        tick,
        registry::PHI_ABLATION_CONTROL,
        registry::RANDOM_METRIC_SAMPLE,
        0,
    )
    .uniform_open01(0)
}

fn emit_paired_analysis_receipts(
    samples_by_condition: &BTreeMap<&'static str, ConditionSamples>,
) {
    // These contrasts are fixed before examining the result. The main target
    // is whether the replicate-keyed random control differs from the legacy
    // seed-invariant control; constant and entropy comparisons are secondary.
    const COMPARISONS: [(&str, &str); 3] = [
        ("RANDOM_LEGACY", "RANDOM_EVENT"),
        ("CONST_0.5", "RANDOM_EVENT"),
        ("CONST_0.5", "ENTROPY_PROXY"),
    ];
    const OUTCOMES: [&str; 3] = ["clustering", "alive", "energy"];
    const BOOTSTRAP_REPLICATES: usize = 20_000;
    const BOOTSTRAP_SEED_BASE: u64 = 0x5343_414c_4152_0001;

    println!("paired_analysis_unit=complete_seed_replicate");
    println!("paired_analysis_key=actual_seed_value");
    println!("paired_analysis_estimand=treatment_minus_control");
    println!("paired_bootstrap_method=percentile_resample_whole_seed_pairs");
    println!("paired_bootstrap_replicates={BOOTSTRAP_REPLICATES}");
    println!("paired_p_values=NOT_EVALUATED_SIGN_EXCHANGEABILITY_UNQUALIFIED");
    println!("paired_equivalence=NOT_EVALUATED_NO_PREREGISTERED_MARGIN");

    for (comparison_index, (control_name, treatment_name)) in
        COMPARISONS.iter().copied().enumerate()
    {
        let control = samples_by_condition
            .get(control_name)
            .expect("preregistered control condition must be present");
        let treatment = samples_by_condition
            .get(treatment_name)
            .expect("preregistered treatment condition must be present");

        for (metric_index, metric_name) in OUTCOMES.iter().copied().enumerate() {
            let pairs = pair_by_seed(
                control.values(metric_name),
                treatment.values(metric_name),
            )
            .expect("conditions must contain the same unique finite seed keys");
            let summary = summarize_paired(&pairs)
                .expect("non-empty matched seed pairs must be summarizable");
            let bootstrap_seed =
                BOOTSTRAP_SEED_BASE + (comparison_index * OUTCOMES.len() + metric_index) as u64;
            let interval = bootstrap_mean_interval(
                &pairs,
                BootstrapConfig {
                    replicates: BOOTSTRAP_REPLICATES,
                    confidence_level: 0.95,
                    rng_seed: bootstrap_seed,
                },
            )
            .expect("fixed bootstrap contract must be valid");

            let seed_ids = pairs
                .iter()
                .map(|pair| pair.seed_id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let cohens_dz = summary
                .cohens_dz
                .map(|value| format!("{value:.17e}"))
                .unwrap_or_else(|| "null".to_string());

            // JSON lines are append-only analysis receipts. No p-value or
            // equivalence verdict is emitted without their design assumptions.
            println!(
                "PAIRED_ANALYSIS_JSON={{\"schema\":\"luminous.symtropy.paired-ablation-receipt.v0.1\",\"control\":\"{control_name}\",\"treatment\":\"{treatment_name}\",\"metric\":\"{metric_name}\",\"independent_unit\":\"complete_seed_replicate\",\"pairing_key\":\"actual_seed_value\",\"estimand\":\"treatment_minus_control\",\"seed_ids\":[{seed_ids}],\"n\":{},\"mean_difference\":{:.17e},\"median_difference\":{:.17e},\"sample_standard_deviation\":{},\"standard_error\":{},\"cohens_dz\":{},\"bootstrap\":{{\"method\":\"percentile_whole_pair_resampling\",\"confidence_level\":0.95,\"lower\":{:.17e},\"upper\":{:.17e},\"replicates\":{},\"rng\":\"SplitMix64_non_cryptographic\",\"rng_seed\":{}}},\"sign_flip\":{{\"status\":\"not_evaluated\",\"reason\":\"sign_exchangeability_or_randomized_pair_assignment_not_established\"}},\"equivalence\":{{\"status\":\"not_evaluated\",\"reason\":\"no_preregistered_metric_specific_margin\"}}}}",
                summary.n,
                summary.mean_difference,
                summary.median_difference,
                summary.sample_standard_deviation
                    .map(|value| format!("{value:.17e}"))
                    .unwrap_or_else(|| "null".to_string()),
                summary.standard_error
                    .map(|value| format!("{value:.17e}"))
                    .unwrap_or_else(|| "null".to_string()),
                cohens_dz,
                interval.lower,
                interval.upper,
                interval.replicates,
                interval.rng_seed,
            );
        }
    }
}

fn run(metric: ScalarControl, seed: u64) -> RunResult {
    let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, 0.0]));
    let mut consciousness = ConsciousnessField::<2>::new();
    consciousness.constants = ThermodynamicConstants {
        initial_energy: 400.0, max_energy: 400.0,
        consciousness_maintenance_per_tick: 0.12, movement_cost_per_unit: 0.008,
        sprint_cost_multiplier: 2.5, collision_energy_drain: 0.05,
        harmony_resonance_regen_rate: 0.12, energy_well_regen_rate: 0.25,
        ambient_regen_rate: 0.02, collapse_recovery_harmony_threshold: 0.5, harmony_range: 40.0,
    };

    let mut rng = seed;
    let mut handles = Vec::new();
    for i in 0..AGENTS {
        let x = (nrng(&mut rng) - 0.5) * 100.0;
        let y = (nrng(&mut rng) - 0.5) * 100.0;
        let h = world.add_sphere(Point::new([x, y]), 1.0, 1.0);
        if let Some(b) = world.body_mut(h) {
            b.linear_damping = 0.2;
            b.linear_velocity = SVector::from([(nrng(&mut rng)-0.5)*10.0, (nrng(&mut rng)-0.5)*10.0]);
        }
        consciousness.register(h, 400.0, 20.0);
        if let Some(e) = consciousness.entities.get_mut(&h) {
            match i % 4 {
                0 => e.harmony_activations = [0.9,0.1,0.1,0.1,0.1,0.1,0.1,0.8],
                1 => e.harmony_activations = [0.1,0.9,0.1,0.1,0.1,0.1,0.8,0.1],
                2 => e.harmony_activations = [0.1,0.1,0.9,0.1,0.1,0.8,0.1,0.1],
                _ => e.harmony_activations = [0.4;8],
            }
        }
        handles.push(h);
    }

    for tick in 0..TICKS {
        // Apply the selected scalar control as ConsciousnessInputs.phi
        for (idx, &h) in handles.iter().enumerate() {
            let ef = consciousness.entities.get(&h).map(|e| e.energy.fraction_remaining()).unwrap_or(0.0);
            let phi_value = metric.compute(ef, tick as u64, idx, seed);
            let inputs = ConsciousnessInputs {
                phi: phi_value, broadcast: 0.6, working_memory: 0.5, attention: 0.5,
                recurrence: 0.4, embodiment: 0.6, knowledge: 0.4, synchrony: 0.5,
            };
            consciousness.update_entity(h, &inputs, Point::origin());
        }

        // FEP gradient movement
        let agent_data: Vec<_> = handles.iter().filter_map(|&h| {
            Some((world.body(h)?.position().0, consciousness.entities.get(&h)?.harmony_activations))
        }).collect();
        for (idx, &h) in handles.iter().enumerate() {
            if consciousness.entities.get(&h).map(|e| e.energy.is_collapsed()).unwrap_or(true) { continue; }
            let pos = match world.body(h) { Some(b) => b.position().0, None => continue };
            let ef = consciousness.entities.get(&h).map(|e| e.energy.fraction_remaining()).unwrap_or(0.0);
            let harm = consciousness.entities.get(&h).map(|e| e.harmony_activations).unwrap_or([0.5;8]);
            let nearby: Vec<_> = agent_data.iter().enumerate().filter(|(i,_)| *i!=idx).map(|(_,d)| d.clone()).collect();
            let dir = fep_gradient::free_energy_gradient(&pos, ef, &harm, &nearby, &[], None, 0.0);
            if let Some(b) = world.body_mut(h) { b.linear_velocity = dir * 30.0; }
        }

        // Maintenance + offloading
        let rm = consciousness.resource_regeneration_multiplier();
        for &h in &handles {
            if let Some(e) = consciousness.entities.get_mut(&h) {
                e.energy.tick_reset();
                let phi = e.phi();
                e.energy.consume(consciousness.constants.consciousness_maintenance_per_tick * (1.0 + phi * 0.5));
                e.energy.regenerate(consciousness.constants.ambient_regen_rate * rm);
            }
        }
        for i in 0..handles.len() {
            for j in (i+1)..handles.len() {
                let (ha, hb) = (handles[i], handles[j]);
                let dist = match (world.body(ha), world.body(hb)) { (Some(a),Some(b)) => a.position().distance(b.position()), _ => continue };
                if dist > consciousness.constants.harmony_range { continue; }
                let (ha_h, hb_h) = match (consciousness.entities.get(&ha), consciousness.entities.get(&hb)) {
                    (Some(a),Some(b)) => (a.harmony_activations, b.harmony_activations), _ => continue };
                let res = HarmonyField::<2>::resonance(&ha_h, &hb_h);
                if res > 0.5 {
                    let off = (res-0.5)*2.0;
                    if let Some(e) = consciousness.entities.get_mut(&ha) {
                        e.prediction_error *= 1.0-off*0.1; e.motor_precision = 1.0/(1.0+e.prediction_error);
                        e.energy.regenerate(consciousness.constants.consciousness_maintenance_per_tick * off * 0.5);
                    }
                    if let Some(e) = consciousness.entities.get_mut(&hb) {
                        e.prediction_error *= 1.0-off*0.1; e.motor_precision = 1.0/(1.0+e.prediction_error);
                        e.energy.regenerate(consciousness.constants.consciousness_maintenance_per_tick * off * 0.5);
                    }
                }
            }
        }
        consciousness.tick_prediction_errors();
        world.step_with_callback(1.0/64.0, &mut consciousness);
        consciousness.tick_thermodynamics();
    }

    let alive = handles.iter().filter(|h| consciousness.entities.get(h).map(|e| !e.energy.is_collapsed()).unwrap_or(false)).count();
    let avg_e = handles.iter().filter_map(|h| consciousness.entities.get(h).map(|e| e.energy.available)).sum::<f64>() / AGENTS as f64;
    let mut tn = 0.0;
    for i in 0..handles.len() {
        let mut md = f64::MAX;
        for j in 0..handles.len() { if i==j {continue;} if let (Some(a),Some(b)) = (world.body(handles[i]),world.body(handles[j])) { let d=a.position().distance(b.position()); if d<md {md=d;} } }
        tn += md;
    }
    RunResult { clustering: tn / AGENTS as f64, alive, avg_energy: avg_e }
}

fn synthetic_receipt_samples() -> BTreeMap<&'static str, ConditionSamples> {
    const CONDITIONS: [&str; 5] = [
        "ENTROPY_PROXY",
        "RANDOM_LEGACY",
        "RANDOM_EVENT",
        "CONST_0.5",
        "ZERO",
    ];

    let mut samples_by_condition = BTreeMap::new();
    for (condition_index, condition_name) in CONDITIONS.iter().copied().enumerate() {
        let condition_offset = condition_index as f64 * 0.25;
        let mut samples = ConditionSamples::default();
        for seed_index in 0..SEEDS {
            let seed_id = 42 + seed_index as u64 * 997;
            let replicate = seed_index as f64;
            samples.clustering.push(SeededValue {
                seed_id,
                value: 1.0 + replicate * 0.125 + condition_offset,
            });
            samples.alive.push(SeededValue {
                seed_id,
                value: (seed_index % 7) as f64 + condition_offset,
            });
            samples.energy.push(SeededValue {
                seed_id,
                value: 100.0 + replicate * 0.75 + condition_offset * 2.0,
            });
        }
        samples_by_condition.insert(condition_name, samples);
    }
    samples_by_condition
}

fn main() {
    if std::env::args().any(|argument| argument == "--receipt-smoke") {
        let synthetic = synthetic_receipt_samples();
        emit_paired_analysis_receipts(&synthetic);
        return;
    }

    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  SCALAR CONTROL ABLATION: Event-addressed randomness ║");
    println!("╚══════════════════════════════════════════════════════╝\n");
    println!("{} agents, {} ticks, {} seeds per condition", AGENTS, TICKS, SEEDS);
    println!("initial_state_rng=historical_lcg_seeded_per_replicate");
    println!("random_event_rng=Philox4x64-10");
    println!("random_event_key=(replicate_seed, namespace)");
    println!("random_event_counter=(stable_agent_id, tick, event_kind, purpose, occurrence)");
    println!("actual_phi_equation=NOT_MEASURED");
    println!("phi_input_semantics=supplied_scalar_not_equation_output");
    println!("legacy_random_control=does_not_include_replicate_seed");
    println!();

    let metrics = [
        ScalarControl::Entropy,
        ScalarControl::RandomLegacy,
        ScalarControl::RandomEvent,
        ScalarControl::Constant,
        ScalarControl::Zero,
    ];

    println!("{:<12} {:<12} {:<12} {:<12} {:<12}", "Metric", "Clustering", "95% CI", "Alive", "Energy");
    println!("{}", "─".repeat(60));

    let mut samples_by_condition = BTreeMap::<&'static str, ConditionSamples>::new();

    for metric in &metrics {
        let mut samples = ConditionSamples::default();

        for s in 0..SEEDS {
            let seed_id = 42 + s as u64 * 997;
            let result = run(*metric, seed_id);
            samples.clustering.push(SeededValue {
                seed_id,
                value: result.clustering,
            });
            samples.alive.push(SeededValue {
                seed_id,
                value: result.alive as f64,
            });
            samples.energy.push(SeededValue {
                seed_id,
                value: result.avg_energy,
            });
        }

        let clusterings = samples
            .clustering
            .iter()
            .map(|sample| sample.value)
            .collect::<Vec<_>>();
        let alives = samples
            .alive
            .iter()
            .map(|sample| sample.value)
            .collect::<Vec<_>>();
        let energies = samples
            .energy
            .iter()
            .map(|sample| sample.value)
            .collect::<Vec<_>>();

        let mean_c = clusterings.iter().sum::<f64>() / SEEDS as f64;
        let std_c = (clusterings.iter().map(|x| (x - mean_c).powi(2)).sum::<f64>() / (SEEDS as f64 - 1.0)).sqrt();
        let ci = 1.96 * std_c / (SEEDS as f64).sqrt(); // Descriptive per-condition normal interval.
        let mean_a = alives.iter().sum::<f64>() / SEEDS as f64;
        let mean_e = energies.iter().sum::<f64>() / SEEDS as f64;

        println!("{:<12} {:<12.2} ±{:<10.2} {:<12.1} {:<12.1}",
            metric.name(), mean_c, ci, mean_a, mean_e);

        samples_by_condition.insert(metric.name(), samples);
    }

    emit_paired_analysis_receipts(&samples_by_condition);

    println!("\n═══════════════════════════════════════════════════════");
    println!("No equation-level Phi measurement is included in this harness.");
    println!("Compare RANDOM_LEGACY with RANDOM_EVENT to audit seed-keyed noise.");
    println!("All conditions share the same seed-indexed initial-state schedule.");
    println!("═══════════════════════════════════════════════════════");
}

fn nrng(s: &mut u64) -> f64 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (*s >> 11) as f64 / (1u64 << 53) as f64 }

#[cfg(test)]
mod event_random_control_tests {
    use super::*;

    #[test]
    fn legacy_control_depends_only_on_tick_and_agent_identity() {
        assert_eq!(legacy_random_control(17, 3), legacy_random_control(17, 3));
    }

    #[test]
    fn event_random_control_is_repeatable_for_one_replicate_agent_and_tick() {
        let first = event_random_control(42, 17, 3);
        let second = event_random_control(42, 17, 3);
        assert_eq!(first.to_bits(), second.to_bits());
    }

    #[test]
    fn event_random_control_separates_replicates_agents_and_ticks() {
        let baseline = event_random_control(42, 17, 3);
        assert_ne!(baseline.to_bits(), event_random_control(1_039, 17, 3).to_bits());
        assert_ne!(baseline.to_bits(), event_random_control(42, 17, 4).to_bits());
        assert_ne!(baseline.to_bits(), event_random_control(42, 18, 3).to_bits());
    }

    #[test]
    fn event_random_control_is_strictly_inside_zero_and_one() {
        for seed in [42, 1_039, 2_036, u64::MAX] {
            for agent in 0..AGENTS {
                for tick in [0, 1, 17, TICKS as u64 - 1] {
                    let value = event_random_control(seed, tick, agent);
                    assert!(value > 0.0 && value < 1.0);
                }
            }
        }
    }

    #[test]
    fn legacy_and_event_addressed_controls_are_not_accidentally_identical() {
        assert_ne!(
            legacy_random_control(17, 3).to_bits(),
            event_random_control(42, 17, 3).to_bits()
        );
    }
}

