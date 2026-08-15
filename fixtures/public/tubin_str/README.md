# TUBIN star-tracker evaluation excerpt

This excerpt is copied from the public TUBIN AOCS telemetry dataset:

- Jonglez et al., *AOCS telemetry open data of the TUBIN small satellite mission*, Zenodo, 10.5281/zenodo.19708907, version 0.1.0, CC BY 4.0.
- Source file: `TmStr_136.csv`, voter quaternion and SAT-frame rate columns, 16 consecutive valid samples starting 2021-07-04 13:57:04.500000 UTC.

The adapter maps `VOTER_Q_{S,X,Y,Z}` as Hamilton `wxyz` from SAT to TOD and converts `VOTER_{X,Y,Z}_RATE` from deg/s to rad/s. Empty voter rows are skipped. This is an evaluation corpus, not a claim that TUBIN flight software used Quatopsy.

```bash
quatopsy adapt --format tubin-str --input fixtures/public/tubin_str/source.csv --output-dir /tmp/quatopsy-tubin
quatopsy analyze --input /tmp/quatopsy-tubin/input.csv --manifest /tmp/quatopsy-tubin/manifest.json --report /tmp/quatopsy-tubin/report.json
```
