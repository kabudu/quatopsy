# Million-sample budget

The asserted budget is 1,000,000 samples analysed in under 10 seconds with peak RSS under 512 MiB, excluding viewer generation. The check is `cargo test --release --locked -p quatopsy-core --test million -- --ignored` and is invoked from `./scripts/ci-local.sh`.

The workload is a synthetic increasing-time identity series generated in process. It exercises ingest, the closed rule registry, and report assembly at the documented sample count. It is not a substitute for a named laboratory reference machine or for third-party flight telemetry.

Local checksum packaging is `scripts/package-local.sh`. It copies the release CLI and writes `SHA256SUMS` plus `PROVENANCE.txt`. It does not sign artefacts or publish crates. GitHub Release publication is a separate fail-closed script.

## M6-M8 bounded-work contract

The numerical planner uses 17 nodes, at most 40 Levenberg-Marquardt iterations, bounded actuator counts, a duration cap, and at most 100,000 emitted samples. The controller accepts at most 100,000 cycles. Delay lookup and guidance interpolation are logarithmic in retained samples, gain schedules are limited to 1,024 entries, navigation audit growth is linear in cycles, and worker messages are limited to 1 MiB with a five-second response deadline. Cancellation is checked inside planner iterations, controller cycles, and campaign trials.

These are algorithmic and protocol bounds, not target-processor WCET evidence. The repository continues to refuse hard-real-time and flight-processor qualification claims.
