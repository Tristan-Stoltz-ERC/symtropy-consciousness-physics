// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hierarchical semantic event identity for counterfactual random draws.
//!
//! Stable roots and semantic child roles replace global counters and call-order
//! identity. The 128-bit Philox-derived ID is deterministic, not cryptographic,
//! and not collision-free. Register captured paths to fail closed on collisions.

use std::collections::HashMap;

use crate::event_rng::{philox4x64_10, DrawPurpose, EventKind, SubjectId};

pub const EVENT_LINEAGE_SCHEMA_VERSION: u32 = 1;
pub const EVENT_LINEAGE_ALGORITHM: &str = "philox4x64-10-lineage-128-v1";
const LINEAGE_KEY: [u64; 2] = [0x6c69_6e65_6167_6531, 0x5f70_6174_685f_7631];
const ROOT_DOMAIN: u64 = 0x524f_4f54_5f56_3031;
const CHILD_DOMAIN: u64 = 0x4348_494c_445f_5631;
const OPEN_UNIT_SCALE: f64 = 1.0 / ((1_u64 << 52) as f64);

/// A 128-bit deterministic identifier for one semantic event-lineage node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventLineageId([u64; 2]);

impl EventLineageId {
    pub const fn words(self) -> [u64; 2] { self.0 }
}

/// One semantic edge in a causal event path. The occurrence ordinal is model
/// semantics, never array position, execution order, or a global draw counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineageSegment {
    event_kind: EventKind,
    purpose: DrawPurpose,
    occurrence_ordinal: u32,
}

impl LineageSegment {
    pub const fn new(event_kind: EventKind, purpose: DrawPurpose, occurrence_ordinal: u32) -> Self {
        Self { event_kind, purpose, occurrence_ordinal }
    }

    pub const fn packed_code(self) -> u64 {
        ((self.event_kind.tag() as u64) << 48)
            | ((self.purpose.tag() as u64) << 32)
            | self.occurrence_ordinal as u64
    }
}

/// Semantic path from one stable model-assigned root subject to an event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventLineagePath {
    experiment_namespace: u64,
    root_subject: SubjectId,
    segments: Vec<LineageSegment>,
}

impl EventLineagePath {
    pub fn root(experiment_namespace: u64, root_subject: SubjectId) -> Self {
        Self { experiment_namespace, root_subject, segments: Vec::new() }
    }

    pub fn child(&self, segment: LineageSegment) -> Self {
        let mut child = self.clone();
        child.segments.push(segment);
        child
    }

    pub const fn experiment_namespace(&self) -> u64 { self.experiment_namespace }
    pub const fn root_subject(&self) -> SubjectId { self.root_subject }
    pub fn segments(&self) -> &[LineageSegment] { &self.segments }

    /// Fold root and semantic child segments into a versioned 128-bit ID.
    pub fn id(&self) -> EventLineageId {
        let root = philox4x64_10(
            [self.experiment_namespace, self.root_subject.raw(), ROOT_DOMAIN, EVENT_LINEAGE_SCHEMA_VERSION as u64],
            LINEAGE_KEY,
        );
        let mut id = EventLineageId([root[0], root[1]]);
        for segment in &self.segments {
            let child = philox4x64_10(
                [id.0[0], id.0[1], segment.packed_code(), CHILD_DOMAIN],
                LINEAGE_KEY,
            );
            id = EventLineageId([child[0], child[1]]);
        }
        id
    }
}

/// Two distinct semantic paths deriving one ID are rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageCollision {
    pub id: EventLineageId,
    pub existing_path: EventLineagePath,
    pub conflicting_path: EventLineagePath,
}

/// Per-trace collision registry. Re-registering the exact same path is safe.
#[derive(Debug, Default)]
pub struct LineageRegistry {
    paths_by_id: HashMap<EventLineageId, EventLineagePath>,
}

impl LineageRegistry {
    pub fn register(&mut self, path: EventLineagePath) -> Result<EventLineageId, LineageCollision> {
        self.register_with_id(path.id(), path)
    }

    fn register_with_id(
        &mut self,
        id: EventLineageId,
        path: EventLineagePath,
    ) -> Result<EventLineageId, LineageCollision> {
        if let Some(existing) = self.paths_by_id.get(&id) {
            if existing != &path {
                return Err(LineageCollision { id, existing_path: existing.clone(), conflicting_path: path });
            }
            return Ok(id);
        }
        self.paths_by_id.insert(id, path);
        Ok(id)
    }

    pub fn len(&self) -> usize { self.paths_by_id.len() }
    pub fn is_empty(&self) -> bool { self.paths_by_id.is_empty() }
}

/// Random address using hierarchical semantic identity. The lineage is
/// replicate-independent; the Philox key includes the replicate seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineageEventAddress {
    replicate_seed: u64,
    experiment_namespace: u64,
    lineage_id: EventLineageId,
    simulation_tick: u64,
}

impl LineageEventAddress {
    pub fn from_path(replicate_seed: u64, path: &EventLineagePath, simulation_tick: u64) -> Self {
        Self {
            replicate_seed,
            experiment_namespace: path.experiment_namespace,
            lineage_id: path.id(),
            simulation_tick,
        }
    }

