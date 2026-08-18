# ADR 0004: Software GN&C plane

## Status

Accepted on 2026-08-18.

## Context

M7 ships a geometric PD controller with independent command inhibition and a fail-closed hardware-use gate. Estimation is a measurement copy. The cycle tracks a fixed rest setpoint with zero feed-forward rate. Actuation is a body-torque box. Frozen inertial field and nadir are not an orbit. That nucleus is not navigation, not guidance, and not a flight control plane.

Physical actuators, target processors, hard real-time operating systems, and certification records remain absent. Those absences cannot be filled by software.

## Decision

Add a software GN&C plane behind `quatopsy control`:

- `quatopsy-nav` owns a 6-state MEKF and a UKF on the same error-state model. The UKF is a 13-point scaled sigma-point filter, not a copied Joseph MEKF update. Star and gyro measurements are asynchronous. Future samples are refused. Outliers are χ²-gated. A reject is not a monitor trip. `sensor.status: failed` and eclipse are propagate-only; process noise is allowed to grow covariance. Audit rows record NIS, optional SIL NEES, rejected counts, covariance trace, and bias. The crate does not depend on `quatopsy-oracle`.
- `quatopsy-guidance` owns time-tagged attitude references `(t, q, ω, α)`, optional plan-CSV ingest, keep-out, a named sun-pointing constraint, and terminal rest. It does not assign report `result`.
- Control tracks the guidance reference, including nonzero `ω_d` and `α_d`. Optional gain scheduling is a declared error-to-gain table. Reaction-wheel allocation is control-side and does not import the planner.
- Optional declared two-body geometry supplies nadir, sun, eclipse, and dipole `B(t)`. Translational state is propagated, not estimated.
- The cycle is a sequential deterministic partition. Phase durations are software clocks. `latency_class: hard-real-time` and `hardware.class: physical` stay refused.

`analyze` remains the only owner of `quatopsy.report/1` `result`. `nav.json`, `guidance.json`, and `control.json` are refused if they contain `result`.

## Consequences

A tracked candidate with a finite NIS is not a navigation solution, not flight approval, and not a filter qualification. The error-state UKF reconstructs covariance from finite quaternion sigmas; that map can contract slightly, so process noise is verified against a Q=0 run rather than as unbounded random-walk growth. Host-CPU PIL and loopback HIL stay software evidence. Open M8 boxes (physical HIL, CMG gimbals, WCET, certification, orbit determination) stay open.
