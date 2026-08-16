# Report protocol

## Versioning

The initial protocol family is `quatopsy.report/1`. The root contains `schema`, `analysis_id`, `tool`, `input`, `declarations`, `limits`, `result`, `rule_results`, `findings`, `repairs`, and `diagnostics`.

## Result aggregation

Required rule states are `pass`, `finding`, `refused`, or `error`. Overall `error` dominates `refused`, which dominates `findings`, which dominates `pass`. Informational rules cannot turn a pass into findings unless promoted by the versioned policy.

## Findings

Each finding includes a stable identifier, rule identifier and version, class, severity, source row interval, integer timestamp interval, evidence values with units, human summary, machine reason code, and repair references. Findings are ordered by first source row, rule ID, then deterministic occurrence index.

## Repairs

Each repair has an algorithm identifier, source analysis ID, disposition (`proposed`, `inapplicable`, `unsafe`, or `none`), preconditions, exact affected rows, semantic declaration, numeric tolerance, optional output digest after generation, and whether physical rotations are expected to remain equivalent. Repair application is a separate explicit CLI action (`quatopsy repair`) and never overwrites the source CSV.

## Viewer

`quatopsy view` writes a static HTML/CSS/JS bundle. Verdicts are copied from `quatopsy.report/1` and are never recomputed in the browser. Derived geometry is a separate `quatopsy.view/1` payload labelled non-authoritative. Unknown report major versions produce an explanatory non-authoritative bundle and return refusal exit 2. The bundle loads no remote resources.

`QAT-UNWIND-001` is enabled for every analysis. When commanded quaternion columns are absent it records `commanded-path-absent` and passes. When they are present it compares adjacent covering angle `2 acos(p·q)` with the quotient-shortest angle `2 acos(|p·q|)` and records `commanded-long-way` findings when the commanded covering is longer.

`QAT-CONV-001` is enabled for every analysis. When rotation-matrix columns are absent it records `redundant-evidence-absent` and passes. When they are present it compares the declared `R(q)` with the supplied matrix and records `component-order-mismatch`, `rotation-sense-mismatch`, or `matrix-inconsistent`.

`QAT-OMEGA-001` is enabled for every analysis. When angular-velocity columns are absent it records `omega-absent` and passes. When they are present it compares supplied body rate with the kinematics of the lifted adjacent pair.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | `pass`, or `findings` under `--policy advisory`, or non-blocking `findings` under `--policy selective` |
| 1 | `findings` under `--policy required` (default), or blocking selective findings |
| 2 | `refused`, or invalid/expired override document |
| 3 | `error` |
| 64 | CLI usage error before analysis identity exists |

`--policy` and `--override-file` change process exit only. They never rewrite `report.result`.

## Compatibility

Consumers reject unknown major schema versions. Unknown optional fields in a known major version are retained or ignored without changing verdicts. Rule semantic changes require a new rule version and cannot overwrite historical meaning.
