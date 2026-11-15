#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Uncertainty-aware, per-event-family common-random-number coupling.
//!
//! Training covariance is estimated on independent outer seed clusters. A
//! deterministic cluster bootstrap supplies a 99% interval. Sharing is allowed
//! only when the lower bound clears a predeclared positive deadband. Clear
//! negative covariance selects independent draws. An interval overlapping the
//! deadband abstains and fails safe to independent draws.
//!
//! This is a known-ATE software benchmark, not an empirical Symtropy result.

use std::process;

use symtropy_consciousness_physics::event_rng::{registry, EventAddress, SubjectId};

const TRAIN_REPLICATES: usize = 256;
const HOLDOUT_REPLICATES: usize = 256;
const SUBJECTS_PER_REPLICATE: u32 = 2_048;
const COVARIANCE_BOOTSTRAP_DRAWS: usize = 10_000;
const VARIANCE_BOOTSTRAP_DRAWS: usize = 2_000;
const COVARIANCE_CI_LOWER: f64 = 0.005;
const COVARIANCE_CI_UPPER: f64 = 0.995;

// Synthetic benchmark threshold only. A real model needs a domain-justified,
// preregistered threshold in the units of its event-family covariance.
const COVARIANCE_DEADBAND: f64 = 0.001;

const ROOT_SEED: u64 = 0x554e_4345_5254_4149;
const REPLICATE_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;
const COMMON_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const INDEPENDENT_U_NAMESPACE: u64 = 0x494e_445f_555f_3031;
const INDEPENDENT_V_NAMESPACE: u64 = 0x494e_445f_565f_3031;
const INTERCEPT: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Shared,
    Independent,
    Abstain,
}

impl Decision {
    const fn label(self) -> &'static str {
        match self {
            Self::Shared => "SHARED",
            Self::Independent => "INDEPENDENT",
            Self::Abstain => "ABSTAIN_TO_INDEPENDENT",
        }
    }

    const fn uses_shared_draw(self) -> bool {
        matches!(self, Self::Shared)
    }
}

#[derive(Clone, Copy, Debug)]
struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    fn is_valid(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower <= self.upper
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

    fn population(self) -> f64 {
        assert!(self.n > 0);
        self.co_moment / self.n as f64
    }
}

#[derive(Clone, Copy)]
struct FamilyTraining {
    mean_covariance: f64,
    interval_99: Interval,
    point_decision: Decision,
    guarded_decision: Decision,
    oracle_decision: Decision,
}

#[derive(Clone, Copy)]
struct HoldoutReplicate {
    estimates: [f64; 5],
}

struct CaseResult {
    beta_u: f64,
    beta_v: f64,
    known_ate: f64,
    family_u: FamilyTraining,
    family_v: FamilyTraining,
    means: [f64; 5],
    variances: [f64; 5],
    variance_ratio_to_independent: [f64; 5],
    ratio_intervals: [Interval; 4],
    pass: bool,
}

fn seed_for(index: usize) -> u64 {
    ROOT_SEED.wrapping_add((index as u64).wrapping_mul(REPLICATE_STRIDE))
}

fn event_uniform(seed: u64, namespace: u64, subject_index: u32, family: u8) -> f64 {
    let subject = SubjectId::agent(subject_index + 1)
        .expect("event-addressed subjects must have nonzero IDs");
    let (kind, purpose) = if family == 0 {
        (registry::ENVIRONMENT, registry::ENVIRONMENTAL_NOISE)
    } else {
        (registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE)
    };
    EventAddress::new(seed, namespace, subject, 1, kind, purpose, 0).uniform_open01(0)
}

fn family_covariance(beta: f64) -> f64 {
    // For U ~ Uniform(0,1):
    // Cov(U^2, U^2 + beta*U) = Var(U^2) + beta*Cov(U^2,U).
    4.0 / 45.0 + beta / 12.0
}

fn choose_with_deadband(interval: Interval, margin: f64) -> Decision {
    if interval.lower > margin {
        Decision::Shared
    } else if interval.upper < -margin {
        Decision::Independent
    } else {
        Decision::Abstain
    }
}

