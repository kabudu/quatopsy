# Private investigation workflow

## Purpose

`quatopsy investigate` packages a recorded attitude incident as a local, deterministic evidence bundle. It is for post-test, post-simulation, or post-pass analysis. It does not receive live telemetry, send commands, approve a recovery, or open an actuator interface.

## Operational basis

The workflow is based on primary spacecraft engineering and ground-system practice:

- [NASA-STD-8070.1](https://www.nasa.gov/sites/default/files/atoms/files/std8070.1.pdf) requires telemetry to expose intended commanded state, actual state, discrepancies, health, and enough engineering detail for anomaly investigation and reconstruction.
- [NASA AMMOS Mission Control System](https://ammos.nasa.gov/mcs/) treats reception, management, and visualisation of real-time and historical telemetry as ground-system responsibilities.
- [F Prime GDS](https://fprime.jpl.nasa.gov/devel/docs/user-manual/overview/gds-introduction/) keeps telemetry, event history, and command history distinct. Its documentation warns that a ground command-history entry does not prove onboard receipt; command events provide separate execution evidence.
- [F Prime data constructs](https://fprime.jpl.nasa.gov/devel/docs/user-manual/overview/04-cmd-evt-chn-prm/) distinguish periodic telemetry snapshots from intermittent events recorded for reconstruction.
- [NASA cFS Test Framework](https://ntrs.nasa.gov/citations/20210009725) uses machine-readable scenarios, declared CCSDS message definitions, logging, and reports for automated verification.

Quatopsy adopts the useful parts of that pattern within its narrower scope: immutable telemetry capture, declared interpretation, separation of telemetry from contextual histories, reproducible analysis, separately named candidates, and a reviewable digest manifest.

## Command

Canonical CSV input:

```bash
quatopsy investigate \
  --case-id ops-2026-042 \
  --input attitude.csv \
  --manifest attitude-manifest.json \
  --event-log events.log \
  --command-log commands.log \
  --notes operator-notes.txt \
  --plan-problem recovery-plan.json \
  --control-problem recovery-control.json \
  --output-dir ops-2026-042-evidence
```

External telemetry supported by an existing adapter:

```bash
quatopsy investigate \
  --case-id str-pass-042 \
  --input star-tracker-export.csv \
  --format tubin-str \
  --output-dir str-pass-042-evidence
```

Supply exactly one of `--manifest` or `--format`. The output directory must have an existing, non-symlink parent and must not already exist. There is intentionally no overwrite mode for an evidence bundle.

Verify a completed bundle before handover:

```bash
quatopsy verify-evidence --bundle ops-2026-042-evidence
```

## Bundle contract

`evidence.json` uses `quatopsy.evidence/1`. Its `bundle_id` is SHA-256 over the case ID, tool version, and sorted relative artifact path and artifact digest pairs. Every artifact also records its byte length, digest, and role. `verify-evidence` rejects missing, added, reordered, changed, or role-drifted files; altered safety or context declarations; incomplete source forms; duplicate or undeclared candidate trees; and report summaries that no longer bind to their canonical inputs.

The bundle layout is:

```text
evidence.json
source/
  input.csv | external-input.bin
  manifest.json | adapted/{input.csv,manifest.json,provenance.json}
context/
  events.log
  commands.log
  notes.txt
observed/
  report.json
  repairs/
  repro/
  viewer/{index.html,viewer.css,viewer.js}
candidates/
  plan/{problem.json,generated/,analysis/}
  control/{problem.json,generated/,analysis/}
```

Only supplied files appear. Event, command, and note files are opaque context: they are preserved and hashed but never interpreted by the rule kernel. A command-history record is therefore never presented as proof of spacecraft receipt or execution.

The observed report owns its result. Candidate plan and control documents still cannot write a result; their generated trajectories pass through the ordinary kernel and receive separate reports. Repair candidates remain separate files and never overwrite observed telemetry.

## Bounds and failure behaviour

- Observed canonical or external input: 256 MiB and one million samples.
- Each context file: 64 MiB.
- Each plan or control problem: 16 MiB.
- Evidence verification: at most 10,000 artifacts, 256 MiB per artifact, 1 GiB total, and 16 directory levels.
- Findings: 1,024 per rule, with the existing 1,024 total repro-slice export cap.
- Analysis deadline: 120 seconds, bounded below the compiled safe maximum.
- One local case runs sequentially. No network, service, cache, credential, or concurrent fan-out is introduced.

Creation atomically reserves a new output directory. Existing bundles are never modified. Any handled validation, candidate, cancellation, or write failure removes the directory created by that invocation. A process kill can leave a directory without `evidence.json`; such a directory is incomplete and verification refuses it.

## Privacy and handover

An evidence bundle deliberately contains copied telemetry and may contain operational histories or notes. It inherits the highest sensitivity of any supplied file. Quatopsy does not redact, upload, retain, encrypt, or access-control it. Operators must use their organisation's approved storage, transfer, retention, and deletion controls.

The bundle is evidence of deterministic local processing, not authenticity. SHA-256 detects later mutation relative to the manifest but does not identify who captured the source. Signed custody, mission time correlation, authenticated downlink, and classified-data handling remain external responsibilities.
