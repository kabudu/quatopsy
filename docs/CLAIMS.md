# Frozen public claims

Status date: 2026-08-15. Version: `0.1.0`. Visibility: private research repository.

## Permitted statements

- Quatopsy is a local-first, advisory quaternion trajectory linter plus a non-authoritative static viewer.
- For declared `quatopsy.manifest/1` inputs and enabled V1 rules, reports follow `quatopsy.report/1` and the deterministic numeric profile `quatopsy.numeric/1`.
- Supported rules are `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, `QAT-PI-001`, `QAT-REPAIR-001`, `QAT-UNWIND-001` when commanded columns are declared, `QAT-CONV-001` when rotation-matrix columns are declared, and `QAT-OMEGA-001` when angular-velocity columns are declared.
- Sign-lift repair candidates preserve represented orientation under the independent rotation-matrix oracle used in tests.
- One million synthetic identity samples meet the documented time and RSS budget on the local CI host.
- Local checksum packaging is available via `scripts/package-local.sh`.
- `quatopsy adapt` converts IDS Jason-1 ASCII, ROS JSON, uncompressed MCAP JSON poses, SPICE CK type 3, and TUBIN star-tracker CSV into canonical CSV plus manifest. Adapters never assign report `result` values.
- `--policy advisory|selective|required` and `--override-file` change process exit only.

## Unreleased M6

- `quatopsy plan` emits a torque-limited rest-to-rest candidate plus `quatopsy.plan/1`. Closed-form eigenaxis and bounded multiple-shooting paths are available. Residuals are computed by an independent oracle, including stored momentum and keep-out cones. Optional actuator models, weighted objectives, and perturbation campaigns never assign report `result`. A subsequent `analyze` owns the only verdict. Optimality is not claimed. Gyroscopic torque or actuator limits that cannot be met are infeasible.

## Unreleased M7

- `quatopsy control` runs a geometric PD controller on SO(3) plus `quatopsy.control/1`. Attitude error is the rotation-matrix vee map. An independent oracle monitor inhibits commands. Estimator freshness, saturation, anti-windup, momentum dump, modes, deterministic SIL campaigns, and declared software plant models (command-to-torque lag, gyro ARW, star-tracker delay, magnetic residual, gravity-gradient) never assign report `result`. `execution` may be `sil`, host-CPU `pil`, or loopback `hil`. Hard real-time, physical actuators, qualified processors, and hardware command are refused.

## Required non-claims

Do not state or imply that Quatopsy is novel, safe, flight-proven, certified, production-ready, complete, optimal, or independently validated. Do not state that a `pass` result is flight approval, actuator permission, or energy optimality. Do not state that a feasible plan or a tracked SIL candidate is globally optimal, dynamically verified under uncertainty, or authorised for command. Do not state that commanded-path findings measure control effort or mission risk. Do not state that the candidate name is a cleared trademark. Do not state that the TUBIN excerpt is a mission reconstruction or that adapters certify source conventions.

## Out of V1 supported scope

- Automatic convention inference and automatic convention repair remain refused.
- Hardware command, hard real-time execution, target-processor PIL, physical-actuator HIL, and flight assurance remain refused. Host-CPU PIL and loopback HIL are software evidence only. The controller does not open actuator devices.
- Hosted CI, crates.io publication, signed binaries, public repository visibility, websites, and production support remain distinct unauthorised gates.
- Full visual brand assets are absent because productisation is not approved.

## Evidence map

Behavioural evidence lives in the conformance fixtures, hostile and lifecycle tests, million-sample release check, local CI log, and this claims freeze. Optional independent reproduction does not block this release and is not claimed.
