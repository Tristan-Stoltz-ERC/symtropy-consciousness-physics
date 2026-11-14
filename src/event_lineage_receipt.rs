// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical export, validation, and cross-scenario reconciliation of dynamic
//! entity birth-lineage assignments. The receipt format is dependency-free and
//! strict so its bytes can be hashed and independently reviewed.

use std::collections::{HashMap, HashSet};

use crate::event_identity::SemanticEventId;
use crate::event_lineage::{
    BirthKey, BirthKeyError, LineageAssignment, LineageIdRegistry, LineageIdentityError,
};
use crate::event_rng::SubjectId;

pub const LINEAGE_RECEIPT_SCHEMA: &str = "luminous.dynamic-entity-lineage-receipt.v1";
const COLUMNS: &str =
    "world_namespace,parent_subject_ids,semantic_birth_event_id,offspring_slot,subject_id";
const MAX_RECEIPT_ROWS: usize = 1_000_000;

/// A canonical, ordered set of dynamic-entity birth assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageReceipt {
    assignments: Vec<LineageAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageReceiptError {
    BadSchema,
    BadRecordCount,
    BadColumns,
    TooManyRows,
    InvalidRow {
        line: usize,
        reason: String,
    },
    DuplicateBirthKey,
    DuplicateSubjectId {
        subject_id: u64,
    },
    NonCanonicalEncoding,
    DerivedSubjectMismatch {
        recorded: SubjectId,
        derived: SubjectId,
    },
    BirthSubjectMismatchAcrossScenarios {
        reference: SubjectId,
        counterfactual: SubjectId,
    },
    Identity(LineageIdentityError),
}

impl From<LineageIdentityError> for LineageReceiptError {
    fn from(value: LineageIdentityError) -> Self {
        Self::Identity(value)
    }
}

impl From<BirthKeyError> for LineageReceiptError {
    fn from(value: BirthKeyError) -> Self {
        Self::InvalidRow {
            line: 0,
            reason: format!("{value:?}"),
        }
    }
}

impl LineageReceipt {
    pub fn from_registry(registry: &LineageIdRegistry) -> Result<Self, LineageReceiptError> {
        Self::from_assignments(registry.assignments())
    }

    pub fn from_assignments(
        mut assignments: Vec<LineageAssignment>,
    ) -> Result<Self, LineageReceiptError> {
        assignments.sort_unstable_by_key(|assignment| assignment.subject_id.raw());

        let mut birth_keys = HashSet::with_capacity(assignments.len());
        let mut subject_ids = HashSet::with_capacity(assignments.len());
        for assignment in &assignments {
            if !birth_keys.insert(assignment.birth_key.clone()) {
                return Err(LineageReceiptError::DuplicateBirthKey);
            }
            if !subject_ids.insert(assignment.subject_id.raw()) {
                return Err(LineageReceiptError::DuplicateSubjectId {
                    subject_id: assignment.subject_id.raw(),
                });
            }
        }

        Ok(Self { assignments })
    }

    pub fn assignments(&self) -> &[LineageAssignment] {
        &self.assignments
    }

    /// Encode using strict decimal fields, fixed-width lowercase event IDs,
    /// sorted parent lists, and ascending derived subject IDs.
    pub fn encode(&self) -> String {
        let mut output = format!(
            "{LINEAGE_RECEIPT_SCHEMA}\nrecords,{}\n{COLUMNS}\n",
            self.assignments.len()
        );

        for assignment in &self.assignments {
            let birth = &assignment.birth_key;
            let parents = if birth.parent_subject_ids().is_empty() {
                "-".to_string()
            } else {
                birth
                    .parent_subject_ids()
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(":")
            };
            output.push_str(&format!(
                "{},{},{:032x},{},{}\n",
                birth.world_namespace(),
                parents,
                birth.birth_event_id().raw(),
                birth.offspring_slot(),
                assignment.subject_id.raw()
            ));
        }
        output
    }

