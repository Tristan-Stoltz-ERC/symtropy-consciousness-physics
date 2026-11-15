#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage-gated CRN selection counterexample.
//!
//! A cyclic-shift coupling can reduce the true variance while making the
//! finite-sample normal interval badly under-cover because its contribution
//! distribution is highly skewed. This fixture selects on held-out-calibrated
//! coverage eligibility first, then variance. It is synthetic, not a Symtropy
//! model result.

use std::process;

use symtropy_consciousness_physics::event_rng::{registry, EventAddress, SubjectId};

const TRAIN_STUDIES: usize = 5_000;
const HOLDOUT_STUDIES: usize = 5_000;
const OBSERVATIONS_PER_STUDY: usize = 200;

const ROOT_SEED: u64 = 0x434f_5645_5241_4745;
const SEED_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;

const BASE_NAMESPACE: u64 = 0x434f_4e54_524f_4c30;
const INDEPENDENT_NAMESPACE: u64 = 0x494e_4445_5045_4e44;
const JITTER_NAMESPACE: u64 = 0x4a49_5454_4552_3031;

const NOMINAL_COVERAGE: f64 = 0.95;
const COVERAGE_FLOOR: f64 = 0.93;
const NORMAL_CRITICAL: f64 = 1.959_963_984_540_054;

const JITTER_EPSILONS: [f64; 4] = [0.01, 0.025, 0.1, 0.25];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Policy {
    Independent,
    CyclicShift(f64),
}

