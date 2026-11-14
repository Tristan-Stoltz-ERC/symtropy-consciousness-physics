# Semantic event identity claim audit v0.1

## Purpose

Philox makes one EventAddress deterministic. The alignment ledger reports which addresses appear in two scenario traces. Neither component can know whether a model has assigned the same address to two different causal events.

This module adds an explicit model-claim audit. It does not infer identity. The model owns the semantic event IDs and the sharing policy.

## Claim contract

Each claim contains:
- a nonzero model-assigned u128 semantic event ID;
- a complete EventAddress;
- one sharing policy.

The semantic ID is an identity token, not a string hash, vector index, wall-clock value, or runtime allocation order. Address comparison includes replicate seed, experiment namespace, subject, tick and packed event kind/purpose/occurrence fields.

## Sharing policies

| Policy | Audit rule |
|---|---|
| MustShare | The same semantic ID and complete address must exist in both alternatives. Missing on either side is a hard error. |
| Conditional | May be present in one or both alternatives. Unmatched claims are counted explicitly and never synthesized into matches. |
| Distinct | Must remain unmatched across alternatives. A semantic ID declared on both sides is a hard error. |

## Fail-closed conditions

- duplicate semantic claims within one scenario;
- one semantic ID mapped to two addresses within a scenario;
- two semantic IDs mapped to one address within a scenario;
- two different semantic IDs mapped to the same address across alternatives;
- one semantic ID mapped to different addresses across alternatives;
- sharing-policy drift across alternatives;
- a missing MustShare event;
- a Distinct event claimed in both alternatives.

The audit is order-independent because identity indexes are keyed and compared using ordered maps.

## Software tests

Tests cover all fail-closed conditions, conditional and distinct event accounting, exact MustShare alignment, zero-ID reservation, and input-order invariance.

## Boundaries

- A successful audit proves internal consistency of declared claims, not that the modeler assigned scientifically correct semantic IDs.
- It does not implement dynamic genealogical IDs or birth-event derivation.
- It does not automatically instrument existing Symtropy simulation call sites.
- It is not a substitute for independent replicate design, matched-pair inference, or causal identification.
- Unknown event identity or sharing policy must block qualification rather than default to MustShare.
