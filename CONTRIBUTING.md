# Contributing to Quatopsy

Thank you for helping make orientation-trajectory diagnostics more trustworthy. Quatopsy welcomes focused contributions to correctness, adapters, fixtures, documentation, accessibility, and bounded performance.

## Before you begin

- Read the [architecture](docs/ARCHITECTURE.md), [frozen claims](docs/CLAIMS.md), and [soundness case](docs/SOUNDNESS_CASE.md).
- Search existing issues before proposing substantial work.
- Open an issue before a large protocol, rule, solver, or safety-boundary change so the design can be reviewed before implementation.
- Report security issues privately as described in [SECURITY.md](SECURITY.md).

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Development setup

Quatopsy requires Rust 1.97 or newer and Python 3. After cloning:

```bash
cargo build --workspace --locked
./scripts/ci-local.sh
```

The local CI script is the authoritative gate. It is deterministic and offline-capable after dependencies have been fetched.

## Contribution boundaries

Keep these invariants intact:

- The conformance kernel is the only owner of report results.
- Adapters, planners, controllers, and viewers cannot assign or reinterpret a verdict.
- Original trajectory inputs remain read-only. Repairs and generated candidates are separately named.
- Unsupported semantics and exhausted resource limits fail closed.
- No physical actuator interface, hard real-time claim, flight approval, or certification claim may be introduced.
- Public protocol changes require versioning, compatibility analysis, fixtures, tests, and documentation.
- Repository text must not contain Unicode U+2014.

New rules need a stable rule ID, mathematical obligation, applicability and refusal behavior, conformance fixtures, an independent oracle where practical, mutation coverage, report-protocol documentation, traceability, and claim review.

New adapters must preserve source provenance, declare all conventions explicitly, remain outside verdict ownership, bound input work, and include a redistributable or synthetic fixture with its licence recorded.

## Pull requests

1. Branch from an up-to-date `master` using a descriptive name.
2. Keep the change focused and include behavioral tests for user-visible behavior.
3. Update documentation, traceability, risks, and changelog when the public contract changes.
4. Run `./scripts/ci-local.sh` at the final commit.
5. Complete the pull-request template with the exact validation command and result.

Pull requests are squash-merged after material correctness, security, compatibility, performance, and documentation findings are resolved. Hosted checks may be absent while the repository is private; that absence is not a passing check.

## Commit and review quality

Use clear imperative commit subjects. Avoid unrelated formatting or generated artifacts. Do not commit telemetry, credentials, private incident bundles, proprietary data, or fixtures without redistribution rights.

Reviews prioritize correctness, safety boundaries, source integrity, deterministic behavior, bounded resource use, compatibility, and maintainability. A contributor may be asked to split an oversized change or provide stronger evidence before merge.

## Licence

Unless explicitly stated otherwise, contributions are submitted under the repository's [Apache License 2.0](LICENSE). You must have the right to contribute every file you add.
