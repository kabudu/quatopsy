# ADR 0002: Offline candidate trajectory generator

## Status

Accepted on 2026-08-17.

## Context

V1 was specified as a diagnostic linter. A general trajectory optimiser needs actuator suites, keep-out zones, a chosen NLP strategy, and a dynamics model that this kernel does not own. Users still need a way to produce a candidate attitude path that the existing analyser can judge.

## Decision

Add `quatopsy plan` in this repository as an M6 increment. The first algorithm is a closed-form eigenaxis bang-coast-bang generator for a torque-limited, rest-to-rest rigid body. Inertia may be spherical or diagonal. Torque may be a scalar or a three-component box.

The planner emits canonical CSV, a declared manifest, and `quatopsy.plan/1`. Body-rate columns are declared for the kernel. Torque columns are present for an independent residual oracle and are not declared in the analysis manifest. Residual checks use `quatopsy-oracle` rotation-matrix kinematics and Euler's equation; they do not share the planner generator. The planner never writes `quatopsy.report/1` or a `result` field. The diagnostic kernel remains the only verdict owner.

Wheels, thrusters, CMGs, keep-out zones, weighted objectives, multiple shooting, and model-uncertainty campaigns are in-repo candidate-generation features. They do not assign a report result. Any controller remains out of scope. Global optimality is not claimed.

## Consequences

Problem documents use `quatopsy.plan-problem/1` with `deny_unknown_fields`. Unsupported models are refused. Independent residuals are recorded in the plan document. A feasible plan is a candidate, not flight approval.
