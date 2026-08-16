# Changelog

## Unreleased

### Added

- Responsive forensic investigation console with synchronized trajectory playback, timeline scrubbing, evidence navigation, sample telemetry, and richer offline canvas rendering.
- Forensic gap closure for strict adoption-policy validation, canonical UTC overrides, unknown-major refusal exits, transactionally committed output sets, per-finding reproducers, and bounded viewer finding navigation.
- `QAT-CONV-001` matrix comparison and `QAT-OMEGA-001` body-rate comparison.
- `quatopsy adapt` for `ids-jason1`, `ros-json`, `tubin-str`, uncompressed `mcap-json`, and `spice-ck` type 3, with provenance that never contains `result`.
- Adoption policy `--policy` / `--fail-on` / `--override-file` (exit only).
- CC BY 4.0 TUBIN star-tracker excerpt and mutation/fuzz/privacy/permission E2E coverage.

### Claims

These close the previously deferred V1 gaps. They do not claim flight safety, TUBIN reconstruction, novelty, or independent external validation.

## 0.1.0 - 2026-08-15

### Added

- Apache-2.0 licence, frozen claims, stop-ship audit, curated `v0.1.0` notes, and fail-closed GitHub Release publishing.
- Supply-chain license allowlist, package provenance, and desktop/narrow release-note preview.

### Claims

M5 completes the private research release policy for `0.1.0`. It does not open the repository, publish crates, sign binaries, enable hosted CI, ship a full brand system, or claim flight safety, novelty, or independent external validation.

## M4

### Added

- Frozen spacecraft CSV profile `quatopsy.spacecraft-csv/1` with synthetic representative fixtures.
- `QAT-UNWIND-001` commanded-path comparison against the quotient-shortest baseline.
- One-million-sample release budget check and local checksum packaging via `scripts/package-local.sh`.
- Local binary copy/install, repeated-analysis compatibility, removal, and report-version lifecycle tests.

### Claims

M4 proves the frozen spacecraft CSV profile, commanded-path diagnostics, and the million-sample budget on the local CI host. It does not sign or publish binaries, open the repository, or claim flight safety, novelty, or independent external validation.

## M3

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
