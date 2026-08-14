# Soundness case

## Claim boundary

The initial assurance claim is limited: for supported declared inputs and enabled V1 rules, the emitted result and evidence conform to the versioned rule specification and deterministic numeric profile. This is not a claim that a trajectory is safe, dynamically feasible, optimal, or suitable for flight.

## Assurance argument

1. Complete input identity: raw data, manifest, rules, limits, versions, and numeric profile are length-delimited into the analysis digest.
2. Complete obligation generation: the rule registry is closed for a release and the report records every enabled rule as passed, finding, refused, or error.
3. Sound checking: mathematical primitives are tested against independent rotation-matrix and high-precision reference oracles where applicable.
4. Fail-closed aggregation: any required rule refusal or error prevents `pass`.
5. Governed specification: semantic changes require a report or rule-set version change, traceability update, conformance fixtures, and release review.
6. Fresh-path equivalence: any future cache must match clean analysis by report digest or be discarded.

## Mathematical obligations

For non-zero quaternion `q`, normalisation is `q / ||q||`. Physical distance between unit quaternions `p` and `q` is `2 acos(clamp(abs(dot(p,q)), 0, 1))`. The deterministic lift chooses the sign of each next sample that maximises the dot product with the previously lifted sample. At an exact or tolerance-defined tie (`|p · q| <= 1e-12` in `quatopsy.numeric/1`), it retains the raw sign and emits near-pi ambiguity rather than claiming a unique lift. `QAT-SIGN-001` reports raw consecutive unit samples with negative dot product, which is independent of whether the already-lifted predecessor still required a flip.

A sign-only repair must satisfy `R(q_raw) = R(q_repaired)` within the independent matrix oracle tolerance at every repaired sample. Derived angular rate for interval `dt > 0` is quotient angle divided by `dt`. Dynamic interpretation beyond this kinematic quantity requires an explicitly supplied model and is outside V1.

## Trusted computing base ledger

| Element | Role | Independence limitation |
| --- | --- | --- |
| Parser and manifest validator | Establish declared sequence | Shares data model with kernel |
| Quaternion primitives | Norm, dot, product, matrix conversion | Primary implementation under test |
| Rule registry and aggregator | Completeness and result state | Must not share fixture generation logic |
| Canonical serializer | Stable evidence | Does not validate mathematical truth |
| Reference oracle suite | Cross-check primitives and repairs | Must use independently encoded matrix or high-precision formulas |

The viewer is not an oracle. Visual agreement is supporting evidence only.

## Unsupported cases

Zero quaternions, missing convention declarations, unordered time, non-finite values, discontinuous clock domains, intentionally multi-turn commands without a declared command path, and dynamics-dependent cost claims without a model are refused or bounded to narrower kinematic findings.

