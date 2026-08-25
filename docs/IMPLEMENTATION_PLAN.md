# Implementation plan

## Delivery policy

Implementation begins only after explicit authorization. Each milestone uses a scoped `codex/` feature branch, the repository's authoritative local CI, a reviewed pull request, squash merge, default-branch synchronisation, and branch cleanup. Checked boxes require behavioural evidence, not documentation alone.

## M0: Planning and claim boundary

- [x] Define the first vertical, candidate mechanism, non-goals, and result semantics in the planning corpus.
- [x] Record point-in-time prior-art, product, package-name, and patent-surface searches.
- [x] Define the optional status of independent validation and separate claims that depend on it.
- [x] Complete the initial Lazarus engineering review and bootstrap validation.

Exit evidence: planning commit on the private default branch. This milestone does not implement product behaviour.

## M1: Conformance kernel and protocol

- [x] Persist the implementation workflow in `AGENTS.md` and create executable `scripts/ci-local.sh` before other implementation is counted complete.
- [x] Create the latest stable Rust workspace with library, schema, and CLI crates.
- [x] Implement canonical manifest and streaming CSV validation with bounded inputs.
- [x] Implement quaternion primitives, quotient distance, deterministic lift, and near-pi conditioning metadata.
- [x] Implement rule registry, fail-closed aggregation, canonical report schema, and exit codes.
- [x] Build independently encoded matrix and high-precision conformance oracles.
- [x] Verify deterministic reports across macOS and Linux-like clean environments where locally available.

Exit: hand-audited fixtures prove `QAT-NORM-001`, `QAT-TIME-001`, `QAT-LIFT-001`, `QAT-SIGN-001`, `QAT-RATE-001`, and `QAT-PI-001`; local CI passes.

## M2: Repairs and reproducible counterexamples

- [x] Implement sign-lift and normalisation repair candidates without source overwrite.
- [x] Prove sign repairs preserve the independent rotation-matrix oracle within tolerance.
- [x] Emit one context-bounded reproducible fixture slice per finding with source provenance and path privacy controls.
- [x] Add staged output-set commit with rollback, race-safe no-clobber defaults, cancellation cleanup, and clean mode.
- [x] Test hostile CSV, path, Unicode, numeric, and resource-exhaustion inputs.

Exit: every initial defect fixture has a deterministic finding, evidence interval, repair disposition, and regression case.

## M3: Local visual debugger

- [x] Build the static browser viewer over the canonical report without redefining verdicts.
- [x] Synchronise physical 3D, projected `S^3`, timeline, evidence, and repair views by sample identity.
- [x] Label projection artefacts and distinguish raw, derived, and proposed data.
- [x] Meet keyboard, screen-reader, contrast, reduced-motion, colour-state, and bounded-rendering requirements.
- [x] Validate large-report downsampling retains bounded geometry extrema and a navigation link for every finding.
- [x] Refine the static viewer into a responsive forensic investigation console with synchronized playback, direct timeline navigation, dense evidence summaries, and visually distinct canonical, derived, proposed, and projection layers.

Exit: public-workflow E2E tests reproduce all five initial defect stories, accessibility checks pass, and the generated bundle passes desktop and narrow visual QA without becoming a free-form quaternion learning laboratory.

## M4: Spacecraft qualification and packaging

- [x] Define and freeze the supported spacecraft CSV profile with representative synthetic and public fixtures.
- [x] Add optional commanded-path comparison for bounded unwinding diagnostics.
- [x] Benchmark one million samples against the documented performance budget.
- [x] Package reproducible signed or checksummed binaries only when release infrastructure and authorization exist.
- [x] Complete local binary copy/install, repeated-analysis compatibility, removal, V1 report-reader compatibility, and unknown-major refusal tests.

Exit: supported scope passes adversarial, portability, lifecycle, performance, and documentation gates.

## M5: Product release gate

- [x] Resolve every stop-ship correctness, security, privacy, compatibility, and legal/name risk.
- [x] Freeze evidence-bounded public claims and curated release presentation.
- [x] Produce and validate the full brand system only if productisation is approved.
- [x] Generate and validate release-note previews at desktop and narrow widths before publication.
- [x] Publish only with explicit release authorization and documented credentials path.

Exit: release policy passes for a private `0.1.0` GitHub Release. External independent validation is welcome but is not required for this exit; unsupported independent-validation claims remain prohibited. Public opening, hosted CI, crates.io, and signed binaries remain distinct unauthorised gates. The brand system is tracked as M9.

## V1 gap closure

- [x] Evaluate against redistributable public flight attitude samples (TUBIN excerpt) plus format-compatible IDS/ROS adapters.
- [x] Implement `QAT-CONV-001` against supplied rotation-matrix columns.
- [x] Keep adapters outside verdict ownership (`INT-2`) with contract tests.
- [x] Ship advisory, selective, and required adoption with scoped overrides (`INT-3`).
- [x] Use optional angular-velocity columns in `QAT-OMEGA-001`.
- [x] Meet the documented automated E2E bar for mutation, seeded fuzz, default stderr privacy, output cleanup, and permission failures.
- [x] Ship uncompressed MCAP JSON and SPICE CK type 3 adapters outside verdict ownership.

Exit: local CI plus the named fixtures and CLI tests.

## M6: Offline candidate trajectory generator

