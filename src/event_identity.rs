// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed audit of model-owned semantic event identity claims.
//!
//! The event RNG guarantees that one complete EventAddress maps to one
//! deterministic random value. It cannot decide whether two model events are
//! scientifically the same cause. This module audits that declaration without
//! attempting to infer causal identity from values, names, or execution order.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::event_rng::EventAddress;

/// Stable, model-assigned identity for one semantic event cause.
///
/// Zero is reserved. IDs are explicit identity tokens, not hashes of event
/// names or runtime traversal positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticEventId(u128);

impl SemanticEventId {
    pub const fn new(value: u128) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    pub const fn raw(self) -> u128 {
        self.0
    }
}

/// Whether a semantic event is expected to align across counterfactual worlds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharingPolicy {
    /// The same exogenous cause must exist in both alternatives.
    MustShare,
    /// The cause exists only when a model condition is satisfied.
    Conditional,
    /// The cause is genuinely world-specific and must not be paired.
    Distinct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioSide {
    Reference,
    Counterfactual,
}

impl fmt::Display for ScenarioSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reference => formatter.write_str("reference"),
            Self::Counterfactual => formatter.write_str("counterfactual"),
        }
    }
}

/// Canonical sortable representation of every EventAddress identity field.
///
/// The fourth counter word is fixed to zero because block index identifies an
/// output block inside one event, not a different semantic event address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventAddressKey {
    pub replicate_seed: u64,
    pub experiment_namespace: u64,
    pub subject_id: u64,
    pub simulation_tick: u64,
    pub packed_event_code: u64,
}

impl EventAddressKey {
    pub const fn from_address(address: EventAddress) -> Self {
        let key = address.key_words();
        let counter = address.counter_words(0);
        Self {
            replicate_seed: key[0],
            experiment_namespace: key[1],
            subject_id: counter[0],
            simulation_tick: counter[1],
            packed_event_code: counter[2],
        }
    }
}

/// One model-owned assertion linking a semantic cause to an RNG address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventIdentityClaim {
    semantic_id: SemanticEventId,
    address: EventAddress,
    policy: SharingPolicy,
}

impl EventIdentityClaim {
    pub const fn new(
        semantic_id: SemanticEventId,
        address: EventAddress,
        policy: SharingPolicy,
    ) -> Self {
        Self {
            semantic_id,
            address,
            policy,
        }
    }

    pub const fn semantic_id(self) -> SemanticEventId {
        self.semantic_id
    }

    pub const fn address(self) -> EventAddress {
        self.address
    }

    pub const fn address_key(self) -> EventAddressKey {
        EventAddressKey::from_address(self.address)
    }

