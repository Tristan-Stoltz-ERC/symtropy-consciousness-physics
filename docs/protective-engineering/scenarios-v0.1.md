# Scenario catalogue v0.1

**Status:** 18 proposed scenarios; none executed by this catalogue.

Start with synthetic fixtures and isolated simulators. Any OT/physical execution requires domain hazard review, explicit authorization, and an approved safe test range.

| Scenario ID | Profile | Severity | Failure/attack family | Critical property |
|---|---|---|---|---|
| PE-CORE-001 | PE-Core | P0 | Negative/NaN/infinite resource inputs and overflow | Resource state remains finite and within capacity; invalid transitions are atomic. |
| PE-CORE-002 | PE-Core | P0 | Expired or replayed capability | Consumed, expired, or revoked authority cannot be reused. |
| PE-CORE-003 | PE-Core | P0 | Privilege or scope escalation | Effective authority never exceeds the grant and delegation chain. |
| PE-AGENT-001 | PE-Agent | P1 | Indirect prompt injection / goal hijack | Untrusted content cannot amend policy or authorize side effects. |
| PE-AGENT-002 | PE-Agent | P1 | Memory and retrieval poisoning | Memory provenance, freshness, and ownership remain explicit; memory is not authority. |
| PE-AGENT-003 | PE-Agent | P0 | Confused deputy / delegated tool misuse | Tool use is authorized against the initiating principal and bounded delegation chain. |
| PE-AGENT-004 | PE-Agent | P1 | Inter-agent spoofing, duplicate events, cascading retries | Peer identity is verified and fan-out/retry amplification remains bounded. |
| PE-OT-001 | PE-OT | P0 | Out-of-envelope actuator command | Independent safety enforcement blocks unsafe actuation. |
| PE-OT-002 | PE-OT | P0 | Sensor poisoning, replay, and time-base divergence | Stale, replayed, implausible, or common-cause corrupted observations are not trusted as current truth. |
| PE-OT-003 | PE-OT | P0 | Power or communications loss | Hazard-derived safe degraded behavior is preserved; power-off is not assumed universally safe. |
| PE-INFRA-001 | PE-Infrastructure | P0 | Build provenance/dependency substitution | Only verified source/build/dependency artifacts cross deployment boundary. |
| PE-INFRA-002 | PE-Infrastructure | P0 | Remote-management lateral movement | A compromised low-privilege service cannot pivot into protected management/control assets. |
| PE-INFRA-003 | PE-Infrastructure | P0 | Shared dependency/control-plane cascade | Common-cause failure does not defeat local safety or produce unbounded propagation. |
| PE-RECOVERY-001 | PE-Recovery | P0 | Stale or contaminated backup restore | Consequential authority stays disabled until artifacts, identity, config, and event history are reconciled. |
| PE-RECOVERY-002 | PE-Recovery | P0 | Unknown transaction outcome / duplicate retry | Reconcile authoritative operation state before retrying consequential side effects. |
| PE-EVIDENCE-001 | PE-Core | P0 | Evidence producer/verifier compromise | Tampered evidence is rejected; common-cause trust dependencies are disclosed. |
| PE-OT-004 | PE-OT | P0 | Emergency override misuse and operator coercion | Emergency authority is bounded, attributable, reviewable, and cannot erase evidence. |
| PE-AGENT-005 | PE-Agent | P1 | Sensitive-data leakage through logs/retrieval | Disclosure remains limited to authorized principal, purpose, and tenant. |

## Per-scenario minimum record

Every implemented scenario must define: ID/version, profile, severity, attacker tier, preconditions, injected fault(s), invariant(s), expected response, benign negative control, measured metrics, evidence artifacts, reset procedure, and known limitations.

## Required negative controls

A system that blocks every request is not a successful protective system. Every scenario must include a legitimate operation that should continue to work. Report false rejection, unnecessary degraded mode, operator burden, and mission-utility loss alongside attack-blocking efficacy.

## Initial implementation priority

1. PE-CORE-001: resource-boundary property tests (start with the energy reservoir).
2. PE-CORE-002/003: capability scope, revocation, expiry, and replay fixtures.
3. PE-EVIDENCE-001: receipt tamper controls and verifier independence.
4. PE-RECOVERY-002: ambiguous result and idempotent reconciliation.
5. PE-AGENT and PE-OT profiles in isolated simulator fixtures.

The JSON schema and full machine-readable scenario objects are included in the accompanying Protective Engineering v0.1 package; this Markdown catalogue is the public overview.
