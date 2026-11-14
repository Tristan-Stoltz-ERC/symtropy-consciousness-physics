// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event-addressed, counter-based randomness for counterfactual simulations.
//!
//! Opt-in prototype only. It does not replace the historical LCG or alter
//! existing experiment outputs. Philox is not a cryptographic RNG.

pub const EVENT_KEY_SCHEMA_VERSION: u32 = 1;
pub const EVENT_RNG_ALGORITHM: &str = "philox4x64-10";

const PHILOX_M4X64_0: u64 = 0xD2E7_470E_E14C_6C93;
const PHILOX_M4X64_1: u64 = 0xCA5A_8263_9512_1157;
const PHILOX_W64_0: u64 = 0x9E37_79B9_7F4A_7C15;
const PHILOX_W64_1: u64 = 0xBB67_AE85_84CA_A73B;
const OPEN_UNIT_SCALE: f64 = 1.0 / ((1_u64 << 52) as f64);

/// A stable u16 tag from the versioned event-kind registry. Zero is reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventKind(u16);

impl EventKind {
    pub const fn new(tag: u16) -> Option<Self> {
        if tag == 0 {
            None
        } else {
            Some(Self(tag))
        }
    }

    pub const fn tag(self) -> u16 {
        self.0
    }
}

/// A stable u16 tag from the versioned draw-purpose registry. Zero is reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawPurpose(u16);

impl DrawPurpose {
    pub const fn new(tag: u16) -> Option<Self> {
        if tag == 0 {
            None
        } else {
            Some(Self(tag))
        }
    }

    pub const fn tag(self) -> u16 {
        self.0
    }
}

/// Initial v1 registry entries. Additions must be append-only and reviewed.
pub mod registry {
    use super::{DrawPurpose, EventKind};

    pub const INITIALIZATION: EventKind = EventKind(1);
    pub const RANDOM_WALK: EventKind = EventKind(2);
    pub const ENVIRONMENT: EventKind = EventKind(3);

    pub const INITIAL_POSITION_X: DrawPurpose = DrawPurpose(1);
    pub const INITIAL_POSITION_Y: DrawPurpose = DrawPurpose(2);
    pub const RANDOM_WALK_ANGLE: DrawPurpose = DrawPurpose(3);
    pub const ENVIRONMENTAL_NOISE: DrawPurpose = DrawPurpose(4);
}

/// Stable identity for a modeled event's subject.
///
/// ID 0 is reserved for environment-global events. Agent IDs are stable,
/// model-assigned, nonzero u32 values. Pair identity uses canonical ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubjectId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectIdError {
    ZeroAgentId,
    SameAgentPair,
}

impl SubjectId {
    pub const fn global_environment() -> Self {
        Self(0)
    }

    pub fn agent(agent_id: u32) -> Result<Self, SubjectIdError> {
        if agent_id == 0 {
            return Err(SubjectIdError::ZeroAgentId);
        }
        Ok(Self(agent_id as u64))
    }

    pub fn pair(first_agent_id: u32, second_agent_id: u32) -> Result<Self, SubjectIdError> {
        if first_agent_id == 0 || second_agent_id == 0 {
            return Err(SubjectIdError::ZeroAgentId);
        }
        if first_agent_id == second_agent_id {
            return Err(SubjectIdError::SameAgentPair);
        }
        let low = first_agent_id.min(second_agent_id);
        let high = first_agent_id.max(second_agent_id);
        Ok(Self(((low as u64) << 32) | high as u64))
    }

    /// Construct a nonzero internal subject ID from a checked lineage derivation.
    ///
    /// This is crate-visible only; public callers must use stable agent/pair
    /// constructors or the collision-checked LineageIdRegistry.
    pub(crate) const fn from_nonzero_raw(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Complete immutable address for one random event.
///
/// Deliberately absent: treatment/controller/scenario label, branch path,
/// thread ID, execution order, wall clock, and intervention-dependent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventAddress {
    replicate_seed: u64,
    experiment_namespace: u64,
    subject_id: SubjectId,
    simulation_tick: u64,
    event_kind: EventKind,
    purpose: DrawPurpose,
    occurrence_ordinal: u32,
}

impl EventAddress {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        replicate_seed: u64,
        experiment_namespace: u64,
        subject_id: SubjectId,
        simulation_tick: u64,
        event_kind: EventKind,
        purpose: DrawPurpose,
        occurrence_ordinal: u32,
    ) -> Self {
        Self {
            replicate_seed,
            experiment_namespace,
            subject_id,
            simulation_tick,
            event_kind,
            purpose,
            occurrence_ordinal,
        }
    }