    pub const fn lineage_id(self) -> EventLineageId { self.lineage_id }
    pub const fn key_words(self) -> [u64; 2] { [self.replicate_seed, self.experiment_namespace] }
    pub const fn counter_words(self, block_index: u64) -> [u64; 4] {
        [self.lineage_id.0[0], self.lineage_id.0[1], self.simulation_tick, block_index]
    }
    pub fn block(self, block_index: u64) -> [u64; 4] {
        philox4x64_10(self.counter_words(block_index), self.key_words())
    }
    pub fn word(self, draw_index: u64) -> u64 {
        self.block(draw_index / 4)[(draw_index % 4) as usize]
    }
    pub fn uniform_open01(self, draw_index: u64) -> f64 {
        let top52 = self.word(draw_index) >> 12;
        (top52 as f64 + 0.5) * OPEN_UNIT_SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_rng::registry;

    const NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;

    fn root() -> EventLineagePath {
        EventLineagePath::root(NAMESPACE, SubjectId::agent(7).unwrap())
    }

    fn shared_segment() -> LineageSegment {
        LineageSegment::new(registry::INITIALIZATION, registry::INITIAL_POSITION_X, 4)
    }

    #[test]
    fn lineage_root_and_child_match_schema_v1_known_answer() {
        assert_eq!(
            root().id().words(),
            [0x47a2_f70c_2d0b_5a9b, 0x6e0e_00f7_46f9_c4f5]
        );
        assert_eq!(
            root().child(shared_segment()).id().words(),
            [0x931e_6edf_6100_ce94, 0x53d0_e02c_1c77_6516]
        );
        assert_eq!(EVENT_LINEAGE_SCHEMA_VERSION, 1);
        assert_eq!(EVENT_LINEAGE_ALGORITHM, "philox4x64-10-lineage-128-v1");
    }

    #[test]
    fn same_causal_path_has_same_id_across_reconstructions() {
        assert_eq!(root().child(shared_segment()).id(), root().child(shared_segment()).id());
    }

    #[test]
    fn sibling_creation_order_does_not_change_identity() {
        let p = root();
        let a = LineageSegment::new(registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE, 0);
        let b = LineageSegment::new(registry::ENVIRONMENT, registry::ENVIRONMENTAL_NOISE, 2);
        let a_then_b = (p.child(a).id(), p.child(b).id());
        let b_then_a = (p.child(b).id(), p.child(a).id());
        assert_eq!(a_then_b.0, b_then_a.1);
        assert_eq!(a_then_b.1, b_then_a.0);
    }

    #[test]
    fn parent_role_and_occurrence_changes_change_identity() {
        let base = root().child(shared_segment()).id();
        let other_parent = EventLineagePath::root(NAMESPACE, SubjectId::agent(8).unwrap())
            .child(shared_segment()).id();
        let other_role = root().child(LineageSegment::new(
            registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE, 4
        )).id();
        let other_occurrence = root().child(LineageSegment::new(
            registry::INITIALIZATION, registry::INITIAL_POSITION_X, 5
        )).id();
        assert_ne!(base, other_parent);
        assert_ne!(base, other_role);
        assert_ne!(base, other_occurrence);
    }

    #[test]
    fn shared_and_conditional_branches_are_not_conflated() {
        let p = root();
        let common_control = p.child(shared_segment());
        let common_treatment = p.child(shared_segment());
        let treatment_only = p.child(LineageSegment::new(
            registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE, 0
        ));
        assert_eq!(common_control.id(), common_treatment.id());
        assert_ne!(common_control.id(), treatment_only.id());
    }

    #[test]
    fn registry_is_idempotent_and_rejects_a_forced_collision() {
        let a = root().child(shared_segment());
        let b = root().child(LineageSegment::new(
            registry::RANDOM_WALK, registry::RANDOM_WALK_ANGLE, 4
        ));
        let mut registry = LineageRegistry::default();
        let id = registry.register(a.clone()).unwrap();
        assert_eq!(registry.register(a).unwrap(), id);
        assert_eq!(registry.len(), 1);
        assert!(registry.register_with_id(id, b).is_err());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn lineage_address_is_deterministic_and_replicate_keyed() {
        let path = root().child(shared_segment());
        let a = LineageEventAddress::from_path(42, &path, 99);
        let b = LineageEventAddress::from_path(42, &path, 99);
        let other_replicate = LineageEventAddress::from_path(43, &path, 99);
        assert_eq!(a.word(0), b.word(0));
        assert_eq!(a.key_words(), [42, NAMESPACE]);
        assert_ne!(a.word(0), other_replicate.word(0));
        let id = a.lineage_id().words();
        assert_eq!(a.counter_words(3), [id[0], id[1], 99, 3]);
    }

    #[test]
    fn lineage_uniform_conversion_is_strictly_open_interval() {
        let path = root().child(shared_segment());
        let address = LineageEventAddress::from_path(42, &path, 1);
        assert!(address.uniform_open01(0) > 0.0);
        assert!(address.uniform_open01(u64::MAX) < 1.0);
        for draw in 0..1024 {
            let value = address.uniform_open01(draw);
            assert!(value > 0.0 && value < 1.0);
        }
    }
}
