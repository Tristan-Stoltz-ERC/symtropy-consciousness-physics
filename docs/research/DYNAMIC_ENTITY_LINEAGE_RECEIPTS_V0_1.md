# Dynamic entity lineage receipts v0.1

## Purpose

PR #26 derives dynamic subject IDs from canonical BirthKeys and detects collisions within a LineageIdRegistry. This contribution makes those assignments portable across independent scenario executions and processes.

A registry is not evidence until its full BirthKey-to-SubjectId mapping can be exported, hashed, imported, re-derived, and reconciled. Comparing only compact IDs is unsafe: a matching number does not prove matching birth provenance.

## Canonical receipt format

The format is dependency-free, line-oriented UTF-8:

1. Fixed schema identifier.
2. Declared record count.
3. Fixed column header.
4. Rows sorted by ascending SubjectId.

Each row contains world namespace, sorted parent IDs, 32-digit lowercase hexadecimal semantic birth-event ID, offspring slot, and derived SubjectId. Parentless births use a single hyphen. All numeric fields have canonical decimal or fixed-width lowercase hexadecimal encodings.

The decoder rejects noncanonical row ordering, duplicate BirthKeys, duplicate subject IDs, malformed numbers, count mismatches, CRLF, and alternate encodings. One logical assignment set therefore has one byte representation and one stable external digest.

## Validation

An imported receipt is untrusted data. Validation:
- reserves the complete set of static/global/pair subject IDs supplied by the model;
- recomputes every dynamic SubjectId from its BirthKey using the qualified lineage algorithm;
- rejects any recorded/derived mismatch;
- rejects collisions with static IDs and collisions among distinct births.

The receipt does not independently prove that the supplied parentage or birth event is scientifically correct. That remains a model-owner claim.

## Cross-scenario reconciliation

Reconciliation validates both receipts and then merges their assignments into one collision registry. This is important: distinct BirthKeys that happen to collide across two separate scenario registries must not escape detection.

A birth is matched only when the complete canonical BirthKey exists in both receipts and maps to the same SubjectId. Reference-only and counterfactual-only births are reported separately as unmatched/conditional events. Equal SubjectId values alone are never used as proof of a match.

## Tests

- Canonical byte round-trip and insertion-order invariance.
- Strict row ordering and record-count rejection.
- Tampered subject ID rejected by re-derivation.
- Static subject collision rejected during import.
- Shared, reference-only and counterfactual-only birth counts.
- Different BirthKeys never treated as matched by ID alone.
- Duplicate assignment rejection.
- Parentless birth canonical encoding.

## Limits

- No signing, authentication, or trusted-builder identity is provided.
- External storage should hash the exact encoded receipt and bind that digest to the experiment/run record.
- Cross-process merge is deterministic reconciliation, not distributed consensus.
- The model still defines stable parent identities, semantic birth events, offspring slots, and which namespaces should be shared across alternatives.
- No live Symtropy birth site is migrated in this PR.
