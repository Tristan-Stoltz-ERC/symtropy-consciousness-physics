// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Offline event-alignment accounting for paired counterfactual scenarios.
//!
//! This ledger does not create or repair event identity. It compares the
//! identities and output words supplied by the model and rejects duplicate
//! event/draw keys so matching assumptions are visible and auditable.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::event_rng::EventAddress;

/// One observed output for one event-address and scalar draw index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventOutputRecord {
    pub address: EventAddress,
    pub draw_index: u64,
    pub output_word: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioSide {
    Reference,
    Counterfactual,
}

impl fmt::Display for ScenarioSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reference => write!(f, "reference"),
            Self::Counterfactual => write!(f, "counterfactual"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentError {
    EmptyScenario {
        side: ScenarioSide,
    },
    DuplicateEventDraw {
        side: ScenarioSide,
        draw_index: u64,
        event_code: u64,
        subject_id: u64,
        simulation_tick: u64,
    },
}

impl fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScenario { side } => write!(f, "{side} scenario has no event outputs"),
            Self::DuplicateEventDraw {
                side,
                draw_index,
                event_code,
                subject_id,
                simulation_tick,
            } => write!(
                f,
                "{side} scenario emitted duplicate event/draw key: subject={subject_id}, \
                 tick={simulation_tick}, event_code={event_code:#018x}, draw_index={draw_index}"
            ),
        }
    }
}
impl Error for AlignmentError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EventDrawKey {
    address: EventAddress,
    draw_index: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventAlignmentSummary {
    pub reference_events: usize,
    pub counterfactual_events: usize,
    pub matched_events: usize,
    pub reference_only_events: usize,
    pub counterfactual_only_events: usize,
    /// Matched event/draw keys with identical u64 output words.
    pub bit_identical: usize,
    /// Matched event/draw keys whose output words differ.
    pub mismatched: usize,
}

impl EventAlignmentSummary {
    /// Fraction of the smaller scenario's event set that has an explicit match.
    ///
    /// This is a coverage measure, not a causal-validity or random-quality score.
    pub fn matched_fraction_of_smaller_scenario(self) -> f64 {
        let denominator = self.reference_events.min(self.counterfactual_events);
        if denominator == 0 {
            0.0
        } else {
            self.matched_events as f64 / denominator as f64
        }
    }

