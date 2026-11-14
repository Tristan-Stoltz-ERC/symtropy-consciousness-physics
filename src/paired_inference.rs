// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seed-keyed paired inference for counterfactual simulation experiments.
//!
//! Additive qualification primitives. Historical analyses and verdicts remain
//! untouched. The independent unit is one complete simulation seed replicate.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const EXACT_SIGN_FLIP_MAX_PAIRS: usize = 20;
pub const DEFAULT_SIGN_FLIP_MONTE_CARLO_DRAWS: usize = 99_999;
pub const DEFAULT_BOOTSTRAP_REPLICATES: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeededValue {
    pub seed_id: u64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairedDifference {
    pub seed_id: u64,
    pub control: f64,
    pub treatment: f64,
    /// Treatment minus control.
    pub difference: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PairingError {
    EmptySample { condition: &'static str },
    DuplicateSeed { condition: &'static str, seed_id: u64 },
    MissingSeed { condition: &'static str, seed_id: u64 },
    NonFiniteValue { condition: &'static str, seed_id: u64 },
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySample { condition } => write!(f, "{condition} sample is empty"),
            Self::DuplicateSeed { condition, seed_id } =>
                write!(f, "{condition} contains duplicate seed {seed_id}"),
            Self::MissingSeed { condition, seed_id } =>
                write!(f, "{condition} is missing seed {seed_id}"),
            Self::NonFiniteValue { condition, seed_id } =>
                write!(f, "{condition} has a non-finite value for seed {seed_id}"),
        }
    }
}
impl Error for PairingError {}

/// Pair by explicit seed identity, never array position. Reject mismatched,
/// duplicated, empty, or non-finite samples; return ascending seed order.
pub fn pair_by_seed(
    control: &[SeededValue],
    treatment: &[SeededValue],
) -> Result<Vec<PairedDifference>, PairingError> {
    fn index(
        values: &[SeededValue],
        condition: &'static str,
    ) -> Result<BTreeMap<u64, f64>, PairingError> {
        if values.is_empty() {
            return Err(PairingError::EmptySample { condition });
        }
        let mut indexed = BTreeMap::new();
        for item in values {
            if !item.value.is_finite() {
                return Err(PairingError::NonFiniteValue {
                    condition,
                    seed_id: item.seed_id,
                });
            }
            if indexed.insert(item.seed_id, item.value).is_some() {
                return Err(PairingError::DuplicateSeed {
                    condition,
                    seed_id: item.seed_id,
                });
            }
        }
        Ok(indexed)
    }

    let control = index(control, "control")?;
    let treatment = index(treatment, "treatment")?;

    for seed_id in control.keys() {
        if !treatment.contains_key(seed_id) {
            return Err(PairingError::MissingSeed {
                condition: "treatment",
                seed_id: *seed_id,
            });
        }
    }
    for seed_id in treatment.keys() {
        if !control.contains_key(seed_id) {
            return Err(PairingError::MissingSeed {
                condition: "control",
                seed_id: *seed_id,
            });
        }
    }

    let mut result = Vec::with_capacity(control.len());
    for (seed_id, control_value) in control {
        let treatment_value = treatment[&seed_id];
        let difference = treatment_value - control_value;
        if !difference.is_finite() {
            return Err(PairingError::NonFiniteValue {
                condition: "paired difference",
                seed_id,
            });
        }
        result.push(PairedDifference {
            seed_id,
            control: control_value,
            treatment: treatment_value,
            difference,
        });
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferenceError {
    EmptyInput,
    NonFiniteDifference { seed_id: u64 },
    InvalidConfidenceLevel(f64),
    ZeroReplicates,
    InvalidMargin(f64),
    InvalidInterval,
    InvalidAlpha(f64),
    InvalidPValue { label: String, value: f64 },
    DuplicateLabel(String),
}
impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "at least one paired observation is required"),
            Self::NonFiniteDifference { seed_id } =>
                write!(f, "seed {seed_id} has a non-finite paired difference"),
            Self::InvalidConfidenceLevel(v) =>
                write!(f, "confidence level must be in (0, 1), got {v}"),
            Self::ZeroReplicates => write!(f, "replicate count must be greater than zero"),
            Self::InvalidMargin(v) => write!(f, "equivalence margin must be positive, got {v}"),
            Self::InvalidInterval => write!(f, "confidence interval is invalid"),
            Self::InvalidAlpha(v) => write!(f, "alpha must be in (0, 1), got {v}"),
            Self::InvalidPValue { label, value } =>
                write!(f, "p-value for {label} must be in [0, 1], got {value}"),
            Self::DuplicateLabel(label) => write!(f, "duplicate comparison label: {label}"),
        }
    }
}
impl Error for InferenceError {}

fn differences(pairs: &[PairedDifference]) -> Result<Vec<f64>, InferenceError> {
    if pairs.is_empty() {
        return Err(InferenceError::EmptyInput);
    }
    let mut values = Vec::with_capacity(pairs.len());
    for pair in pairs {
        if !pair.difference.is_finite() {
            return Err(InferenceError::NonFiniteDifference { seed_id: pair.seed_id });
        }
        values.push(pair.difference);
    }
    Ok(values)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairedSummary {
    pub n: usize,
    pub mean_difference: f64,
    pub median_difference: f64,
    pub sample_standard_deviation: Option<f64>,
    pub standard_error: Option<f64>,
    /// Paired Cohen's d_z; absent when sample SD is zero or undefined.
    pub cohens_dz: Option<f64>,
}

pub fn summarize_paired(pairs: &[PairedDifference]) -> Result<PairedSummary, InferenceError> {
    let mut values = differences(pairs)?;
    let mut mean = 0.0;
    let mut sum_squared_deviations = 0.0;
    for (index, value) in values.iter().copied().enumerate() {
        let delta = value - mean;
        mean += delta / (index + 1) as f64;
        sum_squared_deviations += delta * (value - mean);
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    let median = if values.len() % 2 == 0 {
        values[middle - 1] / 2.0 + values[middle] / 2.0
    } else {
        values[middle]
    };
    let sd = if values.len() > 1 {
        Some((sum_squared_deviations.max(0.0) / (values.len() - 1) as f64).sqrt())
    } else {
        None
    };
    let se = sd.map(|value| value / (values.len() as f64).sqrt());
    let dz = sd.filter(|value| *value > 0.0).map(|value| mean / value);
    Ok(PairedSummary {
        n: values.len(),
        mean_difference: mean,
        median_difference: median,
        sample_standard_deviation: sd,
        standard_error: se,
        cohens_dz: dz,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapConfig {
    pub replicates: usize,
    pub confidence_level: f64,
    /// Record this seed in the analysis receipt.
    pub rng_seed: u64,
}
impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            replicates: DEFAULT_BOOTSTRAP_REPLICATES,
            confidence_level: 0.95,
            rng_seed: 0x5041_4952_4544_0001,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapInterval {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
    pub replicates: usize,
    pub rng_seed: u64,
}

/// Percentile bootstrap of whole seed-level paired differences, not agents/ticks.
pub fn bootstrap_mean_interval(
    pairs: &[PairedDifference],
    config: BootstrapConfig,
) -> Result<BootstrapInterval, InferenceError> {
    let values = differences(pairs)?;
    if config.replicates == 0 {
        return Err(InferenceError::ZeroReplicates);
    }
    if !config.confidence_level.is_finite()
        || !(0.0..1.0).contains(&config.confidence_level)
    {
        return Err(InferenceError::InvalidConfidenceLevel(config.confidence_level));
    }
    let mut rng = SplitMix64::new(config.rng_seed);
    let mut means = Vec::with_capacity(config.replicates);
    for _ in 0..config.replicates {
        let mut mean = 0.0;
        for index in 0..values.len() {
            let sample = values[rng.index(values.len())];
            mean += (sample - mean) / (index + 1) as f64;
        }
        means.push(mean);
    }
    means.sort_by(f64::total_cmp);
    let tail = (1.0 - config.confidence_level) / 2.0;
    Ok(BootstrapInterval {
        lower: quantile(&means, tail),
        upper: quantile(&means, 1.0 - tail),
        confidence_level: config.confidence_level,
        replicates: config.replicates,
        rng_seed: config.rng_seed,
    })
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability * (sorted.len() - 1) as f64;
    let lo = position.floor() as usize;
    let hi = position.ceil() as usize;
    sorted[lo] + (sorted[hi] - sorted[lo]) * (position - lo as f64)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquivalenceStatus {
    EquivalentWithinMargin,
    NotEstablished,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquivalenceScreen {
    pub margin: f64,
    pub status: EquivalenceStatus,
}

/// Interval-containment screen, not a TOST p-value. Margin must be justified
/// and preregistered before examining the comparison.
pub fn screen_equivalence(
    interval: BootstrapInterval,
    margin: f64,
) -> Result<EquivalenceScreen, InferenceError> {
    if !margin.is_finite() || margin <= 0.0 {
        return Err(InferenceError::InvalidMargin(margin));
    }
    if !interval.lower.is_finite()
        || !interval.upper.is_finite()
        || interval.lower > interval.upper
        || !interval.confidence_level.is_finite()
        || !(0.0..1.0).contains(&interval.confidence_level)
        || interval.replicates == 0
    {
        return Err(InferenceError::InvalidInterval);
    }
    let status = if interval.lower > -margin && interval.upper < margin {
        EquivalenceStatus::EquivalentWithinMargin
    } else {
        EquivalenceStatus::NotEstablished
    };
    Ok(EquivalenceScreen { margin, status })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignFlipConfig {
    pub monte_carlo_draws: usize,
    pub rng_seed: u64,
}
impl Default for SignFlipConfig {
    fn default() -> Self {
        Self {
            monte_carlo_draws: DEFAULT_SIGN_FLIP_MONTE_CARLO_DRAWS,
            rng_seed: 0x5349_474E_464C_4950,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignFlipResult {
    pub n: usize,
    pub observed_mean_difference: f64,
    pub p_value: f64,
    pub exact: bool,
    pub permutations_evaluated: u64,
    pub rng_seed: Option<u64>,
}

/// Two-sided sign-flip test. Exact enumeration through 20 pairs; beyond that,
/// deterministic Monte Carlo and the plus-one correction are used.
///
/// Exactness requires within-pair random assignment or a justified
/// sign-exchangeability/symmetry null. Matching seed IDs alone is not enough.
pub fn paired_sign_flip_test(
    pairs: &[PairedDifference],
    config: SignFlipConfig,
) -> Result<SignFlipResult, InferenceError> {
    let values = differences(pairs)?;
    let n = values.len();
    let observed_sum: f64 = values.iter().sum();
    if !observed_sum.is_finite() || values.iter().map(|v| v.abs()).sum::<f64>().is_infinite() {
        return Err(InferenceError::NonFiniteDifference { seed_id: pairs[0].seed_id });
    }
    let observed_mean = observed_sum / n as f64;
    let observed = observed_sum.abs();
    let tolerance = 1e-12 * (1.0 + values.iter().map(|v| v.abs()).sum::<f64>() + observed);

    if n <= EXACT_SIGN_FLIP_MAX_PAIRS {
        let total = 1_u64 << n;
        let mut extreme = 0_u64;
        for mask in 0..total {
            let mut statistic = 0.0;
            for (index, value) in values.iter().copied().enumerate() {
                statistic += if (mask >> index) & 1 == 0 { value } else { -value };
            }
            if statistic.abs() + tolerance >= observed {
                extreme += 1;
            }
        }
        return Ok(SignFlipResult {
            n,
            observed_mean_difference: observed_mean,
            p_value: extreme as f64 / total as f64,
            exact: true,
            permutations_evaluated: total,
            rng_seed: None,
        });
    }

    if config.monte_carlo_draws == 0 {
        return Err(InferenceError::ZeroReplicates);
    }
    let mut rng = SplitMix64::new(config.rng_seed);
    let mut extreme = 0_u64;
    for _ in 0..config.monte_carlo_draws {
        let mut statistic = 0.0;
        for value in values.iter().copied() {
            statistic += if rng.next_u64() & 1 == 0 { value } else { -value };
        }
        if statistic.abs() + tolerance >= observed {
            extreme += 1;
        }
    }
    let draws = config.monte_carlo_draws as u64;
    Ok(SignFlipResult {
        n,
        observed_mean_difference: observed_mean,
        p_value: (extreme + 1) as f64 / (draws + 1) as f64,
        exact: false,
        permutations_evaluated: draws,
        rng_seed: Some(config.rng_seed),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct HolmAdjusted {
    pub label: String,
    pub raw_p_value: f64,
    pub adjusted_p_value: f64,
    pub reject_at_alpha: bool,
}

/// Holm step-down adjustment with monotone adjusted p-values.
pub fn holm_adjust(
    p_values: &[(&str, f64)],
    alpha: f64,
) -> Result<Vec<HolmAdjusted>, InferenceError> {
    if !alpha.is_finite() || !(0.0..1.0).contains(&alpha) {
        return Err(InferenceError::InvalidAlpha(alpha));
    }
    let mut labels = BTreeSet::new();
    for (label, value) in p_values {
        if !labels.insert(*label) {
            return Err(InferenceError::DuplicateLabel((*label).to_string()));
        }
        if !value.is_finite() || !(0.0..=1.0).contains(value) {
            return Err(InferenceError::InvalidPValue {
                label: (*label).to_string(),
                value: *value,
            });
        }
    }
    let mut indexed: Vec<(usize, &str, f64)> = p_values.iter().enumerate()
        .map(|(index, (label, value))| (index, *label, *value))
        .collect();
    indexed.sort_by(|left, right| left.2.total_cmp(&right.2).then(left.0.cmp(&right.0)));
    let mut output: Vec<Option<HolmAdjusted>> = vec![None; p_values.len()];
    let mut running_max = 0.0_f64;
    let total = indexed.len();
    for (rank, (original_index, label, raw)) in indexed.into_iter().enumerate() {
        running_max = running_max.max((raw * (total - rank) as f64).min(1.0));
        output[original_index] = Some(HolmAdjusted {
            label: label.to_string(),
            raw_p_value: raw,
            adjusted_p_value: running_max,
            reject_at_alpha: running_max <= alpha,
        });
    }
    Ok(output.into_iter().map(|item| item.expect("every index is assigned")).collect())
}

/// Deterministic non-cryptographic generator for resampling only. It is not
/// used to generate simulation events or security-sensitive values.
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn index(&mut self, bound: usize) -> usize {
        let bound = bound as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold { return (value % bound) as usize; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pairs(values: &[f64]) -> Vec<PairedDifference> {
        values.iter().copied().enumerate().map(|(i, difference)| {
            PairedDifference {
                seed_id: i as u64,
                control: 0.0,
                treatment: difference,
                difference,
            }
        }).collect()
    }

    #[test]
    fn pairing_is_by_seed_key_not_input_order() {
        let control = [
            SeededValue { seed_id: 20, value: 10.0 },
            SeededValue { seed_id: 10, value: 5.0 },
        ];
        let treatment = [
            SeededValue { seed_id: 10, value: 7.0 },
            SeededValue { seed_id: 20, value: 14.0 },
        ];
        let result = pair_by_seed(&control, &treatment).unwrap();
        assert_eq!(result.iter().map(|p| p.seed_id).collect::<Vec<_>>(), vec![10, 20]);
        assert_eq!(result.iter().map(|p| p.difference).collect::<Vec<_>>(), vec![2.0, 4.0]);
    }

    #[test]
    fn pairing_rejects_duplicate_missing_empty_and_non_finite_keys() {
        let duplicate = [
            SeededValue { seed_id: 1, value: 1.0 },
            SeededValue { seed_id: 1, value: 2.0 },
        ];
        let one = [SeededValue { seed_id: 1, value: 3.0 }];
        assert!(matches!(
            pair_by_seed(&duplicate, &one),
            Err(PairingError::DuplicateSeed { .. })
        ));
        let other = [SeededValue { seed_id: 2, value: 4.0 }];
        assert!(matches!(
            pair_by_seed(&one, &other),
            Err(PairingError::MissingSeed { .. })
        ));
        assert!(matches!(
            pair_by_seed(&[], &one),
            Err(PairingError::EmptySample { .. })
        ));
        let nan = [SeededValue { seed_id: 1, value: f64::NAN }];
        assert!(matches!(
            pair_by_seed(&nan, &one),
            Err(PairingError::NonFiniteValue { .. })
        ));
    }

    #[test]
    fn paired_summary_reports_mean_median_and_dz() {
        let summary = summarize_paired(&make_pairs(&[1.0, 2.0, 3.0])).unwrap();
        assert_eq!(summary.n, 3);
        assert!((summary.mean_difference - 2.0).abs() < 1e-12);
        assert!((summary.median_difference - 2.0).abs() < 1e-12);
        assert!(summary.cohens_dz.unwrap() > 0.0);
    }

    #[test]
    fn exact_sign_flip_enumerates_all_small_sample_assignments() {
        let result = paired_sign_flip_test(&make_pairs(&[1.0, 1.0, 1.0]), SignFlipConfig::default()).unwrap();
        assert!(result.exact);
        assert_eq!(result.permutations_evaluated, 8);
        assert!((result.p_value - 0.25).abs() < 1e-12);
    }

    #[test]
    fn zero_difference_sign_flip_has_unit_p_value() {
        let result = paired_sign_flip_test(&make_pairs(&[0.0, 0.0]), SignFlipConfig::default()).unwrap();
        assert_eq!(result.p_value, 1.0);
    }

    #[test]
    fn monte_carlo_sign_flip_is_reproducible_and_receipted() {
        let input = make_pairs(&vec![1.0; EXACT_SIGN_FLIP_MAX_PAIRS + 1]);
        let cfg = SignFlipConfig { monte_carlo_draws: 2_000, rng_seed: 17 };
        let first = paired_sign_flip_test(&input, cfg).unwrap();
        assert_eq!(first, paired_sign_flip_test(&input, cfg).unwrap());
        assert!(!first.exact);
        assert_eq!(first.rng_seed, Some(17));
    }

    #[test]
    fn bootstrap_is_reproducible_and_records_its_contract() {
        let input = make_pairs(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let cfg = BootstrapConfig { replicates: 2_000, confidence_level: 0.95, rng_seed: 91 };
        let first = bootstrap_mean_interval(&input, cfg).unwrap();
        assert_eq!(first, bootstrap_mean_interval(&input, cfg).unwrap());
        assert_eq!(first.rng_seed, 91);
        assert!(first.lower <= 3.0 && first.upper >= 3.0);
    }

    #[test]
    fn equivalence_requires_interval_containment_inside_preregistered_margin() {
        let interval = BootstrapInterval {
            lower: -0.25, upper: 0.30, confidence_level: 0.90,
            replicates: 10_000, rng_seed: 9,
        };
        assert_eq!(
            screen_equivalence(interval, 0.5).unwrap().status,
            EquivalenceStatus::EquivalentWithinMargin
        );
        assert_eq!(
            screen_equivalence(interval, 0.2).unwrap().status,
            EquivalenceStatus::NotEstablished
        );
        assert!(screen_equivalence(interval, 0.0).is_err());
    }

    #[test]
    fn holm_adjusted_values_are_monotone_and_validation_is_strict() {
        let values = holm_adjust(&[("A", 0.01), ("B", 0.03), ("C", 0.02)], 0.05).unwrap();
        let by_label: BTreeMap<_, _> = values.iter()
            .map(|item| (item.label.as_str(), item.adjusted_p_value)).collect();
        assert!((by_label["A"] - 0.03).abs() < 1e-12);
        assert!((by_label["C"] - 0.04).abs() < 1e-12);
        assert!((by_label["B"] - 0.04).abs() < 1e-12);
        assert!(holm_adjust(&[("A", 0.01), ("A", 0.02)], 0.05).is_err());
        assert!(holm_adjust(&[("A", f64::NAN)], 0.05).is_err());
    }

    #[test]
    fn invalid_bootstrap_contract_is_rejected() {
        let input = make_pairs(&[1.0, 2.0]);
        assert!(bootstrap_mean_interval(
            &input,
            BootstrapConfig { replicates: 0, ..BootstrapConfig::default() }
        ).is_err());
        assert!(bootstrap_mean_interval(
            &input,
            BootstrapConfig { confidence_level: 1.0, ..BootstrapConfig::default() }
        ).is_err());
    }
}
