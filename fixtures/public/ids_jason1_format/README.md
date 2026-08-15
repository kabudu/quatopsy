# IDS Jason-1 format evaluation fixture

This file follows the published Jason-1 body-quaternion ASCII layout (UTC date, UTC time, Q0 Q1 Q2 Q3 with Q0 scalar) described by IDS document SALP-IF-M/IDS-EA15938-CN.

The numeric samples are synthetic. They are not copied from CNES proprietary examples and are not a CDDIS Jason telemetry redistribution. CDDIS archives require Earthdata login, so raw flight files are not vendored.

Evaluate with:

```bash
quatopsy adapt --format ids-jason1 --input fixtures/public/ids_jason1_format/source.qbody --output-dir /tmp/quatopsy-ids
quatopsy analyze --input /tmp/quatopsy-ids/input.csv --manifest /tmp/quatopsy-ids/manifest.json --report /tmp/quatopsy-ids/report.json
```