    /// True only when all matched keys return identical words and no event key
    /// is missing from either scenario.
    pub const fn fully_aligned(self) -> bool {
        self.reference_only_events == 0
            && self.counterfactual_only_events == 0
            && self.mismatched == 0
    }
}

fn index_events(
    records: &[EventOutputRecord],
    side: ScenarioSide,
) -> Result<HashMap<EventDrawKey, u64>, AlignmentError> {
    if records.is_empty() {
        return Err(AlignmentError::EmptyScenario { side });
    }

    let mut indexed = HashMap::with_capacity(records.len());
    for record in records {
        let key = EventDrawKey {
            address: record.address,
            draw_index: record.draw_index,
        };
        if indexed.insert(key, record.output_word).is_some() {
            return Err(AlignmentError::DuplicateEventDraw {
                side,
                draw_index: record.draw_index,
                event_code: record.address.packed_event_code(),
                subject_id: record.address.counter_words(0)[0],
                simulation_tick: record.address.counter_words(0)[1],
            });
        }
    }
    Ok(indexed)
}

/// Compare two independently captured scenarios by event identity and draw index.
///
/// Scenario labels are metadata at the call site and are deliberately absent
/// from EventAddress. Keys are matched before comparing words. A coincidentally
/// equal output from two different event keys is never treated as a match.
pub fn compare_event_outputs(
    reference: &[EventOutputRecord],
    counterfactual: &[EventOutputRecord],
) -> Result<EventAlignmentSummary, AlignmentError> {
    let reference_index = index_events(reference, ScenarioSide::Reference)?;
    let counterfactual_index = index_events(counterfactual, ScenarioSide::Counterfactual)?;

    let mut matched_events = 0usize;
    let mut bit_identical = 0usize;
    let mut mismatched = 0usize;

    for (key, reference_word) in &reference_index {
        if let Some(counterfactual_word) = counterfactual_index.get(key) {
            matched_events += 1;
            if reference_word == counterfactual_word {
                bit_identical += 1;
            } else {
                mismatched += 1;
            }
        }
    }

    Ok(EventAlignmentSummary {
        reference_events: reference_index.len(),
        counterfactual_events: counterfactual_index.len(),
        matched_events,
        reference_only_events: reference_index.len() - matched_events,
        counterfactual_only_events: counterfactual_index.len() - matched_events,
        bit_identical,
        mismatched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_rng::{registry, EventAddress, SubjectId};

    const EXPERIMENT_NAMESPACE: u64 = 0x4c55_4d49_4e4f_5553;

    fn shared_address(agent: u32) -> EventAddress {
        EventAddress::new(
            42,
            EXPERIMENT_NAMESPACE,
            SubjectId::agent(agent).unwrap(),
            17,
            registry::INITIALIZATION,
            registry::INITIAL_POSITION_X,
            0,
        )
    }

    fn branch_only_address(agent: u32) -> EventAddress {
        EventAddress::new(
            42,
            EXPERIMENT_NAMESPACE,
            SubjectId::agent(agent).unwrap(),
            17,
            registry::RANDOM_WALK,
            registry::RANDOM_WALK_ANGLE,
            0,
        )
    }

    #[test]
    fn identical_event_keys_match_even_when_scenario_order_differs() {
        let reference: Vec<_> = (1..=8)
            .map(|agent| {
                let address = shared_address(agent);
                EventOutputRecord {
                    address,
                    draw_index: 0,
                    output_word: address.word(0),
                }
            })
            .collect();
        let counterfactual: Vec<_> = reference.iter().rev().copied().collect();

        let summary = compare_event_outputs(&reference, &counterfactual).unwrap();
        assert_eq!(summary.reference_events, 8);
        assert_eq!(summary.counterfactual_events, 8);
        assert_eq!(summary.matched_events, 8);
        assert_eq!(summary.bit_identical, 8);
        assert_eq!(summary.mismatched, 0);
        assert_eq!(summary.reference_only_events, 0);
        assert_eq!(summary.counterfactual_only_events, 0);
        assert_eq!(summary.matched_fraction_of_smaller_scenario(), 1.0);
        assert!(summary.fully_aligned());
    }

    #[test]
    fn conditional_events_are_reported_not_fabricated_as_matches() {
        let reference: Vec<_> = (1..=8)
            .map(|agent| {
                let address = shared_address(agent);
                EventOutputRecord {
                    address,
                    draw_index: 0,
                    output_word: address.word(0),
                }
            })
            .collect();
        let mut counterfactual = reference.clone();
        for agent in 1..=8 {
            let address = branch_only_address(agent);
            counterfactual.push(EventOutputRecord {
                address,
                draw_index: 0,
                output_word: address.word(0),
            });
        }

        let summary = compare_event_outputs(&reference, &counterfactual).unwrap();
        assert_eq!(summary.matched_events, 8);
        assert_eq!(summary.counterfactual_only_events, 8);
        assert_eq!(summary.reference_only_events, 0);
        assert_eq!(summary.bit_identical, 8);
        assert!(!summary.fully_aligned());
    }

    #[test]
    fn event_key_match_is_required_even_when_output_words_coincide() {
        let left_address = shared_address(1);
        let right_address = shared_address(2);
        let same_word = left_address.word(0);
        let reference = [EventOutputRecord {
            address: left_address,
            draw_index: 0,
            output_word: same_word,
        }];
        let counterfactual = [EventOutputRecord {
            address: right_address,
            draw_index: 0,
            output_word: same_word,
        }];

        let summary = compare_event_outputs(&reference, &counterfactual).unwrap();
        assert_eq!(summary.matched_events, 0);
        assert_eq!(summary.reference_only_events, 1);
        assert_eq!(summary.counterfactual_only_events, 1);
        assert_eq!(summary.bit_identical, 0);
    }

    #[test]
    fn different_words_for_the_same_event_key_are_counted_as_mismatches() {
        let address = shared_address(3);
        let reference = [EventOutputRecord {
            address,
            draw_index: 2,
            output_word: address.word(2),
        }];
        let counterfactual = [EventOutputRecord {
            address,
            draw_index: 2,
            output_word: address.word(2) ^ 1,
        }];

        let summary = compare_event_outputs(&reference, &counterfactual).unwrap();
        assert_eq!(summary.matched_events, 1);
        assert_eq!(summary.bit_identical, 0);
        assert_eq!(summary.mismatched, 1);
        assert!(!summary.fully_aligned());
    }

    #[test]
    fn duplicate_event_draw_keys_fail_closed_per_scenario() {
        let address = shared_address(4);
        let duplicate = [
            EventOutputRecord {
                address,
                draw_index: 0,
                output_word: address.word(0),
            },
            EventOutputRecord {
                address,
                draw_index: 0,
                output_word: address.word(0),
            },
        ];
        let valid = [EventOutputRecord {
            address,
            draw_index: 0,
            output_word: address.word(0),
        }];

        assert!(matches!(
            compare_event_outputs(&duplicate, &valid),
            Err(AlignmentError::DuplicateEventDraw {
                side: ScenarioSide::Reference,
                ..
            })
        ));
    }

    #[test]
    fn empty_scenarios_are_not_mistaken_for_zero_mismatch() {
        let address = shared_address(9);
        let valid = [EventOutputRecord {
            address,
            draw_index: 0,
            output_word: address.word(0),
        }];
        assert!(matches!(
            compare_event_outputs(&[], &valid),
            Err(AlignmentError::EmptyScenario {
                side: ScenarioSide::Reference
            })
        ));
    }

    #[test]
    fn known_structural_causal_model_recovers_the_exact_additive_effect() {
        const AGENTS: u32 = 32;
        const KNOWN_TREATMENT_EFFECT: f64 = 0.25;

        let mut reference_records = Vec::new();
        let mut counterfactual_records = Vec::new();
        let mut paired_effect = 0.0;

        for agent in 1..=AGENTS {
            let exogenous_event = shared_address(agent);
            let exogenous_noise = exogenous_event.uniform_open01(0);

            let control_outcome = exogenous_noise;
            let treatment_outcome = exogenous_noise + KNOWN_TREATMENT_EFFECT;
            paired_effect += treatment_outcome - control_outcome;

            let shared_word = exogenous_event.word(0);
            reference_records.push(EventOutputRecord {
                address: exogenous_event,
                draw_index: 0,
                output_word: shared_word,
            });
            counterfactual_records.push(EventOutputRecord {
                address: exogenous_event,
                draw_index: 0,
                output_word: shared_word,
            });

            // A treatment-only branch asks for an additional random event.
            // It cannot move the address or value of the shared event above.
            let conditional_event = branch_only_address(agent);
            counterfactual_records.push(EventOutputRecord {
                address: conditional_event,
                draw_index: 0,
                output_word: conditional_event.word(0),
            });
        }

        let average_effect = paired_effect / AGENTS as f64;
        assert!((average_effect - KNOWN_TREATMENT_EFFECT).abs() < 1e-15);

        let alignment = compare_event_outputs(&reference_records, &counterfactual_records).unwrap();
        assert_eq!(alignment.matched_events, AGENTS as usize);
        assert_eq!(alignment.bit_identical, AGENTS as usize);
        assert_eq!(alignment.mismatched, 0);
        assert_eq!(alignment.counterfactual_only_events, AGENTS as usize);
        assert_eq!(alignment.reference_only_events, 0);
        assert_eq!(alignment.matched_fraction_of_smaller_scenario(), 1.0);
    }
}
