// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic experiment seed schedules with opt-in, disjoint range selection.
//!
//! The default schedule is deliberately frozen to the historical sequence:
//! seed[i] = 42 + i * 997. Environment overrides exist for benchmark
//! partitioning only; they do not change any published experiment by default.

use std::collections::BTreeSet;
use std::env;

pub const DEFAULT_SEED_BASE: u64 = 42;
pub const DEFAULT_SEED_STRIDE: u64 = 997;
pub const MAX_EXPERIMENT_SEEDS: usize = 10_000;

/// An ordered, validated mapping from logical seed indices to historical seed values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentSeedSchedule {
    indices: Vec<u64>,
    seeds: Vec<u64>,
    overridden: bool,
}

impl ExperimentSeedSchedule {
    /// Reads optional benchmark selectors, or returns the historical default schedule.
    ///
    /// Supported selectors:
    /// - SYMTROPY_SEED_INDICES=0,2,4,6 selects an explicit ordered index set.
    /// - SYMTROPY_SEED_START_INDEX=10 and SYMTROPY_SEED_COUNT=10 select a range.
    ///
    /// Explicit-list and range selectors are mutually exclusive. Invalid requests
    /// fail closed rather than silently reverting to the default schedule.
    pub fn from_env(default_count: usize) -> Result<Self, String> {
        let explicit = env::var("SYMTROPY_SEED_INDICES").ok();
        let start = env::var("SYMTROPY_SEED_START_INDEX").ok();
        let count = env::var("SYMTROPY_SEED_COUNT").ok();
        Self::from_config(default_count, explicit.as_deref(), start.as_deref(), count.as_deref())
    }