    /// kind[63:48] | purpose[47:32] | occurrence[31:0].
    pub const fn packed_event_code(self) -> u64 {
        ((self.event_kind.0 as u64) << 48)
            | ((self.purpose.0 as u64) << 32)
            | self.occurrence_ordinal as u64
    }

    /// Philox key words: replicate seed and fixed experiment namespace.
    pub const fn key_words(self) -> [u64; 2] {
        [self.replicate_seed, self.experiment_namespace]
    }

    /// Philox counter words for one four-word output block.
    pub const fn counter_words(self, block_index: u64) -> [u64; 4] {
        [
            self.subject_id.0,
            self.simulation_tick,
            self.packed_event_code(),
            block_index,
        ]
    }

    /// Four deterministic u64 values for one addressed draw block.
    pub fn block(self, block_index: u64) -> [u64; 4] {
        philox4x64_10(self.counter_words(block_index), self.key_words())
    }

    /// A deterministic u64 at a scalar draw index. No mutable stream is consumed.
    ///
    /// This random-access method computes the containing four-word block on
    /// every call. Use words() for sequential draws so adjacent lanes reuse a
    /// cached block instead of recomputing the ten-round permutation.
    pub fn word(self, draw_index: u64) -> u64 {
        let words = self.block(draw_index / 4);
        words[(draw_index % 4) as usize]
    }

    /// Iterate scalar words from this event, caching each four-word block.
    ///
    /// The iterator is only a consumption optimization: its values are exactly
    /// the same as word(0), word(1), ... and are independent of other iterators.
    pub fn words(self) -> EventWordIter {
        self.words_from(0)
    }

    /// Iterate scalar words beginning at the specified random-access index.
    pub fn words_from(self, draw_index: u64) -> EventWordIter {
        EventWordIter {
            address: self,
            next_draw_index: Some(draw_index),
            cached_block_index: None,
            cached_words: [0; 4],
        }
    }

    /// A deterministic, strictly open-interval uniform value in (0, 1).
    ///
    /// The top 52 bits are mapped to bin midpoints. This avoids the f64
    /// rounding edge where (top53 + 0.5) * 2^-53 can round to exactly 1.0.
    pub fn uniform_open01(self, draw_index: u64) -> f64 {
        open_unit_interval_from_word(self.word(draw_index))
    }

    /// Iterate strictly open-interval uniform values in (0, 1), caching
    /// Philox blocks through the underlying sequential word iterator.
    pub fn uniforms_open01(self) -> EventUniformIter {
        self.uniforms_open01_from(0)
    }

    /// Iterate open-interval uniform values beginning at a scalar draw index.
    pub fn uniforms_open01_from(self, draw_index: u64) -> EventUniformIter {
        EventUniformIter {
            words: self.words_from(draw_index),
        }
    }
}

/// Sequential scalar draws from one EventAddress with a one-block cache.
///
/// The cache is local to this iterator. Cloning an iterator copies its current
/// position and cached block; it does not change the event address or any
/// random value returned by another iterator.
#[derive(Debug, Clone)]
pub struct EventWordIter {
    address: EventAddress,
    next_draw_index: Option<u64>,
    cached_block_index: Option<u64>,
    cached_words: [u64; 4],
}

impl Iterator for EventWordIter {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let draw_index = self.next_draw_index?;
        let block_index = draw_index / 4;
        if self.cached_block_index != Some(block_index) {
            self.cached_words = self.address.block(block_index);
            self.cached_block_index = Some(block_index);
        }

        self.next_draw_index = draw_index.checked_add(1);
        Some(self.cached_words[(draw_index % 4) as usize])
    }
}

impl std::iter::FusedIterator for EventWordIter {}

/// Sequential open-interval f64 values from one EventAddress.
///
/// This wrapper reuses EventWordIter's four-word Philox block cache and applies
/// the same top-52-bit midpoint conversion as uniform_open01(draw_index).
#[derive(Debug, Clone)]
pub struct EventUniformIter {
    words: EventWordIter,
}

