# Release policy

## Release model

Quatopsy uses one lockstep Semantic Version across nine Cargo packages. The installable package is `quatopsy`; the supporting packages are `quatopsy-schema`, `quatopsy-oracle`, `quatopsy-nav`, `quatopsy-guidance`, `quatopsy-adapt`, `quatopsy-core`, `quatopsy-plan`, and `quatopsy-control`. Internal dependencies carry both an exact registry version and a local workspace path.

The repository is the source of truth. `CHANGELOG.md` follows Keep a Changelog 1.1.0, retains an empty `[Unreleased]` section after each release, uses ISO dates, and links every release to its Git history. GitHub Release notes are rendered from the matching changelog section instead of being maintained separately.

## Quality gates

`./scripts/ci-local.sh` is the authoritative implementation gate locally and on GitHub-hosted CI. It runs formatting, Clippy, the complete workspace tests, CLI smoke coverage, the million-sample budget, local checksum packaging, supply-chain inspection, brand and community validation, the release contract, and a fail-closed publication check.

A release stops for any unresolved critical or high correctness flaw in supported semantics; error or refusal capable of becoming pass; repair that lacks independent equivalence evidence; unbounded supported input path; source overwrite or partial-output risk; credential, privacy, path, viewer-content, or supply-chain exposure; incompatible protocol drift; failing local or hosted CI; unsupported public claim; missing licence or provenance; material patent concern; package drift; changelog drift; or inconsistent release metadata.

## Preparing a release

Normal changes add human-readable entries under `[Unreleased]`. The `Prepare release` workflow accepts a stable `major.minor.patch` version, runs `scripts/release.py prepare`, updates `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`, executes the authoritative gate, and opens a `codex/release-vX.Y.Z` pull request. It never tags or publishes from an unreviewed branch.

The preparation script rejects an empty `[Unreleased]` section, non-increasing versions, malformed dates, inconsistent internal package versions, missing changelog links, and non-SemVer release identifiers.

## Publishing a release

After the release pull request is reviewed and merged, an annotated `vX.Y.Z` tag on `master` triggers `.github/workflows/release.yml`. The workflow binds the tag to the Cargo workspace and changelog, proves the tagged commit is contained in `master`, reruns the authoritative gate, and publishes packages in dependency order through `scripts/publish-crates.sh`.

Publication is idempotent. If a package version already exists, the workflow requires its crates.io checksum to match the locally packaged crate. A mismatch stops the release. New packages are polled until crates.io exposes the expected checksum before dependent packages proceed. The GitHub Release is created or refreshed only after all Cargo packages are verified.

The release workflow requires both `QUATOPSY_RELEASE_AUTHORIZE=1`, which is set only inside the tag workflow, and the repository `CARGO_REGISTRY_TOKEN` secret. The token is exposed only to the publication step and is never written to the repository or artifacts. Local publication without both controls fails closed.

## Public repository controls

Pull requests and `master` run the same hosted `quality` job. `master` requires that status, blocks force pushes and deletion, and retains review through pull requests. Dependency updates cover Cargo and GitHub Actions. Public vulnerability reporting is enabled, and security reports follow `SECURITY.md`.

Public visibility, hosted CI, and Cargo publication do not broaden the product safety boundary. Quatopsy remains local advisory software. Standalone binary signing, a website, production support, physical hardware, hard real-time qualification, certification, and orbit determination remain separate or refused capabilities.
