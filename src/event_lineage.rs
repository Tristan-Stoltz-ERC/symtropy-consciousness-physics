// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic dynamic-entity identity from semantic birth provenance.
//!
//! This module avoids allocator-order IDs for counterfactual entities. It
//! derives a candidate ID from a canonical BirthKey and maintains an explicit
//! collision registry. The derivation is deterministic, not cryptographic;
//! collision detection and a shared registry are required for safe use.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use crate::event_identity::SemanticEventId;
use crate::event_rng::{philox4x64_10, SubjectId};

const DYNAMIC_ID_MARKER: u64 = 1_u64 << 63;
const LINEAGE_KEY_DOMAIN: u64 = 0x4c49_4e45_4147_4531;
const LINEAGE_COUNTER_DOMAIN: u64 = 0x4249_5254_484b_4559;

/// Complete, canonical provenance for one dynamic entity's birth.
///
/// Parent order is canonicalized during construction. The offspring slot is a
/// model-defined semantic slot (for example, "left product" or "offspring 0"),
/// never a position in a runtime vector or an allocator counter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BirthKey {
    world_namespace: u64,
    parent_subject_ids: Vec<u64>,
    birth_event_id: SemanticEventId,
    offspring_slot: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BirthKeyError {
    ZeroWorldNamespace,
    DuplicateParentSubject { subject_id: u64 },
}

impl fmt::Display for BirthKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWorldNamespace => {
                formatter.write_str("world namespace zero is reserved")
            }
            Self::DuplicateParentSubject { subject_id } => write!(
                formatter,
                "birth key contains duplicate parent subject ID {subject_id}"
            ),
        }
    }
}

impl Error for BirthKeyError {}

impl BirthKey {
    pub fn new(
        world_namespace: u64,
        parent_subjects: impl IntoIterator<Item = SubjectId>,
        birth_event_id: SemanticEventId,
        offspring_slot: u32,
    ) -> Result<Self, BirthKeyError> {
        if world_namespace == 0 {
            return Err(BirthKeyError::ZeroWorldNamespace);
        }

        let mut parent_subject_ids = parent_subjects
            .into_iter()
            .map(SubjectId::raw)
            .collect::<Vec<_>>();
        parent_subject_ids.sort_unstable();

        for pair in parent_subject_ids.windows(2) {
            if pair[0] == pair[1] {
                return Err(BirthKeyError::DuplicateParentSubject {
                    subject_id: pair[0],
                });
            }
        }

        Ok(Self {
            world_namespace,
            parent_subject_ids,
            birth_event_id,
            offspring_slot,
        })
    }

    pub const fn world_namespace(&self) -> u64 {
        self.world_namespace
    }

    pub fn parent_subject_ids(&self) -> &[u64] {
        &self.parent_subject_ids
    }

    pub const fn birth_event_id(&self) -> SemanticEventId {
        self.birth_event_id
    }

    pub const fn offspring_slot(&self) -> u32 {
        self.offspring_slot
    }

    /// Derive a deterministic candidate value. This is a non-cryptographic
    /// identity mixer; the registry must detect collisions before use.
    fn candidate_id(&self) -> u64 {
        let event = self.birth_event_id.raw();
        let key = [
            self.world_namespace,
            ((event >> 64) as u64) ^ LINEAGE_KEY_DOMAIN,
        ];
        let mut state = philox4x64_10(
            [
                event as u64,
                self.offspring_slot as u64,
                self.parent_subject_ids.len() as u64,
                LINEAGE_COUNTER_DOMAIN,
            ],
            key,
        );

        // Fold sorted parents in order. The previous state is mixed into every
        // next counter, so both parent-set membership and order are represented.
        for parent_id in &self.parent_subject_ids {
            state = philox4x64_10(
                [*parent_id, state[0], state[1], state[2]],
                key,
            );
        }

        // Set the high marker bit to distinguish this derivation family in
        // diagnostics. This is not a disjoint namespace: static pair IDs may
        // also occupy that range, so the registry still checks collisions.
        state[0] | DYNAMIC_ID_MARKER
    }
}

/// One complete lineage receipt entry, suitable for deterministic evidence export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageAssignment {
    pub birth_key: BirthKey,
    pub subject_id: SubjectId,
}

/// Fail-closed registry for static and dynamically derived subject identities.
///
/// Share one registry across the counterfactual alternatives being compared.
/// Repeated resolution of the same BirthKey is idempotent. Distinct BirthKeys
/// that collide, or collide with a previously reserved static subject, fail.
#[derive(Debug, Default)]
pub struct LineageIdRegistry {
    reserved_static_subjects: HashSet<u64>,
    subjects_by_birth_key: HashMap<BirthKey, SubjectId>,
    birth_keys_by_subject: HashMap<u64, BirthKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageIdentityError {
    StaticSubjectConflictsWithDerived { subject_id: u64 },
    DerivedSubjectConflictsWithStatic { subject_id: u64 },
    DerivedSubjectCollision {
        subject_id: u64,
        first_birth: BirthKey,
        second_birth: BirthKey,
    },
}

impl fmt::Display for LineageIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticSubjectConflictsWithDerived { subject_id } => write!(
                formatter,
                "static subject ID {subject_id} was registered after a derived entity used it"
            ),
            Self::DerivedSubjectConflictsWithStatic { subject_id } => write!(
                formatter,
                "derived subject ID {subject_id} collides with a reserved static subject"
            ),
            Self::DerivedSubjectCollision {
                subject_id,
                first_birth,
                second_birth,
            } => write!(
                formatter,
                "distinct birth keys {:?} and {:?} collide on subject ID {subject_id}",
                first_birth,
                second_birth
            ),
        }
    }
}

