# Spacecraft CSV profile `quatopsy.spacecraft-csv/1`

This profile is frozen for Quatopsy V1 spacecraft attitude analysis. It is a documentation and fixture contract, not a flight-approved interface.

## Supported input

- UTF-8 CSV with a header row and one sample per subsequent row.
- Explicit `quatopsy.manifest/1` with `component_order` `wxyz` or `xyzw`, `rotation_sense`, distinct `frame_from` and `frame_to`, and `time_unit` `ns`, `us`, `ms`, or `s`.
- Required columns: monotonically increasing time and four quaternion components in the declared order.
- Optional commanded quaternion columns, same component order, used only by `QAT-UNWIND-001`.
- Optional angular-velocity columns, interpreted in `frame_from`, used by `QAT-OMEGA-001`.
- Optional row-major rotation-matrix columns used by `QAT-CONV-001`.

Unknown manifest fields are refused. Conventions are never guessed.

## Representative fixtures

| Fixture | Role |
| --- | --- |
| `fixtures/profile/spacecraft_v1/` | Synthetic constant-rate BODY to J2000 slew with a matching commanded path |
| `fixtures/conformance/clean_slew/` | Same slew without commanded columns |
| `fixtures/conformance/commanded_long_way/` | Commanded antipodal covering to the same orientation |
| `fixtures/public/tubin_str/` | CC BY 4.0 excerpt of TUBIN star-tracker voter quaternions and rates |
| `fixtures/public/ids_jason1_format/` | Synthetic IDS Jason-1 ASCII layout (not a CDDIS redistribution) |

CDDIS Jason archives require Earthdata login, so those flight files are not vendored. The TUBIN excerpt is attributed in `fixtures/public/tubin_str/README.md`.

## Non-claims

Passing this profile does not mean the trajectory is dynamically feasible, actuator-safe, or suitable for flight.
