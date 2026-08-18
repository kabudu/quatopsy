# Agent workflow

Quatopsy is a private local-first research repository. Implementation follows this loop for every completed roadmap increment. Hosted CI is disabled until the owner explicitly approves it at the public-opening or product-release gate documented in `docs/RELEASE.md`.

## Delivery loop

1. Start from an up-to-date, clean `master` branch.
2. Select the next unchecked item from `docs/IMPLEMENTATION_PLAN.md` and define its acceptance evidence before editing.
3. Create a scoped feature branch `codex/<short-item-name>`.
4. Implement only that increment. Update tests, fixtures, traceability, risk, changelog, and roadmap state together.
5. Run the authoritative local gate:

```bash
./scripts/ci-local.sh
```

6. Commit only intended files, push the branch, and open a pull request against `master`.
7. Record the exact local CI command and result in the pull request. Absent hosted checks are policy-compliant; they are not passing hosted CI.
8. Inspect the remote diff, reviews, and mergeability. Fix every material finding on the branch and rerun `./scripts/ci-local.sh`.
9. Squash-merge only when the reviewed head is mergeable and local CI passed at that commit.
10. Fast-forward local `master` to the merged commit, verify the merged state, delete the merged local feature branch, and continue with the next unchecked roadmap item.

Do not create GitHub Actions workflows, publish packages, open the repository, or claim safety, novelty, or independent validation without the documented authorization and evidence.

## Local CI

`./scripts/ci-local.sh` is the single authoritative quality gate while the repository is private. The script is non-interactive, fail-fast, deterministic, and offline-capable after dependency bootstrap. It runs formatting, Clippy, tests, documentation and metadata checks, a Unicode U+2014 scan, and diff hygiene.

## Claim and text rules

- Do not mark a roadmap box complete without behavioural evidence.
- Do not introduce Unicode U+2014 in repository text, release metadata, or PR copy.
- Advisory diagnostics are not flight approval. Public copy must keep that boundary.
- Original trajectory inputs are read-only. Repair candidates, when implemented, are separately named.

## Milestone notes

M1 owns the conformance kernel, canonical report protocol, and closed V1 rule registry. M2 owns repair candidates, digest-bound apply, and privacy-preserving repro slices. M3 owns the non-authoritative local static viewer. M4 owns the frozen spacecraft CSV profile, commanded-path comparison, the million-sample budget, and local checksum packaging. M5 owns the private `0.1.0` release policy, frozen claims, and fail-closed GitHub Release path. M6 owns the offline torque-limited candidate generator. M7 owns the SIL attitude controller plus host-CPU PIL, loopback HIL, and a fail-closed hardware-use gate. Public opening, hosted CI, crates.io, and full brand productisation remain later distinct gates.
