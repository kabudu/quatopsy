# Public-opening decision

## Decision

**Go decision accepted; M12 execution authorised on 2026-08-30.**

The private engineering evidence supports opening Quatopsy as a claim-bounded research repository. On 2026-08-30, the owner authorised public visibility, hosted CI, vulnerability reporting, protected release operations, crates.io publication, and changelog-derived GitHub Releases. Signed standalone binaries, a website, production support, certification, and hardware authority remain outside this authorisation.

## Evidence rubric

| Gate | Evidence | Disposition |
| --- | --- | --- |
| Coherent operator workflow | `quatopsy investigate` snapshots canonical or adapted telemetry, analyses it, emits reproducers and repairs, optionally evaluates plan/control candidates, and creates local viewers | Pass |
| Real telemetry boundary | Public TUBIN STR telemetry is adapted inside the bundle with provenance and no adapter verdict | Pass, limited public slice |
| Intended versus actual comparison | Canonical commanded-quaternion columns remain available to `QAT-UNWIND-001`; contextual command history stays distinct and uninterpreted | Pass |
| Integrity and reproducibility | `quatopsy.evidence/1`, per-file SHA-256, deterministic bundle identity, `verify-evidence`, repeated-run equality, and tamper rejection | Pass |
| Failure containment | Existing output refusal, invalid-candidate rollback, bounded inputs, cancellation propagation, symlink refusal, and no network or hardware interface | Pass |
| Operator review surface | Generated anomaly viewer inspected at desktop and 390 px; semantic regions, responsive width, evidence selection, playback state, and browser logs checked | Pass |
| Technical release gate | `./scripts/ci-local.sh` including the million-sample budget and all M10 lifecycle tests | Pass on the M10 reviewed head |
| Claims and provenance | Apache-2.0 repository, dependency licence inventory, source-attributed workflow rationale, and explicit non-claims | Pass for research visibility |
| External validation and demand | No independent expert reproduction, practitioner pilot, or measured adoption outcome | Not established; blocks those claims, not research visibility |
| Hosted CI | Authoritative local gate and SHA-pinned workflow | Enable and verify during M12 |

## Why the recommendation is conditional

M10 demonstrates a usable and bounded private investigation path, including one adapter over a public spacecraft telemetry excerpt. It does not demonstrate mission adoption, operational correctness on a complete flight incident, flight safety, certification, or market demand. Public copy must describe Quatopsy as research software and invite scrutiny rather than imply deployment heritage.

## Authorised execution

M12 changes repository visibility, enables hosted CI using the same local gate, enables private vulnerability reporting and repository protections, checks public issue and security metadata, previews the canonical release page, publishes the reviewed Cargo workspace, and verifies the public URLs after publication.

The M12 checklist remains open until the live repository, hosted run, crates.io packages, GitHub Release, and anonymous entry points are verified.

## M11 readiness addendum

M11 closes the repository-presentation and community-health gaps identified after the technical recommendation: a product-led README, branded architecture visual, contribution and conduct policies, private security reporting, support boundaries, issue and pull-request templates, and versioned GitHub About metadata. These improvements strengthen the conditional go recommendation but do not execute it. Visibility and hosted CI remain unchanged until explicit owner approval.

The About description and topics are now applied while private. GitHub Actions is explicitly disabled. GitHub permits public vulnerability reporting only for public repositories, so the opening sequence must change visibility, enable that reporting control immediately, and verify its logged-out path before announcing availability.

## Public identity disposition

On 2026-08-30, after reviewing the completed point-in-time name and ecosystem due diligence, the owner approved `Quatopsy` as the public open-source project identity. No further name-approval gate blocks public opening. This disposition does not itself change repository visibility or enable hosted CI.
