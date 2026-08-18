# Engineering review

## Review scope and date

Initial Lazarus-mode planning review completed on 2026-08-14. Scope: user mandate, candidate contribution, mathematical semantics, Rust control-plane architecture, security, resources, operations, adoption, optional independent validation, brand gate, release policy, and documentation truthfulness. No implementation was reviewed or claimed.

## Requirement inventory

The project combines the trajectory linter and visual debugger, targets spacecraft attitude data first, uses Rust for the control plane and semantic core, keeps the quaternion learning laboratory separate, applies the bootstrap and Lazarus workflows, and treats external independent validation as optional and non-blocking for implementation and final release.

## Review findings and resolutions

| Severity | Finding | Resolution |
| --- | --- | --- |
| High | A raw quaternion sign jump can be falsely described as physical motion | Separate representation and physical finding classes; use sign-invariant distance; require linked evidence |
| High | Ambiguous component/frame conventions can produce confident wrong output | Require explicit manifest and refuse ambiguity; defer automatic convention repair |
| High | A repair tool can silently alter source or physical orientation | No-clobber separate outputs, provenance, independent matrix oracle, explicit apply step |
| High | Failure, limit, or partial analysis could be mistaken for pass | Closed result algebra with fail-closed precedence and per-rule accounting |
| High | Broad novelty language collides with extensive established work | Narrow candidate claim and preserve established ingredients and negative results |
| Medium | A web service would expand privacy and operational risk before value is proven | Local Rust CLI plus static viewer; no accounts, server, or telemetry |
| Medium | Shared WebAssembly computations could make the viewer a second rule engine | Canonical report remains authoritative; WASM sharing limited to non-authoritative view computations |
| Medium | Large profiles can exhaust browser or CLI resources | Streaming parse, safe maxima, finding caps, bounded concurrency, evidence-preserving downsampling |
| Medium | Full visual branding may imply maturity | Restrained research identity until explicit productisation and legal gates |
| Medium | External review could become an accidental schedule dependency | Explicit optional policy across ADR, roadmap, validation, traceability, and release |

## Simplicity decisions

Use one Rust workspace rather than services or polyglot control planes. Use CSV plus manifest and open JSON before ROS/SPICE-native parsing. Keep rules closed and compiled before considering plug-ins. Avoid persistence and cache until measured need. Keep dynamics-dependent energy and controller safety claims outside V1. The M6 planner is a candidate generator under an explicit torque-limited rigid-body model and cannot assign a report result.

## Soundness and security summary

The proposed claim is bounded to supported rule conformance, not safe flight. The core invariants, trusted computing base, aggregation order, numeric conditioning, immutable analysis identity, resource limits, atomic output, privacy defaults, adapter boundary, and independent oracles are explicit. The implementation must demonstrate them before any behavioural box is checked.

## Residual risks

Novelty remains unestablished; the patent search is preliminary; legal name/mark clearance is absent for public productisation; user comprehension of representation versus physical motion is untested in a practitioner study; and kernel numeric behaviour is locked by `quatopsy.numeric/1` and fixture oracles, not by an independent external audit. These risks constrain public claims. The private `0.1.0` release accepts them by remaining private, unsigned, unpublished to crates.io, and claim-bounded.

## Documents tailored

`SOUNDNESS_CASE.md` and `REPORT_PROTOCOL.md` were added because the product makes machine-checkable semantic claims. Adapter provenance and `quatopsy-adapt` contract tests cover the integration boundary. Full brand assets are intentionally absent because productisation is not approved. `AGENTS.md` and `scripts/ci-local.sh` exist as of M1.

## Completion statement

M5 records a private `0.1.0` research release: Apache-2.0 licence, frozen claims, curated notes, supply-chain allowlist, and a fail-closed GitHub Release path. Full brand assets remain absent because productisation is not approved. Public opening, hosted CI, and crates.io remain distinct unauthorised gates.