impl Error for LineageIdentityError {}

impl LineageIdRegistry {
    /// Reserve an already-assigned static/global/pair subject ID.
    ///
    /// Call this for every static identity that can occur in the model before
    /// resolving dynamic births. Re-registering the same static ID is harmless.
    pub fn reserve_static_subject(
        &mut self,
        subject: SubjectId,
    ) -> Result<(), LineageIdentityError> {
        let subject_id = subject.raw();
        if self.birth_keys_by_subject.contains_key(&subject_id) {
            return Err(LineageIdentityError::StaticSubjectConflictsWithDerived {
                subject_id,
            });
        }
        self.reserved_static_subjects.insert(subject_id);
        Ok(())
    }

    /// Resolve a BirthKey to its stable subject ID, rejecting collisions.
    pub fn resolve(
        &mut self,
        birth: BirthKey,
    ) -> Result<SubjectId, LineageIdentityError> {
        if let Some(existing) = self.subjects_by_birth_key.get(&birth) {
            return Ok(*existing);
        }

        let candidate = birth.candidate_id();
        self.insert_candidate(birth, candidate)
    }

    /// Number of distinct dynamic births registered so far.
    pub fn dynamic_entity_count(&self) -> usize {
        self.subjects_by_birth_key.len()
    }

    /// Return complete assignments in stable SubjectId order for evidence export.
    ///
    /// HashMap iteration order is intentionally not exposed as a receipt order.
    pub fn assignments(&self) -> Vec<LineageAssignment> {
        let mut assignments = self
            .subjects_by_birth_key
            .iter()
            .map(|(birth_key, subject_id)| LineageAssignment {
                birth_key: birth_key.clone(),
                subject_id: *subject_id,
            })
            .collect::<Vec<_>>();
        assignments.sort_unstable_by_key(|assignment| assignment.subject_id.raw());
        assignments
    }

    /// Number of static subject IDs reserved so far.
    pub fn reserved_static_count(&self) -> usize {
        self.reserved_static_subjects.len()
    }

