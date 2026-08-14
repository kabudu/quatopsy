# Report protocol

## Versioning

The initial protocol family is `quatopsy.report/1`. The root contains `schema`, `analysis_id`, `tool`, `input`, `declarations`, `limits`, `result`, `rule_results`, `findings`, `repairs`, and `diagnostics`.

## Result aggregation

Required rule states are `pass`, `finding`, `refused`, or `error`. Overall `error` dominates `refused`, which dominates `findings`, which dominates `pass`. Informational rules cannot turn a pass into findings unless promoted by the versioned policy.

## Findings

Each finding includes a stable identifier, rule identifier and version, class, severity, source row interval, integer timestamp interval, evidence values with units, human summary, machine reason code, and repair references. Findings are ordered by first source row, rule ID, then deterministic occurrence index.

## Repairs

Each repair has an algorithm identifier, source analysis ID, preconditions, exact affected rows, semantic declaration, output digest after generation, and whether physical rotations are expected to remain equivalent. Repair application is a separate explicit CLI action.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | `pass` |
| 1 | `findings` |
| 2 | `refused` |
| 3 | `error` |
| 64 | CLI usage error before analysis identity exists |

## Compatibility

Consumers reject unknown major schema versions. Unknown optional fields in a known major version are retained or ignored without changing verdicts. Rule semantic changes require a new rule version and cannot overwrite historical meaning.