impl Iterator for EventUniformIter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.words.next().map(open_unit_interval_from_word)
    }
}

impl std::iter::FusedIterator for EventUniformIter {}

/// Random123 Philox4x64 with ten rounds. Counter is four u64 words; key is two.
pub fn philox4x64_10(mut counter: [u64; 4], mut key: [u64; 2]) -> [u64; 4] {
    for round in 0..10 {
        counter = philox4x64_round(counter, key);
        if round != 9 {
            key[0] = key[0].wrapping_add(PHILOX_W64_0);
            key[1] = key[1].wrapping_add(PHILOX_W64_1);
        }
    }
    counter
}

#[inline]
fn multiply_high_low(left: u64, right: u64) -> (u64, u64) {
    let product = (left as u128) * (right as u128);
    ((product >> 64) as u64, product as u64)
}

#[inline]
fn philox4x64_round(counter: [u64; 4], key: [u64; 2]) -> [u64; 4] {
    let (high0, low0) = multiply_high_low(PHILOX_M4X64_0, counter[0]);
    let (high1, low1) = multiply_high_low(PHILOX_M4X64_1, counter[2]);
    [
        high1 ^ counter[1] ^ key[0],
        low1,
        high0 ^ counter[3] ^ key[1],
        low0,
    ]
}

