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
| Prior art and patent surfaces | Pass | Point-in-time diligence recorded; no novelty or patentability claim |
| Release metadata | Pass | Workspace version `0.1.0`, curated notes, changelog, `publish = false` |

## Prior-art disposition

The recorded point-in-time search covers adjacent products, open-source projects, literature, and patent surfaces. The owner approved the Quatopsy identity for public open-source use on 2026-08-30. The project makes no novelty or patentability claim.

This historical increment authorises only a private GitHub Release of checksummed local artefacts. It does not authorise public repository visibility, crates.io, signed binaries, hosted CI, a website, or production support. Those remain distinct gates.

## Brand

This historical M5 audit preceded owner productisation approval. The later owner decision and canonical `quatopsy.brand/2` system are recorded in `docs/BRAND_IDENTITY.md` and ADR 0006. That later decision did not itself authorise public opening.

## Residual accepted limitations

Kernel numeric behaviour is locked by `quatopsy.numeric/1` and fixture oracles, not by an external audit. Million-sample evidence is a synthetic identity series on the local CI host. The CC BY 4.0 TUBIN STR excerpt added after M5 is redistributed with source and licence provenance. It is an adapter evaluation slice, not a mission reconstruction.

## M10 addendum

The 2026-08-27 M10 evidence and public-opening rubric are recorded in `docs/PUBLIC_OPENING_DECISION.md`. They support a conditional recommendation for claim-bounded research visibility. Repository visibility and hosted CI remain unchanged pending explicit owner execution approval.
