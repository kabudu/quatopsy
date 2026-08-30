# Frozen public claims

Status date: 2026-08-30. Version: `0.1.0`. Visibility: private research repository.

## Permitted statements

- Quatopsy is a local-first, advisory quaternion trajectory linter plus a non-authoritative static viewer.
- Quatopsy may be described as early-stage, production-quality research software for local advisory evaluation. "Production-quality" describes engineering discipline within the declared scope; it does not claim production readiness, certification, independent validation, a support SLA, or flight suitability.
- For declared `quatopsy.manifest/1` inputs and enabled V1 rules, reports follow `quatopsy.report/1` and the deterministic numeric profile `quatopsy.numeric/1`.
- Supported rules are `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, `QAT-PI-001`, `QAT-REPAIR-001`, `QAT-UNWIND-001` when commanded columns are declared, `QAT-CONV-001` when rotation-matrix columns are declared, and `QAT-OMEGA-001` when angular-velocity columns are declared.
- Sign-lift repair candidates preserve represented orientation under the independent rotation-matrix oracle used in tests.
- One million synthetic identity samples meet the documented time and RSS budget on the local CI host.
- Local checksum packaging is available via `scripts/package-local.sh`.
- `quatopsy adapt` converts IDS Jason-1 ASCII, ROS JSON, uncompressed MCAP JSON poses, SPICE CK type 3, and TUBIN star-tracker CSV into canonical CSV plus manifest. Adapters never assign report `result` values.
- `--policy advisory|selective|required` and `--override-file` change process exit only.

## Unreleased M6

- `quatopsy plan` emits a torque-limited rest-to-rest candidate plus `quatopsy.plan/1`. Closed-form eigenaxis and bounded nonlinear direct-shooting paths are available. Residuals are computed by an independent oracle, including stored momentum and keep-out cones. Optional actuator models, weighted objectives, and perturbation campaigns never assign report `result`. A subsequent `analyze` owns the only verdict. Optimality is not claimed. Gyroscopic torque or actuator limits that cannot be met are infeasible.

## Unreleased M7

- `quatopsy control` runs a geometric PD controller on SO(3) plus `quatopsy.control/1`. Attitude error is the rotation-matrix vee map. An independent oracle monitor inhibits commands. Estimator freshness, saturation, anti-windup, momentum dump, modes, deterministic SIL campaigns, and declared software plant models (command-to-torque lag, gyro ARW, star-tracker delay, magnetic residual, gravity-gradient) never assign report `result`. `execution` may be `sil`, host-CPU `pil`, or loopback `hil`. Hard real-time, physical actuators, qualified processors, and hardware command are refused.

## Unreleased M8

- `quatopsy control` may run a software GN&C plane: a 6-state MEKF or UKF with χ² outlier rejection and NIS/NEES audit, time-tagged guidance profiles, reference-tracking geometric PD, reaction-wheel allocation, and a declared two-body geometry source. `nav.json` and `guidance.json` never assign report `result`. This is not orbit determination, not hard real-time, not physical hardware, and not a certification artefact.

## Unreleased M9

- Canonical Quatopsy marks, tokens, and lockups exist as `quatopsy.brand/2`. The viewer uses the woven-lift ribbon symbol. The CLI about line carries the tagline. This is not trademark clearance and not public opening.

## Unreleased M10

- `quatopsy investigate` creates a bounded local incident evidence directory from copied canonical or adapter-supported telemetry, optional uninterpreted context, canonical diagnostics, reproducers, repairs, static viewers, and optional plan/control candidates that receive separate kernel reports. `quatopsy.evidence/1` binds artifact paths, roles, sizes, and SHA-256 digests. `quatopsy verify-evidence` detects later bundle mutation within that contract.
- The evidence manifest is not authenticated custody, a mission archive standard, proof that a logged command executed, flight approval, or permission to send a command. Bundles can contain sensitive copied data and receive no automatic redaction, encryption, upload, or retention management.

## Unreleased M11

- The product-led README, branded system architecture, community-health files, and GitHub About metadata are prepared and validated while the repository remains private. They improve discoverability and contribution readiness but do not imply public availability, hosted CI, package publication, trademark clearance, production support, or independent validation.

## Required non-claims

Do not state or imply that Quatopsy is novel, safe, flight-proven, certified, production-ready, complete, optimal, or independently validated. Do not state that a `pass` result is flight approval, actuator permission, or energy optimality. Do not state that a feasible plan or a tracked SIL candidate is globally optimal, dynamically verified under uncertainty, or authorised for command. Do not state that commanded-path findings measure control effort or mission risk. Do not state that the candidate name is a cleared trademark. Do not state that the TUBIN excerpt is a mission reconstruction or that adapters certify source conventions.

## Out of V1 supported scope

- Automatic convention inference and automatic convention repair remain refused.
- Hardware command, hard real-time execution, target-processor PIL, physical-actuator HIL, and flight assurance remain refused. Host-CPU PIL and loopback HIL are software evidence only. The controller does not open actuator devices.
- Hosted CI, crates.io publication, signed binaries, public repository visibility, websites, and production support remain distinct unauthorised gates.
- Full visual brand assets exist as `quatopsy.brand/2`. They do not authorise public opening, trademark claims, or production support.

## Evidence map

Behavioural evidence lives in the conformance fixtures, hostile and lifecycle tests, M10 investigation and tamper tests, million-sample release check, local CI log, and this claims freeze. Optional independent reproduction does not block this release and is not claimed.
