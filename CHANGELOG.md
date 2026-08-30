# Changelog

All notable changes to Quatopsy are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added a responsive, accessible, SEO-complete static GitHub Pages website with a reproducible, least-privilege deployment pipeline.

### Fixed

- Added a transparent universal-host wordmark that remains legible when repository renderers do not honour light and dark responsive image selection.

## [0.2.1] - 2026-08-30

### Fixed

- Honoured crates.io server-directed new-crate cooldown deadlines with bounded automatic backoff instead of exhausting fixed-interval retries.
- Included the embedded static viewer in the installable Cargo package and made payload inspection require all three viewer assets.

## [0.2.0] - 2026-08-30

### Added

- Added `quatopsy investigate` and `quatopsy verify-evidence` for bounded, digest-bound incident evidence bundles containing copied inputs, canonical diagnostics, reproducers, repairs, local viewers, and separately analysed plan and control candidates.
- Added the `quatopsy.brand/2` product identity, release lockups, deterministic raster exports, responsive architecture diagrams, and brand validation.
- Added `quatopsy control` with geometric SO(3) control, independent command inhibition, bounded SIL, host-CPU PIL, loopback HIL, MEKF and UKF navigation, guidance profiles, wheel allocation, and declared software plant models.
- Added `quatopsy plan` with torque-limited rest-to-rest candidates, independent dynamics residuals, actuator models, keep-out constraints, perturbation campaigns, and bounded direct shooting.
- Added IDS Jason-1, ROS JSON, TUBIN STR, uncompressed MCAP JSON, and SPICE CK type 3 adapters that emit provenance without assigning diagnostic verdicts.
- Added `QAT-CONV-001` rotation-matrix comparison, `QAT-OMEGA-001` body-rate comparison, adoption policies, scoped overrides, canonical UTC handling, and per-finding reproducers.
- Added an open-source community contract, product-led README, support and vulnerability-reporting routes, issue forms, pull-request template, GitHub About metadata, and offline consistency checks.
- Added hosted CI, Cargo workspace publication, automated release preparation, GitHub Releases sourced from this changelog, and dependency update configuration.

### Changed

- Raised the workspace version to `0.2.0`, renamed the installable Cargo package from `quatopsy-cli` to `quatopsy`, and made all nine workspace packages publishable in dependency order.
- Positioned Quatopsy as early-stage, production-quality research software for local advisory evaluation while retaining its certification, flight-use, independent-validation, and production-readiness boundaries.
- Pinned Rust 1.97.1 and declared MSRV 1.97.
- Preserved original trajectories as read-only evidence while keeping repairs, plans, and control outputs separately named.

### Fixed

- Made multi-file output commits transactional, race-safe, no-clobber by default, and recoverable after cancellation or partial failure.
- Corrected pointing weights, redundant-wheel momentum, delayed estimator updates, NIS and NEES recording, control-artifact reproducibility, and bounded PIL and HIL worker framing.
- Corrected magnetic and gravity-gradient torque application, logged plant torque, zero star-tracker delay, UKF covariance updates, and trapezoidal gyro propagation.
- Improved deterministic brand rasterisation and retained the approved woven-lift geometry across generated assets.

## [0.1.0] - 2026-08-15

### Added

- Added the deterministic quaternion conformance kernel, canonical manifest and report protocols, fail-closed aggregation, stable exit codes, and the closed V1 rule registry.
- Added digest-bound sign-lift and normalisation repair candidates, explicit repair application, privacy-preserving repro slices, cancellation cleanup, and output-path hardening.
- Added the dependency-free static forensic viewer with linked physical SO(3), projected S3, timeline, evidence, and repair views.
- Added the frozen spacecraft CSV profile, commanded-path unwinding diagnostics, one-million-sample performance gate, lifecycle compatibility tests, and checksummed local packaging.
- Added the Apache-2.0 licence, supply-chain allowlist, frozen public claims, curated private release notes, and fail-closed release policy.

[Unreleased]: https://github.com/kabudu/quatopsy/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/kabudu/quatopsy/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kabudu/quatopsy/compare/b6e0ffe...v0.2.0
[0.1.0]: https://github.com/kabudu/quatopsy/commit/b6e0ffe