    /// Strictly parse a canonical receipt. Non-canonical but semantically
    /// equivalent encodings are rejected so one assignment set has one byte
    /// representation and one stable digest.
    pub fn decode(input: &str) -> Result<Self, LineageReceiptError> {
        if input.contains('\r') {
            return Err(LineageReceiptError::NonCanonicalEncoding);
        }

        let mut lines = input.split_terminator('\n');
        if lines.next() != Some(LINEAGE_RECEIPT_SCHEMA) {
            return Err(LineageReceiptError::BadSchema);
        }

        let count_line = lines.next().ok_or(LineageReceiptError::BadRecordCount)?;
        let count_text = count_line
            .strip_prefix("records,")
            .ok_or(LineageReceiptError::BadRecordCount)?;
        let declared_count = count_text
            .parse::<usize>()
            .map_err(|_| LineageReceiptError::BadRecordCount)?;
        if declared_count.to_string() != count_text {
            return Err(LineageReceiptError::NonCanonicalEncoding);
        }
        if declared_count > MAX_RECEIPT_ROWS {
            return Err(LineageReceiptError::TooManyRows);
        }

        if lines.next() != Some(COLUMNS) {
            return Err(LineageReceiptError::BadColumns);
        }

        let mut assignments = Vec::with_capacity(declared_count);
        for (index, row) in lines.enumerate() {
            let line_number = index + 4;
            if assignments.len() >= MAX_RECEIPT_ROWS {
                return Err(LineageReceiptError::TooManyRows);
            }
            if row.is_empty() {
                return Err(LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "empty row".to_string(),
                });
            }
            let fields = row.split(',').collect::<Vec<_>>();
            if fields.len() != 5 {
                return Err(LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "expected exactly five comma-separated fields".to_string(),
                });
            }

            let world_namespace = parse_decimal_u64(fields[0], line_number, "world_namespace")?;
            let parent_ids = if fields[1] == "-" {
                Vec::new()
            } else {
                let mut parsed = Vec::new();
                for parent in fields[1].split(':') {
                    parsed.push(parse_decimal_u64(parent, line_number, "parent_subject_id")?);
                }
                parsed
            };

            if fields[2].len() != 32
                || !fields[2]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "semantic_birth_event_id must be 32 lowercase hexadecimal digits"
                        .to_string(),
                });
            }
            let event_value = u128::from_str_radix(fields[2], 16).map_err(|_| {
                LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "invalid semantic_birth_event_id".to_string(),
                }
            })?;
            let event_id = SemanticEventId::new(event_value).ok_or_else(|| {
                LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "semantic_birth_event_id zero is reserved".to_string(),
                }
            })?;

            let offspring_slot = parse_decimal_u32(fields[3], line_number, "offspring_slot")?;
            let subject_raw = parse_decimal_u64(fields[4], line_number, "subject_id")?;
            let subject_id = SubjectId::from_nonzero_raw(subject_raw).ok_or_else(|| {
                LineageReceiptError::InvalidRow {
                    line: line_number,
                    reason: "subject_id zero is reserved".to_string(),
                }
            })?;

            let birth_key = BirthKey::new(
                world_namespace,
                parent_ids
                    .into_iter()
                    .map(|raw| {
                        SubjectId::from_nonzero_raw(raw).ok_or_else(|| {
                            LineageReceiptError::InvalidRow {
                                line: line_number,
                                reason: "parent subject ID zero is reserved".to_string(),
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                event_id,
                offspring_slot,
            )
            .map_err(|error| LineageReceiptError::InvalidRow {
                line: line_number,
                reason: format!("{error:?}"),
            })?;

            assignments.push(LineageAssignment {
                birth_key,
                subject_id,
            });
        }

        if assignments.len() != declared_count {
            return Err(LineageReceiptError::BadRecordCount);
        }

        let receipt = Self::from_assignments(assignments)?;
        if receipt.encode() != input {
            return Err(LineageReceiptError::NonCanonicalEncoding);
        }
        Ok(receipt)
    }

    /// Re-derive every subject ID and reserve the model's static identities.
    /// Imported receipts are data, not authority: the model must validate them
    /// against the same lineage algorithm and a complete static-ID reservation.
    pub fn validate(
        &self,
        static_subjects: impl IntoIterator<Item = SubjectId>,
    ) -> Result<(), LineageReceiptError> {
        let mut registry = LineageIdRegistry::default();
        for subject in static_subjects {
            registry.reserve_static_subject(subject)?;
        }

        for assignment in &self.assignments {
            let derived = registry.resolve(assignment.birth_key.clone())?;
            if derived != assignment.subject_id {
                return Err(LineageReceiptError::DerivedSubjectMismatch {
                    recorded: assignment.subject_id,
                    derived,
                });
            }
        }
        Ok(())
    }
}

fn parse_decimal_u64(value: &str, line: usize, field: &str) -> Result<u64, LineageReceiptError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| LineageReceiptError::InvalidRow {
            line,
            reason: format!("{field} must be an unsigned decimal integer"),
        })?;
    if parsed.to_string() != value {
        return Err(LineageReceiptError::InvalidRow {
            line,
            reason: format!("{field} is not in canonical decimal form"),
        });
    }
    Ok(parsed)
}

