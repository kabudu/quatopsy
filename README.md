<p align="center">
  <img src="assets/brand/templates/release-lockup.svg" width="468" alt="Quatopsy">
</p>

<p align="center"><strong>See where rotations go wrong.</strong></p>

<p align="center">Find, explain, and visualise quaternion attitude failures, then evaluate controller and trajectory candidates locally.</p>

<p align="center">
  <a href="#quick-start">Quick start</a> |
  <a href="#what-quatopsy-finds">Capabilities</a> |
  <a href="#how-it-fits-together">Architecture</a> |
  <a href="docs/INVESTIGATION_WORKFLOW.md">Incident workflow</a> |
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

Quatopsy helps engineers find, explain, visualise, and safely investigate failures in recorded orientation data. It checks quaternion trajectories for defects that can look harmless in component plots but cause sudden jumps, unnecessary rotations, convention mismatches, invalid rates, or misleading interpolation. Every finding links back to the original samples, and every proposed repair remains separate from the measured input.

It is built primarily for spacecraft guidance, navigation, and control engineers, with the same analysis also applicable to robotics, simulation, and graphics pipelines. Quatopsy runs locally, requires no service or account, and does not collect telemetry. Its bounded planning and control tools can generate candidates for software evaluation, but they cannot command physical hardware.

> [!IMPORTANT]
> Quatopsy is advisory research software. A passing report is not flight approval, actuator permission, certification evidence, or proof of operational safety. Physical hardware, hard real-time execution, and orbit determination remain outside the supported boundary.

## See the trajectory, not just the components

The static investigation console keeps four views synchronized by source-sample identity:

- the physical orientation path in three-dimensional rotation space (`SO(3)`);
- the corresponding quaternion path on the four-dimensional unit sphere, shown through a projected `S3` view;
- a quotient-angle timeline with finding-linked geometry;
- canonical findings, raw values, derived values, and unapplied repair candidates.

The viewer is generated as a local, dependency-free bundle. It does not recompute rules or reinterpret the canonical report.

## What Quatopsy finds

| Failure mode | Why it matters | Evidence |
| --- | --- | --- |
| Norm drift and zero quaternions | Invalid rotations can contaminate every downstream calculation | Norm residuals, refusal boundaries, reversible normalisation candidates |
| Sign discontinuities | Equivalent orientations can still break interpolation and differentiation | Adjacent-sample dot products, lift continuity, sign-lift candidates |
| Long-way commanded paths | A representation path can rotate unnecessarily through the double cover | Commanded versus physical path evidence |
| Convention mismatches | Component order, frame direction, or rotation sense can silently invert meaning | Declared manifest plus independent matrix checks |
| Time and rate inconsistency | Duplicate time, decreasing time, or incompatible angular velocity undermines dynamics | Timestamp and body-rate obligations |
| Near-half-turn ambiguity | Numerical and sign choices become especially fragile near pi | Explicit near-pi findings without invented unique repairs |

Supported rule IDs and their exact claim boundaries are frozen in [docs/CLAIMS.md](docs/CLAIMS.md).

## Quick start

Install the released command-line tool with Rust 1.97 or newer:

```bash
cargo install quatopsy --locked
```

To build the repository instead, clone it and use the locked workspace:

```bash
git clone https://github.com/kabudu/quatopsy.git
cd quatopsy
cargo build --release --locked
```

Then run the included sign-discontinuity example:

```bash
./target/release/quatopsy analyze \
  --input fixtures/conformance/sign_alternating/input.csv \
  --manifest fixtures/conformance/sign_alternating/manifest.json \
  --report /tmp/quatopsy-report.json \
  --repairs-dir /tmp/quatopsy-repairs \
  --repro-dir /tmp/quatopsy-repro

./target/release/quatopsy view \
  --report /tmp/quatopsy-report.json \
  --input fixtures/conformance/sign_alternating/input.csv \
  --manifest fixtures/conformance/sign_alternating/manifest.json \
  --output /tmp/quatopsy-view
```

The fixture intentionally exits with code `1` because findings were produced. Open `/tmp/quatopsy-view/index.html` locally to inspect them.

Exit codes are stable: `0` pass, `1` findings, `2` refused, `3` error, and `64` usage error.

## A complete incident bundle

Use `investigate` when the work needs to be handed to another engineer or retained with its exact evidence:

```bash
./target/release/quatopsy investigate \
  --case-id sign-discontinuity-review \
  --input fixtures/conformance/sign_alternating/input.csv \
  --manifest fixtures/conformance/sign_alternating/manifest.json \
  --plan-problem fixtures/plan/spherical_rest_to_rest/problem.json \
  --control-problem fixtures/control/so3_rest_to_rest/problem.json \
  --output-dir /tmp/quatopsy-case

./target/release/quatopsy verify-evidence \
  --bundle /tmp/quatopsy-case
```

