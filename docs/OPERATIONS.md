# Operations

## Deployment model

Quatopsy V1 is a local command-line binary and static local viewer. It has no daemon, hosted control plane, account, database, or required network. The Rust CLI owns the job lifecycle.

## Job lifecycle

1. Resolve explicit input and output targets without mutation.
2. Validate file type, size, limits, and output collision policy.
3. Snapshot and hash input and manifest bytes.
4. Parse into validated canonical samples or refuse.
5. Evaluate the closed rule registry with cancellation checks.
6. Generate repair proposals and bounded evidence.
7. Serialize into a temporary sibling output.
8. Flush, verify digest, and atomically commit.
9. Remove temporary state on cancellation or failure.

## Concurrency and backpressure

One job runs by default. Batch concurrency is explicit and bounded by jobs, memory budget, and open-file limit. Findings are capped per rule with a truncation record that prevents `pass`. Viewer geometry is downsampled offline with important intervals pinned.

## Cache

No cache is required initially. A future cache is content-addressed by the full analysis identity, integrity checked, bounded by size and age, removable, and bypassed by `--clean`. Cache failure falls back to fresh analysis or errors; it never supplies unverifiable success.

## Observability

Structured local logs contain job phase, durations, counts, limits, versions, and reason codes. Sample payloads and full paths are absent by default. Terminal output separates user findings from operational diagnostics. Exit codes match the report protocol.

## Recovery

Jobs are replayable from immutable inputs and configuration. Partial outputs are not committed. A repair is reproduced only from its source analysis identity and algorithm version. Downgrade and rollback retain open JSON reports. No backup service is needed because Quatopsy does not own source data.

## Capacity targets

The design target is 10 million samples maximum, one million samples under 10 seconds, and under 512 MiB resident memory on the registered reference machine. These are unverified targets until milestone M4 evidence exists. Browser limits may be lower and must be explicit.

## Incident response

Correctness incidents freeze affected public claims and releases, identify rule and report versions, publish affected scope and workarounds, add a regression fixture, and version changed semantics. Security incidents rotate release credentials if implicated, assess artefacts and dependencies, publish scoped remediation, and preserve forensic logs without exposing trajectory data. False acceptance has stop-ship priority over cosmetic or performance defects.

## Privacy and retention

Quatopsy has no server-side retention. Users own inputs, reports, repairs, and optional local logs. Removal documentation names every local path. Future telemetry or hosted operation requires a new architecture, privacy analysis, and explicit authorization.

