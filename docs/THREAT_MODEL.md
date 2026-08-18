# Threat model

## Assets

Source trajectory confidentiality and integrity; semantic correctness of findings; report and repair provenance; local filesystem integrity; availability of the developer workstation; release and dependency integrity; and clarity of safety limitations.

## Adversaries and failures

Inputs may be malicious, corrupted, ambiguous, extremely large, numerically adversarial, or crafted to flood findings. An adapter may misdeclare frames or units. A planner problem may declare an oversimplified model. A dependency or cache may be compromised. A user may mistake advisory output or a feasible plan for flight approval. A future integration may expose credentials or reinterpret verdicts.

## Trust boundaries

All input files, manifests, adapters, planner problems, report files, viewer bundles, and future integrations are untrusted at entry. The Rust kernel, rule registry, canonical serializer, and release artefacts form the primary trusted computing base. The browser is sandboxed and non-authoritative. The planner is outside verdict ownership. No production credentials are required.

## Security invariants

- Analysis never executes input content or follows embedded URLs.
- Paths are explicitly resolved; output is no-clobber and atomic by default.
- Symlinks and special files are detected before writes.
- Parsers enforce byte, row, field, numeric, time, memory, and finding limits.
- Errors, timeouts, unsupported cases, and partial results never become pass.
- Reports bind evidence to immutable input and policy digests.
- Default operation performs no network access or telemetry.
- Viewer content is escaped, has a restrictive content security policy, and loads no remote resources.

## Controls

Use memory-safe Rust, bounded streaming parse, checked arithmetic, finite-value validation, path and Unicode tests, dependency locking and audit, reproducible builds where supported, least-privilege release credentials, atomic writes, explicit overwrite flags, cancellation cleanup, fuzzing, mutation tests, and canonical digest verification.

Archives, ROS bags, MCAP, and SPICE files are handled only by isolated adapters with record, message, compression, and size limits. Compressed MCAP chunks and nested chunks are refused. SPICE reading is limited to little-endian IEEE DAF/CK type 3 discrete pointing. These formats do not enter core CSV parsing. Planner problems are bounded JSON with unknown fields refused; generated trajectories enter the kernel only as ordinary CSV plus manifest.

## Privacy

Reports include only evidence needed for a finding and can redact file paths. Crash logs exclude samples by default. Temporary files use restrictive permissions. Browser storage is disabled unless a later version documents a local-only preference store. No cloud upload exists in the initial architecture.

## Supply chain

Dependencies are minimised, pinned, licensed, audited, and reviewed for unsafe code. Build metadata records compiler and dependency lock digest. Releases require reproducible or independently cross-checked artefact digests and a secure credential path. Hosted CI remains disabled until explicitly approved.

## Residual risks

Memory safety does not guarantee mathematical correctness. Floating-point boundary behaviour, convention declarations, misleading user interpretation, maliciously valid worst-case inputs, and compromised build tooling remain. These receive conformance oracles, explicit refusals, resource bounds, copy controls, and release gates rather than hidden assurances.

## Out of scope

Flight certification, live actuator command, classified-data handling accreditation, protection against a fully compromised host, and correctness of undeclared external adapters are outside V1.

