// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Persistent energy reservoir for consciousness-coupled entities.
//!
//! Energy is a finite, conserved quantity. It depletes through movement,
//! consciousness maintenance, and collisions. It regenerates through
//! harmony resonance, energy wells, and slow ambient recovery.
//!
//! When energy reaches zero, the entity collapses — Φ drops to 0,
//! motor output halts, and recovery requires another agent's help.

/// Persistent energy reservoir for a consciousness-coupled entity.
///
/// Tracks internal energy (U), temperature (T), and entropy (S)
/// for 2nd Law compliance. Available work = Helmholtz free energy F = U - TS.
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Maximum energy capacity.
    pub max_energy: f64,
    /// Current internal energy U (persistent — does NOT reset each tick).
    pub available: f64,
    /// Entity temperature in Kelvin (rises with dissipated heat).
    pub temperature: f64,
    /// Cumulative entropy in J/K (monotonically increases per 2nd Law).
    pub entropy: f64,
    /// Heat capacity in J/K.
    pub heat_capacity: f64,
    /// Total energy consumed this tick (reset each tick for telemetry).
    pub consumed_this_tick: f64,
    /// Total energy regenerated this tick (reset each tick for telemetry).
    pub regenerated_this_tick: f64,
    /// Cumulative energy spent across all ticks.
    pub lifetime_consumed: f64,
    /// Whether this entity has collapsed (energy depleted to zero).
    pub collapsed: bool,
}

impl EnergyBudget {
    /// Create a new energy budget with starting energy.
    ///
    /// A non-finite or non-positive capacity is treated as an empty,
    /// collapsed budget. This keeps invalid external configuration from
    /// creating a reservoir with an impossible or undefined energy state.
    pub fn new(max_energy: f64) -> Self {
        let max_energy = if max_energy.is_finite() && max_energy > 0.0 {
            max_energy
        } else {
            0.0
        };

        Self {
            max_energy,
            available: max_energy,
            temperature: 310.0,
            entropy: 0.0,
            heat_capacity: 100.0,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
            lifetime_consumed: 0.0,
            collapsed: max_energy == 0.0,
        }
    }

    /// Reset per-tick counters (call at start of each tick).
    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    /// Try to consume energy. Returns the amount actually consumed
    /// (may be less than requested if budget is exhausted).
    ///
    /// Non-positive and non-finite requests are rejected without mutation.
    #[inline]
    pub fn consume(&mut self, amount: f64) -> f64 {
        if self.collapsed || !amount.is_finite() || amount <= 0.0 {
            return 0.0;
        }
        if !self.max_energy.is_finite() || self.max_energy <= 0.0 {
            self.available = 0.0;
            self.collapsed = true;
            return 0.0;
        }
        if !self.available.is_finite() || self.available <= 0.0 {
            self.available = 0.0;
            self.collapsed = true;
            return 0.0;
        }

        // Compute the entire state transition before mutating anything. This
        // prevents arithmetic overflow or a corrupted public counter from
        // partially applying an energy operation.
        if !self.temperature.is_finite()
            || self.temperature <= 0.0
            || !self.entropy.is_finite()
            || !self.consumed_this_tick.is_finite()
            || !self.lifetime_consumed.is_finite()
        {
            return 0.0;
        }

        let current_available = self.available.min(self.max_energy);
        let actual = amount.min(current_available);
        let next_available = current_available - actual;
        let next_consumed = self.consumed_this_tick + actual;
        let next_lifetime_consumed = self.lifetime_consumed + actual;
        let next_entropy = self.entropy + actual / self.temperature;
        if !next_available.is_finite()
            || !next_consumed.is_finite()
            || !next_lifetime_consumed.is_finite()
            || !next_entropy.is_finite()
        {
            return 0.0;
        }

        self.available = next_available;
        self.consumed_this_tick = next_consumed;
        self.lifetime_consumed = next_lifetime_consumed;
        self.entropy = next_entropy;

        // Check for collapse
        if self.available <= 0.0 {
            self.available = 0.0;
            self.collapsed = true;
        }
        actual
    }

    /// Absorb heat from dissipation (damping, friction, collision).
    /// Increases temperature and entropy per 2nd Law.
    ///
    /// Invalid heat inputs or thermal parameters are rejected without mutation.
    pub fn dissipate_heat(&mut self, energy: f64) {
        if !energy.is_finite()
            || energy <= 0.0
            || !self.heat_capacity.is_finite()
            || self.heat_capacity <= 0.0
            || !self.temperature.is_finite()
            || self.temperature <= 0.0
        {
            return;
        }
        let next_temperature = self.temperature + energy / self.heat_capacity;
        if !next_temperature.is_finite() {
            return;
        }
        let next_entropy = self.entropy + energy / next_temperature;
        if !next_entropy.is_finite() {
            return;
        }

        self.temperature = next_temperature;
        self.entropy = next_entropy;
    }

