# Protective Engineering Benchmark (PEB)

**Research draft v0.1 — 2026-11-15**

Protective Engineering asks whether a system preserves its declared mission, safety, authority, integrity, and recovery properties under hostile inputs, component faults, compromised identities, manipulated observations, partial failure, and restoration.

This work complements existing cybersecurity and safety standards. It is not a certification or a claim of SOTA performance.

## Core proposition

Do not measure only whether controls are installed or attacks are detected. Measure whether critical invariants remain true, whether impact is bounded, whether safe mission continuity is preserved, whether recovery restores trust, and whether independent evidence supports the claim.

`Mission/invariants → authority → execution → observation → independent verification → containment/recovery`

## Seven-plane reference model

1. **Mission and invariants:** define observable critical properties, hazard assumptions, owners, and allowed degraded states.
2. **Identity and authority:** scoped, short-lived, revocable capabilities; explicit delegation; replay resistance; least privilege.
3. **Planning and decision:** treat model/agent outputs and retrieved content as untrusted proposals, never as self-authorizing instructions.
4. **Execution and safety kernel:** deterministic enforcement independent of the planner; preconditions, safety envelopes, rate limits, idempotency, and interlocks.
5. **Observation and telemetry:** provenance, freshness, calibration, clock source, plausibility, and independent corroboration.
6. **Independent evidence:** bind exact source, build, configuration, policy decision, operation, observation, verifier, scenario, and digests.
7. **Containment and recovery:** bound blast radius; use hazard-derived degraded modes; reconcile unknown outcomes; restore service and trust before resuming consequential authority.

## Domain profiles

- **PE-Core:** resource/state invariants, identity, capability scope, transactions, provenance, and evidence verification.
- **PE-Agent:** goal and memory integrity, tool boundaries, delegated authority, inter-agent communication, and cascading failures.
- **PE-OT:** industrial control, energy, maritime, water, rail, building automation, and robotics; safety envelopes and hazard-derived degraded modes.
- **PE-Infrastructure:** identity, remote management, segmentation, cloud control planes, updates, builds, and common dependencies.
- **PE-Recovery:** clean restore, key rotation, state reconciliation, replay prevention, backup validation, and return-to-service evidence.

## Scoring principle: hard gates plus a Pareto profile

A P0 invariant violation fails the relevant profile. It cannot be offset by throughput, detection rate, or a weighted aggregate. Report per-dimension results: invariant preservation, authority integrity, detection quality, containment, mission continuity, recovery of trust, evidence integrity, privacy/human impact, and performance/cost.

There is no universal safe state. In particular, OT “fail closed” behavior must be derived from the hazard analysis; simply removing power is not inherently safe.

## Initial public work

- [Issue #42 — energy boundary input and ledger invariants](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/issues/42)
- [Draft PR #43 — energy boundary hardening](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/pull/43)
- [Issue #44 — state-machine and fault-injection qualification](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/issues/44)
- [Issue #45 — PE-Core benchmark and evidence receipts](https://github.com/Tristan-Stoltz-ERC/symtropy-consciousness-physics/issues/45)

Existing repository work on content-addressed compiler artifact passports, event-addressed randomness, paired inference, dynamic entity lineage, and deterministic reductions can support reproducibility. It does not by itself establish security, physical safety, or causal validity.

## Qualification boundaries

- Scenarios begin in synthetic fixtures and isolated simulators; no disruptive production tests.
- Include positive and negative controls so overblocking is visible.
- Record exact source/build/configuration, scenario version, seed, raw trace, and verifier identity.
- No missing, skipped, stale, or inconclusive evidence is treated as a pass.
- Targets are hypotheses until preregistered and independently evaluated.
- Publish failures, minimized reproductions, exclusions, and limitations.

See [benchmark methodology](./benchmark-v0.1.md) and [scenario catalogue](./scenarios-v0.1.md).
