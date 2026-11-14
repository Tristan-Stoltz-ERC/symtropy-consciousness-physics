#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-event-family cross-fitted CRN coupling portfolio.
//!
//! Two exogenous event families have opposite covariance signs. A global
//! covariance rule sees positive total covariance and shares both families.
//! The event-family policy selects shared/independent coupling independently
//! for each family using training seeds, then evaluates on disjoint holdout.

use std::process;

use symtropy_consciousness_physics::event_rng::{registry, EventAddress, SubjectId};

const SUBJECTS: u32 = 2_048;
const TRAIN_REPLICATES: usize = 256;
const HOLDOUT_REPLICATES: usize = 256;
const BOOTSTRAP_DRAWS: usize = 2_000;
const ROOT_SEED: u64 = 0x4556_454e_545f_5046;
const REPLICATE_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;
const COMMON_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;
const INDEPENDENT_U_NAMESPACE: u64 = 0x494e_445f_555f_3031;
const INDEPENDENT_V_NAMESPACE: u64 = 0x494e_445f_565f_3031;
const INTERCEPT: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FamilyPolicy {
    Shared,
    Independent,
}

impl FamilyPolicy {
    const fn label(self) -> &'static str {
        match self {
            Self::Shared => "SHARED",
            Self::Independent => "INDEPENDENT",
        }
    }
}

#[derive(Default)]
struct Covariance {
    n: u64,
    mean_x: f64,
    mean_y: f64,
    co_moment: f64,
}

impl Covariance {
    fn push(&mut self, x: f64, y: f64) {
        self.n += 1;
        let n = self.n as f64;
        let dx = x - self.mean_x;
        self.mean_x += dx / n;
        let dy = y - self.mean_y;
        self.mean_y += dy / n;
        self.co_moment += dx * (y - self.mean_y);
    }

    fn population(self) -> f64 {
        assert!(self.n > 0);
        self.co_moment / self.n as f64
    }
}

#[derive(Clone, Copy)]
struct Holdout {
    all_independent: f64,
    all_shared: f64,
    global: f64,
    per_family: f64,
}

#[derive(Clone, Copy)]
struct Interval {
    lower: f64,
    upper: f64,
}

#[derive(Clone, Copy)]
struct Case {
    beta_u: f64,
    beta_v: f64,
    known_ate: f64,
    covariance_u_train: f64,
    covariance_v_train: f64,
    covariance_total_train: f64,
    share_u: bool,
    share_v: bool,
    global_share_both: bool,
    means: [f64; 4],
    variances: [f64; 4],
    ratios: [f64; 3],
    ratio_intervals: [Interval; 3],
    portfolio_to_global_interval: Interval,
    theoretical_shared_ratio: f64,
    theoretical_portfolio_ratio: f64,
}

fn seed_for(index: usize) -> u64 {
    ROOT_SEED.wrapping_add((index as u64).wrapping_mul(REPLICATE_STRIDE))
}

fn draw(seed: u64, namespace: u64, subject_index: u32, family: u8) -> f64 {
    let subject =
        SubjectId::agent(subject_index + 1).expect("event family subject ID must be nonzero");
    let (kind, purpose) = if family == 0 {
        (registry::ENVIRONMENT, registry::ENVIRONMENTAL_NOISE)
    } else {
        (registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE)
    };
    EventAddress::new(seed, namespace, subject, 1, kind, purpose, 0).uniform_open01(0)
}

fn y0(u: f64, v: f64) -> f64 {
    u * u + v * v
}

fn y1(u: f64, v: f64, beta_u: f64, beta_v: f64) -> f64 {
    u * u + beta_u * u + v * v + beta_v * v + INTERCEPT
}

fn family_covariance(beta: f64) -> f64 {
    4.0 / 45.0 + beta / 12.0
}

fn training_covariances(beta_u: f64, beta_v: f64) -> (f64, f64) {
    let mut u_moments = Covariance::default();
    let mut v_moments = Covariance::default();

    for replicate in 0..TRAIN_REPLICATES {
        let seed = seed_for(replicate);
        for subject in 0..SUBJECTS {
            let u = draw(seed, COMMON_NAMESPACE, subject, 0);
            let v = draw(seed, COMMON_NAMESPACE, subject, 1);
            u_moments.push(u * u, u * u + beta_u * u);
            v_moments.push(v * v, v * v + beta_v * v);
        }
    }

    (u_moments.population(), v_moments.population())
}

