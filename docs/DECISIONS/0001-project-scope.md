# ADR 0001: Project scope and first vertical

## Status

Accepted for planning on 2026-08-14.

## Context

The broad idea spans quaternion education, trajectory linting, attitude control, visualisation, and motion planning. A defensible first product needs a falsifiable mechanism and a user workflow that does not require flight hardware or production credentials.

## Decision

Build Quatopsy as a local-first orientation-trajectory linter plus visual debugger. Start with offline spacecraft attitude CSV data and explicit conventions. Keep the deterministic diagnostic kernel authoritative, the viewer non-authoritative, and later ecosystem adapters outside the semantic trust boundary.

External independent validation is an optional evidence track. Its absence does not block implementation, product completion, or release. It blocks only claims explicitly requiring independent evidence.

The quaternion learning laboratory is a separate idea and repository to be explored later.

## Alternatives

A pure visual learning tool is easier to demonstrate but offers weaker product differentiation. A live telemetry platform creates security and operational scope before the mechanism is proven. A trajectory optimiser competes with mature research and makes dynamics assumptions outside the initial diagnostic claim.

## Consequences

V1 must refuse ambiguous conventions, provide machine-readable evidence, preserve original data, and prove quotient-invariant semantics. Live control, automatic frame repair, dynamics-based energy claims, ROS ingestion, and educational curricula are excluded from the first release.

