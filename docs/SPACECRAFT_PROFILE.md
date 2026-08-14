# Spacecraft CSV profile `quatopsy.spacecraft-csv/1`

This profile is frozen for Quatopsy V1 spacecraft attitude analysis. It is a documentation and fixture contract, not a flight-approved interface.

## Supported input

- UTF-8 CSV with a header row and one sample per subsequent row.
- Explicit `quatopsy.manifest/1` with `component_order` `wxyz` or `xyzw`, `rotation_sense`, distinct `frame_from` and `frame_to`, and `time_unit` `ns`, `us`, `ms`, or `s`.
- Required columns: monotonically increasing time and four quaternion components in the declared order.
- Optional commanded quaternion columns, same component order, used only by `QAT-UNWIND-001`.
- Optional angular-velocity columns are accepted and ignored by V1 rules.

Unknown manifest fields are refused. Conventions are never guessed.

## Representative fixtures

| Fixture | Role |
| --- | --- |
| `fixtures/profile/spacecraft_v1/` | Synthetic constant-rate BODY to J2000 slew with a matching commanded path |
| `fixtures/conformance/clean_slew/` | Same slew without commanded columns |
| `fixtures/conformance/commanded_long_way/` | Commanded antipodal covering to the same orientation |

Third-party public flight telemetry is not redistributed here. Inclusion requires a licence that permits repository distribution and an explicit owner decision.

## Non-claims

Passing this profile does not mean the trajectory is dynamically feasible, actuator-safe, or suitable for flight.
