# Frozen public claims

Status date: 2026-08-15. Version: `0.1.0`. Visibility: private research repository.

## Permitted statements

- Quatopsy is a local-first, advisory quaternion trajectory linter plus a non-authoritative static viewer.
- For declared `quatopsy.manifest/1` inputs and enabled V1 rules, reports follow `quatopsy.report/1` and the deterministic numeric profile `quatopsy.numeric/1`.
- Supported rules are `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, `QAT-PI-001`, `QAT-REPAIR-001`, and `QAT-UNWIND-001` when commanded columns are declared.
- Sign-lift repair candidates preserve represented orientation under the independent rotation-matrix oracle used in tests.
- One million synthetic identity samples meet the documented time and RSS budget on the local CI host.
- Local checksum packaging is available via `scripts/package-local.sh`.

## Required non-claims

Do not state or imply that Quatopsy is novel, safe, flight-proven, certified, production-ready, complete, optimal, or independently validated. Do not state that a `pass` result is flight approval, actuator permission, or energy optimality. Do not state that commanded-path findings measure control effort or mission risk. Do not state that the candidate name is a cleared trademark.

## Out of V1 supported scope

- `QAT-CONV-001` remains limited: conventions are declared, not inferred, and automatic convention repair is refused.
- Adapters (ROS, MCAP, SPICE, and similar) are outside the semantic core (`INT-2`).
- Advisory, selective, and required adoption-policy engines with scoped overrides are not shipped (`INT-3`). Exit codes remain the only enforcement hook.
- Hosted CI, crates.io publication, signed binaries, public repository visibility, websites, and production support remain distinct unauthorised gates.
- Full visual brand assets are absent because productisation is not approved.

## Evidence map

Behavioural evidence lives in the conformance fixtures, hostile and lifecycle tests, million-sample release check, local CI log, and this claims freeze. Optional independent reproduction does not block this release and is not claimed.
