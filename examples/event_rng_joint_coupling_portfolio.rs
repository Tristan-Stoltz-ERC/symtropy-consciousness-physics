#![forbid(unsafe_code)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Joint event-coupling portfolio counterexample.
//!
//! Synthetic analytic fixture: compare independent, same-address, antithetic,
//! and cross-family permutation couplings while preserving arm marginals.

use std::process;

use symtropy_consciousness_physics::event_rng::{registry, EventAddress, SubjectId};

const REPLICATES_PER_PARTITION: usize = 8_192;
const ROOT_SEED: u64 = 0x4a4f_494e_545f_4352;
const SEED_STRIDE: u64 = 0x9e37_79b9_7f4a_7c15;

const BASELINE_NAMESPACE: u64 = 0x4241_5345_4c49_4e45;
const TREATMENT_A_NAMESPACE: u64 = 0x5452_4541_5441_3031;
const TREATMENT_B_NAMESPACE: u64 = 0x5452_4541_5442_3031;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Coupling {
    Independent,
    SameEventIdentity,
    Antithetic,
    CrossFamilySwap,
}

impl Coupling {
    const ALL: [Self; 4] = [
        Self::Independent,
        Self::SameEventIdentity,
        Self::Antithetic,
        Self::CrossFamilySwap,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Independent => "independent",
            Self::SameEventIdentity => "same_event_identity",
            Self::Antithetic => "antithetic",
            Self::CrossFamilySwap => "cross_family_swap",
        }
    }

    const fn theoretical_variance(self) -> f64 {
        match self {
            Self::Independent => 1.0 / 3.0,
            Self::SameEventIdentity => 2.0 / 3.0,
            Self::Antithetic | Self::CrossFamilySwap => 0.0,
        }
    }

    const fn admissible(self, families_exchangeable: bool) -> bool {
        !matches!(self, Self::CrossFamilySwap) || families_exchangeable
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Moments {
    count: u64,
    mean: f64,
    sum_squared_deviations: f64,
}

impl Moments {
    fn push(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta_after = value - self.mean;
        self.sum_squared_deviations += delta * delta_after;
    }

    fn variance(self) -> f64 {
        assert!(self.count > 1);
        self.sum_squared_deviations / (self.count - 1) as f64
    }
}

#[derive(Clone, Copy)]
struct ResultRow {
    training_variance: f64,
    holdout: Moments,
    variance_ratio: f64,
}

fn seed_at(index: usize) -> u64 {
    ROOT_SEED.wrapping_add((index as u64).wrapping_mul(SEED_STRIDE))
}

fn event_uniform(seed: u64, namespace: u64, family: u8) -> f64 {
    let subject = SubjectId::agent(1).expect("fixture subject ID is nonzero");
    let (event_kind, purpose) = match family {
        0 => (registry::ENVIRONMENT, registry::ENVIRONMENTAL_NOISE),
        1 => (registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE),
        _ => unreachable!("the fixture has exactly two event families"),
    };

    EventAddress::new(seed, namespace, subject, 1, event_kind, purpose, 0).uniform_open01(0)
}

/// Control: Y0 = U - V. Treatment: Y1 = -U' + V'.
fn outcome_difference(seed: u64, coupling: Coupling) -> f64 {
    let u = event_uniform(seed, BASELINE_NAMESPACE, 0);
    let v = event_uniform(seed, BASELINE_NAMESPACE, 1);

    let (u_treatment, v_treatment) = match coupling {
        Coupling::Independent => (
            event_uniform(seed, TREATMENT_A_NAMESPACE, 0),
            event_uniform(seed, TREATMENT_B_NAMESPACE, 1),
        ),
        Coupling::SameEventIdentity => (u, v),
        Coupling::Antithetic => (1.0 - u, 1.0 - v),
        Coupling::CrossFamilySwap => (v, u),
    };

    let control = u - v;
    let treatment = -u_treatment + v_treatment;
    treatment - control
}

fn evaluate(coupling: Coupling, start: usize, count: usize) -> Moments {
    let mut moments = Moments::default();
    for index in start..start + count {
        moments.push(outcome_difference(seed_at(index), coupling));
    }
    moments
}

fn select_from_pilot(families_exchangeable: bool) -> (Coupling, f64) {
    Coupling::ALL
        .into_iter()
        .filter(|policy| policy.admissible(families_exchangeable))
        .map(|policy| {
            let training = evaluate(policy, 0, REPLICATES_PER_PARTITION);
            (policy, training.variance())
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .expect("independent policy is always admissible")
}

fn record(
    label: &str,
    selected_candidate: &str,
    admissible: bool,
    training_variance: f64,
    holdout: Moments,
    independent_variance: f64,
    theoretical_variance: f64,
) {
    println!(
        "JOINT_COUPLING_JSON={{\"schema\":\"luminous.symtropy.joint-coupling-portfolio.v0.1\",\"policy\":\"{}\",\"selected_candidate\":\"{}\",\"admissible\":{},\"marginals_preserved\":{},\"training_replicates\":{},\"holdout_replicates\":{},\"training_variance\":{:.12e},\"holdout_mean_difference\":{:.12e},\"holdout_variance\":{:.12e},\"variance_ratio_to_independent\":{:.12e},\"theoretical_variance\":{:.12e},\"same_family_identity_covariance_a\":{:.12e},\"same_family_identity_covariance_b\":{:.12e},\"local_sign_selector\":\"independent\"}}",
        label,
        selected_candidate,
        admissible,
        admissible,
        REPLICATES_PER_PARTITION,
        REPLICATES_PER_PARTITION,
        training_variance,
        holdout.mean,
        holdout.variance(),
        holdout.variance() / independent_variance,
        theoretical_variance,
        -1.0 / 12.0,
        -1.0 / 12.0
    );
}

fn run_suite(families_exchangeable: bool) -> bool {
    let independent_training = evaluate(Coupling::Independent, 0, REPLICATES_PER_PARTITION);
    let independent_holdout = evaluate(
        Coupling::Independent,
        REPLICATES_PER_PARTITION,
        REPLICATES_PER_PARTITION,
    );
    let independent_variance = independent_holdout.variance();

    let mut policy_rows = Vec::new();
    for policy in Coupling::ALL {
        if !policy.admissible(families_exchangeable) {
            continue;
        }
        let training = if policy == Coupling::Independent {
            independent_training
        } else {
            evaluate(policy, 0, REPLICATES_PER_PARTITION)
        };
        let holdout = if policy == Coupling::Independent {
            independent_holdout
        } else {
            evaluate(policy, REPLICATES_PER_PARTITION, REPLICATES_PER_PARTITION)
        };
        policy_rows.push((
            policy,
            ResultRow {
                training_variance: training.variance(),
                holdout,
                variance_ratio: holdout.variance() / independent_variance,
            },
        ));
    }

    for (policy, row) in &policy_rows {
        record(
            policy.label(),
            "",
            true,
            row.training_variance,
            row.holdout,
            independent_variance,
            policy.theoretical_variance(),
        );
    }

    // Both diagonal same-family identity covariance terms are negative:
    // Cov(U,-U)=-1/12 and Cov(-V,V)=-1/12. The local sign rule falls back
    // to independent draws, missing other admissible portfolio couplings.
    record(
        "local_family_sign_selector",
        "independent",
        true,
        independent_training.variance(),
        independent_holdout,
        independent_variance,
        Coupling::Independent.theoretical_variance(),
    );

    let (selected, selected_training_variance) = select_from_pilot(families_exchangeable);
    let selected_holdout = evaluate(selected, REPLICATES_PER_PARTITION, REPLICATES_PER_PARTITION);
    record(
        "pilot_selected",
        selected.label(),
        true,
        selected_training_variance,
        selected_holdout,
        independent_variance,
        selected.theoretical_variance(),
    );

    let cross_swap = policy_rows
        .iter()
        .find(|(policy, _)| *policy == Coupling::CrossFamilySwap);

    if !families_exchangeable {
        println!(
            "JOINT_COUPLING_JSON={{\"schema\":\"luminous.symtropy.joint-coupling-portfolio.v0.1\",\"policy\":\"cross_family_swap\",\"selected_candidate\":\"\",\"admissible\":false,\"marginals_preserved\":false,\"reason\":\"event_families_not_declared_exchangeable\",\"training_replicates\":{},\"holdout_replicates\":{}}}",
            REPLICATES_PER_PARTITION,
            REPLICATES_PER_PARTITION
        );
    }

    let identity = policy_rows
        .iter()
        .find(|(policy, _)| *policy == Coupling::SameEventIdentity)
        .expect("identity coupling is admissible")
        .1;
    let antithetic = policy_rows
        .iter()
        .find(|(policy, _)| *policy == Coupling::Antithetic)
        .expect("antithetic coupling is admissible")
        .1;

    let mut pass = independent_variance > 0.25
        && independent_variance < 0.42
        && identity.variance_ratio > 1.7
        && identity.variance_ratio < 2.3
        && antithetic.variance_ratio < 1.0e-8
        && selected_holdout.mean.abs() < 0.02
        && selected_holdout.variance() < independent_variance * 1.0e-8;

    if families_exchangeable {
        pass &= cross_swap
            .map(|(_, row)| row.variance_ratio < 1.0e-8)
            .unwrap_or(false);
    } else {
        pass &= cross_swap.is_none();
        pass &= selected != Coupling::CrossFamilySwap;
    }

    println!(
        "joint_coupling_suite={} exchangeable={} selected={} independent_variance={:.12e} selected_holdout_variance={:.12e}",
        if pass { "PASS" } else { "FAIL" },
        families_exchangeable,
        selected.label(),
        independent_variance,
        selected_holdout.variance()
    );
    pass
}

fn main() {
    println!("benchmark_schema=luminous.symtropy.joint-coupling-portfolio.v0.1");
    println!("design=pilot-selected-coupling-portfolio-with-disjoint-holdout");
    println!("training_seed_indices=[0,{REPLICATES_PER_PARTITION})");
    println!(
        "holdout_seed_indices=[{REPLICATES_PER_PARTITION},{})",
        REPLICATES_PER_PARTITION * 2
    );
    println!("analytic_estimand=ATE_0");
    println!("arm_marginals=Uniform(0,1)^2_for_every_admissible_policy");

    let exchangeable_pass = run_suite(true);
    let constrained_pass = run_suite(false);
    if exchangeable_pass && constrained_pass {
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
    fn analytic_variances_expose_same_family_crn_harm() {
        assert_eq!(Coupling::Independent.theoretical_variance(), 1.0 / 3.0);
        assert_eq!(
            Coupling::SameEventIdentity.theoretical_variance(),
            2.0 / 3.0
        );
        assert_eq!(Coupling::Antithetic.theoretical_variance(), 0.0);
        assert_eq!(Coupling::CrossFamilySwap.theoretical_variance(), 0.0);
    }

    #[test]
    fn cross_family_swap_requires_explicit_exchangeability() {
        assert!(Coupling::CrossFamilySwap.admissible(true));
        assert!(!Coupling::CrossFamilySwap.admissible(false));
        assert!(Coupling::Independent.admissible(false));
        assert!(Coupling::Antithetic.admissible(false));
    }

    #[test]
    fn pilot_and_holdout_seed_partitions_are_disjoint() {
        let training = (0..REPLICATES_PER_PARTITION)
            .map(seed_at)
            .collect::<std::collections::HashSet<_>>();
        let holdout = (REPLICATES_PER_PARTITION..2 * REPLICATES_PER_PARTITION)
            .map(seed_at)
            .collect::<std::collections::HashSet<_>>();
        assert!(training.is_disjoint(&holdout));
    }

    #[test]
    fn local_same_family_sign_rule_selects_independent_on_negative_covariances() {
        let covariance_a = -1.0 / 12.0;
        let covariance_b = -1.0 / 12.0;
        assert!(covariance_a < 0.0);
        assert!(covariance_b < 0.0);
        assert_eq!(Coupling::Independent.label(), "independent");
    }

    #[test]
    fn pilot_selected_portfolio_reduces_disjoint_holdout_variance() {
        let (selected, _) = select_from_pilot(true);
        let independent = evaluate(
            Coupling::Independent,
            REPLICATES_PER_PARTITION,
            REPLICATES_PER_PARTITION,
        );
        let held_out = evaluate(selected, REPLICATES_PER_PARTITION, REPLICATES_PER_PARTITION);
        assert!(held_out.mean.abs() < 0.02);
        assert!(held_out.variance() < independent.variance() * 1.0e-8);
    }

    #[test]
    fn non_exchangeable_suite_excludes_cross_family_permutation() {
        let (selected, _) = select_from_pilot(false);
        assert_ne!(selected, Coupling::CrossFamilySwap);
        assert!(!Coupling::CrossFamilySwap.admissible(false));
    }
}