fn mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty());
    values.iter().sum::<f64>() / values.len() as f64
}

fn variance(values: &[f64]) -> f64 {
    assert!(values.len() >= 2);
    let average = mean(values);
    values
        .iter()
        .map(|value| {
            let centered = value - average;
            centered * centered
        })
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    assert!(!sorted.is_empty());
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

fn bootstrap_mean_interval(values: &[f64], seed: u64) -> Interval {
    assert!(values.len() >= 2);
    let mut rng = SplitMix64::new(seed);
    let mut bootstrap_means = Vec::with_capacity(COVARIANCE_BOOTSTRAP_DRAWS);

    // Resample whole outer-replicate covariance estimates. Subjects within one
    // outer replicate are not treated as independent outer replications.
    for _ in 0..COVARIANCE_BOOTSTRAP_DRAWS {
        let mut sum = 0.0;
        for _ in 0..values.len() {
            sum += values[rng.index(values.len())];
        }
        bootstrap_means.push(sum / values.len() as f64);
    }

    bootstrap_means.sort_by(f64::total_cmp);
    Interval {
        lower: percentile(&bootstrap_means, COVARIANCE_CI_LOWER),
        upper: percentile(&bootstrap_means, COVARIANCE_CI_UPPER),
    }
}

fn bootstrap_variance_ratio_interval(
    candidate: &[f64],
    independent: &[f64],
    seed: u64,
) -> Interval {
    assert_eq!(candidate.len(), independent.len());
    assert!(candidate.len() >= 2);
    let mut rng = SplitMix64::new(seed);
    let mut ratios = Vec::with_capacity(VARIANCE_BOOTSTRAP_DRAWS);

    // Paired resampling preserves the common outer-replicate index.
    for _ in 0..VARIANCE_BOOTSTRAP_DRAWS {
        let mut candidate_sample = Vec::with_capacity(candidate.len());
        let mut independent_sample = Vec::with_capacity(independent.len());
        for _ in 0..candidate.len() {
            let index = rng.index(candidate.len());
            candidate_sample.push(candidate[index]);
            independent_sample.push(independent[index]);
        }
        let denominator = variance(&independent_sample);
        if denominator > 0.0 {
            ratios.push(variance(&candidate_sample) / denominator);
        }
    }

    assert!(
        ratios.len() > VARIANCE_BOOTSTRAP_DRAWS * 9 / 10,
        "too many degenerate bootstrap variance ratios"
    );
    ratios.sort_by(f64::total_cmp);
    Interval {
        lower: percentile(&ratios, 0.025),
        upper: percentile(&ratios, 0.975),
    }
}

fn train_family(beta: f64, family: u8, case_index: usize) -> FamilyTraining {
    let mut replicate_covariances = Vec::with_capacity(TRAIN_REPLICATES);

    for replicate in 0..TRAIN_REPLICATES {
        let seed = seed_for(replicate);
        let mut covariance = OnlineCovariance::default();

        for subject in 0..SUBJECTS_PER_REPLICATE {
            let value = event_uniform(seed, COMMON_NAMESPACE, subject, family);
            covariance.push(value * value, value * value + beta * value);
        }

        replicate_covariances.push(covariance.population());
    }

    let mean_covariance = mean(&replicate_covariances);
    let bootstrap_seed = 0x434f_5641_5249_414e_u64
        ^ (case_index as u64).rotate_left(17)
        ^ (family as u64).rotate_left(41)
        ^ beta.to_bits();
    let interval_99 = bootstrap_mean_interval(&replicate_covariances, bootstrap_seed);

    let point_decision = if mean_covariance > 0.0 {
        Decision::Shared
    } else {
        Decision::Independent
    };

    FamilyTraining {
        mean_covariance,
        interval_99,
        point_decision,
        guarded_decision: choose_with_deadband(interval_99, COVARIANCE_DEADBAND),
        oracle_decision: if family_covariance(beta) > 0.0 {
            Decision::Shared
        } else {
            Decision::Independent
        },
    }
}

fn treatment_outcome(u: f64, v: f64, beta_u: f64, beta_v: f64) -> f64 {
    u * u + beta_u * u + v * v + beta_v * v + INTERCEPT
}

fn control_outcome(u: f64, v: f64) -> f64 {
    u * u + v * v
}

fn select_draw(decision: Decision, shared: f64, independent: f64) -> f64 {
    if decision.uses_shared_draw() {
        shared
    } else {
        // Both explicit INDEPENDENT and ABSTAIN use the fail-safe stream.
        independent
    }
}

fn estimate_holdout(
    beta_u: f64,
    beta_v: f64,
    family_u: FamilyTraining,
    family_v: FamilyTraining,
) -> Vec<HoldoutReplicate> {
    let mut output = Vec::with_capacity(HOLDOUT_REPLICATES);

    for replicate in 0..HOLDOUT_REPLICATES {
        let seed = seed_for(TRAIN_REPLICATES + replicate);
        let mut sums = [0.0_f64; 5];

        for subject in 0..SUBJECTS_PER_REPLICATE {
            let common_u = event_uniform(seed, COMMON_NAMESPACE, subject, 0);
            let common_v = event_uniform(seed, COMMON_NAMESPACE, subject, 1);
            let independent_u =
                event_uniform(seed, INDEPENDENT_U_NAMESPACE, subject, 0);
            let independent_v =
                event_uniform(seed, INDEPENDENT_V_NAMESPACE, subject, 1);

            let control = control_outcome(common_u, common_v);

            let all_independent =
                treatment_outcome(independent_u, independent_v, beta_u, beta_v) - control;
            let all_shared =
                treatment_outcome(common_u, common_v, beta_u, beta_v) - control;

            let point_u = select_draw(family_u.point_decision, common_u, independent_u);
            let point_v = select_draw(family_v.point_decision, common_v, independent_v);
            let point = treatment_outcome(point_u, point_v, beta_u, beta_v) - control;

            let guarded_u = select_draw(family_u.guarded_decision, common_u, independent_u);
            let guarded_v = select_draw(family_v.guarded_decision, common_v, independent_v);
            let guarded = treatment_outcome(guarded_u, guarded_v, beta_u, beta_v) - control;

            let oracle_u = select_draw(family_u.oracle_decision, common_u, independent_u);
            let oracle_v = select_draw(family_v.oracle_decision, common_v, independent_v);
            let oracle = treatment_outcome(oracle_u, oracle_v, beta_u, beta_v) - control;

            sums[0] += all_independent;
            sums[1] += all_shared;
            sums[2] += point;
            sums[3] += guarded;
            sums[4] += oracle;
        }

        let scale = 1.0 / SUBJECTS_PER_REPLICATE as f64;
        output.push(HoldoutReplicate {
            estimates: sums.map(|sum| sum * scale),
        });
    }

    output
}

fn column(replicates: &[HoldoutReplicate], index: usize) -> Vec<f64> {
    replicates.iter().map(|replicate| replicate.estimates[index]).collect()
}

fn run_case(beta_u: f64, beta_v: f64, case_index: usize) -> CaseResult {
    let family_u = train_family(beta_u, 0, case_index);
    let family_v = train_family(beta_v, 1, case_index);

    // The coupling decisions are frozen here. No holdout output is read by the
    // selector or used to change either event-family decision.
    let holdout = estimate_holdout(beta_u, beta_v, family_u, family_v);
    let independent = column(&holdout, 0);
    let shared = column(&holdout, 1);
    let point = column(&holdout, 2);
    let guarded = column(&holdout, 3);
    let oracle = column(&holdout, 4);

    let means = [
        mean(&independent),
        mean(&shared),
        mean(&point),
        mean(&guarded),
        mean(&oracle),
    ];
    let variances = [
        variance(&independent),
        variance(&shared),
        variance(&point),
        variance(&guarded),
        variance(&oracle),
    ];
    let baseline = variances[0];
    let variance_ratio_to_independent = variances.map(|value| value / baseline);

    let base_seed = 0x4352_4e5f_4755_4152_u64
        ^ (case_index as u64).rotate_left(19)
        ^ beta_u.to_bits()
        ^ beta_v.to_bits().rotate_left(31);
    let ratio_intervals = [
        bootstrap_variance_ratio_interval(&shared, &independent, base_seed),
        bootstrap_variance_ratio_interval(&point, &independent, base_seed + 1),
        bootstrap_variance_ratio_interval(&guarded, &independent, base_seed + 2),
        bootstrap_variance_ratio_interval(&oracle, &independent, base_seed + 3),
    ];

    let known_ate = INTERCEPT + (beta_u + beta_v) / 2.0;
    let means_ok = means.iter().all(|value| (value - known_ate).abs() < 0.02);
    let intervals_ok = family_u.interval_99.is_valid()
        && family_v.interval_99.is_valid()
        && ratio_intervals.iter().all(|interval| interval.is_valid());
    let finite_outputs = means.iter().all(|value| value.is_finite())
        && variances.iter().all(|value| value.is_finite())
        && variance_ratio_to_independent
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0);

    CaseResult {
        beta_u,
        beta_v,
        known_ate,
        family_u,
        family_v,
        means,
        variances,
        variance_ratio_to_independent,
        ratio_intervals,
        pass: means_ok && intervals_ok && finite_outputs,
    }
}