    pub const fn policy(self) -> SharingPolicy {
        self.policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityAuditError {
    DuplicateSemanticClaim {
        side: ScenarioSide,
        semantic_id: SemanticEventId,
    },
    SplitIdentityWithinScenario {
        side: ScenarioSide,
        semantic_id: SemanticEventId,
        first_address: EventAddressKey,
        second_address: EventAddressKey,
    },
    SemanticAliasWithinScenario {
        side: ScenarioSide,
        address: EventAddressKey,
        first_semantic_id: SemanticEventId,
        second_semantic_id: SemanticEventId,
    },
    SemanticAliasAcrossScenarios {
        address: EventAddressKey,
        reference_semantic_id: SemanticEventId,
        counterfactual_semantic_id: SemanticEventId,
    },
    SplitIdentityAcrossScenarios {
        semantic_id: SemanticEventId,
        reference_address: EventAddressKey,
        counterfactual_address: EventAddressKey,
    },
    PolicyMismatchAcrossScenarios {
        semantic_id: SemanticEventId,
        reference_policy: SharingPolicy,
        counterfactual_policy: SharingPolicy,
    },
    MustShareMissing {
        semantic_id: SemanticEventId,
        missing_from: ScenarioSide,
    },
    DistinctMatchedAcrossScenarios {
        semantic_id: SemanticEventId,
    },
}

impl fmt::Display for IdentityAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSemanticClaim { side, semantic_id } => write!(
                formatter,
                "{side} scenario contains duplicate semantic event claim {}",
                semantic_id.raw()
            ),
            Self::SplitIdentityWithinScenario {
                side,
                semantic_id,
                first_address,
                second_address,
            } => write!(
                formatter,
                "{side} scenario maps semantic event {} to both {:?} and {:?}",
                semantic_id.raw(),
                first_address,
                second_address
            ),
            Self::SemanticAliasWithinScenario {
                side,
                address,
                first_semantic_id,
                second_semantic_id,
            } => write!(
                formatter,
                "{side} scenario maps distinct semantic events {} and {} to address {:?}",
                first_semantic_id.raw(),
                second_semantic_id.raw(),
                address
            ),
            Self::SemanticAliasAcrossScenarios {
                address,
                reference_semantic_id,
                counterfactual_semantic_id,
            } => write!(
                formatter,
                "reference event {} and counterfactual event {} alias address {:?}",
                reference_semantic_id.raw(),
                counterfactual_semantic_id.raw(),
                address
            ),
            Self::SplitIdentityAcrossScenarios {
                semantic_id,
                reference_address,
                counterfactual_address,
            } => write!(
                formatter,
                "semantic event {} has different reference/counterfactual addresses {:?} and {:?}",
                semantic_id.raw(),
                reference_address,
                counterfactual_address
            ),
            Self::PolicyMismatchAcrossScenarios {
                semantic_id,
                reference_policy,
                counterfactual_policy,
            } => write!(
                formatter,
                "semantic event {} changes sharing policy from {:?} to {:?}",
                semantic_id.raw(),
                reference_policy,
                counterfactual_policy
            ),
            Self::MustShareMissing {
                semantic_id,
                missing_from,
            } => write!(
                formatter,
                "must-share semantic event {} is missing from the {missing_from} scenario",
                semantic_id.raw()
            ),
            Self::DistinctMatchedAcrossScenarios { semantic_id } => write!(
                formatter,
                "distinct semantic event {} was claimed in both scenarios",
                semantic_id.raw()
            ),
        }
    }
}

impl Error for IdentityAuditError {}

/// Counts returned by a successful semantic identity audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventIdentitySummary {
    pub reference_claims: usize,
    pub counterfactual_claims: usize,
    pub must_share_matched: usize,
    pub conditional_matched: usize,
    pub conditional_reference_only: usize,
    pub conditional_counterfactual_only: usize,
    pub distinct_reference_only: usize,
    pub distinct_counterfactual_only: usize,
}

#[derive(Debug)]
struct IndexedClaims {
    by_semantic: BTreeMap<SemanticEventId, EventIdentityClaim>,
    by_address: BTreeMap<EventAddressKey, SemanticEventId>,
}

fn index_claims(
    claims: &[EventIdentityClaim],
    side: ScenarioSide,
) -> Result<IndexedClaims, IdentityAuditError> {
    let mut by_semantic = BTreeMap::new();
    let mut by_address = BTreeMap::new();

    for claim in claims {
        let semantic_id = claim.semantic_id();
        let address = claim.address_key();

        if let Some(previous) = by_semantic.get(&semantic_id) {
            let previous: &EventIdentityClaim = previous;
            if previous.address_key() != address {
                return Err(IdentityAuditError::SplitIdentityWithinScenario {
                    side,
                    semantic_id,
                    first_address: previous.address_key(),
                    second_address: address,
                });
            }
            return Err(IdentityAuditError::DuplicateSemanticClaim { side, semantic_id });
        }

        if let Some(previous_semantic_id) = by_address.get(&address) {
            return Err(IdentityAuditError::SemanticAliasWithinScenario {
                side,
                address,
                first_semantic_id: *previous_semantic_id,
                second_semantic_id: semantic_id,
            });
        }

        by_semantic.insert(semantic_id, *claim);
        by_address.insert(address, semantic_id);
    }

    Ok(IndexedClaims {
        by_semantic,
        by_address,
    })
}

