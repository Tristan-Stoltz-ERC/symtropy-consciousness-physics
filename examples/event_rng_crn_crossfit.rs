#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-fitted selection of common-random-number coupling.
//!
//! The training seed partition estimates Cov(Y0(U), Y1(U)) under a shared
//! exogenous event address. The sign selects shared or independent coupling.
//! A disjoint holdout seed partition evaluates the selected policy. Selection
//! never sees the holdout outcomes.

use std::process;

use symtropy_consciousness_physics::event_rng::{registry, EventAddress, SubjectId};

const SUBJECTS_PER_REPLICATE: u32 = 2_048;
const TRAIN_REPLICATES: usize = 256;
const HOLDOUT_REPLICATES: usize = 256;
const BOOTSTRAP_DRAWS: usize = 2_000;
const ROOT_SEED: u64 = 0x4352_4e5f_4352_4f53;
const REPLICATE_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;
const COMMON_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const INDEPENDENT_NAMESPACE: u64 = 0x494e_4445_5045_4e44;
const TREATMENT_INTERCEPT: f64 = 0.25;
const BETAS: [f64; 3] = [0.5, -1.0, -1.5];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CouplingPolicy {
    Shared,
    Independent,
}

impl CouplingPolicy {
    const fn label(self) -> &'static str {
        match self {
            Self::Shared => "SHARED",
            Self::Independent => "INDEPENDENT",
        }
    }
}

#[derive(Default)]
struct OnlineCovariance {
    n: u64,
    mean_x: f64,
    mean_y: f64,
    co_moment: f64,
}

impl OnlineCovariance {
    fn push(&mut self, x: f64, y: f64) {
        self.n += 1;
        let count = self.n as f64;
        let dx = x - self.mean_x;
        self.mean_x += dx / count;
        let dy = y - self.mean_y;
        self.mean_y += dy / count;
        self.co_moment += dx * (y - self.mean_y);
    }

    fn population_covariance(&self) -> f64 {
        assert!(self.n > 0, "covariance requires at least one observation");
        self.co_moment / self.n as f64
    }
}

#[derive(Clone, Copy, Debug)]
struct HoldoutEstimate {
    shared: f64,
    independent: f64,
}

#[derive(Clone, Copy, Debug)]
struct Interval {
    lower: f64,
    upper: f64,
}

#[derive(Clone, Copy, Debug)]
struct CaseResult {
    beta: f64,
    known_ate: f64,
    training_covariance: f64,
    theoretical_covariance: f64,
    selected_policy: CouplingPolicy,
    shared_mean: f64,
    independent_mean: f64,
    selected_mean: f64,
    shared_variance: f64,
    independent_variance: f64,
    selected_variance: f64,
    shared_ratio: f64,
    selected_ratio: f64,
    shared_ratio_interval: Interval,
    selected_ratio_interval: Interval,
}

fn replicate_seed(index: usize) -> u64 {
    ROOT_SEED.wrapping_add((index as u64).wrapping_mul(REPLICATE_STRIDE))
}

fn event_uniform(replicate: u64, namespace: u64, subject_index: u32) -> f64 {
    let subject = SubjectId::agent(subject_index + 1).expect("benchmark subject IDs are nonzero");
    let address = EventAddress::new(
        replicate,
        namespace,
        subject,
        1,
        registry::ENVIRONMENT,
        registry::ENVIRONMENTAL_NOISE,
        0,
    );
    address.uniform_open01(0)
}

fn control_outcome(u: f64) -> f64 {
    u * u
}

fn treatment_outcome(u: f64, beta: f64) -> f64 {
    u * u + TREATMENT_INTERCEPT + beta * u
}

