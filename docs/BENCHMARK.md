# Million-sample budget

The asserted budget is 1,000,000 samples analysed in under 10 seconds with peak RSS under 512 MiB, excluding viewer generation. The check is `cargo test --release --locked -p quatopsy-core --test million -- --ignored` and is invoked from `./scripts/ci-local.sh`.

The workload is a synthetic increasing-time identity series generated in process. It exercises ingest, the closed rule registry, and report assembly at the documented sample count. It is not a substitute for a named laboratory reference machine or for third-party flight telemetry.

Local checksum packaging is `scripts/package-local.sh`. It copies the release CLI and writes `SHA256SUMS` plus `PROVENANCE.txt`. It does not sign artefacts or publish crates. GitHub Release publication is a separate fail-closed script.

## M6-M8 bounded-work contract

The numerical planner uses 17 nodes, at most 40 Levenberg-Marquardt iterations, bounded actuator counts, a duration cap, and at most 100,000 emitted samples. The controller accepts at most 100,000 cycles. Delay lookup and guidance interpolation are logarithmic in retained samples, gain schedules are limited to 1,024 entries, navigation audit growth is linear in cycles, and worker messages are limited to 1 MiB with a five-second response deadline. Cancellation is checked inside planner iterations, controller cycles, and campaign trials.

These are algorithmic and protocol bounds, not target-processor WCET evidence. The repository continues to refuse hard-real-time and flight-processor qualification claims.

## Investigation workflow budget

`python3 scripts/check-workflow-budget.py target/release/quatopsy` exercises the public CLI with one million nominal samples through analyze/view/investigate/verify-evidence, 50,000 samples with dense sign findings through analyze/view, and 100,000 samples carrying rate and matrix columns through the full workflow. Each stage has a 60-second subprocess deadline; cumulative child peak RSS must stay below 512 MiB and each viewer bundle below 32 MiB. Dense investigation is excluded because that command deliberately has a 1,024-findings-per-rule boundary. The check is in local CI. It is a workload budget, not a hard allocation limit or browser responsiveness measurement.

View generation makes two streaming geometry passes, retaining only selected geometry and at most six deduplicated source-context rows per finding. The kernel borrows original samples rather than deep-cloning optional columns. Browser selection restores cached static layers; only selection overlays change each frame. Actual browser load/selection latency remains part of pending rendered QA.