    fn insert_candidate(
        &mut self,
        birth: BirthKey,
        candidate: u64,
    ) -> Result<SubjectId, LineageIdentityError> {
        // Enforce the dynamic marker even in the collision-injection test path.
        let subject_id = candidate | DYNAMIC_ID_MARKER;

        if self.reserved_static_subjects.contains(&subject_id) {
            return Err(LineageIdentityError::DerivedSubjectConflictsWithStatic {
                subject_id,
            });
        }

        if let Some(first_birth) = self.birth_keys_by_subject.get(&subject_id) {
            if first_birth != &birth {
                return Err(LineageIdentityError::DerivedSubjectCollision {
                    subject_id,
                    first_birth: first_birth.clone(),
                    second_birth: birth,
                });
            }
            // This branch is defensive: same key should already have been
            // returned by resolve() through subjects_by_birth_key.
            return Ok(SubjectId::from_nonzero_raw(subject_id)
                .expect("dynamic marker guarantees nonzero ID"));
        }

        let subject = SubjectId::from_nonzero_raw(subject_id)
            .expect("dynamic marker guarantees nonzero ID");
        self.birth_keys_by_subject
            .insert(subject_id, birth.clone());
        self.subjects_by_birth_key.insert(birth, subject);
        Ok(subject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_identity::SemanticEventId;

    fn semantic_event(value: u128) -> SemanticEventId {
        SemanticEventId::new(value).expect("semantic event ID is nonzero")
    }

    fn parent(value: u32) -> SubjectId {
        SubjectId::agent(value).expect("parent ID is nonzero")
    }

    fn birth(
        namespace: u64,
        parents: &[u32],
        event: u128,
        slot: u32,
    ) -> BirthKey {
        BirthKey::new(
            namespace,
            parents.iter().copied().map(parent),
            semantic_event(event),
            slot,
        )
        .unwrap()
    }

    #[test]
    fn parent_order_is_canonical() {
        let first = birth(7, &[11, 3, 19], 0xabc, 2);
        let second = birth(7, &[19, 11, 3], 0xabc, 2);
        assert_eq!(first, second);
        assert_eq!(first.parent_subject_ids(), &[3, 11, 19]);
    }

    #[test]
    fn same_birth_key_derives_same_subject_across_registries() {
        let key = birth(7, &[11, 3], 0xabc, 2);
        let mut first_registry = LineageIdRegistry::default();
        let mut second_registry = LineageIdRegistry::default();
        let first = first_registry.resolve(key.clone()).unwrap();
        let second = second_registry.resolve(key).unwrap();
        assert_eq!(first, second);
        assert_ne!(first.raw(), 0);
    }

    #[test]
    fn repeated_resolution_is_idempotent() {
        let key = birth(7, &[11, 3], 0xabc, 2);
        let mut registry = LineageIdRegistry::default();
        let first = registry.resolve(key.clone()).unwrap();
        let second = registry.resolve(key).unwrap();
        assert_eq!(first, second);
        assert_eq!(registry.dynamic_entity_count(), 1);
    }

    #[test]
    fn unrelated_birth_registration_does_not_shift_existing_identity() {
        let target = birth(7, &[11, 3], 0xabc, 2);
        let unrelated = birth(7, &[99], 0xdef, 5);

        let mut first_registry = LineageIdRegistry::default();
        let target_first = first_registry.resolve(target.clone()).unwrap();

        let mut reordered_registry = LineageIdRegistry::default();
        let _unrelated_first = reordered_registry.resolve(unrelated).unwrap();
        let target_second = reordered_registry.resolve(target).unwrap();

        assert_eq!(target_first, target_second);
    }

    #[test]
    fn world_namespace_birth_event_and_offspring_slot_change_identity_fixture() {
        let base = birth(7, &[11, 3], 0xabc, 2);
        let changed_namespace = birth(8, &[11, 3], 0xabc, 2);
        let changed_event = birth(7, &[11, 3], 0xabd, 2);
        let changed_slot = birth(7, &[11, 3], 0xabc, 3);
        let changed_parent = birth(7, &[11, 4], 0xabc, 2);
        let mut registry = LineageIdRegistry::default();
        let base_id = registry.resolve(base).unwrap();
        for other in [
            changed_namespace,
            changed_event,
            changed_slot,
            changed_parent,
        ] {
            assert_ne!(registry.resolve(other).unwrap(), base_id);
        }
        assert_eq!(registry.dynamic_entity_count(), 5);
    }

    #[test]
    fn assignment_receipts_are_complete_and_sorted_independent_of_insertion_order() {
        let first = birth(7, &[11, 3], 0xabc, 2);
        let second = birth(7, &[99], 0xdef, 5);

        let mut left = LineageIdRegistry::default();
        left.resolve(first.clone()).unwrap();
        left.resolve(second.clone()).unwrap();

        let mut right = LineageIdRegistry::default();
        right.resolve(second).unwrap();
        right.resolve(first).unwrap();

        let left_receipts = left.assignments();
        let right_receipts = right.assignments();
        assert_eq!(left_receipts, right_receipts);
        assert_eq!(left_receipts.len(), 2);
        assert!(left_receipts
            .windows(2)
            .all(|pair| pair[0].subject_id.raw() < pair[1].subject_id.raw()));
        assert!(left_receipts.iter().all(|receipt| {
            receipt.subject_id.raw() != 0 && receipt.birth_key.world_namespace() == 7
        }));
    }

    #[test]
    fn static_subject_collision_is_rejected() {
        let key = birth(7, &[11, 3], 0xabc, 2);
        let candidate = key.candidate_id();
        let mut registry = LineageIdRegistry::default();
        registry
            .reserve_static_subject(SubjectId::from_nonzero_raw(candidate).unwrap())
            .unwrap();

        assert_eq!(
            registry.resolve(key),
            Err(LineageIdentityError::DerivedSubjectConflictsWithStatic {
                subject_id: candidate
            })
        );
    }

    #[test]
    fn dynamic_birth_collision_is_rejected() {
        let first = birth(7, &[11, 3], 0xabc, 2);
        let second = birth(8, &[99], 0xdef, 5);
        let forced_candidate = 0x1234_u64;
        let mut registry = LineageIdRegistry::default();

        registry
            .insert_candidate(first.clone(), forced_candidate)
            .unwrap();
        assert_eq!(
            registry.insert_candidate(second.clone(), forced_candidate),
            Err(LineageIdentityError::DerivedSubjectCollision {
                subject_id: forced_candidate | DYNAMIC_ID_MARKER,
                first_birth: first,
                second_birth: second
            })
        );
    }

    #[test]
    fn reserving_static_after_dynamic_collision_fails_closed() {
        let key = birth(7, &[11, 3], 0xabc, 2);
        let mut registry = LineageIdRegistry::default();
        let dynamic = registry.resolve(key).unwrap();
        assert_eq!(
            registry.reserve_static_subject(dynamic),
            Err(LineageIdentityError::StaticSubjectConflictsWithDerived {
                subject_id: dynamic.raw()
            })
        );
    }

    #[test]
    fn duplicate_parents_are_rejected() {
        assert_eq!(
            BirthKey::new(7, [parent(5), parent(5)], semantic_event(1), 0),
            Err(BirthKeyError::DuplicateParentSubject { subject_id: 5 })
        );
    }

    #[test]
    fn world_namespace_zero_is_rejected() {
        assert_eq!(
            BirthKey::new(0, [parent(5)], semantic_event(1), 0),
            Err(BirthKeyError::ZeroWorldNamespace)
        );
    }
}