fn holdout_estimates(
    beta_u: f64,
    beta_v: f64,
    share_u: bool,
    share_v: bool,
    global_share_both: bool,
) -> Vec<Holdout> {
    let mut output = Vec::with_capacity(HOLDOUT_REPLICATES);

    for replicate in 0..HOLDOUT_REPLICATES {
        let seed = seed_for(TRAIN_REPLICATES + replicate);
        let mut sums = [0.0_f64; 4];

        for subject in 0..SUBJECTS {
            let u_common = draw(seed, COMMON_NAMESPACE, subject, 0);
            let v_common = draw(seed, COMMON_NAMESPACE, subject, 1);
            let u_independent = draw(seed, INDEPENDENT_U_NAMESPACE, subject, 0);
            let v_independent = draw(seed, INDEPENDENT_V_NAMESPACE, subject, 1);
            let control = y0(u_common, v_common);

            let independent_treatment = y1(u_independent, v_independent, beta_u, beta_v);
            let shared_treatment = y1(u_common, v_common, beta_u, beta_v);

            let global_treatment = if global_share_both {
                shared_treatment
            } else {
                independent_treatment
            };

            let portfolio_u = if share_u { u_common } else { u_independent };
            let portfolio_v = if share_v { v_common } else { v_independent };
            let portfolio_treatment = y1(portfolio_u, portfolio_v, beta_u, beta_v);

            sums[0] += independent_treatment - control;
            sums[1] += shared_treatment - control;
            sums[2] += global_treatment - control;
            sums[3] += portfolio_treatment - control;
        }

        let scale = 1.0 / SUBJECTS as f64;
        output.push(Holdout {
            all_independent: sums[0] * scale,
            all_shared: sums[1] * scale,
            global: sums[2] * scale,
            per_family: sums[3] * scale,
        });
    }

    output
}

fn mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty());
    values.iter().sum::<f64>() / values.len() as f64
}