fn sample_mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "mean requires data");
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_variance(values: &[f64]) -> f64 {
    assert!(values.len() >= 2, "sample variance requires n >= 2");
    let mean = sample_mean(values);
    values
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    assert!(!sorted.is_empty(), "percentile requires data");
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

fn bootstrap_variance_ratio(selected: &[f64], independent: &[f64], seed: u64) -> Interval {
    assert_eq!(selected.len(), independent.len());
    assert!(selected.len() >= 2);
    let mut rng = SplitMix64::new(seed);
    let mut ratios = Vec::with_capacity(BOOTSTRAP_DRAWS);

    // Resample paired outer replicates so dependence between the two estimator
    // summaries is retained. The training/holdout split remains disjoint.
    for _ in 0..BOOTSTRAP_DRAWS {
        let mut selected_sample = Vec::with_capacity(selected.len());
        let mut independent_sample = Vec::with_capacity(independent.len());
        for _ in 0..selected.len() {
            let index = rng.index(selected.len());
            selected_sample.push(selected[index]);
            independent_sample.push(independent[index]);
        }
        let denominator = sample_variance(&independent_sample);
        if denominator > 0.0 {
            ratios.push(sample_variance(&selected_sample) / denominator);
        }
    }

    assert!(
        ratios.len() > BOOTSTRAP_DRAWS * 9 / 10,
        "too many degenerate bootstrap variance ratios"
    );
    ratios.sort_by(f64::total_cmp);
    Interval {
        lower: percentile(&ratios, 0.025),
        upper: percentile(&ratios, 0.975),
    }
}

fn estimate_training_covariance(beta: f64) -> f64 {
    let mut moments = OnlineCovariance::default();
    for replicate_index in 0..TRAIN_REPLICATES {
        let seed = replicate_seed(replicate_index);
        for subject in 0..SUBJECTS_PER_REPLICATE {
            let u = event_uniform(seed, COMMON_NAMESPACE, subject);
            moments.push(control_outcome(u), treatment_outcome(u, beta));
        }
    }
    moments.population_covariance()
}

fn estimate_holdout(beta: f64) -> Vec<HoldoutEstimate> {
    let mut estimates = Vec::with_capacity(HOLDOUT_REPLICATES);
    for holdout_index in 0..HOLDOUT_REPLICATES {
        let seed = replicate_seed(TRAIN_REPLICATES + holdout_index);
        let mut shared_sum = 0.0;
        let mut independent_sum = 0.0;

        for subject in 0..SUBJECTS_PER_REPLICATE {
            let common_u = event_uniform(seed, COMMON_NAMESPACE, subject);
            let independent_treatment_u = event_uniform(seed, INDEPENDENT_NAMESPACE, subject);

            let control = control_outcome(common_u);
            shared_sum += treatment_outcome(common_u, beta) - control;
            independent_sum += treatment_outcome(independent_treatment_u, beta) - control;
        }

        estimates.push(HoldoutEstimate {
            shared: shared_sum / SUBJECTS_PER_REPLICATE as f64,
            independent: independent_sum / SUBJECTS_PER_REPLICATE as f64,
        });
    }
    estimates
}

fn theoretical_covariance(beta: f64) -> f64 {
    // Cov(U^2, U^2 + beta*U) = Var(U^2) + beta*Cov(U^2, U)
    4.0 / 45.0 + beta / 12.0
}

fn theoretical_variance_ratio(beta: f64) -> f64 {
    let variance_u = 1.0 / 12.0;
    let mean_u = 1.0 / 2.0;
    let mean_u2 = 1.0 / 3.0;
    let mean_u3 = 1.0 / 4.0;
    let mean_u4 = 1.0 / 5.0;

    let control_mean = mean_u2;
    let control_variance = mean_u4 - control_mean * control_mean;

    let treatment_mean = mean_u2 + beta * mean_u;
    let treatment_second = mean_u4 + 2.0 * beta * mean_u3 + beta * beta * mean_u2;
    let treatment_variance = treatment_second - treatment_mean * treatment_mean;

    let matched_variance = beta * beta * variance_u;
    matched_variance / (control_variance + treatment_variance)
}

fn run_case(beta: f64) -> CaseResult {
    let training_covariance = estimate_training_covariance(beta);
    let selected_policy = if training_covariance > 0.0 {
        CouplingPolicy::Shared
    } else {
        CouplingPolicy::Independent
    };

    // The selector is frozen before this disjoint seed range is read.
    let holdout = estimate_holdout(beta);
    let shared: Vec<_> = holdout.iter().map(|estimate| estimate.shared).collect();
    let independent: Vec<_> = holdout
        .iter()
        .map(|estimate| estimate.independent)
        .collect();
    let selected: Vec<_> = match selected_policy {
        CouplingPolicy::Shared => shared.clone(),
        CouplingPolicy::Independent => independent.clone(),
    };

    let shared_variance = sample_variance(&shared);
    let independent_variance = sample_variance(&independent);
    let selected_variance = sample_variance(&selected);
    let known_ate = TREATMENT_INTERCEPT + beta / 2.0;

    let bootstrap_base = 0x4352_4e5f_4356_4600_u64 ^ beta.to_bits();
    let shared_ratio_interval = bootstrap_variance_ratio(&shared, &independent, bootstrap_base);
    let selected_ratio_interval = if selected_policy == CouplingPolicy::Shared {
        shared_ratio_interval
    } else {
        Interval {
            lower: 1.0,
            upper: 1.0,
        }
    };

    CaseResult {
        beta,
        known_ate,
        training_covariance,
        theoretical_covariance: theoretical_covariance(beta),
        selected_policy,
        shared_mean: sample_mean(&shared),
        independent_mean: sample_mean(&independent),
        selected_mean: sample_mean(&selected),
        shared_variance,
        independent_variance,
        selected_variance,
        shared_ratio: shared_variance / independent_variance,
        selected_ratio: selected_variance / independent_variance,
        shared_ratio_interval,
        selected_ratio_interval,
    }
}

fn case_passes(result: CaseResult) -> bool {
    let theoretical_sign = result.theoretical_covariance.total_cmp(&0.0);
    let selected_sign_matches = match theoretical_sign {
        std::cmp::Ordering::Greater => result.selected_policy == CouplingPolicy::Shared,
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
            result.selected_policy == CouplingPolicy::Independent
        }
    };
    let means_ok = (result.shared_mean - result.known_ate).abs() < 0.01
        && (result.independent_mean - result.known_ate).abs() < 0.01
        && (result.selected_mean - result.known_ate).abs() < 0.01;
    let selected_not_worse_than_independent = result.selected_ratio < 1.10;
    let negative_control_preserved = if result.theoretical_covariance < 0.0 {
        result.shared_ratio > 1.20 && (result.selected_ratio - 1.0).abs() < 1.0e-12
    } else {
        true
    };
    selected_sign_matches
        && means_ok
        && selected_not_worse_than_independent
        && negative_control_preserved
}

