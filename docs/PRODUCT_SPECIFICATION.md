# Product specification

## Problem

Orientation samples can be physically equivalent while differing as four-component quaternions. Incomplete handling of unit norm, antipodal equivalence, interpolation choice, timestamps, component order, multiplication order, active versus passive rotation, and frame direction can produce discontinuities, excessive motion, or misleading plots. Existing libraries expose primitives and existing viewers replay motion, but an engineer must still assemble the diagnostic argument.

## Users and first vertical

The first user is a spacecraft guidance, navigation, and control engineer analysing a planned or recorded attitude sequence offline. Secondary future users are robotics, simulation, computer-graphics, and animation engineers. Quatopsy is advisory and never commands an actuator.

## Input profile V1

A UTF-8 CSV file plus an explicit manifest provides monotonically increasing timestamps, four finite quaternion components, component order, rotation sense, frame relationship, time unit, and optional observed angular velocity. The input byte digest, manifest digest, tool version, rule-set version, and numeric policy form the analysis identity.

Unsupported ambiguity is refused. Quatopsy must not guess component order, reference frames, units, or active/passive semantics.

## Outputs

The command produces a versioned JSON report, a concise terminal summary, optional repaired CSV candidates, and a self-contained local visualisation bundle. Each finding includes stable rule ID, severity, sample interval, evidence values, interpretation, confidence class, and applicable repair proposals. Original inputs are never modified.

## Result semantics

- `pass`: every enabled release-critical obligation was evaluated and satisfied.
- `findings`: evaluation completed and one or more obligations were violated.
- `refused`: declared semantics or input values are unsupported or ambiguous.
- `error`: the analysis could not complete. An error never becomes a pass.

Findings are classified as `invalid-data`, `representation-discontinuity`, `physical-discontinuity`, `convention-mismatch`, `dynamic-threshold`, or `informational`. Representation discontinuity alone does not prove undesirable physical motion.

## Initial obligations

| Rule | Behaviour | V1 disposition |
| --- | --- | --- |
| QAT-NORM-001 | Detect non-finite, zero, and off-unit samples under explicit tolerances | Required |
| QAT-TIME-001 | Detect duplicate, decreasing, and non-finite timestamps | Required |
| QAT-LIFT-001 | Construct the minimum adjacent-distance lift in `S^3`, with deterministic tie handling | Required |
| QAT-SIGN-001 | Report raw antipodal sign discontinuities separately from physical motion | Required |
| QAT-RATE-001 | Derive quotient-invariant inter-sample angles and angular rates | Required |
| QAT-PI-001 | Identify numerically ambiguous near-pi intervals | Required |
| QAT-CONV-001 | Validate declared convention against optional redundant evidence or fixtures | Limited |
| QAT-REPAIR-001 | Emit sign-continuity and normalisation repair candidates with provenance | Required |
| QAT-UNWIND-001 | Compare a supplied commanded path with the quotient-shortest baseline | Planned after V1 kernel |

## Repairs

Repairs are proposals, not automatic corrections. Each repair records changed rows, preconditions, algorithm version, numeric tolerance, source digest, output digest, and semantic effects. Sign-lift repair may change quaternion representation without changing represented orientation. Normalisation repair changes invalid numeric samples and must report magnitude of change. Quatopsy will not infer or repair frame conventions automatically in V1.

## UX invariants

The physical 3D view, projected `S^3` view, timeline, and finding selection refer to the same sample identity. Colour is never the sole carrier of result state. The UI distinguishes measured data, derived data, and proposed repairs. Projection artefacts are labelled and cannot be presented as physical paths.

## Success measures

V1 succeeds when a hand-audited conformance suite proves deterministic detection and refusal semantics, every supplied defect fixture yields the expected rule and interval, clean trajectories do not receive release-critical false findings, repaired sign lifts preserve rotation matrices within tolerance, and a report reproduces byte-for-byte across supported platforms under the deterministic numeric profile.

## Non-goals

Quatopsy V1 is not a controller, trajectory optimiser, simulator, sensor-fusion filter, collision planner, certification tool, live mission-control system, proof of safe motion, energy estimator without a supplied dynamics model, or quaternion learning laboratory.

