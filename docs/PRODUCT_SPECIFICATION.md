# Product specification

## Problem

Orientation samples can be physically equivalent while differing as four-component quaternions. Incomplete handling of unit norm, antipodal equivalence, interpolation choice, timestamps, component order, multiplication order, active versus passive rotation, and frame direction can produce discontinuities, excessive motion, or misleading plots. Existing libraries expose primitives and existing viewers replay motion, but an engineer must still assemble the diagnostic argument.

## Users and first vertical

The first user is a spacecraft guidance, navigation, and control engineer analysing a planned or recorded attitude sequence offline. Secondary future users are robotics, simulation, computer-graphics, and animation engineers. Quatopsy is advisory. Hardware actuator command remains refused.

## Input profile V1

A UTF-8 CSV file plus an explicit `quatopsy.manifest/1` document provides monotonically increasing timestamps, four quaternion components, component order (`wxyz` or `xyzw`), rotation sense, distinct `frame_from`/`frame_to` names, time unit (`ns`, `us`, `ms`, or `s`), optional angular-velocity columns, and optional commanded quaternion columns in the same component order. Manifest JSON rejects unknown fields. The quaternion column array is in the declared component order and is assembled into internal Hamilton `(w, x, y, z)` storage. The input byte digest, manifest digest, tool version, rule-set version, numeric profile, enabled rules, and limits form the analysis identity.

Unsupported ambiguity is refused. Quatopsy must not guess component order, reference frames, units, or active/passive semantics. Numeric profile `quatopsy.numeric/1` uses absolute unit tolerance `1e-6`, near-zero refusal below `1e-12`, and near-pi lift ties when `|p · q| <= 1e-12` after unit normalisation.

The M1 CLI is `quatopsy analyze --input <csv> --manifest <json> --report <json>`. It writes compact canonical JSON atomically and does not overwrite an existing report unless `--overwrite` is passed. `quatopsy view --report <json> --output <dir>` writes a static local viewer bundle. Optional `--input` and `--manifest` bind derived geometry to the same digests as the report. `quatopsy plan --problem <json> --output-dir <dir>` writes a candidate trajectory for later analysis. `quatopsy control --problem <json> --output-dir <dir>` writes a closed-loop trajectory for later analysis. Software-in-the-loop, host-CPU processor-in-the-loop, and loopback hardware-in-the-loop are supported.

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
| QAT-CONV-001 | Compare declared quaternion to an optional supplied rotation matrix; refuse incomplete matrix evidence; never infer or repair convention | Required when matrix columns are declared |
| QAT-REPAIR-001 | Emit sign-continuity and normalisation repair candidates with provenance | Required |
| QAT-UNWIND-001 | Compare a supplied commanded path with the quotient-shortest baseline | Required when commanded columns are declared |
| QAT-OMEGA-001 | Compare optional body angular-velocity columns with quaternion kinematics | Required when angular-velocity columns are declared |

## Repairs

Repairs are proposals, not automatic corrections. Each repair records changed rows, preconditions, algorithm version, numeric tolerance, source digest, output digest, and semantic effects. Sign-lift repair may change quaternion representation without changing represented orientation. Normalisation repair changes invalid numeric samples and must report magnitude of change. Quatopsy will not infer or repair frame conventions automatically in V1.

## UX invariants

The physical 3D view, projected `S^3` view, timeline, and finding selection refer to the same sample identity. Colour is never the sole carrier of result state. The UI distinguishes measured data, derived data, and proposed repairs. Projection artefacts are labelled and cannot be presented as physical paths.

## Success measures

V1 succeeds when a hand-audited conformance suite proves deterministic detection and refusal semantics, every supplied defect fixture yields the expected rule and interval, clean trajectories do not receive release-critical false findings, repaired sign lifts preserve rotation matrices within tolerance, and a report reproduces byte-for-byte across supported platforms under the deterministic numeric profile.

## Non-goals

Quatopsy V1 is not a general trajectory optimiser, hardware controller, simulator of unstated plants, collision planner, certification tool, live mission-control system, proof of safe motion, energy estimator without a supplied dynamics model, or quaternion learning laboratory. Viewer interaction is bound to report evidence, retained sample identity, and separately labelled repair candidates; it does not provide free-form quaternion construction or tutorial exercises.

M6 adds `quatopsy plan`, an offline candidate generator for a torque-limited rest-to-rest rigid body. Algorithms are eigenaxis bang-coast-bang and bounded multiple shooting. The command writes CSV, a declared manifest, and `quatopsy.plan/1`. Residuals are computed by an independent oracle. It never writes `quatopsy.report/1` `result`.

M7 adds `quatopsy control`, a geometric PD controller on SO(3). Independent oracle inhibition, estimator freshness contracts, fail-closed safe fallback, host-CPU PIL, loopback HIL, and declared software plant models are in-repo. It never writes `quatopsy.report/1` `result`, never opens a physical actuator, and is not a hard-real-time or flight controller. The systems-safety programme cannot qualify hardware.

M8 adds a software GN&C plane: MEKF/UKF attitude navigation with NIS/NEES audit, time-tagged guidance, reference tracking, reaction-wheel allocation, and declared two-body geometry. It is not orbit determination, not a certified estimator, and not permission to command hardware.

M9 adds the enduring product brand `quatopsy.brand/1`: antipodal paired-point mark, tokens, lockups, and local CI validation. It is not trademark clearance and not public opening.
