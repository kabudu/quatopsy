# Public-opening decision

## Decision

**Conditional go recommendation, execution not authorised.**

As of 2026-08-27, the private engineering evidence supports opening Quatopsy as a claim-bounded research repository. The repository must remain private until the owner separately authorises the visibility change and hosted CI activation. This decision does not authorise crates.io, signed binaries, a website, production support, or a GitHub Release.

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
| Hosted CI | Intentionally absent while private | Enable only in the separately authorised public-opening change |

## Why the recommendation is conditional

M10 demonstrates a usable and bounded private investigation path, including one adapter over a public spacecraft telemetry excerpt. It does not demonstrate mission adoption, operational correctness on a complete flight incident, flight safety, certification, or market demand. Public copy must describe Quatopsy as research software and invite scrutiny rather than imply deployment heritage.

## Authorised next action if the owner accepts

Create a separate public-opening increment that changes repository visibility, enables minimal hosted CI using the same local gate, checks public issue/security/contact metadata, previews the canonical release page, and verifies the public URL after publication. Do not combine package publication, binary signing, website deployment, or production-support promises with that change.

Until that explicit approval, `REL-2`, the M9 public-opening boxes, and post-publication verification remain open.

## M11 readiness addendum

M11 closes the repository-presentation and community-health gaps identified after the technical recommendation: a product-led README, branded architecture visual, contribution and conduct policies, private security reporting, support boundaries, issue and pull-request templates, and versioned GitHub About metadata. These improvements strengthen the conditional go recommendation but do not execute it. Visibility and hosted CI remain unchanged until explicit owner approval.
