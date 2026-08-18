# Control systems-safety programme

This document is the in-repo systems-safety programme for `quatopsy control`. It is a hazard analysis, independence record, and fail-closed hardware-use gate. It is not a qualification dossier, not an organisational safety case, and not permission to command hardware.

## Boundary

The controller may emit a closed-loop CSV under a declared plant and envelope. `analyze` remains the only owner of `quatopsy.report/1` `result`. Control status values are candidates, not flight approval.

Physical actuators, target flight processors, hard real-time operating systems, and hardware qualification remain refused. A qualification record does not exist in this repository and cannot be supplied to authorize hardware use.

## Execution classes

| Class | Cycle law and monitor | Plant | Evidence |
| --- | --- | --- | --- |
| `sil` | In-process | In-process | SO(3) rest-to-rest fixture |
| `pil` | Isolated child process on the host CPU | In-process | Isolated-controller fixture |
| `hil` | In-process | Loopback emulator child process | Loopback command-bus fixture |

`latency_class` is `bounded-software` only. `hard-real-time` is refused. `hardware.class` defaults to `loopback-emulator`. `hardware.class: physical` is refused and leaves no output directory.

## Hazards

| ID | Hazard | Detection | Response |
| --- | --- | --- | --- |
| H1 | Antipodal quaternion unwind | Rotation-matrix vee error; antipodal fixture | Same torque for `q` and `-q` |
| H2 | Stale, future, or mismatched estimate | Oracle freshness, timestamp, and frame contracts | Inhibit, then fail-closed rate-damp |
| H3 | Excess torque or momentum | Independent envelope monitor | Inhibit; PD cannot override |
| H4 | Keep-out cone violation | Independent cone check | Inhibit or avoid |
| H5 | Non-finite measurement | Numerical-fault campaign | Inhibit |
| H6 | Actuator-axis loss | Deterministic fail-axis trial | Continue under remaining torque, still monitored |
| H7 | Isolated worker crash or closed pipe | Parent JSON-line timeout as closed stdout | Refuse the job; no committed output |
| H8 | Physical actuator or flight-board command | `hardware.class` and unknown-field refusal | Refuse before any output directory |
| H9 | Hard-real-time claim | `latency_class` refusal | Refuse |
| H10 | Report-result impersonation | Serializer guard against a `result` field | Refuse |
| H11 | Declared plant models treated as hardware | Protocol copy; loopback-only class | Keep software boundary |
| H12 | Environmental torque stored as wheel momentum | Motor torque updates `h`; magnetic and gravity-gradient enter Euler only | Keep the split; wheels-plus-magnetic tests |
| H13 | Pass-through estimate treated as a navigation solution | MEKF/UKF with independent NIS; χ² reject is not a monitor trip; `nav.json` has no `result` | Keep filter-vs-shim and reject-vs-inhibit tests |
| H14 | Guidance profile treated as flight approval | `guidance.json` has no `result`; plan CSV is a candidate | Keep kernel-owned analyze |
| H15 | Two-body geometry treated as orbit determination | Protocol copy; propagated Kepler only | Keep not-OD wording |

## Independence

The PD law lives in `quatopsy-control`. Command permission lives in `quatopsy-oracle`. The monitor reviews the saturated command against the envelope before the plant integrates. Tests cover excess torque, stale measurements, keep-out, and numerical faults. The law cannot mark a command allowed after a monitor trip.

Processor-in-the-loop isolates the cycle worker from the plant process. Hardware-in-the-loop isolates the plant emulator from the cycle. Neither worker opens a device node, serial port, socket, or GPIO.

## Stop-ship

Do not merge or release controller changes that:

- write `result` on `control.json`
- accept `hardware.class: physical`
- accept `latency_class: hard-real-time`
- spawn workers that write files other than JSON lines on stdout
- claim a tracked candidate is flight approval, actuator permission, or a qualified processor

## Qualification record

Status: **absent**.

This repository has no hardware-qualification record, no target-processor board support package, no actuator identity, no environmental test log, and no authorised safety-assessment sign-off. Absence is fail-closed. A future record, if one is ever created outside this programme, is a separate organisational artefact and is not an in-repo authorization switch.

## Non-claims

The programme does not claim completeness, residual-risk acceptance, IEC 61508 / DO-178 / NASA NPR coverage, timing guarantees, or independent safety assessment. SIL, PIL, and loopback HIL are software evidence for the documented execution classes only.