The no-clobber bundle preserves observed bytes, optional uninterpreted event/command/note context, canonical reports, reproducers, repairs, viewers, and separately analysed plan/control candidates. `quatopsy.evidence/1` binds relative paths, roles, sizes, and SHA-256 digests. It detects mutation but does not provide authenticated chain of custody.

## How it fits together

<picture>
  <source media="(max-width: 600px)" srcset="assets/brand/templates/diagram-workflow-narrow.svg">
  <img src="assets/brand/templates/diagram-workflow.svg" alt="Quatopsy local-first system architecture">
</picture>

The central invariant is simple: adapters, planners, controllers, and viewers never own diagnostic verdicts. They emit declared trajectories or presentation artifacts; only the conformance kernel produces `pass`, `findings`, `refused`, or `error`.

## Capability map

| Surface | Shipped capability | Boundary |
| --- | --- | --- |
| `analyze` | Deterministic quaternion trajectory rules and canonical JSON reports | Advisory diagnostics only |
| `repair` | Digest-bound normalisation and sign-lift candidates | Writes a separately named file; never overwrites source data |
| `view` | Responsive, offline forensic console | Displays report evidence; never recomputes verdicts |
| `adapt` | IDS Jason-1, ROS JSON, uncompressed MCAP JSON, SPICE CK type 3, and TUBIN STR inputs | Adapters declare provenance and cannot assign a result |
| `plan` | Torque-limited rest-to-rest candidates, wheel/thruster/CMG models, keep-out constraints, and bounded direct shooting | Candidate generation; global optimality is not claimed |
| `control` | Geometric SO(3) control, software GN&C, wheel allocation, SIL, host-CPU PIL, and loopback HIL | Software evidence only; physical actuator access is refused |
| `investigate` | Reproducible local incident evidence bundles | Recorded-file workflow; no live telemetry or command path |

## Design principles

- **Local-first:** no hosted control plane, account, database, analytics, or required runtime network.
- **Declared semantics:** frame, convention, component order, rotation sense, and time unit are inputs, not guesses.
- **Verdict isolation:** one canonical kernel owns report results.
- **Read-only evidence:** source trajectories are never modified; repairs and candidates are separately named.
- **Fail closed:** unsupported semantics, malformed values, exhausted limits, and unsafe hardware modes cannot become pass.
- **Reproducible:** reports bind tool version, numeric limits, input bytes, and manifests.

## Build, test, and package

```bash
./scripts/ci-local.sh
./scripts/package-local.sh dist
```

`ci-local.sh` is the authoritative implementation gate. It runs formatting, Clippy, all tests, adversarial checks, the million-sample performance budget, checksum packaging, licence inspection, brand validation, README/community checks, Cargo package validation, and release-presentation checks. GitHub-hosted CI runs the same gate for pull requests and protected `master` updates.

## Project status

Version `0.2.0` is early-stage, production-quality research software for local advisory evaluation. "Production-quality" describes the engineering discipline within Quatopsy's documented scope; it does not mean flight-qualified, safety-certified, independently validated, or supported by a production SLA. Cargo releases are published from reviewed tags, while standalone binaries remain unsigned. The evidence-based opening record is maintained in [docs/PUBLIC_OPENING_DECISION.md](docs/PUBLIC_OPENING_DECISION.md).

## Documentation

- Start with the [product specification](docs/PRODUCT_SPECIFICATION.md), [architecture](docs/ARCHITECTURE.md), and [incident workflow](docs/INVESTIGATION_WORKFLOW.md).
- Read the [report protocol](docs/REPORT_PROTOCOL.md), [spacecraft CSV profile](docs/SPACECRAFT_PROFILE.md), [plan protocol](docs/PLAN_PROTOCOL.md), and [control protocol](docs/CONTROL_PROTOCOL.md) for machine-facing contracts.
- Review the [soundness case](docs/SOUNDNESS_CASE.md), [control safety boundary](docs/CONTROL_SAFETY.md), [threat model](docs/THREAT_MODEL.md), and [frozen claims](docs/CLAIMS.md) before operational evaluation.
- See the [implementation plan](docs/IMPLEMENTATION_PLAN.md), [validation record](docs/VALIDATION.md), and [requirements traceability](docs/REQUIREMENTS_TRACEABILITY.md) for engineering evidence.
- The visual system and reusable assets are documented in [docs/BRAND_IDENTITY.md](docs/BRAND_IDENTITY.md).

## Community

Contributions that improve correctness, adapters, fixtures, documentation, accessibility, or bounded performance are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Use [SUPPORT.md](SUPPORT.md) for help routes and [SECURITY.md](SECURITY.md) for private vulnerability reporting. Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).

## Licence

Quatopsy is licensed under the [Apache License 2.0](LICENSE). Third-party fixture and brand-font provenance is recorded beside the relevant assets and in [NOTICE](NOTICE).
