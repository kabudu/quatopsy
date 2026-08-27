# Changelog

## Unreleased

- Prepared the private repository for open-source review with a product-led README, branded system architecture, community health files, validated repository metadata, and explicit publication boundaries.

### Added

- Private `quatopsy investigate` and `quatopsy verify-evidence` workflows for bounded canonical or adapter-backed incident capture, opaque operations context, observed diagnostics, reproducers, repairs, separately analysed plan/control candidates, local viewers, and deterministic `quatopsy.evidence/1` integrity verification.
- Canonical product brand `quatopsy.brand/2` with the owner-selected woven-lift ribbon mark, outlined Space Grotesk wordmark, violet/cerise/ivory palette, tokens, lockups, deterministic PNG exports, and local CI validation. `quatopsy.brand/1` remains identifiable in repository history.
- `quatopsy control` for a geometric PD controller on SO(3), with independent command inhibition, estimator freshness contracts, saturation and safe fallback, host-CPU processor-in-the-loop, loopback hardware-in-the-loop, declared software plant models, a software GN&C plane (MEKF/UKF, guidance profiles, wheel allocation, declared two-body geometry), a fail-closed hardware-use gate, deterministic robustness trials, no physical actuator I/O, and no report `result`.
- `quatopsy plan` for a torque-limited rest-to-rest candidate, with independent Euler/kinematics residuals, body-rate columns, actuator and keep-out models, behaviorally tested weighted objectives, bounded nonlinear direct shooting, declared-actuator perturbation campaigns, infeasibility on constraint violation, and no report `result`.
- Forensic gap closure for strict adoption-policy validation, canonical UTC overrides, unknown-major refusal exits, transactionally committed output sets, per-finding reproducers, and bounded viewer finding navigation.
- `QAT-CONV-001` matrix comparison and `QAT-OMEGA-001` body-rate comparison.
- `quatopsy adapt` for `ids-jason1`, `ros-json`, `tubin-str`, uncompressed `mcap-json`, and `spice-ck` type 3, with provenance that never contains `result`.
- Adoption policy `--policy` / `--fail-on` / `--override-file` (exit only).
- CC BY 4.0 TUBIN star-tracker excerpt and mutation/fuzz/privacy/permission E2E coverage.

### Changed

- Toolchain pin is Rust 1.97.1. Declared MSRV `rust-version` is 1.97, matching current stable rather than the edition-2024 floor. Clippy 1.97.1 let-chain and `is_multiple_of` lints are applied.

### Fixed

- Brand raster exports now use deterministic 4x supersampled scan conversion, and the canonical ribbon geometry retains the broad curves and folded lower return of the owner-approved concept instead of a faceted approximation.
- M6-M8 hardening makes pointing weights effective, models redundant-wheel momentum per wheel, bounds delay/profile/schedule work, aligns delayed star updates to the filter epoch, records per-update NIS/NEES, makes canonical control artifacts reproducible, and gives PIL/HIL workers bounded messages and response deadlines.
- Control plant applies magnetic residual and gravity-gradient torque to Euler's equation only. Stored wheel momentum follows motor torque.
- Logged control CSV torque is the plant-applied body torque after command-to-torque lag and declared environmental models. The initial sample is zero.
- `star_tracker_delay_s: 0` is zero attitude delay. It does not fall back to gyro `delay_s`.
- Software UKF measurement update uses sigma-point `Pzz`/`Pxz` rather than a copied MEKF Joseph form. Filter covariance is no longer floored at `1e-6`, so propagate-only runs can grow `P` into the envelope.
- Gyro predict uses trapezoidal rate hold so star innovations during a slew stay inside the χ² gate without a fake covariance floor.

### Claims

These close the previously deferred V1 gaps. They do not claim flight safety, TUBIN reconstruction, novelty, trademark clearance, or independent external validation. The unreleased planner is a candidate generator, not a certified or globally optimal trajectory. The unreleased controller is not flight software, not hard real-time, not a qualified processor, and not actuator permission. The brand system is not public opening.

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
