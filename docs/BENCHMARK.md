# Million-sample budget

The asserted budget is 1,000,000 samples analysed in under 10 seconds with peak RSS under 512 MiB, excluding viewer generation. The check is `cargo test --release --locked -p quatopsy-core --test million -- --ignored` and is invoked from `./scripts/ci-local.sh`.

The workload is a synthetic increasing-time identity series generated in process. It exercises ingest, the closed rule registry, and report assembly at the documented sample count. It is not a substitute for a named laboratory reference machine or for third-party flight telemetry.

Local checksum packaging is `scripts/package-local.sh`. It copies the release CLI and writes `SHA256SUMS`. It does not sign artefacts, publish packages, or create tags.