- [x] Ship `quatopsy plan` for a spherical torque-limited rest-to-rest rigid body using eigenaxis bang-coast-bang.
- [x] Emit canonical CSV plus `quatopsy.plan/1` with no report `result`.
- [x] Keep the diagnostic kernel as the only verdict owner via plan-then-analyze tests.
- [x] Replace same-crate residual checks with an independent dynamics oracle and mutation coverage.
- [x] Emit body-rate columns and prove Euler residuals, including switch intervals, against `Jω̇ + ω × Jω = τ`.
- [x] Add wheels, thrusters, CMGs, keep-out zones, and a general inertia tensor.
- [x] Add weighted objectives beyond minimum-time rest-to-rest.
- [x] Add a bounded collocation, multiple-shooting, or sequential-convexification solver.
- [x] Add simulation campaigns under model uncertainty and actuator saturation.

Exit: local CI plus the spherical rest-to-rest fixture. Open boxes are not shipped.

## M7: Software-in-the-loop attitude controller

- [x] Ship `quatopsy control` for SIL geometric PD on SO(3) under a declared plant and envelope.
- [x] Use rotation-matrix attitude error so antipodal quaternions do not unwind.
- [x] Enforce estimator contracts for frame, timestamp, covariance, and freshness.
- [x] Add saturation, anti-windup, momentum dump, mode transitions, arbitration, and safe fallback.
- [x] Inhibit commands with an independent oracle monitor outside the PD law.
- [x] Run deterministic SIL campaigns under noise, delay, inertia error, disturbance, actuator failure, and numerical faults.
- [x] Refuse hard real-time and physical actuator command. Hardware use remains fail-closed without a qualification record.
- [x] Processor-in-the-loop with an isolated controller process on the host CPU. Target flight processors are not claimed.
- [x] Hardware-in-the-loop command bus against a loopback actuator emulator. Physical actuators are refused.
- [x] Systems-safety programme and fail-closed hardware-use gate. Hardware is not qualified.
- [x] Add declared higher-fidelity software plant models: first-order command-to-torque lag, gyro ARW, star-tracker delay, magnetic residual, and gravity-gradient.

Exit: local CI plus the SO(3) rest-to-rest SIL, PIL, loopback HIL, and declared-plant fixtures. Open boxes are not shipped.

## M8: Software GN&C plane

This milestone is a software navigation, guidance, and control plane. It does not solve space navigation, qualify hardware, or produce a certification artefact.

- [x] Replace the measurement pass-through with a 6-state MEKF (attitude error plus gyro bias), asynchronous star/gyro ingest, χ² outlier rejection, and NIS/NEES audit output.
- [x] Add a UKF on the same error-state and measurement model, selected by `navigation.filter`.
- [x] Add `quatopsy-guidance` time-tagged `(t, q, ω, α)` profiles, including optional plan-CSV ingest, keep-out, named sun-pointing, and terminal rest.
- [x] Track time-varying guidance references in geometric PD (nonzero `ω_d`/`α_d`) with optional gain scheduling.
- [x] Add control-side reaction-wheel allocation (3-axis or 4-wheel pyramid) with per-wheel limits and an independent allocation residual.
- [x] Add a declared two-body geometry source for LVLH/nadir, sun, eclipse, and dipole `B(t)`. This is not orbit determination.
- [x] Record a sequential deterministic cycle partition with software phase durations. `hard-real-time` remains refused.
- [ ] Physical 6-DOF hardware-in-the-loop, real star-tracker/gyro drivers, and actuator I/O.
- [ ] CMG gimbal-rate allocation.
- [ ] Target-processor qualification, WCET per phase, and a flight telemetry/command security bus.
- [ ] Certification evidence bundles or organisational safety-case sign-off.
- [ ] Orbit determination, GPS, or map-aiding filters.

Exit: local CI plus the rest-to-rest fixtures and a profile-tracking fixture. Open boxes are not shipped.

## M9: Product brand system

Owner productisation was approved. Trademark filing was declined. Public opening remains a later gate.

- [x] Record owner productisation approval and the decision to keep `Quatopsy` without a trademark filing, without claiming clearance.
- [x] Construct and score three mark directions; select the antipodal paired-point system.
- [x] Ship `quatopsy.brand/1` sources, exports, tokens, templates, licences, and a digest manifest.
- [x] Validate contrast, SVG safety, overlay separation, prohibited-claim copy, and deterministic export in local CI.
- [x] Apply the symbol and tagline on the local viewer and CLI without weakening CSP or report ownership.
- [ ] Public repository visibility.
- [ ] Hosted CI.
- [ ] crates.io, signed binaries, website, and production support.

Exit: local CI including `python3 scripts/brandkit.py check`. Open boxes are not shipped.

## Optional post-release evidence track

- [ ] Verify the canonical release page at desktop and narrow widths after an authorised publication.
- [ ] Invite independent reproduction or expert challenge and record positive and negative results.
- [ ] Conduct practitioner interviews or pilots if evidence of demand or workflow value is desired.
- [ ] Measure debugging-time, false-finding, and adoption outcomes with a pre-registered protocol.
- [ ] Explore ecosystem priorities using evidence rather than assumption.

This track is optional and does not block implementation, product completion, publication, or release.

## Separate future project

- [ ] Explore the quaternion learning laboratory under a distinct mandate, name audit, claim boundary, and repository.
