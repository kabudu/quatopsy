# ADR 0003: Software-in-the-loop attitude controller

## Status

Accepted on 2026-08-18. Amended on 2026-08-18 to add host-CPU processor-in-the-loop, loopback hardware-in-the-loop, and a fail-closed systems-safety programme.

## Context

M6 added an offline candidate generator. A controller still adds feedback, estimation, timing, and physical-consequence risk. The diagnostic kernel must not become a flight-control authority. Physical actuators, target flight processors, and organisational assurance cannot be faked in this repository.

## Decision

Add `quatopsy control` as a geometric PD controller on `SO(3)` under a declared plant and envelope. Attitude error is the rotation-matrix vee map. State estimates carry frame, timestamp, covariance, and freshness contracts. Saturation, anti-windup, momentum dump, mode transitions, command arbitration, and safe fallback are in-repo. Command inhibition lives in `quatopsy-oracle` and cannot be overridden by the PD law.

`execution` may be `sil`, `pil`, or `hil`. `pil` isolates the control cycle in a child process on the host CPU. `hil` sends commands to a loopback actuator-emulator process. `latency_class` is `bounded-software` only. `hardware.class: physical` is refused. The command writes CSV, a declared manifest, and `quatopsy.control/1` with no report `result`. `analyze` remains the only verdict owner.

The systems-safety programme is `docs/CONTROL_SAFETY.md`. It records hazards, independence, stop-ship rules, and an absent qualification record. Target flight processors, physical actuator drivers, hard real-time operating systems, hardware qualification, and the applicable organisational or regulatory assurance regime remain out of scope.

## Consequences

A tracked candidate is not flight approval and not actuator permission. Public copy must keep that boundary. Independent validation of closed-loop robustness is not claimed. Host-CPU PIL is not a qualified processor. Loopback HIL is not a physical actuator.
