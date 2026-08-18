# Quatopsy

Quatopsy is a candidate local-first product for diagnosing quaternion orientation trajectories before or after they drive a spacecraft, robot, simulator, or animation system. It combines a deterministic trajectory linter with a responsive forensic investigation console that links physical motion in `SO(3)` to a projected lift in `S^3`, canonical evidence, and unapplied repair candidates.

## Candidate contribution

The narrow candidate contribution is a representation-aware diagnostic report that detects, explains, quantifies, reproduces, and proposes reversible repairs for topological and convention defects in sampled orientation trajectories. The first vertical is offline spacecraft attitude data in a documented CSV profile.

Quatopsy does not claim to invent quaternions, sign canonicalisation, shortest-path interpolation, unwinding analysis, attitude visualisation, or quaternion trajectory smoothing. Novelty, safety, production readiness, and physical cost estimates remain separate evidence-gated claims.

## Status

M5 is a private `0.1.0` research release with frozen claims and checksummed local artefacts. No safety qualification, public opening, signed publication, crates.io package, hosted CI, full brand system, or independent external validation is claimed.

The learning-laboratory concept is a separate future project and is not part of Quatopsy.

## Local use

```bash
./scripts/ci-local.sh
cargo run --bin quatopsy -- analyze \
  --input fixtures/conformance/clean_slew/input.csv \
  --manifest fixtures/conformance/clean_slew/manifest.json \
  --report /tmp/quatopsy-report.json

cargo run --bin quatopsy -- repair \
  --report /tmp/quatopsy-report.json \
  --input fixtures/conformance/sign_alternating/input.csv \
  --manifest fixtures/conformance/sign_alternating/manifest.json \
  --repair-id repair:sign-lift:1 \
  --output /tmp/quatopsy-repaired.csv

cargo run --bin quatopsy -- view \
  --report /tmp/quatopsy-report.json \
  --input fixtures/conformance/sign_alternating/input.csv \
  --manifest fixtures/conformance/sign_alternating/manifest.json \
  --output /tmp/quatopsy-view

cargo run --bin quatopsy -- adapt \
  --format tubin-str \
  --input fixtures/public/tubin_str/source.csv \
  --output-dir /tmp/quatopsy-tubin
```

`quatopsy adapt --format mcap-json` and `--format spice-ck` convert uncompressed MCAP JSON poses and little-endian CK type 3 kernels into the same canonical CSV and manifest. They never assign a report `result`.

```bash
cargo run --bin quatopsy -- plan \
  --problem fixtures/plan/spherical_rest_to_rest/problem.json \
  --output-dir /tmp/quatopsy-plan
```

`quatopsy plan` writes a candidate CSV, declared manifest, and `plan.json`. Residuals are checked by an independent oracle. The planner never assigns a report `result`. Run `analyze` on the generated files to obtain the only verdict.

```bash
cargo run --bin quatopsy -- control \
  --problem fixtures/control/so3_rest_to_rest/problem.json \
  --output-dir /tmp/quatopsy-control
```

`quatopsy control` writes a closed-loop CSV, declared manifest, and `control.json`. `execution` may be software-in-the-loop, host-CPU processor-in-the-loop, or loopback hardware-in-the-loop. Optional declared plant models add command-to-torque lag, residual dipole, gravity-gradient, gyro ARW, and star-tracker delay. An independent oracle monitor inhibits commands. The controller never assigns a report `result` and never opens a physical actuator. The systems-safety programme is [Control safety](docs/CONTROL_SAFETY.md).

`quatopsy analyze --repro-dir <dir>` writes one context-bounded subdirectory per finding when a report has multiple findings. Each contains `slice.csv`, `manifest.json`, and `provenance.json`; a single finding uses those filenames directly in the requested directory. Export refuses above the compiled 1,024-slice disk-work limit without committing any requested output.

Exit codes: `0` pass, `1` findings, `2` refused, `3` error, `64` usage error.

## Documentation

- [Frozen claims](docs/CLAIMS.md)
- [Release gate audit](docs/RELEASE_GATE.md)
- [Product specification](docs/PRODUCT_SPECIFICATION.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Soundness case](docs/SOUNDNESS_CASE.md)
- [Report protocol](docs/REPORT_PROTOCOL.md)
- [Plan protocol](docs/PLAN_PROTOCOL.md)
- [Control protocol](docs/CONTROL_PROTOCOL.md)
- [Control safety](docs/CONTROL_SAFETY.md)
- [Implementation plan](docs/IMPLEMENTATION_PLAN.md)
- [Novelty and prior art](docs/NOVELTY.md)
- [Validation](docs/VALIDATION.md)
- [Release policy](docs/RELEASE.md)
- [Spacecraft CSV profile](docs/SPACECRAFT_PROFILE.md)
- [Requirements traceability](docs/REQUIREMENTS_TRACEABILITY.md)

## Name audit

`Quatopsy` is a point-in-time candidate, searched on 2026-08-14 across general web results, GitHub repository names, npm, PyPI, and crates.io. No exact product or package collision was found. This is not trademark clearance, domain reservation, patent clearance, or a guarantee of worldwide availability. Legal review remains required before public productisation.
