# Release policy

## Current state

Quatopsy `0.1.0` is a private research release: local CLI, viewer, frozen spacecraft CSV profile, checksummed local packaging, Apache-2.0 licence, and curated GitHub Release notes. It has no public repository visibility, hosted CI, signed binaries, crates.io publication, website, or production support. The private repository uses `./scripts/ci-local.sh` as the authoritative local CI gate for every milestone and pull request.

Hosted CI is disabled by policy while the repository is private. Adding or enabling hosted CI requires explicit user approval at a later public-opening gate. Absent hosted checks are policy-compliant, not passing hosted CI.

## Stop-ship gates

A release stops for any unresolved critical or high correctness flaw in supported semantics; error or refusal capable of becoming pass; repair that lacks independent equivalence evidence; unbounded supported input path; source overwrite or partial-output risk; credential, privacy, path, viewer-content, or supply-chain exposure; incompatible protocol drift; missing deterministic local CI evidence; unsupported public claim; missing licence/provenance; material name or patent concern requiring legal disposition; or inconsistent release metadata.

External independent validation, expert challenge, practitioner interviews, pilots, adoption cohorts, customer discovery, and ecosystem ranking are optional. They do not block implementation, product completion, publication, or release. Their absence blocks only claims that require those forms of evidence.

## Release gates

1. Supported rule semantics pass conformance, mutation, adversarial, deterministic, and portability checks.
2. Resource limits, output-set rollback, cancellation, local binary removal, report compatibility, and documented privacy sinks pass E2E tests. Cross-version executable downgrade testing begins when a second supported version exists.
3. Dependencies, licences, lockfiles, build provenance, and artefact digests pass supply-chain review.
4. Traceability maps every supported requirement to behavioural evidence.
5. Public claims match evidence and state explicit non-claims.
6. Name/mark and relevant patent surfaces have the required owner/legal disposition for the intended visibility and jurisdictions.
7. Installation artefacts, support posture, compatibility, and incident process agree with documentation.
8. The release presentation contract below passes preview and post-publication checks.

## Private repository delivery

Implementation increments begin on an updated clean `master`, use scoped `codex/` branches, run the exact local CI command at the final reviewed commit, record its result in the pull request, receive review, squash-merge, fast-forward local `master`, and delete the merged local branch. No GitHub Actions workflow is added without the explicit approval gate.

## Curated release notes

Versioned curated release notes live under `.github/release-notes/vX.Y.Z.md`. `scripts/check-release-notes.py` fails closed if the matching curated title or body is missing, malformed, mismatched to the workspace version, hard-wrapped, or contains prohibited claims. `scripts/publish-github-release.sh` refuses unless `QUATOPSY_RELEASE_AUTHORIZE=1` and the GitHub repository is still private. It never falls back to a raw changelog body and never publishes crates.

The release title format is `Quatopsy vX.Y.Z: <short human theme>`. The body contains one opening summary, three to five material changes, explicit claim or compatibility boundaries, one primary install path, and links to detailed evidence and the changelog.

Do not hard-wrap release prose at a fixed column. Each prose paragraph and list item occupies one physical source line. The body must not duplicate the release title, add a redundant `Release Notes` heading, paste a raw changelog/date heading, enumerate automatically rendered assets, or expose an internal implementation inventory.

`CHANGELOG.md` remains package history and is linked rather than pasted into the public release.

## Presentation verification

Before publication, create a rendered preview of the exact title and body at desktop and narrow widths. Check hierarchy, list indentation, code fences, wrapping, links, install command, placeholder and prohibited-claim scans, and asset behaviour. After publication, inspect the canonical release URL and correct metadata immediately if it differs from the approved preview.

Local CI and release automation scan all tracked text and release metadata and reject Unicode U+2014. Canonical release notes are also checked for hard wrapping and required content.

## Credentials path

Release credentials are the owner's local GitHub CLI authentication. No token, signing key, or crates.io credential is stored in the repository. `scripts/publish-github-release.sh` is the only GitHub Release entry point and requires `QUATOPSY_RELEASE_AUTHORIZE=1`. crates.io remains blocked by workspace `publish = false`.

## Visibility and publication

Repository visibility remains private until the owner explicitly approves public opening. Public repository visibility, hosted CI, crates.io, signed binaries, website deployment, and production support are distinct gates and authorisations. A private GitHub Release of checksummed local artefacts is authorised only through `scripts/publish-github-release.sh`. No release credential is stored in the repository.

## Brand gate

A restrained research presentation may accompany a claim-bounded release. The owner approved productisation of the enduring brand on 2026-08-19 and declined a trademark filing. Canonical assets are `quatopsy.brand/1`. Public opening, hosted CI, crates.io, signed binaries, website deployment, and production support remain distinct gates. Maturity status remains an overlay and never changes the canonical identity.
