# Security policy

## Supported versions

Security fixes target the current `0.1.x` line on `master`. Older snapshots and unmerged branches are not supported.

## Report a vulnerability privately

Do not open a public issue for a suspected vulnerability. Once the repository is public and private vulnerability reporting has been enabled, use [GitHub private vulnerability reporting](https://github.com/kabudu/quatopsy/security/advisories/new) and include:

- the affected command, protocol, or file path;
- the smallest safe reproducer you can provide;
- expected and observed behavior;
- security impact and preconditions;
- whether untrusted telemetry, filesystem state, or generated viewer content is involved.

Do not attach operational telemetry, credentials, export-controlled material, or sensitive incident bundles unless the repository owner has explicitly agreed on a suitable transfer route. A synthetic reproducer is strongly preferred.

The maintainer will acknowledge a complete report when practicable, investigate it privately, and coordinate disclosure after a fix is available. No response-time or remediation-time SLA is promised.

While the repository remains private, external reporting is not available. Existing private collaborators should ask the repository owner to open a draft security advisory through an established private channel. Enabling public vulnerability reporting is a required step in the separately authorised public-opening change.

## Security boundary

Quatopsy is a local advisory tool. It intentionally has no service, account, runtime analytics, remote command path, or physical actuator interface. Relevant security surfaces include hostile telemetry and manifests, path handling, partial output, viewer content, evidence-bundle integrity, resource exhaustion, dependency provenance, and misleading safety claims.

`quatopsy verify-evidence` detects mutation against a bundle manifest. It does not authenticate the original capture, signer, operator, or chain of custody.

The [threat model](docs/THREAT_MODEL.md), [control safety boundary](docs/CONTROL_SAFETY.md), and [release policy](docs/RELEASE.md) describe supported controls and residual risks.
