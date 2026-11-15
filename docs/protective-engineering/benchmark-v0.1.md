# Benchmark methodology v0.1

**Status:** proposed; no benchmark measurements have been run.

## 1. Evaluation contract

Each run must declare the system and build identity, source revision, configuration digest, toolchain, dependencies, threat model, attacker capability tier, invariant set, scenario version, seed, initial state, fault schedule, independent measurement method, raw trace, evidence receipt, and evaluator identity.

Use matched baselines, fixed scenario versions, blind holdouts, and preregistered success criteria. Report failures and exclusions as well as successes.

## 2. Hard gates

Any observed P0 invariant violation fails that profile. Examples:

- Unauthorized consequential mutation.
- Actuator command outside the validated hazard envelope.
- Accepted replay of an expired, revoked, or consume-once capability.
- Duplicate side effect while the original transaction outcome is unknown.
- Deliberately modified evidence accepted by the verifier.

No average or weighted score can cancel a P0 failure.

## 3. Pareto dimensions

1. Invariant preservation and time-to-violation.
2. Authority integrity: scope, expiry, revocation, replay, and confused-deputy behavior.
3. Detection quality: precision, recall, alert quality, attribution, and detection time.
4. Containment: affected assets/services, propagation depth, privilege reached, and impact.
5. Mission continuity: critical function and safety margin during degradation.
6. Recovery: service restoration, trust restoration, data loss, unresolved operations, and evidence.
7. Evidence integrity: tamper rejection, provenance completeness, independent rebuilds, and verifier independence.
8. Privacy and human impact: leakage, unnecessary collection, legitimate-operation disruption, operator burden, and appealability.
9. Performance and cost: P50/P95/P99 latency, throughput, compute, memory, energy, and operational effort.

Do not publish a single universal scalar score. Publish hard-gate outcomes and a per-dimension profile.

## 4. Proposed first-gate targets (not achieved results)

| Dimension | Initial target |
|---|---:|
| P0 invariant violations | 0 |
| Unauthorized consequential mutations | 0 |
| Accepted expired/revoked/replayed capabilities | 0 |
| Duplicate side effects after unknown outcome | 0 |
| Deliberately altered evidence receipts accepted | 0 |
| Mandatory evidence field completeness | 100% |
| Generated state-machine operations | 100,000 per profile |
| Deterministic fixture digest mismatches | 0 |

These are research targets and must be justified per domain. RTO/RPO, degraded-mode utility, safety margins, and false-rejection ceilings must be set by mission and hazard analysis, not guessed as universal values.

If zero failures are observed in 100,000 independent, representative Bernoulli trials, the rule-of-three yields an approximate 95% upper bound of 3×10⁻⁵ on the failure probability for that tested distribution. This does not generalize to real-world failure rates when trials are dependent or unrepresentative, the threat model is incomplete, or implementation versions differ. Test volume is not formal proof or safety certification.

## 5. Adversary tiers

- **T0 — malformed input:** negative values, NaN, infinities, bad encodings, missing fields, overflow, impossible states.
- **T1 — accidental fault:** crash, timeout, packet loss, stale data, disk full, restart, and clock skew.
- **T2 — authenticated misuse:** stolen or overbroad credentials, insider misuse, compromised low-privilege workload.
- **T3 — adaptive attacker:** chaining, replay, privilege escalation, memory/context poisoning, and lateral movement.
- **T4 — common-cause/control-plane compromise:** identity provider, build/signing path, orchestration plane, verifier, or shared dependency compromise.

These tiers define test assumptions and are not universal adversary rankings.

## 6. Required controls

- Positive controls: permitted operations should work.
- Negative controls: prohibited, stale, replayed, malformed, or compromised operations should fail.
- Detection controls: verify that injected events occurred and measurement can observe them.
- Verifier controls: mutate receipts and confirm rejection.
- Recovery controls: test known-good restore and contaminated-restore rejection.
- Common-cause controls: compromise shared dependencies to challenge independence assumptions.

## 7. Comparison with current practice

Use established frameworks as coverage anchors rather than pretending one score replaces them:

- [MITRE ATT&CK Evaluations Enterprise 2026](https://attackevals.mitre.org/enterprise/er8): adversary emulation, detection/protection quality, false-positive validation, and source-of-action attribution.
- [OWASP ASVS 5.0.0](https://asvs.owasp.org/): application-security verification requirements.
- [OWASP Top 10 for Agentic Applications 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) and [MITRE ATLAS](https://atlas.mitre.org/): agent/AI threat coverage.
- [NIST AI RMF and GenAI Profile](https://www.nist.gov/itl/ai-risk-management-framework) and [NIST AI 100-2 E2025](https://csrc.nist.gov/pubs/ai/100/2/e2025/final): AI risk and adversarial ML terminology.
- [NIST SP 800-160 Vol. 2 Rev. 1](https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final): cyber-resilience engineering.
- [NIST SP 800-82 Rev. 4 initial public draft](https://csrc.nist.gov/pubs/sp/800/82/r4/ipd) and [ISA/IEC 62443](https://www.isa.org/standards-and-publications/isa-standards/isa-iec-62443-series-of-standards): OT security.
- [SLSA Build track](https://slsa.dev/spec/v1.2/build-track-basics) and [in-toto attestations](https://github.com/in-toto/attestation): build and supply-chain provenance.

MITRE ATT&CK Evaluations Enterprise 2026 provides a particularly useful comparison pattern by combining detection and protection quality and attributing whether action came from a platform, AI-augmented workflow, or human/service. It remains a detection/protection evaluation, not whole-system safety or recovery assurance.

A benchmark superiority claim requires matched scenario versions, access, duration, telemetry, business-activity controls, product tuning, human/AI/service attribution, and success criteria. Publish raw traces, exact configuration, independent evaluation, and negative results. Do not claim “better than SOTA” from internal results alone.

## 8. Result receipt

Every run should emit a receipt that binds exact source/build/configuration, policy and verifier versions, scenario/seed, injected fault schedule, raw trace digest, invariants evaluated, observed violations, metrics, and limitations. A hash is an integrity reference; it is not proof that the original statement is true or that the signer is trustworthy.