fn print_case(case: &CaseResult) {
    let abstentions = (if case.family_u.guarded_decision == Decision::Abstain { 1 } else { 0 })
        + (if case.family_v.guarded_decision == Decision::Abstain { 1 } else { 0 });

    println!(
        "CRN_GUARD_JSON={{\"schema\":\"luminous.symtropy.crn-uncertainty-guard.v0.1\",\"beta_u\":{:.12e},\"beta_v\":{:.12e},\"train_replicates\":{},\"holdout_replicates\":{},\"subjects_per_replicate\":{},\"covariance_bootstrap_draws\":{},\"covariance_ci_level\":0.99,\"covariance_deadband\":{:.9},\"known_ate\":{:.12e},\"family_u_covariance_mean\":{:.12e},\"family_u_ci99\":[{:.12e},{:.12e}],\"family_u_point\":\"{}\",\"family_u_guard\":\"{}\",\"family_u_oracle\":\"{}\",\"family_v_covariance_mean\":{:.12e},\"family_v_ci99\":[{:.12e},{:.12e}],\"family_v_point\":\"{}\",\"family_v_guard\":\"{}\",\"family_v_oracle\":\"{}\",\"guard_abstentions\":{},\"mean_all_independent\":{:.12e},\"mean_all_shared\":{:.12e},\"mean_point_selector\":{:.12e},\"mean_guard_selector\":{:.12e},\"mean_oracle\":{:.12e},\"variance_ratio_all_shared\":{:.9},\"variance_ratio_point_selector\":{:.9},\"variance_ratio_guard_selector\":{:.9},\"variance_ratio_oracle\":{:.9},\"guard_variance_ratio_ci95\":[{:.9},{:.9}],\"pass\":{}}}",
        case.beta_u,
        case.beta_v,
        TRAIN_REPLICATES,
        HOLDOUT_REPLICATES,
        SUBJECTS_PER_REPLICATE,
        COVARIANCE_BOOTSTRAP_DRAWS,
        COVARIANCE_DEADBAND,
        case.known_ate,
        case.family_u.mean_covariance,
        case.family_u.interval_99.lower,
        case.family_u.interval_99.upper,
        case.family_u.point_decision.label(),
        case.family_u.guarded_decision.label(),
        case.family_u.oracle_decision.label(),
        case.family_v.mean_covariance,
        case.family_v.interval_99.lower,
        case.family_v.interval_99.upper,
        case.family_v.point_decision.label(),
        case.family_v.guarded_decision.label(),
        case.family_v.oracle_decision.label(),
        abstentions,
        case.means[0],
        case.means[1],
        case.means[2],
        case.means[3],
        case.means[4],
        case.variance_ratio_to_independent[1],
        case.variance_ratio_to_independent[2],
        case.variance_ratio_to_independent[3],
        case.variance_ratio_to_independent[4],
        case.ratio_intervals[2].lower,
        case.ratio_intervals[2].upper,
        case.pass
    );
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
        assert!(bound > 0);
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
    println!("benchmark_schema=luminous.symtropy.crn-uncertainty-guard.v0.1");
    println!("training_and_holdout_seed_ranges_disjoint=true");
    println!("decision_rule=share_if_ci99_lower_gt_positive_deadband;independent_if_ci99_upper_lt_negative_deadband;otherwise_abstain_to_independent");
    println!(
        "replicate_counts=train:{TRAIN_REPLICATES},holdout:{HOLDOUT_REPLICATES},subjects:{SUBJECTS_PER_REPLICATE}"
    );

    // beta=-16/15 is the analytic zero-covariance boundary for this model.
    // beta=-1.05 is a deliberately near-deadband positive-covariance case.
    const CASES: [(f64, f64); 4] = [
        (0.5, -1.5),
        (-1.5, 0.5),
        (-1.05, -1.5),
        (-16.0 / 15.0, 0.5),
    ];

    let mut all_pass = true;
    for (case_index, (beta_u, beta_v)) in CASES.into_iter().enumerate() {
        let result = run_case(beta_u, beta_v, case_index);
        print_case(&result);
        all_pass &= result.pass;
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
    fn theoretical_covariance_has_expected_signs_and_zero_boundary() {
        assert!(family_covariance(0.5) > 0.0);
        assert!(family_covariance(-1.5) < 0.0);
        assert!(family_covariance(-1.05) > COVARIANCE_DEADBAND);
        assert!(family_covariance(-1.05) < 0.002);
        assert!(family_covariance(-16.0 / 15.0).abs() < 1.0e-15);
    }

    #[test]
    fn guarded_selector_requires_the_interval_to_clear_the_deadband() {
        assert_eq!(
            choose_with_deadband(
                Interval { lower: 0.02, upper: 0.04 },
                COVARIANCE_DEADBAND
            ),
            Decision::Shared
        );
        assert_eq!(
            choose_with_deadband(
                Interval { lower: -0.04, upper: -0.02 },
                COVARIANCE_DEADBAND
            ),
            Decision::Independent
        );
        assert_eq!(
            choose_with_deadband(
                Interval { lower: -0.0005, upper: 0.0015 },
                COVARIANCE_DEADBAND
            ),
            Decision::Abstain
        );
        assert_eq!(
            choose_with_deadband(
                Interval { lower: 0.0001, upper: 0.0009 },
                COVARIANCE_DEADBAND
            ),
            Decision::Abstain
        );
    }

    #[test]
    fn abstention_uses_the_independent_fallback() {
        assert!(!Decision::Abstain.uses_shared_draw());
        assert!(Decision::Shared.uses_shared_draw());
        assert!(!Decision::Independent.uses_shared_draw());
    }

    #[test]
    fn event_address_is_repeatable_and_namespace_separated() {
        let first = event_uniform(42, COMMON_NAMESPACE, 7, 0);
        let repeated = event_uniform(42, COMMON_NAMESPACE, 7, 0);
        let independent = event_uniform(42, INDEPENDENT_U_NAMESPACE, 7, 0);
        assert_eq!(first.to_bits(), repeated.to_bits());
        assert_ne!(first.to_bits(), independent.to_bits());
        assert!(first > 0.0 && first < 1.0);
    }

    #[test]
    fn training_and_holdout_seed_sets_are_disjoint() {
        let training = (0..TRAIN_REPLICATES)
            .map(seed_for)
            .collect::<std::collections::HashSet<_>>();
        let holdout = (0..HOLDOUT_REPLICATES)
            .map(|index| seed_for(TRAIN_REPLICATES + index))
            .collect::<std::collections::HashSet<_>>();
        assert!(training.is_disjoint(&holdout));
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_and_ordered() {
        let clusters = (0..64)
            .map(|index| 0.02 + (index as f64 - 31.5) * 0.0001)
            .collect::<Vec<_>>();
        let first = bootstrap_mean_interval(&clusters, 0x5345_4544);
        let second = bootstrap_mean_interval(&clusters, 0x5345_4544);
        assert_eq!(first.lower.to_bits(), second.lower.to_bits());
        assert_eq!(first.upper.to_bits(), second.upper.to_bits());
        assert!(first.is_valid());
    }
}