    /// Helmholtz free energy: maximum extractable work at constant T.
    /// F = U - TS. Always ≥ 0.
    pub fn available_work(&self) -> f64 {
        (self.available - self.temperature * self.entropy).max(0.0)
    }

    /// Add energy (from regeneration sources). Capped at max_energy.
    ///
    /// Only finite, positive requests are admitted. Telemetry records the
    /// amount actually added, not the requested amount beyond remaining capacity.
    #[inline]
    pub fn regenerate(&mut self, amount: f64) {
        if !amount.is_finite()
            || amount <= 0.0
            || !self.max_energy.is_finite()
            || self.max_energy <= 0.0
        {
            return;
        }

        if !self.regenerated_this_tick.is_finite() {
            return;
        }

        // Normalize externally modified state in a local value first. The
        // operation is committed only after every resulting value is finite.
        let current_available = if self.available.is_finite() {
            self.available.clamp(0.0, self.max_energy)
        } else {
            0.0
        };
        let actual = amount.min(self.max_energy - current_available);
        let next_available = current_available + actual;
        let next_regenerated = self.regenerated_this_tick + actual;
        if !next_available.is_finite() || !next_regenerated.is_finite() {
            return;
        }

        self.available = next_available;
        self.regenerated_this_tick = next_regenerated;
        self.collapsed = self.available == 0.0;
    }

    /// Whether any energy is available.
    #[inline]
    pub fn has_energy(&self) -> bool {
        self.available > 1e-10 && !self.collapsed
    }

    /// Whether this entity has collapsed (zero energy).
    #[inline]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Fraction of energy remaining [0.0, 1.0].
    #[inline]
    pub fn fraction_remaining(&self) -> f64 {
        if !self.max_energy.is_finite()
            || self.max_energy < 1e-10
            || !self.available.is_finite()
        {
            return 0.0;
        }
        (self.available / self.max_energy).clamp(0.0, 1.0)
    }

    /// Net energy flow this tick (positive = regenerating, negative = depleting).
    pub fn net_flow_this_tick(&self) -> f64 {
        self.regenerated_this_tick - self.consumed_this_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn new_budget_has_full_energy() {
        let budget = EnergyBudget::new(100.0);
        assert!((budget.available - 100.0).abs() < 1e-10);
        assert!(!budget.collapsed);
    }

    #[test]
    fn invalid_capacity_creates_empty_collapsed_budget() {
        for capacity in [-1.0, 0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let budget = EnergyBudget::new(capacity);
            assert_eq!(budget.max_energy, 0.0);
            assert_eq!(budget.available, 0.0);
            assert!(budget.is_collapsed());
            assert!(!budget.has_energy());
        }
    }

    #[test]
    fn consume_reduces_available() {
        let mut budget = EnergyBudget::new(100.0);
        let consumed = budget.consume(30.0);
        assert!((consumed - 30.0).abs() < 1e-10);
        assert!((budget.available - 70.0).abs() < 1e-10);
    }

    #[test]
    fn consume_capped_by_available() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(80.0);
        let consumed = budget.consume(50.0); // only 20 left
        assert!((consumed - 20.0).abs() < 1e-10);
        assert!(budget.is_collapsed());
    }