fn variance(values: &[f64]) -> f64 {
    assert!(values.len() >= 2);
    let avg = mean(values);
    values
        .iter()
        .map(|value| (value - avg).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let position = p * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

fn bootstrap_ratio_interval(candidate: &[f64], independent: &[f64], seed: u64) -> Interval {
    assert_eq!(candidate.len(), independent.len());
    let mut rng = SplitMix64::new(seed);
    let mut ratios = Vec::with_capacity(BOOTSTRAP_DRAWS);

    for _ in 0..BOOTSTRAP_DRAWS {
        let mut selected = Vec::with_capacity(candidate.len());
        let mut baseline = Vec::with_capacity(independent.len());
        for _ in 0..candidate.len() {
            let index = rng.index(candidate.len());
            selected.push(candidate[index]);
            baseline.push(independent[index]);
        }
        let denominator = variance(&baseline);
        if denominator > 0.0 {
            ratios.push(variance(&selected) / denominator);
        }
    }

    assert!(ratios.len() > BOOTSTRAP_DRAWS * 9 / 10);
    ratios.sort_by(f64::total_cmp);
    Interval {
        lower: percentile(&ratios, 0.025),
        upper: percentile(&ratios, 0.975),
    }
}

fn theoretical_independent_variance(beta_u: f64, beta_v: f64) -> f64 {
    let variance_u2 = 4.0 / 45.0;
    let variance_u2_plus_beta_u = variance_u2 + beta_u * beta_u / 12.0 + beta_u / 6.0;
    let variance_v2_plus_beta_v = variance_u2 + beta_v * beta_v / 12.0 + beta_v / 6.0;
    variance_u2 + variance_u2_plus_beta_u + variance_u2 + variance_v2_plus_beta_v
}

fn theoretical_shared_variance(beta_u: f64, beta_v: f64) -> f64 {
    (beta_u * beta_u + beta_v * beta_v) / 12.0
}

fn theoretical_portfolio_variance(beta_u: f64, beta_v: f64) -> f64 {
    let cov_u = family_covariance(beta_u);
    let cov_v = family_covariance(beta_v);
    let shared_u = cov_u > 0.0;
    let shared_v = cov_v > 0.0;
    let variance_u = if shared_u {
        beta_u * beta_u / 12.0
    } else {
        4.0 / 45.0 + 4.0 / 45.0 + beta_u * beta_u / 12.0 + beta_u / 6.0
    };
    let variance_v = if shared_v {
        beta_v * beta_v / 12.0
    } else {
        4.0 / 45.0 + 4.0 / 45.0 + beta_v * beta_v / 12.0 + beta_v / 6.0
    };
    variance_u + variance_v
}

fn run_case(beta_u: f64, beta_v: f64) -> Case {
    let (covariance_u_train, covariance_v_train) = training_covariances(beta_u, beta_v);
    let covariance_total_train = covariance_u_train + covariance_v_train;

    // Each family is selected independently from training data only.
    let share_u = covariance_u_train > 0.0;
    let share_v = covariance_v_train > 0.0;
    let global_share_both = covariance_total_train > 0.0;

    let holdout = holdout_estimates(beta_u, beta_v, share_u, share_v, global_share_both);
    let all_independent = holdout
        .iter()
        .map(|x| x.all_independent)
        .collect::<Vec<_>>();
    let all_shared = holdout.iter().map(|x| x.all_shared).collect::<Vec<_>>();
    let global = holdout.iter().map(|x| x.global).collect::<Vec<_>>();
    let per_family = holdout.iter().map(|x| x.per_family).collect::<Vec<_>>();

    let v_independent = variance(&all_independent);
    let v_shared = variance(&all_shared);
    let v_global = variance(&global);
    let v_portfolio = variance(&per_family);

    let seed = 0x4556_4641_4d49_4c59_u64 ^ beta_u.to_bits() ^ beta_v.to_bits().rotate_left(23);

    Case {
        beta_u,
        beta_v,
        known_ate: INTERCEPT + (beta_u + beta_v) / 2.0,
        covariance_u_train,
        covariance_v_train,
        covariance_total_train,
        share_u,
        share_v,
        global_share_both,
        means: [
            mean(&all_independent),
            mean(&all_shared),
            mean(&global),
            mean(&per_family),
        ],
        variances: [v_independent, v_shared, v_global, v_portfolio],
        ratios: [
            v_shared / v_independent,
            v_global / v_independent,
            v_portfolio / v_independent,
        ],
        ratio_intervals: [
            bootstrap_ratio_interval(&all_shared, &all_independent, seed),
            bootstrap_ratio_interval(&global, &all_independent, seed + 1),
            bootstrap_ratio_interval(&per_family, &all_independent, seed + 2),
        ],
        portfolio_to_global_interval: bootstrap_ratio_interval(&per_family, &global, seed + 3),
        theoretical_shared_ratio: theoretical_shared_variance(beta_u, beta_v)
            / theoretical_independent_variance(beta_u, beta_v),
        theoretical_portfolio_ratio: theoretical_portfolio_variance(beta_u, beta_v)
            / theoretical_independent_variance(beta_u, beta_v),
    }
}

fn passes(case: Case) -> bool {
    let sign_u = family_covariance(case.beta_u) > 0.0;
    let sign_v = family_covariance(case.beta_v) > 0.0;
    let global_positive = family_covariance(case.beta_u) + family_covariance(case.beta_v) > 0.0;
    let expected_ate = case.known_ate;
    let all_means_ok = case
        .means
        .iter()
        .all(|value| (value - expected_ate).abs() < 0.01);
    let portfolio_beats_global = case.ratios[2] < case.ratios[1] * 0.90;
    let portfolio_beats_independent = case.ratios[2] < 1.0;
    let policy_correct = case.share_u == sign_u
        && case.share_v == sign_v
        && case.global_share_both == global_positive;
    all_means_ok && portfolio_beats_global && portfolio_beats_independent && policy_correct
}

fn print_case(case: Case) -> bool {
    let passed = passes(case);
    println!(
        "FAMILY_PORTFOLIO_JSON={{\"schema\":\"luminous.symtropy.crn-event-family-portfolio.v0.1\",\"beta_u\":{:.6},\"beta_v\":{:.6},\"train_replicates\":{},\"holdout_replicates\":{},\"subjects_per_replicate\":{},\"known_ate\":{:.9},\"training_covariance_u\":{:.12e},\"training_covariance_v\":{:.12e},\"training_covariance_total\":{:.12e},\"global_policy\":\"{}\",\"family_u_policy\":\"{}\",\"family_v_policy\":\"{}\",\"mean_all_independent\":{:.12e},\"mean_all_shared\":{:.12e},\"mean_global_policy\":{:.12e},\"mean_per_family_policy\":{:.12e},\"variance_ratio_all_shared_to_independent\":{:.9},\"variance_ratio_global_to_independent\":{:.9},\"variance_ratio_per_family_to_independent\":{:.9},\"theoretical_ratio_all_shared\":{:.9},\"theoretical_ratio_per_family\":{:.9},\"ratio_ci_all_shared\":[{:.9},{:.9}],\"ratio_ci_global\":[{:.9},{:.9}],\"ratio_ci_per_family\":[{:.9},{:.9}],\"portfolio_to_global_variance_ratio_ci\":[{:.9},{:.9}],\"bootstrap_replicates\":{},\"pass\":{}}}",
        case.beta_u,
        case.beta_v,
        TRAIN_REPLICATES,
        HOLDOUT_REPLICATES,
        SUBJECTS,
        case.known_ate,
        case.covariance_u_train,
        case.covariance_v_train,
        case.covariance_total_train,
        if case.global_share_both { "SHARED_BOTH" } else { "INDEPENDENT_BOTH" },
        if case.share_u { "SHARED" } else { "INDEPENDENT" },
        if case.share_v { "SHARED" } else { "INDEPENDENT" },
        case.means[0],
        case.means[1],
        case.means[2],
        case.means[3],
        case.ratios[0],
        case.ratios[1],
        case.ratios[2],
        case.theoretical_shared_ratio,
        case.theoretical_portfolio_ratio,
        case.ratio_intervals[0].lower,
        case.ratio_intervals[0].upper,
        case.ratio_intervals[1].lower,
        case.ratio_intervals[1].upper,
        case.ratio_intervals[2].lower,
        case.ratio_intervals[2].upper,
        case.portfolio_to_global_interval.lower,
        case.portfolio_to_global_interval.upper,
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
    println!("benchmark_schema=luminous.symtropy.crn-event-family-portfolio.v0.1");
    println!(
        "selection_rule=per_event_family_shared_if_training_covariance_positive_else_independent"
    );
    println!("training_and_holdout_seed_sets_disjoint=true");

    const CASES: [(f64, f64); 2] = [(0.5, -1.5), (-1.5, 0.5)];
    let mut all_pass = true;
    for (beta_u, beta_v) in CASES {
        all_pass &= print_case(run_case(beta_u, beta_v));
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
    fn event_families_have_opposite_covariance_signs_and_positive_total() {
        assert!(family_covariance(0.5) > 0.0);
        assert!(family_covariance(-1.5) < 0.0);
        assert!(family_covariance(0.5) + family_covariance(-1.5) > 0.0);
    }

    #[test]
    fn swapping_coefficients_swaps_the_per_family_policy() {
        let first = (family_covariance(0.5) > 0.0, family_covariance(-1.5) > 0.0);
        let second = (family_covariance(-1.5) > 0.0, family_covariance(0.5) > 0.0);
        assert_eq!(first, (true, false));
        assert_eq!(second, (false, true));
    }

    #[test]
    fn portfolio_has_lower_theoretical_variance_than_global_all_shared() {
        let global = theoretical_shared_variance(0.5, -1.5);
        let portfolio = theoretical_portfolio_variance(0.5, -1.5);
        assert!(portfolio < global * 0.90);
    }

    #[test]
    fn train_and_holdout_seed_ranges_are_disjoint() {
        let training = (0..TRAIN_REPLICATES)
            .map(seed_for)
            .collect::<std::collections::HashSet<_>>();
        let holdout = (0..HOLDOUT_REPLICATES)
            .map(|index| seed_for(TRAIN_REPLICATES + index))
            .collect::<std::collections::HashSet<_>>();
        assert!(training.is_disjoint(&holdout));
    }
}