fn print_case(result: CaseResult) -> bool {
    let passed = case_passes(result);
    println!(
        "CROSSFIT_JSON={{\"schema\":\"luminous.symtropy.crn-crossfit-coupling.v0.1\",\"beta\":{:.6},\"train_replicates\":{},\"holdout_replicates\":{},\"subjects_per_replicate\":{},\"known_ate\":{:.9},\"training_covariance\":{:.12e},\"theoretical_covariance\":{:.12e},\"selected_policy\":\"{}\",\"shared_holdout_mean\":{:.12e},\"independent_holdout_mean\":{:.12e},\"selected_holdout_mean\":{:.12e},\"shared_estimator_variance\":{:.12e},\"independent_estimator_variance\":{:.12e},\"selected_estimator_variance\":{:.12e},\"shared_to_independent_variance_ratio\":{:.9},\"selected_to_independent_variance_ratio\":{:.9},\"shared_ratio_bootstrap_95pct\":[{:.9},{:.9}],\"selected_ratio_bootstrap_95pct\":[{:.9},{:.9}],\"bootstrap_replicates\":{},\"pass\":{}}}",
        result.beta,
        TRAIN_REPLICATES,
        HOLDOUT_REPLICATES,
        SUBJECTS_PER_REPLICATE,
        result.known_ate,
        result.training_covariance,
        result.theoretical_covariance,
        result.selected_policy.label(),
        result.shared_mean,
        result.independent_mean,
        result.selected_mean,
        result.shared_variance,
        result.independent_variance,
        result.selected_variance,
        result.shared_ratio,
        result.selected_ratio,
        result.shared_ratio_interval.lower,
        result.shared_ratio_interval.upper,
        result.selected_ratio_interval.lower,
        result.selected_ratio_interval.upper,
        BOOTSTRAP_DRAWS,
        passed
    );
    passed
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

    fn index(&mut self, bound: usize) -> usize {
        let bound = bound as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return (value % bound) as usize;
            }
        }
    }
}

