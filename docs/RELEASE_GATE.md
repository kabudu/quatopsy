# M5 stop-ship audit

Audit date: 2026-08-15. Auditor role: engineering (owner). Scope: private `0.1.0` research release only.

| Gate | Disposition | Evidence |
| --- | --- | --- |
| Supported semantics | Pass | Conformance fixtures and oracles for the closed enabled rule set |
| Error or refusal cannot become pass | Pass | Fail-closed aggregation tests and incomplete/cancel paths |
| Repair equivalence | Pass | Independent matrix oracle on sign-lift; normalisation records magnitude |
| Bounded supported inputs | Pass | Streaming parse, compiled maxima, hostile-input tests |
| Source overwrite and partial output | Pass | Race-safe no-clobber, staged output-set rollback, cancellation cleanup, lifecycle tests |
| Credential, privacy, path, viewer, supply chain | Pass | Offline default, path redaction, CSP viewer, lockfile plus license allowlist |
| Protocol drift | Pass | Versioned manifest and report schemas; unknown-major viewer bundle plus exit 2 refusal |
| Deterministic local CI | Pass | `./scripts/ci-local.sh` recorded on each implementation PR |
| Public claims | Pass | `docs/CLAIMS.md` freeze; curated notes fail closed on prohibited phrases |
| Licence and provenance | Pass | Apache-2.0 `LICENSE`/`NOTICE`, crate metadata, package checksums |
| Name and patent | Pass for private visibility | Candidate name retained; no public opening; no patentability claim |
| Release metadata | Pass | Workspace version `0.1.0`, curated notes, changelog, `publish = false` |

## Legal name and patent disposition

`Quatopsy` remains a point-in-time candidate from the 2026-08-14 search. Similarity, trademark classes, domains, and international markets are not cleared. Patent surfaces in the prior-art matrix are recorded and are not a freedom-to-operate opinion.

This increment therefore authorises only a private GitHub Release of checksummed local artefacts. It does not authorise public repository visibility, crates.io, signed binaries, hosted CI, a website, or production support. Those remain distinct gates after legal review.

## Brand

Productisation is not approved. The restrained research identity in `docs/BRAND_IDENTITY.md` is the intended presentation. No canonical marks are produced or claimed complete.

## Residual accepted limitations

Kernel numeric behaviour is locked by `quatopsy.numeric/1` and fixture oracles, not by an external audit. Million-sample evidence is a synthetic identity series on the local CI host. Third-party flight telemetry is not redistributed.