#[inline]
fn open_unit_interval_from_word(word: u64) -> f64 {
    let top52 = word >> 12;
    (top52 as f64 + 0.5) * OPEN_UNIT_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn philox4x64_10_matches_random123_zero_known_answer_vector() {
        assert_eq!(
            philox4x64_10([0; 4], [0; 2]),
            [
                0x1655_4d9e_ca36_314c,
                0xdb20_fe9d_672d_0fdc,
                0xd7e7_72ce_e186_176b,
                0x7e68_b68a_ec7b_a23b,
            ]
        );
    }

    #[test]
    fn philox4x64_10_matches_random123_mixed_known_answer_vector() {
        let counter = [
            0x243f_6a88_85a3_08d3,
            0x1319_8a2e_0370_7344,
            0xa409_3822_299f_31d0,
            0x082e_fa98_ec4e_6c89,
        ];
        let key = [0x4528_21e6_38d0_1377, 0xbe54_66cf_34e9_0c6c];
        assert_eq!(
            philox4x64_10(counter, key),
            [
                0xa528_f454_03e6_1d95,
                0x38c7_2dbd_566e_9788,
                0xa5a1_610e_72fd_18b5,
                0x57bd_43b5_e52b_7fe6,
            ]
        );
    }

    #[test]
    fn philox4x64_10_matches_random123_all_ones_known_answer_vector() {
        assert_eq!(
            philox4x64_10([u64::MAX; 4], [u64::MAX; 2]),
            [
                0x87b0_92c3_013f_e90b,
                0x438c_3c67_be8d_0224,
                0x9cc7_d7c6_9cd7_77b6,
                0xa09c_aebf_594f_0ba0,
            ]
        );
    }

    fn address(
        subject: SubjectId,
        tick: u64,
        event: EventKind,
        purpose: DrawPurpose,
        occurrence: u32,
    ) -> EventAddress {
        EventAddress::new(
            42,
            0x4c55_4d49_4e4f_5553,
            subject,
            tick,
            event,
            purpose,
            occurrence,
        )
    }

    #[test]
    fn cached_uniform_iterator_matches_random_access_across_block_boundaries() {
        let addr = address(
            SubjectId::agent(27).unwrap(),
            107,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            2,
        );
        let direct: Vec<_> = (0..13).map(|index| addr.uniform_open01(index)).collect();
        let sequential: Vec<_> = addr.uniforms_open01().take(13).collect();
        assert_eq!(
            sequential
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            direct
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn cached_uniform_iterator_supports_nonzero_start_and_stays_open() {
        let addr = address(
            SubjectId::agent(28).unwrap(),
            108,
            registry::ENVIRONMENT,
            registry::ENVIRONMENTAL_NOISE,
            0,
        );
        let direct: Vec<_> = (3..20).map(|index| addr.uniform_open01(index)).collect();
        let sequential: Vec<_> = addr.uniforms_open01_from(3).take(17).collect();
        assert_eq!(
            sequential
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            direct
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
        assert!(sequential.iter().all(|value| *value > 0.0 && *value < 1.0));
    }

    #[test]
    fn cached_uniform_iterator_emits_terminal_u64_draw_once() {
        let addr = address(
            SubjectId::agent(29).unwrap(),
            109,
            registry::ENVIRONMENT,
            registry::ENVIRONMENTAL_NOISE,
            0,
        );
        let mut final_uniform = addr.uniforms_open01_from(u64::MAX);
        assert_eq!(
            final_uniform.next().map(f64::to_bits),
            Some(addr.uniform_open01(u64::MAX).to_bits())
        );
        assert_eq!(final_uniform.next(), None);
        assert_eq!(final_uniform.next(), None);
    }

    #[test]
    fn cached_word_iterator_matches_random_access_across_block_boundaries() {
        let addr = address(
            SubjectId::agent(23).unwrap(),
            101,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            2,
        );
        let direct: Vec<_> = (0..13).map(|index| addr.word(index)).collect();
        let sequential: Vec<_> = addr.words().take(13).collect();
        assert_eq!(sequential, direct);
    }

    #[test]
    fn cached_word_iterator_supports_nonzero_start_indices() {
        let addr = address(
            SubjectId::agent(24).unwrap(),
            102,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            1,
        );
        let direct: Vec<_> = (3..11).map(|index| addr.word(index)).collect();
        let sequential: Vec<_> = addr.words_from(3).take(8).collect();
        assert_eq!(sequential, direct);
    }

    #[test]
    fn cloned_cached_iterators_have_independent_positions() {
        let addr = address(
            SubjectId::agent(25).unwrap(),
            103,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        );
        let mut original = addr.words();
        assert_eq!(original.next(), Some(addr.word(0)));
        let mut cloned = original.clone();
        assert_eq!(original.next(), Some(addr.word(1)));
        assert_eq!(cloned.next(), Some(addr.word(1)));
        assert_eq!(original.next(), Some(addr.word(2)));
        assert_eq!(cloned.next(), Some(addr.word(2)));
    }

    #[test]
    fn cached_word_iterator_emits_terminal_u64_draw_once() {
        let addr = address(
            SubjectId::agent(26).unwrap(),
            104,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        );
        let mut final_word = addr.words_from(u64::MAX);
        assert_eq!(final_word.next(), Some(addr.word(u64::MAX)));
        assert_eq!(final_word.next(), None);
        assert_eq!(final_word.next(), None);
    }

    #[test]
    fn event_mapping_is_exact_and_versioned() {
        let addr = address(
            SubjectId::agent(7).unwrap(),
            99,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            3,
        );
        assert_eq!(EVENT_KEY_SCHEMA_VERSION, 1);
        assert_eq!(EVENT_RNG_ALGORITHM, "philox4x64-10");
        assert_eq!(
            addr.packed_event_code(),
            (2_u64 << 48) | (3_u64 << 32) | 3_u64
        );
        assert_eq!(addr.key_words(), [42, 0x4c55_4d49_4e4f_5553]);
        assert_eq!(
            addr.counter_words(5),
            [7, 99, (2_u64 << 48) | (3_u64 << 32) | 3, 5]
        );
    }

    #[test]
    fn repeated_queries_are_bit_identical_and_order_independent() {
        let subject = SubjectId::agent(17).unwrap();
        let event_a = address(
            subject,
            12,
            registry::INITIALIZATION,
            registry::INITIAL_POSITION_X,
            0,
        );
        let event_b = address(
            subject,
            12,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        );
        let before = event_a.word(9);
        let _unrelated_draws = (0..257).map(|i| event_b.word(i)).collect::<Vec<_>>();
        assert_eq!(before, event_a.word(9));
        assert_eq!(before, event_a.word(9));
    }

    #[test]
    fn scalar_draw_indices_cover_all_lanes_and_cross_block_boundaries() {
        let addr = address(
            SubjectId::agent(1).unwrap(),
            0,
            registry::INITIALIZATION,
            registry::INITIAL_POSITION_X,
            0,
        );
        let block0 = addr.block(0);
        let block1 = addr.block(1);
        assert_eq!(addr.word(0), block0[0]);
        assert_eq!(addr.word(1), block0[1]);
        assert_eq!(addr.word(2), block0[2]);
        assert_eq!(addr.word(3), block0[3]);
        assert_eq!(addr.word(4), block1[0]);
        assert_eq!(addr.word(5), block1[1]);
    }

    #[test]
    fn event_keys_exclude_treatment_and_execution_order_by_construction() {
        let common = address(
            SubjectId::agent(5).unwrap(),
            123,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        );
        // The v1 API has no treatment, controller, thread, or call-order field.
        let control_event = common;
        let treatment_event = common;
        assert_eq!(control_event.word(0), treatment_event.word(0));
    }

    #[test]
    fn changing_each_identity_dimension_changes_the_sampled_fixture() {
        let base = address(
            SubjectId::agent(5).unwrap(),
            10,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        );
        let reference = base.word(0);
        let changed = [
            address(
                SubjectId::agent(6).unwrap(),
                10,
                registry::RANDOM_WALK,
                registry::RANDOM_WALK_ANGLE,
                0,
            ),
            address(
                SubjectId::agent(5).unwrap(),
                11,
                registry::RANDOM_WALK,
                registry::RANDOM_WALK_ANGLE,
                0,
            ),
            address(
                SubjectId::agent(5).unwrap(),
                10,
                registry::ENVIRONMENT,
                registry::RANDOM_WALK_ANGLE,
                0,
            ),
            address(
                SubjectId::agent(5).unwrap(),
                10,
                registry::RANDOM_WALK,
                registry::ENVIRONMENTAL_NOISE,
                0,
            ),
            address(
                SubjectId::agent(5).unwrap(),
                10,
                registry::RANDOM_WALK,
                registry::RANDOM_WALK_ANGLE,
                1,
            ),
        ];
        assert!(changed.iter().all(|item| item.word(0) != reference));
    }

    #[test]
    fn subject_pair_identity_is_canonical_and_reserved_zero_is_enforced() {
        let ab = SubjectId::pair(7, 19).unwrap();
        let ba = SubjectId::pair(19, 7).unwrap();
        assert_eq!(ab, ba);
        assert_eq!(ab.raw(), (7_u64 << 32) | 19);
        assert_eq!(SubjectId::global_environment().raw(), 0);
        assert_eq!(SubjectId::agent(0), Err(SubjectIdError::ZeroAgentId));
        assert_eq!(SubjectId::pair(0, 4), Err(SubjectIdError::ZeroAgentId));
        assert_eq!(SubjectId::pair(3, 3), Err(SubjectIdError::SameAgentPair));
    }

    #[test]
    fn open_uniform_conversion_is_strictly_inside_zero_and_one() {
        assert!(open_unit_interval_from_word(0) > 0.0);
        assert!(open_unit_interval_from_word(u64::MAX) < 1.0);
        let addr = address(
            SubjectId::global_environment(),
            1,
            registry::ENVIRONMENT,
            registry::ENVIRONMENTAL_NOISE,
            0,
        );
        for draw in 0..512 {
            let value = addr.uniform_open01(draw);
            assert!(value > 0.0 && value < 1.0);
        }
    }

    #[test]
    fn serial_and_parallel_queries_produce_identical_event_values() {
        let addresses: Vec<_> = (1..=128)
            .map(|agent| {
                address(
                    SubjectId::agent(agent).unwrap(),
                    80,
                    registry::RANDOM_WALK,
                    registry::RANDOM_WALK_ANGLE,
                    0,
                )
            })
            .collect();
        let serial: Vec<_> = addresses.iter().map(|item| item.word(0)).collect();
        let parallel = std::thread::scope(|scope| {
            let handles: Vec<_> = addresses
                .iter()
                .map(|item| scope.spawn(move || item.word(0)))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(serial, parallel);
    }

    #[test]
    fn zero_tags_are_not_constructible_through_the_checked_api() {
        assert!(EventKind::new(0).is_none());
        assert!(DrawPurpose::new(0).is_none());
        assert_eq!(EventKind::new(7).unwrap().tag(), 7);
        assert_eq!(DrawPurpose::new(9).unwrap().tag(), 9);
    }
}