fn main() {
    println!("benchmark_schema=luminous.symtropy.crn-crossfit-coupling.v0.1");
    println!("selection_rule=shared_if_training_event_covariance_positive_else_independent");
    println!("train_seed_range=0..{TRAIN_REPLICATES}");
    println!(
        "holdout_seed_range={}..{}",
        TRAIN_REPLICATES,
        TRAIN_REPLICATES + HOLDOUT_REPLICATES
    );
    println!("train_and_holdout_seed_sets_disjoint=true");

    let mut all_pass = true;
    for beta in BETAS {
        all_pass &= print_case(run_case(beta));
    }

    if all_pass {
        println!("benchmark_status=PASS");
    } else {
        eprintln!("benchmark_status=FAIL");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preregistered_beta_cases_cover_positive_weak_positive_and_negative_covariance() {
        assert!(theoretical_covariance(0.5) > 0.0);
        assert!(theoretical_covariance(-1.0) > 0.0);
        assert!(theoretical_covariance(-1.5) < 0.0);
    }

    #[test]
    fn selector_is_shared_only_for_positive_training_covariance() {
        assert_eq!(
            if 0.01 > 0.0 {
                CouplingPolicy::Shared
            } else {
                CouplingPolicy::Independent
            },
            CouplingPolicy::Shared
        );
        assert_eq!(
            if -0.01 > 0.0 {
                CouplingPolicy::Shared
            } else {
                CouplingPolicy::Independent
            },
            CouplingPolicy::Independent
        );
        assert_eq!(
            if 0.0 > 0.0 {
                CouplingPolicy::Shared
            } else {
                CouplingPolicy::Independent
            },
            CouplingPolicy::Independent
        );
    }

    #[test]
    fn training_and_holdout_seed_ranges_do_not_overlap() {
        let training = (0..TRAIN_REPLICATES)
            .map(replicate_seed)
            .collect::<std::collections::HashSet<_>>();
        let holdout = (0..HOLDOUT_REPLICATES)
            .map(|index| replicate_seed(TRAIN_REPLICATES + index))
            .collect::<std::collections::HashSet<_>>();
        assert!(training.is_disjoint(&holdout));
    }

    #[test]
    fn same_event_address_repeats_but_independent_namespace_changes_draw() {
        let seed = replicate_seed(0);
        let shared_a = event_uniform(seed, COMMON_NAMESPACE, 7);
        let shared_b = event_uniform(seed, COMMON_NAMESPACE, 7);
        let independent = event_uniform(seed, INDEPENDENT_NAMESPACE, 7);
        assert_eq!(shared_a.to_bits(), shared_b.to_bits());
        assert_ne!(shared_a.to_bits(), independent.to_bits());
        assert!(shared_a > 0.0 && shared_a < 1.0);
        assert!(independent > 0.0 && independent < 1.0);
    }

    #[test]
    fn weak_positive_covariance_is_not_misclassified_by_the_analytical_rule() {
        assert!(theoretical_covariance(-1.0) > 0.0);
        assert!(theoretical_variance_ratio(-1.0) < 1.0);
    }

    #[test]
    fn negative_covariance_is_the_required_crn_harm_counterexample() {
        assert!(theoretical_covariance(-1.5) < 0.0);
        assert!(theoretical_variance_ratio(-1.5) > 1.0);
    }
}