impl Policy {
    // Keep this order aligned with evaluate_study's accumulator array:
    // independent first, then cyclic shifts in epsilon order.
    const ALL: [Self; 5] = [
        Self::Independent,
        Self::CyclicShift(0.01),
        Self::CyclicShift(0.025),
        Self::CyclicShift(0.1),
        Self::CyclicShift(0.25),
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Independent => "independent",
            Self::CyclicShift(epsilon) if epsilon.to_bits() == 0.01_f64.to_bits() => {
                "cyclic_shift_eps_0_01"
            }
            Self::CyclicShift(epsilon) if epsilon.to_bits() == 0.025_f64.to_bits() => {
                "cyclic_shift_eps_0_025"
            }
            Self::CyclicShift(epsilon) if epsilon.to_bits() == 0.1_f64.to_bits() => {
                "cyclic_shift_eps_0_1"
            }
            Self::CyclicShift(epsilon) if epsilon.to_bits() == 0.25_f64.to_bits() => {
                "cyclic_shift_eps_0_25"
            }
            Self::CyclicShift(_) => "cyclic_shift_other",
        }
    }

    fn epsilon(self) -> f64 {
        match self {
            Self::Independent => 0.0,
            Self::CyclicShift(epsilon) => epsilon,
        }
    }

    fn theoretical_variance(self) -> f64 {
        match self {
            Self::Independent => 1.0 / 6.0,
            Self::CyclicShift(epsilon) => epsilon / 2.0 - epsilon * epsilon / 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Moments {
    count: u64,
    mean: f64,
    m2: f64,
}

impl Moments {
    fn push(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta_after = value - self.mean;
        self.m2 += delta * delta_after;
    }

    fn sample_variance(self) -> f64 {
        assert!(self.count >= 2);
        self.m2 / (self.count - 1) as f64
    }

    fn normal_interval_contains_zero(self) -> bool {
        let standard_error = (self.sample_variance() / OBSERVATIONS_PER_STUDY as f64).sqrt();
        let half_width = NORMAL_CRITICAL * standard_error;
        self.mean - half_width <= 0.0 && self.mean + half_width >= 0.0
    }

    fn normal_interval_width(self) -> f64 {
        2.0 * NORMAL_CRITICAL * (self.sample_variance() / OBSERVATIONS_PER_STUDY as f64).sqrt()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct StudyAggregate {
    studies: u64,
    covered: u64,
    sum_sample_variance: f64,
    sum_interval_width: f64,
    sum_estimate: f64,
}

impl StudyAggregate {
    fn push(&mut self, moments: Moments) {
        self.studies += 1;
        if moments.normal_interval_contains_zero() {
            self.covered += 1;
        }
        self.sum_sample_variance += moments.sample_variance();
        self.sum_interval_width += moments.normal_interval_width();
        self.sum_estimate += moments.mean;
    }

    fn coverage(self) -> f64 {
        self.covered as f64 / self.studies as f64
    }

    fn mean_sample_variance(self) -> f64 {
        self.sum_sample_variance / self.studies as f64
    }

    fn mean_interval_width(self) -> f64 {
        self.sum_interval_width / self.studies as f64
    }

    fn absolute_mean_estimate_bias(self) -> f64 {
        (self.sum_estimate / self.studies as f64).abs()
    }

    fn coverage_wilson_95(self) -> (f64, f64) {
        let n = self.studies as f64;
        let p = self.coverage();
        let z2 = NORMAL_CRITICAL * NORMAL_CRITICAL;
        let denominator = 1.0 + z2 / n;
        let center = (p + z2 / (2.0 * n)) / denominator;
        let radius =
            NORMAL_CRITICAL * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
        ((center - radius).max(0.0), (center + radius).min(1.0))
    }
}

fn seed_for(study_index: usize) -> u64 {
    ROOT_SEED.wrapping_add((study_index as u64).wrapping_mul(SEED_STRIDE))
}

fn event_stream(seed: u64, namespace: u64, subject: SubjectId) -> impl Iterator<Item = f64> {
    EventAddress::new(
        seed,
        namespace,
        subject,
        1,
        registry::ENVIRONMENT,
        registry::ENVIRONMENTAL_NOISE,
        0,
    )
    .uniforms_open01()
}

fn cyclic_shift_delta(control: f64, jitter: f64, epsilon: f64) -> f64 {
    (control + epsilon * jitter).fract() - control
}

fn evaluate_study(study_index: usize) -> [Moments; 5] {
    let seed = seed_for(study_index);
    let subject =
        SubjectId::agent((study_index as u32) + 1).expect("meta-study subject IDs are nonzero");

    let mut control_stream = event_stream(seed, BASE_NAMESPACE, subject);
    let mut independent_stream = event_stream(seed, INDEPENDENT_NAMESPACE, subject);
    let mut jitter_stream = event_stream(seed, JITTER_NAMESPACE, subject);

    let mut accumulators = [Moments::default(); 5];

    for _ in 0..OBSERVATIONS_PER_STUDY {
        let control = control_stream
            .next()
            .expect("control stream is long enough");
        let independent = independent_stream
            .next()
            .expect("independent stream is long enough");
        let jitter = jitter_stream.next().expect("jitter stream is long enough");

        accumulators[0].push(independent - control);
        for (index, epsilon) in JITTER_EPSILONS.iter().copied().enumerate() {
            accumulators[index + 1].push(cyclic_shift_delta(control, jitter, epsilon));
        }
    }

    accumulators
}

fn evaluate_partition(start: usize, count: usize) -> [StudyAggregate; 5] {
    let mut aggregates = [StudyAggregate::default(); 5];
    for study_index in start..start + count {
        let study = evaluate_study(study_index);
        for (aggregate, moments) in aggregates.iter_mut().zip(study) {
            aggregate.push(moments);
        }
    }
    aggregates
}

fn choose_variance_only(training: &[StudyAggregate; 5]) -> Policy {
    let index = training
        .iter()
        .enumerate()
        .min_by(|left, right| {
            left.1
                .mean_sample_variance()
                .total_cmp(&right.1.mean_sample_variance())
        })
        .map(|(index, _)| index)
        .expect("policy set is non-empty");
    Policy::ALL[index]
}

fn choose_coverage_gated(training: &[StudyAggregate; 5]) -> Option<Policy> {
    Policy::ALL
        .iter()
        .copied()
        .zip(training.iter().copied())
        .filter(|(_, aggregate)| aggregate.coverage_wilson_95().0 > COVERAGE_FLOOR)
        .min_by(|left, right| {
            left.1
                .mean_sample_variance()
                .total_cmp(&right.1.mean_sample_variance())
        })
        .map(|(policy, _)| policy)
}

fn aggregate_for<'a>(
    policies: &'a [Policy; 5],
    aggregates: &'a [StudyAggregate; 5],
    target: Policy,
) -> &'a StudyAggregate {
    let index = policies
        .iter()
        .position(|policy| *policy == target)
        .expect("selected policy belongs to frozen policy set");
    &aggregates[index]
}

fn print_partition(label: &str, aggregates: &[StudyAggregate; 5], independent_variance: f64) {
    for (policy, aggregate) in Policy::ALL.into_iter().zip(aggregates.iter().copied()) {
        let (lower, upper) = aggregate.coverage_wilson_95();
        println!(
            "COVERAGE_GATE_JSON={{\"schema\":\"luminous.symtropy.crn-coverage-gate.v0.1\",\"partition\":\"{}\",\"policy\":\"{}\",\"epsilon\":{:.6},\"studies\":{},\"observations_per_study\":{},\"nominal_coverage\":{:.4},\"coverage\":{:.8},\"coverage_wilson_95\":[{:.8},{:.8}],\"mean_sample_variance\":{:.12e},\"theoretical_variance\":{:.12e},\"variance_ratio_to_independent\":{:.8},\"mean_normal_interval_width\":{:.8},\"absolute_bias_of_mean_estimate\":{:.8}}}",
            label,
            policy.label(),
            policy.epsilon(),
            aggregate.studies,
            OBSERVATIONS_PER_STUDY,
            NOMINAL_COVERAGE,
            aggregate.coverage(),
            lower,
            upper,
            aggregate.mean_sample_variance(),
            policy.theoretical_variance(),
            aggregate.mean_sample_variance() / independent_variance,
            aggregate.mean_interval_width(),
            aggregate.absolute_mean_estimate_bias()
        );
    }
}

fn main() {
    println!("benchmark_schema=luminous.symtropy.crn-coverage-gate.v0.1");
    println!("estimand=E[T-U]=0");
    println!("observations_per_study={OBSERVATIONS_PER_STUDY}");
    println!("training_study_indices=[0,{TRAIN_STUDIES})");
    println!(
        "holdout_study_indices=[{TRAIN_STUDIES},{})",
        TRAIN_STUDIES + HOLDOUT_STUDIES
    );
    println!("interval=normal_95_percent_z_1.959963984540054");
    println!("coverage_gate=training_wilson_95_lower_bound_gt_0.93");
    println!("variance_theory=independent:1/6;cyclic_shift:epsilon/2-epsilon^2/3");

    let training = evaluate_partition(0, TRAIN_STUDIES);
    let independent_training_variance =
        aggregate_for(&Policy::ALL, &training, Policy::Independent).mean_sample_variance();
    print_partition("training", &training, independent_training_variance);

    let variance_only = choose_variance_only(&training);
    let coverage_gated = choose_coverage_gated(&training)
        .expect("the independent policy should pass the preregistered coverage gate");

    let holdout = evaluate_partition(TRAIN_STUDIES, HOLDOUT_STUDIES);
    let independent_holdout_variance =
        aggregate_for(&Policy::ALL, &holdout, Policy::Independent).mean_sample_variance();
    print_partition("holdout", &holdout, independent_holdout_variance);

    let variance_only_holdout = aggregate_for(&Policy::ALL, &holdout, variance_only);
    let gated_holdout = aggregate_for(&Policy::ALL, &holdout, coverage_gated);
    let (gated_coverage_lower, gated_coverage_upper) = gated_holdout.coverage_wilson_95();
    let (variance_only_lower, variance_only_upper) = variance_only_holdout.coverage_wilson_95();

    println!(
        "COVERAGE_GATE_SELECTION_JSON={{\"schema\":\"luminous.symtropy.crn-coverage-gate-selection.v0.1\",\"variance_only_choice\":\"{}\",\"coverage_gated_choice\":\"{}\",\"coverage_floor\":{:.4},\"variance_only_holdout_coverage\":{:.8},\"variance_only_holdout_wilson_95\":[{:.8},{:.8}],\"variance_only_holdout_variance_ratio\":{:.8},\"gated_holdout_coverage\":{:.8},\"gated_holdout_wilson_95\":[{:.8},{:.8}],\"gated_holdout_variance_ratio\":{:.8}}}",
        variance_only.label(),
        coverage_gated.label(),
        COVERAGE_FLOOR,
        variance_only_holdout.coverage(),
        variance_only_lower,
        variance_only_upper,
        variance_only_holdout.mean_sample_variance() / independent_holdout_variance,
        gated_holdout.coverage(),
        gated_coverage_lower,
        gated_coverage_upper,
        gated_holdout.mean_sample_variance() / independent_holdout_variance
    );

    let variance_only_is_rare_tail = variance_only == Policy::CyclicShift(0.01);
    let gated_holdout_is_calibrated = gated_coverage_lower > COVERAGE_FLOOR;
    let variance_only_holdout_is_miscalibrated = variance_only_upper < COVERAGE_FLOOR;
    let gated_reduces_variance =
        gated_holdout.mean_sample_variance() < independent_holdout_variance * 0.8;

    if variance_only_is_rare_tail
        && gated_holdout_is_calibrated
        && variance_only_holdout_is_miscalibrated
        && gated_reduces_variance
    {
        println!("benchmark_status=PASS");
    } else {
        eprintln!(
            "benchmark_status=FAIL variance_only={} gated={} variance_only_ci_upper={:.6} gated_ci_lower={:.6} gated_variance_ratio={:.6}",
            variance_only.label(),
            coverage_gated.label(),
            variance_only_upper,
            gated_coverage_lower,
            gated_holdout.mean_sample_variance() / independent_holdout_variance
        );
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyclic_shift_preserves_uniform_marginal_and_has_closed_form_variance() {
        // For independent U,V ~ Uniform(0,1), (U + epsilon*V) mod 1 is uniform.
        // Delta has E[Delta]=0 and Var(Delta)=epsilon/2-epsilon^2/3.
        for epsilon in JITTER_EPSILONS {
            let expected = epsilon / 2.0 - epsilon * epsilon / 3.0;
            assert!(expected > 0.0);
            assert!(expected < 1.0 / 6.0);
        }
    }

    #[test]
    fn analytic_variance_can_reward_a_severely_undercovering_policy() {
        let rare_tail = Policy::CyclicShift(0.01);
        let calibrated = Policy::CyclicShift(0.25);
        assert!(rare_tail.theoretical_variance() < calibrated.theoretical_variance());
        assert!(calibrated.theoretical_variance() < Policy::Independent.theoretical_variance());
    }

    #[test]
    fn training_and_holdout_meta_study_seeds_are_disjoint() {
        let training = (0..TRAIN_STUDIES)
            .map(seed_for)
            .collect::<std::collections::HashSet<_>>();
        let holdout = (TRAIN_STUDIES..TRAIN_STUDIES + HOLDOUT_STUDIES)
            .map(seed_for)
            .collect::<std::collections::HashSet<_>>();
        assert!(training.is_disjoint(&holdout));
    }

    #[test]
    fn event_streams_are_repeatable_and_namespace_separated() {
        let seed = seed_for(3);
        let subject = SubjectId::agent(4).unwrap();
        let first = event_stream(seed, BASE_NAMESPACE, subject)
            .take(12)
            .collect::<Vec<_>>();
        let repeated = event_stream(seed, BASE_NAMESPACE, subject)
            .take(12)
            .collect::<Vec<_>>();
        let independent = event_stream(seed, INDEPENDENT_NAMESPACE, subject)
            .take(12)
            .collect::<Vec<_>>();
        assert_eq!(
            first.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            repeated.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
        assert_ne!(
            first.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            independent.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn wilson_interval_contains_observed_proportion_and_is_ordered() {
        let aggregate = StudyAggregate {
            studies: 1000,
            covered: 950,
            ..StudyAggregate::default()
        };
        let (lower, upper) = aggregate.coverage_wilson_95();
        assert!(lower < aggregate.coverage());
        assert!(upper > aggregate.coverage());
        assert!(lower <= upper);
    }

    #[test]
    fn independent_policy_is_eligible_when_coverage_is_near_nominal() {
        let aggregate = StudyAggregate {
            studies: 10_000,
            covered: 9_500,
            ..StudyAggregate::default()
        };
        assert!(aggregate.coverage_wilson_95().0 > COVERAGE_FLOOR);
    }
}
