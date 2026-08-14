# Release policy

## Current state

Quatopsy has an M4 spacecraft CSV profile, local CLI, viewer, and checksummed local package script. It has no product release, hosted CI, signed binaries, or production support. The private repository uses `./scripts/ci-local.sh` as the authoritative local CI gate for every milestone and pull request.

Hosted CI is disabled by policy while the repository is private. Adding or enabling hosted CI requires explicit user approval at the documented public-opening or product-release gate. Absent hosted checks are policy-compliant, not passing hosted CI.

## Stop-ship gates

A release stops for any unresolved critical or high correctness flaw in supported semantics; error or refusal capable of becoming pass; repair that lacks independent equivalence evidence; unbounded supported input path; source overwrite or partial-output risk; credential, privacy, path, viewer-content, or supply-chain exposure; incompatible protocol drift; missing deterministic local CI evidence; unsupported public claim; missing licence/provenance; material name or patent concern requiring legal disposition; or inconsistent release metadata.

External independent validation, expert challenge, practitioner interviews, pilots, adoption cohorts, customer discovery, and ecosystem ranking are optional. They do not block implementation, product completion, publication, or release. Their absence blocks only claims that require those forms of evidence.

## Release gates

1. Supported rule semantics pass conformance, mutation, adversarial, deterministic, and portability checks.
2. Resource limits, atomic output, cancellation, rollback, downgrade, removal, and privacy sinks pass E2E tests.
3. Dependencies, licences, lockfiles, build provenance, and artefact digests pass supply-chain review.
4. Traceability maps every supported requirement to behavioural evidence.
5. Public claims match evidence and state explicit non-claims.
6. Name/mark and relevant patent surfaces have the required owner/legal disposition for the intended visibility and jurisdictions.
7. Installation artefacts, support posture, compatibility, and incident process agree with documentation.
8. The release presentation contract below passes preview and post-publication checks.

## Private repository delivery

Implementation increments begin on an updated clean `master`, use scoped `codex/` branches, run the exact local CI command at the final reviewed commit, record its result in the pull request, receive review, squash-merge, fast-forward local `master`, and delete the merged local branch. No GitHub Actions workflow is added without the explicit approval gate.

## Curated release notes

Versioned curated release notes will live under `.github/release-notes/vX.Y.Z.md` once a real release is authorised. Release automation must fail closed if the matching curated title or body is missing, malformed, mismatched to the tag, or contains prohibited claims. It must never fall back to a raw changelog body.

The release title format is `Quatopsy vX.Y.Z: <short human theme>`. The body contains one opening summary, three to five material changes, explicit claim or compatibility boundaries, one primary install path, and links to detailed evidence and the changelog.

Do not hard-wrap release prose at a fixed column. Each prose paragraph and list item occupies one physical source line. The body must not duplicate the release title, add a redundant `Release Notes` heading, paste a raw changelog/date heading, enumerate automatically rendered assets, or expose an internal implementation inventory.

`CHANGELOG.md` remains package history and is linked rather than pasted into the public release.

## Presentation verification

Before publication, create a rendered preview of the exact title and body at desktop and narrow widths. Check hierarchy, list indentation, code fences, wrapping, links, install command, placeholder and prohibited-claim scans, and asset behaviour. After publication, inspect the canonical release URL and correct metadata immediately if it differs from the approved preview.

Local CI and release automation scan all tracked text and release metadata and reject Unicode U+2014. Canonical release notes are also checked for hard wrapping and required content.

## Visibility and publication

Repository visibility remains private until the owner explicitly approves public opening. Public repository visibility, hosted CI, packages, binaries, a GitHub Release, website deployment, and production support are distinct gates and authorisations. No release credential is stored in the repository.

## Brand gate

A restrained research presentation may accompany a claim-bounded release. Claiming complete productisation requires separate owner approval, legal name/mark review, complete canonical assets and manifests, deterministic exports, accessibility and cross-channel validation, and governance. Maturity status remains an overlay and never changes the canonical identity.