fn parse_decimal_u32(value: &str, line: usize, field: &str) -> Result<u32, LineageReceiptError> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| LineageReceiptError::InvalidRow {
            line,
            reason: format!("{field} must be an unsigned decimal integer"),
        })?;
    if parsed.to_string() != value {
        return Err(LineageReceiptError::InvalidRow {
            line,
            reason: format!("{field} is not in canonical decimal form"),
        });
    }
    Ok(parsed)
}

/// Compare two scenario receipts by complete BirthKey, not merely by output
/// SubjectId. Different births sharing an ID are a collision, not a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageReceiptSummary {
    pub reference_entities: usize,
    pub counterfactual_entities: usize,
    pub matched_births: usize,
    pub reference_only_births: usize,
    pub counterfactual_only_births: usize,
}

pub fn reconcile_lineage_receipts(
    reference: &LineageReceipt,
    counterfactual: &LineageReceipt,
    static_subjects: &[SubjectId],
) -> Result<LineageReceiptSummary, LineageReceiptError> {
    reference.validate(static_subjects.iter().copied())?;
    counterfactual.validate(static_subjects.iter().copied())?;

    // Merge both scenario assignments into one registry. This detects a
    // dynamic-ID collision that appears only across separate scenario files.
    let mut union_registry = LineageIdRegistry::default();
    for subject in static_subjects {
        union_registry.reserve_static_subject(*subject)?;
    }
    for assignment in reference
        .assignments
        .iter()
        .chain(&counterfactual.assignments)
    {
        let resolved = union_registry.resolve(assignment.birth_key.clone())?;
        if resolved != assignment.subject_id {
            return Err(LineageReceiptError::DerivedSubjectMismatch {
                recorded: assignment.subject_id,
                derived: resolved,
            });
        }
    }

    let reference_map = reference
        .assignments
        .iter()
        .map(|assignment| (assignment.birth_key.clone(), assignment.subject_id))
        .collect::<HashMap<_, _>>();
    let counterfactual_map = counterfactual
        .assignments
        .iter()
        .map(|assignment| (assignment.birth_key.clone(), assignment.subject_id))
        .collect::<HashMap<_, _>>();

    let mut matched = 0usize;
    let mut reference_only = 0usize;
    for (birth, reference_subject) in &reference_map {
        match counterfactual_map.get(birth) {
            Some(counterfactual_subject) => {
                if reference_subject != counterfactual_subject {
                    return Err(LineageReceiptError::BirthSubjectMismatchAcrossScenarios {
                        reference: *reference_subject,
                        counterfactual: *counterfactual_subject,
                    });
                }
                matched += 1;
            }
            None => reference_only += 1,
        }
    }

    Ok(LineageReceiptSummary {
        reference_entities: reference_map.len(),
        counterfactual_entities: counterfactual_map.len(),
        matched_births: matched,
        reference_only_births: reference_only,
        counterfactual_only_births: counterfactual_map.len() - matched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_lineage::BirthKey;

    fn subject(raw: u64) -> SubjectId {
        SubjectId::from_nonzero_raw(raw).unwrap()
    }

    fn birth(namespace: u64, parents: &[u64], event: u128, slot: u32) -> BirthKey {
        BirthKey::new(
            namespace,
            parents.iter().copied().map(subject),
            SemanticEventId::new(event).unwrap(),
            slot,
        )
        .unwrap()
    }

    fn receipt(keys: Vec<BirthKey>) -> LineageReceipt {
        let mut registry = LineageIdRegistry::default();
        for key in keys {
            registry.resolve(key).unwrap();
        }
        LineageReceipt::from_registry(&registry).unwrap()
    }

    #[test]
    fn receipt_round_trips_to_one_canonical_byte_representation() {
        let a = birth(7, &[11, 3], 0xabc, 2);
        let b = birth(7, &[99], 0xdef, 5);
        let left = receipt(vec![a.clone(), b.clone()]);
        let right = receipt(vec![b, a]);
        assert_eq!(left, right);
        let encoded = left.encode();
        assert_eq!(LineageReceipt::decode(&encoded).unwrap(), left);
        assert_eq!(LineageReceipt::decode(&encoded).unwrap().encode(), encoded);
    }

    #[test]
    fn receipt_is_independent_of_registry_insertion_order() {
        let a = birth(7, &[11, 3], 0xabc, 2);
        let b = birth(7, &[99], 0xdef, 5);
        let left = receipt(vec![a.clone(), b.clone()]);
        let right = receipt(vec![b, a]);
        assert_eq!(left.encode(), right.encode());
    }

    #[test]
    fn decoder_rejects_noncanonical_row_order_and_truncated_receipts() {
        let value = receipt(vec![
            birth(7, &[11, 3], 0xabc, 2),
            birth(7, &[99], 0xdef, 5),
        ])
        .encode();
        let mut lines = value.lines().map(str::to_string).collect::<Vec<_>>();
        lines.swap(3, 4);
        let reordered = format!("{}\n", lines.join("\n"));
        assert_eq!(
            LineageReceipt::decode(&reordered),
            Err(LineageReceiptError::NonCanonicalEncoding)
        );
        assert_eq!(
            LineageReceipt::decode("luminous.dynamic-entity-lineage-receipt.v1\nrecords,2\n"),
            Err(LineageReceiptError::BadColumns)
        );
    }

    #[test]
    fn imported_subject_ids_are_rederived_not_trusted() {
        let value = receipt(vec![birth(7, &[11, 3], 0xabc, 2)]);
        let mut lines = value
            .encode()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let row_index = 3;
        let recorded = lines[row_index]
            .rsplit(',')
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        lines[row_index] = format!(
            "{},{}",
            lines[row_index].rsplitn(2, ',').nth(1).unwrap(),
            recorded.wrapping_add(1)
        );
        let tampered = format!("{}\n", lines.join("\n"));
        let parsed = LineageReceipt::decode(&tampered).unwrap();
        assert!(matches!(
            parsed.validate([]),
            Err(LineageReceiptError::DerivedSubjectMismatch { .. })
        ));
    }

    #[test]
    fn static_subject_reservations_are_checked_on_import() {
        let value = receipt(vec![birth(7, &[11, 3], 0xabc, 2)]);
        let dynamic = value.assignments()[0].subject_id;
        assert!(matches!(
            value.validate([dynamic]),
            Err(LineageReceiptError::Identity(
                LineageIdentityError::DerivedSubjectConflictsWithStatic { .. }
            ))
        ));
    }

    #[test]
    fn reconciliation_reports_shared_and_conditional_births() {
        let common = birth(7, &[11, 3], 0xabc, 2);
        let reference_only = birth(7, &[11], 0xdef, 0);
        let counterfactual_only = birth(7, &[12], 0xdef, 0);
        let reference = receipt(vec![common.clone(), reference_only]);
        let counterfactual = receipt(vec![common, counterfactual_only]);

        let summary = reconcile_lineage_receipts(&reference, &counterfactual, &[]).unwrap();
        assert_eq!(
            summary,
            LineageReceiptSummary {
                reference_entities: 2,
                counterfactual_entities: 2,
                matched_births: 1,
                reference_only_births: 1,
                counterfactual_only_births: 1,
            }
        );
    }

    #[test]
    fn distinct_birth_keys_are_never_matched_by_subject_id_alone() {
        let a = birth(7, &[11, 3], 0xabc, 2);
        let b = birth(7, &[12, 3], 0xabc, 2);
        let left = receipt(vec![a]);
        let right = receipt(vec![b]);
        let result = reconcile_lineage_receipts(&left, &right, &[]);
        // Distinct birth identities should remain two unmatched entries.
        assert!(matches!(
            result,
            Ok(LineageReceiptSummary {
                matched_births: 0,
                reference_only_births: 1,
                counterfactual_only_births: 1,
                ..
            })
        ));
    }

    #[test]
    fn duplicate_assignments_and_record_count_tampering_are_rejected() {
        let value = receipt(vec![birth(7, &[11, 3], 0xabc, 2)]);
        let assignment = value.assignments()[0].clone();
        assert_eq!(
            LineageReceipt::from_assignments(vec![assignment.clone(), assignment]),
            Err(LineageReceiptError::DuplicateBirthKey)
        );
        let mut text = value.encode();
        text = text.replacen("records,1", "records,2", 1);
        assert_eq!(
            LineageReceipt::decode(&text),
            Err(LineageReceiptError::BadRecordCount)
        );
    }

    #[test]
    fn parentless_births_have_a_canonical_representation() {
        let value = receipt(vec![birth(7, &[], 0xabc, 2)]);
        assert!(value.encode().contains(", -,") == false);
        assert!(value.encode().contains(",-,"));
        assert_eq!(LineageReceipt::decode(&value.encode()).unwrap(), value);
    }
}
