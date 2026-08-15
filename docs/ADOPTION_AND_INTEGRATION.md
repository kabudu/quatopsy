# Adoption and integration

## Personas

The primary evaluator is a spacecraft guidance, navigation, and control engineer with an attitude CSV export. Maintainers integrate Quatopsy into local analysis scripts or private CI. Later adapter authors translate ROS 2 bags, MCAP, SPICE attitude kernels, simulator logs, or animation data into the canonical profile.

## Staged adoption

1. Local evaluation: run on copied fixtures with no credentials, network, or source mutation.
2. Shadow mode: analyse recorded trajectories and retain reports without gating work.
3. Advisory mode: surface findings in developer review with explicit disposition.
4. Selective enforcement: fail CI on named stable rules and a brownfield baseline.
5. Required mode: enforce a version-pinned policy with scoped, expiring overrides.

An override records authority, rule, input scope, reason, issue link where available, creation time, expiry, and approving identity. Baselines acknowledge existing findings; they do not convert them into passes.

## Canonical integration boundary

Adapters produce the canonical CSV and manifest plus provenance. They cannot assign Quatopsy verdicts, suppress core rules, or reinterpret findings. The CLI remains usable without adapters. Generic stdin/file ingestion is the fallback.

## Ecosystem matrix

| Ecosystem | Existing capability | Planned Quatopsy boundary | Status |
| --- | --- | --- | --- |
| CSV | Universal export | Canonical V1 profile | First release |
| JSON report | Tool-neutral evidence | Stable report protocol | First release |
| ROS 2 / MCAP | Pose and transform logs | Offline converter | Planned |
| SPICE CK | Spacecraft attitude kernels | Read-only converter with frame provenance | Planned |
| Foxglove | 3D pose and plots | Report/marker export, no verdict reinterpretation | Candidate |
| Basilisk | Simulation and Vizard | Fixture/report adapter | Candidate |
| SciPy / NumPy | Rotation analysis | Reference-oracle and import helpers | Candidate |

## Repository and CI workflows

The CLI accepts explicit file lists and emits one report per analysis unit. Monorepos define units in versioned configuration with bounded parallelism. Forks and offline environments work after dependency bootstrap. Local and CI runs use identical engine, rule, limit, and numeric profiles.

Hosted CI is disabled while the repository is private. Repository-owned local CI is authoritative. Public opening remains a distinct gate and is not authorised by the private `0.1.0` GitHub Release.

`INT-3` adoption modes (advisory, selective, required, scoped overrides) are deferred from V1. Operators may treat exit codes as advisory or required in their own scripts. Quatopsy does not ship an override or baseline engine.

## Compatibility and identity

CLI flags, exit codes, schema major versions, rule IDs, and canonical input profiles are public compatibility surfaces. Install and upgrade documentation must pin supported versions. Downgrade retains reports but may refuse newer schemas. Removal consists of deleting the binary, configuration, optional cache, and generated reports; source inputs remain untouched and reports use open JSON.

## Performance budgets

The first target is one million samples analysed in under 10 seconds and under 512 MiB resident memory, excluding viewer generation. Local CI asserts this budget with a generated identity series. Adapters must stream and respect the same limits.

## Privacy and telemetry

No telemetry is collected by default. Analysis is local and network-free. Logs exclude sample payloads and paths unless verbose mode is explicitly selected. Any future usage metrics require opt-in, schema disclosure, bounded retention, deletion, and a separate threat-model update.

## Rollback and removal

Rule upgrades are version-pinned and reversible. A clean analysis bypasses caches. Repair candidates are new files and can be discarded. CI enforcement can step back from required to advisory without changing historical reports.

## Optional adoption evidence

Interviews, pilots, customer discovery, cohort measurement, and ecosystem ranking are optional post-release evidence. They do not gate architecture freeze, implementation, completion, or release. Without them, Quatopsy must not claim validated demand, market fit, practitioner preference, or measured onboarding reduction.

