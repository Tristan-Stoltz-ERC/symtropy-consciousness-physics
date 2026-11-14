#![forbid(unsafe_code)]

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! A controlled common-random-numbers (CRN) variance experiment.
//!
//! This is a software qualification fixture, not an empirical Symtropy result.
//! It uses a structural causal model with an analytically known average effect.
//! The positive-covariance case should benefit from matched exogenous events;
//! the negative-covariance control should demonstrate that CRN can increase
//! estimator variance when the induced covariance has the opposite sign.

use std::process;

use symtropy_consciousness_physics::event_rng::{
    registry, EventAddress, SubjectId,
};

const SUBJECTS_PER_REPLICATE: u32 = 2_048;
const INDEPENDENT_REPLICATES: usize = 512;
const BOOTSTRAP_DRAWS: usize = 2_000;
const ROOT_SEED: u64 = 0x4352_4e5f_5048_494c;
const REPLICATE_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;
const COMMON_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const INDEPENDENT_NAMESPACE: u64 = 0x494e_4445_5045_4e44;
const TREATMENT_INTERCEPT: f64 = 0.25;

#[derive(Clone, Copy, Debug)]
struct TrialEstimate {
    matched_crn: f64,
    independent_draws: f64,
}

#[derive(Clone, Copy, Debug)]
struct CaseResult {
    beta: f64,
    known_effect: f64,
    matched_mean: f64,
    independent_mean: f64,
    matched_variance: f64,
    independent_variance: f64,
    empirical_variance_ratio: f64,
    theoretical_variance_ratio: f64,
    bootstrap_low: f64,
    bootstrap_high: f64,
}

fn event_uniform(replicate_seed: u64, namespace: u64, subject_index: u32) -> f64 {
    let subject = SubjectId::agent(subject_index + 1)
        .expect("benchmark subject IDs are nonzero");
    let address = EventAddress::new(
        replicate_seed,
        namespace,
        subject,
        1,
        registry::ENVIRONMENT,
        registry::ENVIRONMENTAL_NOISE,
        0,
    );
    address.uniform_open01(0)
}

fn control_outcome(exogenous: f64) -> f64 {
    exogenous * exogenous
}

fn treatment_outcome(exogenous: f64, beta: f64) -> f64 {
    exogenous * exogenous + TREATMENT_INTERCEPT + beta * exogenous
}