    fn from_config(
        default_count: usize,
        explicit: Option<&str>,
        start: Option<&str>,
        count: Option<&str>,
    ) -> Result<Self, String> {
        let overridden = explicit.is_some() || start.is_some() || count.is_some();
        if default_count == 0 || default_count > MAX_EXPERIMENT_SEEDS {
            return Err(format!(
                "default seed count must be in 1..={}, got {}",
                MAX_EXPERIMENT_SEEDS, default_count
            ));
        }

        let indices = if let Some(spec) = explicit {
            if start.is_some() || count.is_some() {
                return Err(
                    "SYMTROPY_SEED_INDICES cannot be combined with SYMTROPY_SEED_START_INDEX or SYMTROPY_SEED_COUNT".into(),
                );
            }
            if spec.trim().is_empty() {
                return Err("SYMTROPY_SEED_INDICES must not be empty".into());
            }

            let mut parsed = Vec::new();
            let mut seen = BTreeSet::new();
            for (position, item) in spec.split(',').enumerate() {
                let item = item.trim();
                if item.is_empty() {
                    return Err(format!(
                        "SYMTROPY_SEED_INDICES contains an empty item at position {}",
                        position
                    ));
                }
                let index = item.parse::<u64>().map_err(|_| {
                    format!("invalid seed index '{}' at position {}", item, position)
                })?;
                if !seen.insert(index) {
                    return Err(format!("duplicate seed index {}", index));
                }
                parsed.push(index);
                if parsed.len() > MAX_EXPERIMENT_SEEDS {
                    return Err(format!(
                        "seed index list exceeds the {} entry limit",
                        MAX_EXPERIMENT_SEEDS
                    ));
                }
            }
            parsed
        } else {
            let first = match start {
                Some(value) => value.parse::<u64>().map_err(|_| {
                    format!("invalid SYMTROPY_SEED_START_INDEX '{}'", value)
                })?,
                None => 0,
            };
            let number = match count {
                Some(value) => value.parse::<usize>().map_err(|_| {
                    format!("invalid SYMTROPY_SEED_COUNT '{}'", value)
                })?,
                None => default_count,
            };
            if number == 0 || number > MAX_EXPERIMENT_SEEDS {
                return Err(format!(
                    "SYMTROPY_SEED_COUNT must be in 1..={}, got {}",
                    MAX_EXPERIMENT_SEEDS, number
                ));
            }
            let number_u64 = u64::try_from(number)
                .map_err(|_| "SYMTROPY_SEED_COUNT does not fit in u64".to_string())?;
            let end = first.checked_add(number_u64)
                .ok_or_else(|| "seed index range overflows u64".to_string())?;
            (first..end).collect()
        };

        if indices.is_empty() {
            return Err("experiment seed schedule must contain at least one index".into());
        }

        let seeds = indices
            .iter()
            .map(|index| {
                index
                    .checked_mul(DEFAULT_SEED_STRIDE)
                    .and_then(|offset| DEFAULT_SEED_BASE.checked_add(offset))
                    .ok_or_else(|| format!("seed value overflows for index {}", index))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { indices, seeds, overridden })
    }

    /// Logical indices in execution order.
    pub fn indices(&self) -> &[u64] {
        &self.indices
    }

    /// Historical deterministic seed values in execution order.
    pub fn seeds(&self) -> &[u64] {
        &self.seeds
    }

    pub fn len(&self) -> usize {
        self.seeds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seeds.is_empty()
    }

    /// True when a non-default environment selector was supplied.
    pub fn is_overridden(&self) -> bool {
        self.overridden
    }

    /// Stable textual identity for run receipts and benchmark manifests.
    pub fn describe(&self) -> String {
        let indices = self.indices.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let seeds = self.seeds.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        format!(
            "indices=[{}];seeds=[{}];formula={}+index*{}",
            indices, seeds, DEFAULT_SEED_BASE, DEFAULT_SEED_STRIDE
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_schedule_preserves_historical_seed_values() {
        let schedule = ExperimentSeedSchedule::from_config(4, None, None, None).unwrap();
        assert_eq!(schedule.indices(), &[0, 1, 2, 3]);
        assert_eq!(schedule.seeds(), &[42, 1_039, 2_036, 3_033]);
        assert!(!schedule.is_overridden());
    }

    #[test]
    fn range_selector_produces_disjoint_holdout_ranges() {
        let train = ExperimentSeedSchedule::from_config(20, None, Some("0"), Some("10")).unwrap();
        let holdout = ExperimentSeedSchedule::from_config(20, None, Some("10"), Some("10")).unwrap();
        assert_eq!(train.indices(), &(0_u64..10).collect::<Vec<_>>());
        assert_eq!(holdout.indices(), &(10_u64..20).collect::<Vec<_>>());
        assert!(train.is_overridden());
        assert!(holdout.is_overridden());
        assert!(train.indices().iter().all(|index| !holdout.indices().contains(index)));
    }

    #[test]
    fn explicit_indices_preserve_order_and_seed_formula() {
        let schedule = ExperimentSeedSchedule::from_config(10, Some("7, 2, 9"), None, None).unwrap();
        assert_eq!(schedule.indices(), &[7, 2, 9]);
        assert_eq!(schedule.seeds(), &[6_021, 2_036, 8_973]);
    }

    #[test]
    fn duplicate_explicit_indices_are_rejected() {
        let result = ExperimentSeedSchedule::from_config(10, Some("1,2,1"), None, None);
        assert!(result.unwrap_err().contains("duplicate seed index 1"));
    }

    #[test]
    fn empty_or_malformed_explicit_lists_are_rejected() {
        assert!(ExperimentSeedSchedule::from_config(10, Some(""), None, None).is_err());
        assert!(ExperimentSeedSchedule::from_config(10, Some("1,,2"), None, None).is_err());
        assert!(ExperimentSeedSchedule::from_config(10, Some("one,2"), None, None).is_err());
    }

    #[test]
    fn conflicting_selectors_are_rejected() {
        let result = ExperimentSeedSchedule::from_config(10, Some("1,2"), Some("0"), None);
        assert!(result.unwrap_err().contains("cannot be combined"));
    }

    #[test]
    fn invalid_counts_and_overflow_are_rejected() {
        assert!(ExperimentSeedSchedule::from_config(10, None, None, Some("0")).is_err());
        assert!(ExperimentSeedSchedule::from_config(10, None, Some("nope"), Some("2")).is_err());
        assert!(ExperimentSeedSchedule::from_config(10, None, Some("18446744073709551615"), Some("2")).is_err());
        assert!(ExperimentSeedSchedule::from_config(10, None, Some("18446744073709551615"), None).is_err());
    }

    #[test]
    fn oversized_schedule_is_rejected_before_range_allocation() {
        assert!(ExperimentSeedSchedule::from_config(10, None, Some("0"), Some("10001")).is_err());
    }
}
