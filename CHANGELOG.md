# Changelog

## Unreleased

### Added

- Static local viewer bundle via `quatopsy view`, with CSP, no remote loads, and report-bound derived geometry.
- Linked physical `SO(3)`, stereographic `S^3`, timeline, evidence, and repair views keyed by `source_row`.
- Evidence-preserving downsampling that retains finding endpoints and angle/rate extrema.

### Claims

M3 proves the viewer displays canonical report verdicts and labelled geometry on the fixture corpus. It does not claim flight safety, novelty, independent external validation, or product release.

## M2

### Added

- `QAT-REPAIR-001` with `sign-lift` and `normalise` repair candidates, explicit `quatopsy repair`, and no source overwrite.
- Minimal reproducible slices via `--repro-dir`, with path redaction by default.
- Cancellation cleanup, `--clean` leftover-tmp removal, symlink/special-file output guards, and hostile-input tests.

### Claims

M2 proves repair proposals and digest-bound apply on the fixture corpus. It does not claim flight safety, novelty, independent external validation, or product release.

## M1

### Added

- Rust workspace with `quatopsy-schema`, `quatopsy-core`, `quatopsy-oracle`, and `quatopsy` CLI.
- Closed V1 rules `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, and `QAT-PI-001`.
- Canonical `quatopsy.manifest/1` ingest, `quatopsy.report/1` JSON, fail-closed aggregation, and exit codes 0/1/2/3/64.
- Independent rotation-matrix and fixed-point dot oracles used only in tests.
- Authoritative local CI at `./scripts/ci-local.sh` and the delivery loop in `AGENTS.md`.

### Claims

M1 proves supported-rule conformance on the frozen fixture corpus. It does not claim flight safety, novelty, independent external validation, or product release.
