# Quatopsy

Quatopsy is a candidate local-first product for diagnosing quaternion orientation trajectories before or after they drive a spacecraft, robot, simulator, or animation system. It combines a deterministic trajectory linter with a visual debugger that links physical motion in `SO(3)` to a projected lift in `S^3`.

## Candidate contribution

The narrow candidate contribution is a representation-aware diagnostic report that detects, explains, quantifies, reproduces, and proposes reversible repairs for topological and convention defects in sampled orientation trajectories. The first vertical is offline spacecraft attitude data in a documented CSV profile.

Quatopsy does not claim to invent quaternions, sign canonicalisation, shortest-path interpolation, unwinding analysis, attitude visualisation, or quaternion trajectory smoothing. Novelty, safety, production readiness, and physical cost estimates remain separate evidence-gated claims.

## Status

Planning corpus complete as of 2026-08-14. No product implementation, package, release, safety qualification, or production support is claimed. The first unchecked milestone is the executable mathematical conformance kernel and report schema.

The learning-laboratory concept is a separate future project and is not part of Quatopsy.

## Documentation

- [Product specification](docs/PRODUCT_SPECIFICATION.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Soundness case](docs/SOUNDNESS_CASE.md)
- [Report protocol](docs/REPORT_PROTOCOL.md)
- [Implementation plan](docs/IMPLEMENTATION_PLAN.md)
- [Novelty and prior art](docs/NOVELTY.md)
- [Validation](docs/VALIDATION.md)
- [Release policy](docs/RELEASE.md)
- [Requirements traceability](docs/REQUIREMENTS_TRACEABILITY.md)

## Name audit

`Quatopsy` is a point-in-time candidate, searched on 2026-08-14 across general web results, GitHub repository names, npm, PyPI, and crates.io. No exact product or package collision was found. This is not trademark clearance, domain reservation, patent clearance, or a guarantee of worldwide availability. Legal review remains required before public productisation.

