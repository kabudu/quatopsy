# Novelty

## Candidate contribution

Quatopsy's candidate contribution is a local-first, representation-aware diagnostic contract that automatically detects, separates, explains, quantifies, reproduces, and proposes reversible repairs for supported topological and convention defects in sampled quaternion orientation trajectories, while synchronising physical `SO(3)` motion with the chosen `S^3` lift and machine-readable evidence.

This is a hypothesis to test, not an established novelty claim.

## Established ingredients that must not be claimed

Unit quaternions and the double cover of `SO(3)` by `S^3`; antipodal equivalence; SLERP and shortest-path sign choice; quaternion normalisation; unwinding and anti-unwinding control; quaternion splines and smoothing; angular-rate derivation; attitude visualisation; Hopf and stereographic visualisation; trajectory validation; report schemas; and local CLI linting are established.

## Differentiation hypotheses

**NOV-1:** No known reviewed system provides a closed, versioned diagnostic rule contract that distinguishes representation discontinuity from physical discontinuity and binds every result to immutable input, convention, numeric-policy, and rule-set identity.

Falsification: systematic literature, product, open-source, package, and patent comparison plus a matched workflow prototype.

**NOV-2:** No known reviewed system combines localised evidence, quotient-aware `S^3` lift visualisation, minimal reproducer export, and provenance-preserving repair candidates in one local-first spacecraft attitude workflow.

Falsification: find a system supporting all elements at comparable scope, or show that composing existing tools produces equivalent usability and assurance without material integration work.

**NOV-3:** The combined diagnostic contract yields a higher complete-explanation rate on the frozen defect corpus than established library primitives and generic trajectory viewers, without increasing false physical-motion claims.

Falsification: matched evaluation shows no material improvement or unacceptable false findings.

## Search protocol

Search dates begin 2026-08-14. Academic sources include ACM, IEEE, AIAA, NASA NTRS, arXiv, robotics/control venues, and backward/forward citations for quaternion interpolation, continuity, unwinding, trajectory validation, and attitude debugging. Products include spacecraft mission analysis, telemetry visualisation, robotics log viewers, animation tooling, and scientific rotation packages. Open-source searches cover GitHub exact and semantic queries. Packages cover crates.io, npm, PyPI, and relevant ROS indexes. Patent searches cover Google Patents and later Espacenet/WIPO/USPTO keyword and classification searches. Record exact queries, dates, inclusion decisions, mechanisms, boundaries, evaluation, and implementation availability.

Initial queries included `quaternion trajectory debugging sign flip unwinding`, `quaternion continuity checker trajectory repair`, `SO(3) trajectory linter diagnostics`, `spacecraft attitude visualization telemetry`, `quaternion interpolation sign continuity`, and corresponding patent terms. Search must be refreshed before a public novelty claim.

## Initial negative and boundary results

The initial search found substantial overlap in primitives and adjacent workflows. ROS `tf2`, SciPy, and pytransform3d already implement shortest-path, smoothing, or rotation operations. evo validates pose trajectories including quaternion norm and timestamps. Foxglove, Basilisk Vizard, ESA JUICE tools, Aptus, and other systems visualise attitude or telemetry. Research and patents cover unwinding, constrained trajectories, interpolation, smoothing, and norm monitoring. These findings reject any claim that the individual checks, repairs, or views are new.

No reviewed result yet demonstrated the entire narrow diagnostic contract, but absence from this initial search is weak evidence only.

## Claim and retraction rules

Until the systematic matrix and matched evaluation complete, use `candidate contribution` and never `novel`. If a close system is found, update the matrix immediately, narrow or retract the hypothesis, retain the negative result, and do not rename combination as invention. Production readiness, safety, user value, and adoption remain separate claims.

External independent challenge is optional. Its absence does not block implementation or release, but it prevents claims of independent reproduction or consensus. Patent and trademark clearance require qualified legal review before public productisation and are not established by this repository.