    #[test]
    fn consume_rejects_negative_and_non_finite_requests() {
        for request in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            assert_eq!(budget.consume(request), 0.0);
            assert_eq!(budget.available, 100.0);
            assert_eq!(budget.consumed_this_tick, 0.0);
            assert_eq!(budget.lifetime_consumed, 0.0);
            assert_eq!(budget.entropy, 0.0);
            assert!(!budget.is_collapsed());
        }
    }

    #[test]
    fn consume_rejects_non_finite_ledger_transition_without_partial_mutation() {
        let mut budget = EnergyBudget::new(f64::MAX);
        budget.consumed_this_tick = f64::MAX;
        let before_available = budget.available;
        assert_eq!(budget.consume(f64::MAX), 0.0);
        assert_eq!(budget.available, before_available);
        assert_eq!(budget.consumed_this_tick, f64::MAX);
        assert_eq!(budget.lifetime_consumed, 0.0);
        assert_eq!(budget.entropy, 0.0);
    }

    #[test]
    fn collapse_on_zero_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());
        assert!(!budget.has_energy());

        // Collapsed entity cannot consume
        let consumed = budget.consume(10.0);
        assert!((consumed - 0.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_adds_energy() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(50.0);
        budget.regenerate(20.0);
        assert!((budget.available - 70.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_capped_at_max() {
        let mut budget = EnergyBudget::new(100.0);
        budget.regenerate(50.0); // already at max
        assert!((budget.available - 100.0).abs() < 1e-10);
    }

    #[test]
    fn regenerate_accounts_only_for_energy_actually_added() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(90.0);
        budget.regenerate(50.0);
        assert_eq!(budget.available, 100.0);
        assert_eq!(budget.regenerated_this_tick, 10.0);
    }

    #[test]
    fn regenerate_rejects_negative_and_non_finite_requests() {
        for request in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            budget.consume(10.0);
            let before = budget.available;
            budget.regenerate(request);
            assert_eq!(budget.available, before);
            assert_eq!(budget.regenerated_this_tick, 0.0);
            assert!(!budget.is_collapsed());
        }
    }

    #[test]
    fn regenerate_rejects_non_finite_ledger_transition_without_partial_mutation() {
        let mut budget = EnergyBudget::new(f64::MAX);
        budget.consume(f64::MAX);
        budget.regenerated_this_tick = f64::MAX;
        let before_available = budget.available;
        budget.regenerate(f64::MAX);
        assert_eq!(budget.available, before_available);
        assert_eq!(budget.regenerated_this_tick, f64::MAX);
        assert!(budget.is_collapsed());
    }

    #[test]
    fn regenerate_recovers_from_collapse() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(100.0);
        assert!(budget.is_collapsed());

        budget.regenerate(10.0);
        assert!(!budget.is_collapsed());
        assert!(budget.has_energy());
    }

    #[test]
    fn regenerate_normalizes_invalid_public_state_before_recovery() {
        let mut budget = EnergyBudget::new(100.0);
        budget.available = f64::NAN;
        budget.collapsed = false;
        budget.regenerate(10.0);
        assert_eq!(budget.available, 10.0);
        assert!(!budget.is_collapsed());
        assert_eq!(budget.regenerated_this_tick, 10.0);
    }

    #[test]
    fn dissipate_heat_rejects_negative_and_non_finite_inputs() {
        for amount in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut budget = EnergyBudget::new(100.0);
            budget.dissipate_heat(amount);
            assert_eq!(budget.temperature, 310.0);
            assert_eq!(budget.entropy, 0.0);
        }
    }

    #[test]
    fn dissipate_heat_rejects_invalid_thermal_parameters() {
        let mut budget = EnergyBudget::new(100.0);
        budget.heat_capacity = f64::NAN;
        budget.dissipate_heat(10.0);
        assert_eq!(budget.temperature, 310.0);
        assert_eq!(budget.entropy, 0.0);

        budget.heat_capacity = 100.0;
        budget.temperature = f64::INFINITY;
        budget.dissipate_heat(10.0);
        assert_eq!(budget.temperature, f64::INFINITY);
        assert_eq!(budget.entropy, 0.0);
    }

    #[test]
    fn dissipate_heat_rejects_overflowing_thermal_transition() {
        let mut budget = EnergyBudget::new(100.0);
        budget.heat_capacity = f64::MIN_POSITIVE;
        budget.dissipate_heat(1.0e308);
        assert_eq!(budget.temperature, 310.0);
        assert_eq!(budget.entropy, 0.0);
    }

    #[test]
    fn lifetime_accumulates() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(30.0);
        budget.tick_reset();
        budget.consume(20.0);
        assert!((budget.lifetime_consumed - 50.0).abs() < 1e-10);
    }

    #[test]
    fn net_flow_tracks_correctly() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        budget.regenerate(15.0);
        assert!((budget.net_flow_this_tick() - 5.0).abs() < 1e-10); // +5 net
    }

    #[test]
    fn tick_reset_clears_per_tick() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(10.0);
        budget.regenerate(5.0);
        budget.tick_reset();
        assert!((budget.consumed_this_tick - 0.0).abs() < 1e-10);
        assert!((budget.regenerated_this_tick - 0.0).abs() < 1e-10);
        // But available is unchanged
        assert!((budget.available - 95.0).abs() < 1e-10);
    }

    #[test]
    fn fraction_remaining() {
        let mut budget = EnergyBudget::new(100.0);
        budget.consume(25.0);
        assert!((budget.fraction_remaining() - 0.75).abs() < 1e-10);
    }

    #[test]
    fn fraction_remaining_clamps_public_state_to_valid_range() {
        let mut budget = EnergyBudget::new(100.0);
        budget.available = 150.0;
        assert_eq!(budget.fraction_remaining(), 1.0);

        budget.available = f64::NAN;
        assert_eq!(budget.fraction_remaining(), 0.0);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(1_000))]

        #[test]
        fn generated_operation_sequences_preserve_energy_reference_model(
            operations in proptest::collection::vec(
                (0u8..4, proptest::prelude::any::<f64>()),
                100..101
            )
        ) {
            let max_energy = 1_000.0_f64;
            let heat_capacity = 100.0_f64;
            let mut budget = EnergyBudget::new(max_energy);

            // Independent scalar reference model. It models the public
            // transition contract rather than calling EnergyBudget methods.
            let mut expected_available = max_energy;
            let mut expected_temperature = 310.0_f64;
            let mut expected_entropy = 0.0_f64;
            let mut expected_consumed_tick = 0.0_f64;
            let mut expected_regenerated_tick = 0.0_f64;
            let mut expected_lifetime_consumed = 0.0_f64;
            let mut expected_collapsed = false;

            for (operation, value) in operations {
                match operation {
                    0 => {
                        if value.is_finite()
                            && value > 0.0
                            && !expected_collapsed
                            && expected_available > 0.0
                            && expected_temperature.is_finite()
                            && expected_temperature > 0.0
                            && expected_entropy.is_finite()
                            && expected_consumed_tick.is_finite()
                            && expected_lifetime_consumed.is_finite()
                        {
                            let current = expected_available.min(max_energy);
                            let actual = value.min(current);
                            let next_available = current - actual;
                            let next_consumed = expected_consumed_tick + actual;
                            let next_lifetime = expected_lifetime_consumed + actual;
                            let next_entropy = expected_entropy + actual / expected_temperature;

                            if next_available.is_finite()
                                && next_consumed.is_finite()
                                && next_lifetime.is_finite()
                                && next_entropy.is_finite()
                            {
                                expected_available = next_available;
                                expected_consumed_tick = next_consumed;
                                expected_lifetime_consumed = next_lifetime;
                                expected_entropy = next_entropy;
                                if expected_available <= 0.0 {
                                    expected_available = 0.0;
                                    expected_collapsed = true;
                                }
                            }
                        }
                        let _ = budget.consume(value);
                    }
                    1 => {
                        if value.is_finite()
                            && value > 0.0
                            && expected_regenerated_tick.is_finite()
                        {
                            let current = expected_available.clamp(0.0, max_energy);
                            let actual = value.min(max_energy - current);
                            let next_available = current + actual;
                            let next_regenerated = expected_regenerated_tick + actual;
                            if next_available.is_finite() && next_regenerated.is_finite() {
                                expected_available = next_available;
                                expected_regenerated_tick = next_regenerated;
                                expected_collapsed = expected_available == 0.0;
                            }
                        }
                        budget.regenerate(value);
                    }
                    2 => {
                        if value.is_finite()
                            && value > 0.0
                            && expected_temperature.is_finite()
                            && expected_temperature > 0.0
                        {
                            let next_temperature = expected_temperature + value / heat_capacity;
                            if next_temperature.is_finite() {
                                let next_entropy = expected_entropy + value / next_temperature;
                                if next_entropy.is_finite() {
                                    expected_temperature = next_temperature;
                                    expected_entropy = next_entropy;
                                }
                            }
                        }
                        budget.dissipate_heat(value);
                    }
                    _ => {
                        expected_consumed_tick = 0.0;
                        expected_regenerated_tick = 0.0;
                        budget.tick_reset();
                    }
                }

                prop_assert!(budget.max_energy.is_finite());
                prop_assert!(budget.available.is_finite());
                prop_assert!(budget.available >= 0.0);
                prop_assert!(budget.available <= budget.max_energy);
                prop_assert!(budget.temperature.is_finite());
                prop_assert!(budget.temperature > 0.0);
                prop_assert!(budget.entropy.is_finite());
                prop_assert!(budget.consumed_this_tick.is_finite());
                prop_assert!(budget.consumed_this_tick >= 0.0);
                prop_assert!(budget.regenerated_this_tick.is_finite());
                prop_assert!(budget.regenerated_this_tick >= 0.0);
                prop_assert!(budget.lifetime_consumed.is_finite());
                prop_assert!(budget.lifetime_consumed >= 0.0);
                prop_assert_eq!(budget.available, expected_available);
                prop_assert_eq!(budget.temperature, expected_temperature);
                prop_assert_eq!(budget.entropy, expected_entropy);
                prop_assert_eq!(budget.consumed_this_tick, expected_consumed_tick);
                prop_assert_eq!(budget.regenerated_this_tick, expected_regenerated_tick);
                prop_assert_eq!(budget.lifetime_consumed, expected_lifetime_consumed);
                prop_assert_eq!(budget.is_collapsed(), expected_collapsed);
            }
        }
    }
}