/// Audit model-owned event claims from two counterfactual scenario traces.
///
/// The function does not infer identity. It verifies explicit claims:
/// - one semantic ID maps to one address in each scenario;
/// - one address maps to one semantic ID in each scenario;
/// - an address cannot represent different semantic causes across scenarios;
/// - a must-share event must appear with the same identity and address on both sides;
/// - conditional and distinct unmatched events remain visible in the summary;
/// - a distinct event may not be silently paired across scenarios.
pub fn audit_event_identity_claims(
    reference: &[EventIdentityClaim],
    counterfactual: &[EventIdentityClaim],
) -> Result<EventIdentitySummary, IdentityAuditError> {
    let reference_index = index_claims(reference, ScenarioSide::Reference)?;
    let counterfactual_index = index_claims(counterfactual, ScenarioSide::Counterfactual)?;

    for (address, reference_semantic_id) in &reference_index.by_address {
        if let Some(counterfactual_semantic_id) = counterfactual_index.by_address.get(address) {
            if reference_semantic_id != counterfactual_semantic_id {
                return Err(IdentityAuditError::SemanticAliasAcrossScenarios {
                    address: *address,
                    reference_semantic_id: *reference_semantic_id,
                    counterfactual_semantic_id: *counterfactual_semantic_id,
                });
            }
        }
    }

    let mut summary = EventIdentitySummary {
        reference_claims: reference_index.by_semantic.len(),
        counterfactual_claims: counterfactual_index.by_semantic.len(),
        must_share_matched: 0,
        conditional_matched: 0,
        conditional_reference_only: 0,
        conditional_counterfactual_only: 0,
        distinct_reference_only: 0,
        distinct_counterfactual_only: 0,
    };

    for (semantic_id, reference_claim) in &reference_index.by_semantic {
        match counterfactual_index.by_semantic.get(semantic_id) {
            Some(counterfactual_claim) => {
                if reference_claim.policy() == SharingPolicy::Distinct
                    || counterfactual_claim.policy() == SharingPolicy::Distinct
                {
                    return Err(IdentityAuditError::DistinctMatchedAcrossScenarios {
                        semantic_id: *semantic_id,
                    });
                }

                if reference_claim.address_key() != counterfactual_claim.address_key() {
                    return Err(IdentityAuditError::SplitIdentityAcrossScenarios {
                        semantic_id: *semantic_id,
                        reference_address: reference_claim.address_key(),
                        counterfactual_address: counterfactual_claim.address_key(),
                    });
                }

                if reference_claim.policy() != counterfactual_claim.policy() {
                    return Err(IdentityAuditError::PolicyMismatchAcrossScenarios {
                        semantic_id: *semantic_id,
                        reference_policy: reference_claim.policy(),
                        counterfactual_policy: counterfactual_claim.policy(),
                    });
                }

                match reference_claim.policy() {
                    SharingPolicy::MustShare => summary.must_share_matched += 1,
                    SharingPolicy::Conditional => summary.conditional_matched += 1,
                    SharingPolicy::Distinct => unreachable!("distinct was rejected above"),
                }
            }
            None => match reference_claim.policy() {
                SharingPolicy::MustShare => {
                    return Err(IdentityAuditError::MustShareMissing {
                        semantic_id: *semantic_id,
                        missing_from: ScenarioSide::Counterfactual,
                    });
                }
                SharingPolicy::Conditional => summary.conditional_reference_only += 1,
                SharingPolicy::Distinct => summary.distinct_reference_only += 1,
            },
        }
    }

    for (semantic_id, counterfactual_claim) in &counterfactual_index.by_semantic {
        if reference_index.by_semantic.contains_key(semantic_id) {
            continue;
        }

        match counterfactual_claim.policy() {
            SharingPolicy::MustShare => {
                return Err(IdentityAuditError::MustShareMissing {
                    semantic_id: *semantic_id,
                    missing_from: ScenarioSide::Reference,
                });
            }
            SharingPolicy::Conditional => summary.conditional_counterfactual_only += 1,
            SharingPolicy::Distinct => summary.distinct_counterfactual_only += 1,
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_rng::{registry, EventAddress, SubjectId};

    const NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;

    fn semantic_id(value: u128) -> SemanticEventId {
        SemanticEventId::new(value).expect("test semantic event ID is nonzero")
    }

    fn address(subject: u32, tick: u64, namespace: u64) -> EventAddress {
        EventAddress::new(
            42,
            namespace,
            SubjectId::agent(subject).expect("test subject ID is nonzero"),
            tick,
            registry::ENVIRONMENT,
            registry::ENVIRONMENTAL_NOISE,
            0,
        )
    }

    fn claim(id: u128, event_address: EventAddress, policy: SharingPolicy) -> EventIdentityClaim {
        EventIdentityClaim::new(semantic_id(id), event_address, policy)
    }

    #[test]
    fn must_share_event_requires_same_semantic_identity_and_address() {
        let event_address = address(1, 10, NAMESPACE);
        let reference = [claim(1, event_address, SharingPolicy::MustShare)];
        let counterfactual = [claim(1, event_address, SharingPolicy::MustShare)];

        let summary = audit_event_identity_claims(&reference, &counterfactual).unwrap();
        assert_eq!(summary.reference_claims, 1);
        assert_eq!(summary.counterfactual_claims, 1);
        assert_eq!(summary.must_share_matched, 1);
        assert_eq!(summary.conditional_matched, 0);
    }

    #[test]
    fn missing_must_share_event_fails_closed() {
        let reference = [claim(
            7,
            address(1, 10, NAMESPACE),
            SharingPolicy::MustShare,
        )];

        assert_eq!(
            audit_event_identity_claims(&reference, &[]),
            Err(IdentityAuditError::MustShareMissing {
                semantic_id: semantic_id(7),
                missing_from: ScenarioSide::Counterfactual,
            })
        );
    }

    #[test]
    fn conditional_events_are_counted_not_fabricated_as_matches() {
        let shared = address(1, 10, NAMESPACE);
        let reference_only = address(2, 10, NAMESPACE);
        let counterfactual_only = address(3, 10, NAMESPACE);
        let reference = [
            claim(1, shared, SharingPolicy::Conditional),
            claim(2, reference_only, SharingPolicy::Conditional),
        ];
        let counterfactual = [
            claim(1, shared, SharingPolicy::Conditional),
            claim(3, counterfactual_only, SharingPolicy::Conditional),
        ];

        let summary = audit_event_identity_claims(&reference, &counterfactual).unwrap();
        assert_eq!(summary.conditional_matched, 1);
        assert_eq!(summary.conditional_reference_only, 1);
        assert_eq!(summary.conditional_counterfactual_only, 1);
    }

    #[test]
    fn two_semantic_causes_cannot_alias_one_address_within_a_scenario() {
        let event_address = address(1, 10, NAMESPACE);
        let claims = [
            claim(1, event_address, SharingPolicy::Conditional),
            claim(2, event_address, SharingPolicy::Conditional),
        ];

        assert!(matches!(
            audit_event_identity_claims(&claims, &[]),
            Err(IdentityAuditError::SemanticAliasWithinScenario {
                side: ScenarioSide::Reference,
                ..
            })
        ));
    }

    #[test]
    fn two_scenarios_cannot_assign_one_address_to_different_causes() {
        let event_address = address(1, 10, NAMESPACE);
        let reference = [claim(1, event_address, SharingPolicy::Conditional)];
        let counterfactual = [claim(2, event_address, SharingPolicy::Conditional)];

        assert!(matches!(
            audit_event_identity_claims(&reference, &counterfactual),
            Err(IdentityAuditError::SemanticAliasAcrossScenarios { .. })
        ));
    }

    #[test]
    fn one_semantic_cause_cannot_split_across_addresses() {
        let reference = [claim(
            5,
            address(1, 10, NAMESPACE),
            SharingPolicy::MustShare,
        )];
        let counterfactual = [claim(
            5,
            address(2, 10, NAMESPACE),
            SharingPolicy::MustShare,
        )];

        assert!(matches!(
            audit_event_identity_claims(&reference, &counterfactual),
            Err(IdentityAuditError::SplitIdentityAcrossScenarios { .. })
        ));
    }

    #[test]
    fn duplicate_claims_for_one_semantic_cause_are_rejected() {
        let event_address = address(1, 10, NAMESPACE);
        let claims = [
            claim(4, event_address, SharingPolicy::Conditional),
            claim(4, event_address, SharingPolicy::Conditional),
        ];

        assert!(matches!(
            audit_event_identity_claims(&claims, &[]),
            Err(IdentityAuditError::DuplicateSemanticClaim {
                side: ScenarioSide::Reference,
                ..
            })
        ));
    }

    #[test]
    fn same_semantic_cause_with_two_addresses_inside_one_scenario_is_rejected() {
        let claims = [
            claim(4, address(1, 10, NAMESPACE), SharingPolicy::Conditional),
            claim(4, address(2, 10, NAMESPACE), SharingPolicy::Conditional),
        ];

        assert!(matches!(
            audit_event_identity_claims(&claims, &[]),
            Err(IdentityAuditError::SplitIdentityWithinScenario { .. })
        ));
    }

    #[test]
    fn policy_drift_is_rejected() {
        let event_address = address(1, 10, NAMESPACE);
        let reference = [claim(8, event_address, SharingPolicy::Conditional)];
        let counterfactual = [claim(8, event_address, SharingPolicy::MustShare)];

        assert!(matches!(
            audit_event_identity_claims(&reference, &counterfactual),
            Err(IdentityAuditError::PolicyMismatchAcrossScenarios { .. })
        ));
    }

    #[test]
    fn distinct_events_are_never_paired() {
        let event_address = address(1, 10, NAMESPACE);
        let reference = [claim(9, event_address, SharingPolicy::Distinct)];
        let counterfactual = [claim(9, event_address, SharingPolicy::Distinct)];

        assert_eq!(
            audit_event_identity_claims(&reference, &counterfactual),
            Err(IdentityAuditError::DistinctMatchedAcrossScenarios {
                semantic_id: semantic_id(9),
            })
        );
    }

    #[test]
    fn distinct_and_conditional_unmatched_events_remain_visible() {
        let reference = [
            claim(11, address(1, 10, NAMESPACE), SharingPolicy::Distinct),
            claim(12, address(2, 10, NAMESPACE), SharingPolicy::Conditional),
        ];
        let counterfactual = [
            claim(13, address(3, 10, NAMESPACE), SharingPolicy::Distinct),
            claim(14, address(4, 10, NAMESPACE), SharingPolicy::Conditional),
        ];

        let summary = audit_event_identity_claims(&reference, &counterfactual).unwrap();
        assert_eq!(summary.distinct_reference_only, 1);
        assert_eq!(summary.distinct_counterfactual_only, 1);
        assert_eq!(summary.conditional_reference_only, 1);
        assert_eq!(summary.conditional_counterfactual_only, 1);
    }

    #[test]
    fn reversing_claim_order_does_not_change_the_audit_summary() {
        let shared = address(1, 10, NAMESPACE);
        let reference = [
            claim(1, shared, SharingPolicy::MustShare),
            claim(2, address(2, 10, NAMESPACE), SharingPolicy::Conditional),
            claim(3, address(3, 10, NAMESPACE), SharingPolicy::Distinct),
        ];
        let counterfactual = [
            claim(1, shared, SharingPolicy::MustShare),
            claim(4, address(4, 10, NAMESPACE), SharingPolicy::Conditional),
            claim(5, address(5, 10, NAMESPACE), SharingPolicy::Distinct),
        ];

        let forward = audit_event_identity_claims(&reference, &counterfactual).unwrap();
        let reverse_reference = reference.into_iter().rev().collect::<Vec<_>>();
        let reverse_counterfactual = counterfactual.into_iter().rev().collect::<Vec<_>>();
        let reverse =
            audit_event_identity_claims(&reverse_reference, &reverse_counterfactual).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn zero_semantic_event_id_is_reserved() {
        assert!(SemanticEventId::new(0).is_none());
        assert_eq!(SemanticEventId::new(1).unwrap().raw(), 1);
    }
}
