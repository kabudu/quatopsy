# Changelog

## Unreleased

### Added

- Rust workspace with `quatopsy-schema`, `quatopsy-core`, `quatopsy-oracle`, and `quatopsy` CLI.
- Closed V1 rules `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, and `QAT-PI-001`.
- Canonical `quatopsy.manifest/1` ingest, `quatopsy.report/1` JSON, fail-closed aggregation, and exit codes 0/1/2/3/64.
- Independent rotation-matrix and fixed-point dot oracles used only in tests.
- Authoritative local CI at `./scripts/ci-local.sh` and the delivery loop in `AGENTS.md`.

### Claims

M1 proves supported-rule conformance on the frozen fixture corpus. It does not claim flight safety, novelty, independent external validation, or product release.