fn sample_variance(values: &[f64]) -> f64 {
    assert!(values.len() >= 2, "sample variance requires at least two values");
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn sample_mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn exact_variance_ratio(beta: f64) -> f64 {
    // For U ~ Uniform(0, 1):
    // Var(Y0) = Var(U^2)
    // Var(Y1) = Var(U^2 + beta*U)
    // Var(Y1 - Y0 | same U) = beta^2 Var(U)
    let variance_u = 1.0 / 12.0;
    let mean_u = 1.0 / 2.0;
    let mean_u2 = 1.0 / 3.0;
    let mean_u3 = 1.0 / 4.0;
    let mean_u4 = 1.0 / 5.0;

    let mean_control = mean_u2;
    let second_control = mean_u4;
    let variance_control = second_control - mean_control * mean_control;

    let mean_treatment_without_intercept = mean_u2 + beta * mean_u;
    let second_treatment_without_intercept =
        mean_u4 + 2.0 * beta * mean_u3 + beta * beta * mean_u2;
    let variance_treatment =
        second_treatment_without_intercept
            - mean_treatment_without_intercept * mean_treatment_without_intercept;

    let matched = beta * beta * variance_u;
    let independent = variance_control + variance_treatment;
    matched / independent
}

fn bootstrap_variance_ratio(
    trials: &[TrialEstimate],
    initial_state: u64,
) -> (f64, f64) {
    let mut rng = SplitMix64::new(initial_state);
    let mut ratios = Vec::with_capacity(BOOTSTRAP_DRAWS);

    for _ in 0..BOOTSTRAP_DRAWS {
        let mut matched = Vec::with_capacity(trials.len());
        let mut independent = Vec::with_capacity(trials.len());
        for _ in 0..trials.len() {
            let index = (rng.next_u64() as usize) % trials.len();
            matched.push(trials[index].matched_crn);
            independent.push(trials[index].independent_draws);
        }
        let denominator = sample_variance(&independent);
        if denominator > 0.0 {
            ratios.push(sample_variance(&matched) / denominator);
        }
    }

    ratios.sort_by(f64::total_cmp);
    (
        percentile(&ratios, 0.025),
        percentile(&ratios, 0.975),
    )
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

fn run_case(beta: f64, bootstrap_seed: u64) -> CaseResult {
    let mut trials = Vec::with_capacity(INDEPENDENT_REPLICATES);

    for replicate in 0..INDEPENDENT_REPLICATES {
        let replicate_seed =
            ROOT_SEED.wrapping_add((replicate as u64).wrapping_mul(REPLICATE_STRIDE));
        let mut matched_sum = 0.0;
        let mut independent_sum = 0.0;

        for subject in 0..SUBJECTS_PER_REPLICATE {
            // Same complete event address in the two matched scenarios.
            let common_control = event_uniform(
                replicate_seed,
                COMMON_NAMESPACE,
                subject,
            );
            let common_treatment = event_uniform(
                replicate_seed,
                COMMON_NAMESPACE,
                subject,
            );
            assert_eq!(
                common_control.to_bits(),
                common_treatment.to_bits(),
                "same event address must return the same exogenous value"
            );

            // Independent control/treatment draws use a distinct namespace.
            let independent_control = event_uniform(
                replicate_seed,
                COMMON_NAMESPACE,
                subject,
            );
            let independent_treatment = event_uniform(
                replicate_seed,
                INDEPENDENT_NAMESPACE,
                subject,
            );

            matched_sum +=
                treatment_outcome(common_treatment, beta) - control_outcome(common_control);
            independent_sum +=
                treatment_outcome(independent_treatment, beta)
                    - control_outcome(independent_control);
        }

        trials.push(TrialEstimate {
            matched_crn: matched_sum / SUBJECTS_PER_REPLICATE as f64,
            independent_draws: independent_sum / SUBJECTS_PER_REPLICATE as f64,
        });
    }

    let matched: Vec<f64> = trials.iter().map(|trial| trial.matched_crn).collect();
    let independent: Vec<f64> = trials
        .iter()
        .map(|trial| trial.independent_draws)
        .collect();
    let matched_variance = sample_variance(&matched);
    let independent_variance = sample_variance(&independent);
    let empirical_variance_ratio = matched_variance / independent_variance;
    let known_effect = TREATMENT_INTERCEPT + beta / 2.0;
    let (bootstrap_low, bootstrap_high) =
        bootstrap_variance_ratio(&trials, bootstrap_seed);

    CaseResult {
        beta,
        known_effect,
        matched_mean: sample_mean(&matched),
        independent_mean: sample_mean(&independent),
        matched_variance,
        independent_variance,
        empirical_variance_ratio,
        theoretical_variance_ratio: exact_variance_ratio(beta),
        bootstrap_low,
        bootstrap_high,
    }
}

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

fn print_case(label: &str, result: CaseResult) {
    println!("{label}_beta={:.6}", result.beta);
    println!("{label}_known_effect={:.6}", result.known_effect);
    println!("{label}_matched_effect_mean={:.9}", result.matched_mean);
    println!("{label}_independent_effect_mean={:.9}", result.independent_mean);
    println!("{label}_matched_estimator_variance={:.12e}", result.matched_variance);
    println!(
        "{label}_independent_estimator_variance={:.12e}",
        result.independent_variance
    );
    println!(
        "{label}_empirical_matched_to_independent_variance_ratio={:.6}",
        result.empirical_variance_ratio
    );
    println!(
        "{label}_theoretical_matched_to_independent_variance_ratio={:.6}",
        result.theoretical_variance_ratio
    );
    println!(
        "{label}_bootstrap_95pct_ratio_interval=[{:.6},{:.6}]",
        result.bootstrap_low, result.bootstrap_high
    );
}

fn main() {
    let positive = run_case(0.5, 0x504f_5349_5449_5645);
    let negative = run_case(-1.5, 0x4e45_4741_5449_5645);

    let positive_mean_ok =
        (positive.matched_mean - positive.known_effect).abs() < 0.01
            && (positive.independent_mean - positive.known_effect).abs() < 0.01;
    let negative_mean_ok =
        (negative.matched_mean - negative.known_effect).abs() < 0.01
            && (negative.independent_mean - negative.known_effect).abs() < 0.01;

    // Positive covariance should help; negative covariance is an explicit
    // counterexample to the claim that common random numbers always help.
    let positive_crn_helps = positive.empirical_variance_ratio < 0.20;
    let negative_crn_can_hurt = negative.empirical_variance_ratio > 1.20;

    println!("benchmark_schema=luminous.symtropy.crn-variance-benchmark.v0.1");
    println!("subjects_per_replicate={SUBJECTS_PER_REPLICATE}");
    println!("independent_replicates={INDEPENDENT_REPLICATES}");
    println!("bootstrap_resamples={BOOTSTRAP_DRAWS}");
    println!("known_effect_model=Y0=U^2;Y1=U^2+0.25+beta*U;U~Uniform(0,1)");
    print_case("positive_covariance", positive);
    print_case("negative_covariance_control", negative);
    println!("positive_covariance_crn_variance_reduction_gate={positive_crn_helps}");
    println!("negative_covariance_crn_inflation_control={negative_crn_can_hurt}");

    if positive_mean_ok
        && negative_mean_ok
        && positive_crn_helps
        && negative_crn_can_hurt
    {
        println!("benchmark_status=PASS");
    } else {
        eprintln!("benchmark_status=FAIL");
        process::exit(1);
    }
}
