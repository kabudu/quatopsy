# Risk register

Probability and impact use low, medium, or high. The owner is a role until named contributors exist. Review occurs at every milestone exit and before release.

| ID | Risk | Probability | Impact | Owner | Trigger | Mitigation | Contingency |
| --- | --- | --- | --- | --- | --- | --- | --- |
| R1 | Closest work already provides the candidate diagnostic mechanism | Medium | High | Research claims | Prior-art match across detection, explanation, repair, and report contract | Systematic search and matched prototype comparison | Narrow claim or reposition as integration product |
| R2 | Floating-point boundaries misclassify near-zero or near-pi cases | Medium | High | Engineering | Oracle disagreement or platform digest drift | Explicit numeric profile, conditioning states, high-precision oracle | Refuse affected interval and revise rule version |
| R3 | Frame or convention ambiguity produces confident wrong output | High | High | Product | Missing or contradictory declaration | Fail-closed manifest and redundant-evidence checks | Refuse analysis and improve adapter provenance |
| R4 | Sign finding is mistaken for physical discontinuity | Medium | High | UX | User study or issue shows confusion | Separate classes, linked views, qualified copy | Redesign terminology and suspend misleading view |
| R5 | Repair proposal changes physical orientation | Low | High | Engineering | Matrix oracle mismatch | Independent equivalence test and no source overwrite | Disable repair rule and notify affected versions |
| R6 | Malicious input exhausts memory, disk, or findings | Medium | High | Security | Limit or fuzz failure | Streaming parse, caps, cancellation, atomic output | Refuse input and tighten safe maxima |
| R7 | Sensitive telemetry leaks through logs or viewer | Low | High | Privacy | Sink test or report review failure | Offline default, bounded evidence, redaction, no remote assets | Stop release, purge artefacts, disclose scope |
| R8 | Rust/WebAssembly reuse creates unnecessary coupling | Medium | Medium | Architecture | Viewer blocks on kernel internals | Stable report protocol and non-authoritative viewer | Replace shared view code with independent renderer |
| R9 | Performance target fails on realistic profiles | Medium | Medium | Performance | M4 benchmark miss | Contiguous data, streaming parse, bounded reports | Publish lower supported limit or optimise measured hotspot |
| R10 | Name or mark conflicts with a relevant product | Low | High | Legal | Similarity or trademark review finding | Point-in-time search and public-launch legal gate | Rename before public productisation |
| R11 | Patent claims overlap repair or diagnostic workflows | Medium | High | Legal | Counsel or systematic search identifies material claim | Preserve search record and avoid patentability claims | Redesign feature, license, or exclude jurisdictionally |
| R12 | Generic viewers or libraries add equivalent diagnostics | Medium | Medium | Product | Competitor release closes gap | Open protocol, reproducible evidence, narrow vertical | Differentiate on workflow or discontinue novelty claim |
| R13 | Users treat advisory output as flight certification | Medium | High | Product | Public copy or usage indicates reliance | Persistent non-claim, report scope, no safety badge | Suspend distribution in affected context |
| R14 | Dependency or release supply chain is compromised | Low | High | Security | Audit alert or digest mismatch | Minimal locked dependencies, audit, reproducible artefacts | Revoke artefacts, rotate credentials, issue advisory |
| R15 | Optional external validation is accidentally made release-blocking | Low | Medium | Product | Roadmap or release gate requires outsider action | Explicit optional policy in ADR, validation, and release docs | Correct gate before milestone review |
| R16 | Full brand work implies maturity before evidence | Medium | Medium | Brand | Polished assets appear before productisation approval | Restrained research identity and claims scan | Withdraw assets and revert to research presentation |

