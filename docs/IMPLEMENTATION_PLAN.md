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
- [x] Emit minimal reproducible fixture slices with source provenance and privacy controls.
- [x] Add atomic output, no-clobber defaults, cancellation cleanup, and clean mode.
- [x] Test hostile CSV, path, Unicode, numeric, and resource-exhaustion inputs.

Exit: every initial defect fixture has a deterministic finding, evidence interval, repair disposition, and regression case.

## M3: Local visual debugger

- [x] Build the static browser viewer over the canonical report without redefining verdicts.
- [x] Synchronise physical 3D, projected `S^3`, timeline, evidence, and repair views by sample identity.
- [x] Label projection artefacts and distinguish raw, derived, and proposed data.
- [x] Meet keyboard, screen-reader, contrast, reduced-motion, colour-state, and bounded-rendering requirements.
- [x] Validate large-report downsampling retains findings and extrema.

Exit: public-workflow E2E tests reproduce all five initial defect stories and accessibility checks pass.

## M4: Spacecraft qualification and packaging

- [x] Define and freeze the supported spacecraft CSV profile with representative synthetic and public fixtures.
- [x] Add optional commanded-path comparison for bounded unwinding diagnostics.
- [x] Benchmark one million samples against the documented performance budget.
- [x] Package reproducible signed or checksummed binaries only when release infrastructure and authorization exist.
- [x] Complete install, upgrade, downgrade, rollback, removal, and report-compatibility tests.

Exit: supported scope passes adversarial, portability, lifecycle, performance, and documentation gates.

## M5: Product release gate

- [x] Resolve every stop-ship correctness, security, privacy, compatibility, and legal/name risk.
- [x] Freeze evidence-bounded public claims and curated release presentation.
- [x] Produce and validate the full brand system only if productisation is approved.
- [x] Render release notes at desktop and narrow widths and verify the live release after publication.
- [x] Publish only with explicit release authorization and documented credentials path.

Exit: release policy passes for a private `0.1.0` GitHub Release. External independent validation is welcome but is not required for this exit; unsupported independent-validation claims remain prohibited. Public opening, hosted CI, crates.io, signed binaries, and full brand productisation remain distinct unauthorised gates.

## V1 gap closure

- [x] Evaluate against redistributable public flight attitude samples (TUBIN excerpt) plus format-compatible IDS/ROS adapters.
- [x] Implement `QAT-CONV-001` against supplied rotation-matrix columns.
- [x] Keep adapters outside verdict ownership (`INT-2`) with contract tests.
- [x] Ship advisory, selective, and required adoption with scoped overrides (`INT-3`).
- [x] Use optional angular-velocity columns in `QAT-OMEGA-001`.
- [x] Meet the documented E2E bar for mutation, fuzz, privacy sinks, and permission chaos.
- [x] Ship uncompressed MCAP JSON and SPICE CK type 3 adapters outside verdict ownership.

Exit: local CI plus the named fixtures and CLI tests.

## Optional post-release evidence track

- [ ] Invite independent reproduction or expert challenge and record positive and negative results.
- [ ] Conduct practitioner interviews or pilots if evidence of demand or workflow value is desired.
- [ ] Measure debugging-time, false-finding, and adoption outcomes with a pre-registered protocol.
- [ ] Explore ecosystem priorities using evidence rather than assumption.

This track is optional and does not block implementation, product completion, publication, or release.

## Separate future project

- [ ] Explore the quaternion learning laboratory under a distinct mandate, name audit, claim boundary, and repository.

