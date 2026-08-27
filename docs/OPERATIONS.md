# Operations

## Deployment model

Quatopsy V1 is a local command-line binary and static local viewer. It has no daemon, hosted control plane, account, database, or required network. The Rust CLI owns the job lifecycle. `quatopsy plan` is a separate candidate-generation job; it does not evaluate rules. `quatopsy control` is a separate closed-loop job; it does not evaluate rules or open a physical actuator. `quatopsy investigate` orchestrates those existing boundaries over recorded files and writes `evidence.json` last. Infeasible, refused, or handled failed investigations remove the output directory reserved by that invocation.

## Job lifecycle

1. Resolve explicit input and output targets without mutation.
2. Validate file type, size, limits, and output collision policy.
3. Snapshot and hash input and manifest bytes.
4. Parse into validated canonical samples or refuse.
5. Evaluate the closed rule registry with cancellation checks.
6. Generate repair proposals and bounded evidence.
7. Serialize every requested output into unique temporary sibling files.
8. Flush all staged files, commit them as one rollback-capable output set, and sync parent directories.
9. Restore overwritten files and remove new or temporary files on commit failure.

## Concurrency and backpressure

One job runs by default. Batch concurrency is explicit and bounded by jobs, memory budget, and open-file limit. Findings are capped per rule with a truncation record that prevents `pass`, and repro export refuses above 1,024 per-finding slices before staging. Viewer geometry is downsampled offline with extrema pinned; every canonical finding retains a bounded navigation link even when all finding endpoints cannot fit in the geometry budget. Browser DOM rendering is capped separately while the canonical report remains intact.

## Cache

No cache is required initially. A future cache is content-addressed by the full analysis identity, integrity checked, bounded by size and age, removable, and bypassed by `--clean`. Cache failure falls back to fresh analysis or errors; it never supplies unverifiable success.

## Observability

The CLI emits a concise terminal summary with result, analysis identity, sample count, per-rule states, repairs, and the diagnostic reason. It has no persistent or verbose logger. Sample payload rows are absent from default diagnostics; explicit paths may appear in filesystem errors. Exit codes match the report protocol.

## Recovery

Jobs are replayable from immutable inputs and configuration. Partial outputs are not committed. A repair is reproduced only from its source analysis identity and algorithm version. Downgrade and rollback retain open JSON reports. No backup service is needed because Quatopsy does not own source data.

Investigation bundles are replayable from the copied source and candidate problems. `quatopsy verify-evidence --bundle <dir>` must pass before handover. A directory without `evidence.json` is incomplete. A digest match detects mutation but is not proof of source authenticity or custody.

## Capacity targets

The supported analysis budget is one million samples in under 10 seconds and under 512 MiB peak RSS on the local CI host, excluding viewer generation. The compiled sample maximum remains 10 million. Evidence is the ignored release test `million_samples_meet_budget` run by `./scripts/ci-local.sh`. Browser geometry remains downsampled and is outside this budget.

## Incident response

Correctness incidents freeze affected public claims and releases, identify rule and report versions, publish affected scope and workarounds, add a regression fixture, and version changed semantics. Security incidents rotate release credentials if implicated, assess artefacts and dependencies, publish scoped remediation, and preserve forensic logs without exposing trajectory data. False acceptance has stop-ship priority over cosmetic or performance defects.

## Privacy and retention

Quatopsy has no server-side retention. Users own inputs, reports, repairs, optional local logs, and investigation bundles. A bundle contains copied telemetry plus any supplied event, command, and note files, so it inherits their highest sensitivity. Removal documentation names every local path. Future telemetry or hosted operation requires a new architecture, privacy analysis, and explicit authorization.
