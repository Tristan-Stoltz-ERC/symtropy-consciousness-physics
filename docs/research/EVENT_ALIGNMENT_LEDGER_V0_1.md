# Counterfactual event-alignment ledger v0.1

## Purpose

The event-keyed RNG maps a semantic event address to a deterministic random word. The alignment ledger audits whether two scenario traces actually contain the same event/draw keys and whether the corresponding words match.

It does not decide that two differently named events are scientifically the same event. Event identity remains a model-owner decision.

## Contract

Each record is \`(EventAddress, draw_index, output_word)\`. Scenario identity is supplied by which input collection contains the record; it is not part of the random key.

The comparison:
- rejects empty traces;
- rejects duplicate (address, draw-index) records within either scenario;
- matches only exact event-address and draw-index equality;
- reports matched, reference-only and counterfactual-only events;
- counts bit-identical versus mismatched output words among matched keys;
- treats equal output words from different event keys as unrelated;
- returns deterministic integer counts, independent of input row order.

\`fully_aligned\` means both traces have identical key sets and every matched output word is identical. It does not mean the scientific model is valid. \`matched_fraction_of_smaller_scenario\` is a key coverage metric, not a quality or causal-validity score.

## Known-effect fixture

The module tests a structural causal toy model with 32 stable agent identities. Both scenarios share an exogenous event value. The treatment structurally adds exactly 0.25 to each outcome. The treatment scenario also requests one additional branch-only event per agent.

The test requires:
- exact recovery of the known additive effect;
- all 32 shared event keys matched;
- all 32 shared output words identical;
- 32 treatment-only events accounted for;
- no false matching based on coincidentally equal output words.

This is a software fixture for the mapping and ledger, not a claim that a real Symtropy experiment has passed causal validation.

## Explicit limits

- It does not validate event semantics or stable identity allocation.
- It does not measure the proportion of modeled events that ought to be shared.
- It does not correct an incorrectly specified event mapping.
- It does not replace independent-seed replication or paired inference.
- No historical experiment or RNG call site is changed.
