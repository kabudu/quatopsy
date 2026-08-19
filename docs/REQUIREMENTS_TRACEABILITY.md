# Requirements traceability

States are `planned`, `implemented`, `verified`, `deferred`, or `not-applicable`. Documentation alone never changes a behavioural requirement from planned.

| ID | Requirement | Design owner | Primary verification | Release evidence | State |
| --- | --- | --- | --- | --- | --- |
| SEM-1 | Validate finite non-zero quaternion samples and explicit tolerances | Kernel | Conformance and fuzz suite | Rule evidence report | verified |
| SEM-2 | Make physical comparisons invariant under quaternion sign | Kernel | Matrix/analytic oracle | Conformance digest | verified |
| SEM-3 | Construct deterministic lifted sequence with near-pi refusal metadata | Kernel | Boundary and mutation tests | Rule evidence report | verified |
| SEM-4 | Fail closed across pass, findings, refused, and error | Aggregator | Fault injection | E2E result matrix | verified |
| SEM-5 | Version rules, report protocol, and numeric policy | Schema | Compatibility fixtures | Protocol manifest | verified |
| SEC-1 | Bound bytes, rows, fields, memory, time, and findings | Ingest/CLI | Hostile-input tests | Limit test report | verified |
| SEC-2 | Prevent input execution, unsafe path writes, and remote viewer loads | CLI/viewer | Security E2E and CSP scan | Security evidence | verified |
| SEC-3 | Bind report and repairs to immutable digests | Model/report | Tamper tests | Provenance fixture | verified |
| OPS-1 | Use staged output-set commit, race-safe no-clobber, rollback, and cancellation cleanup | CLI | Filesystem lifecycle and auxiliary-failure E2E | Operations test report | verified |
| OPS-2 | Provide clean, cache-bypassable deterministic analysis | CLI | Repeated clean runs | Digest comparison | verified |
| INT-1 | Support canonical CSV plus explicit manifest without credentials | Ingest | Public workflow E2E | Install/use evidence | verified |
| INT-2 | Keep adapters outside semantic verdict ownership | Adapter contract | Contract tests | Adapter conformance report | verified |
| INT-3 | Support advisory, selective, and required adoption with strict rule names and scoped canonical-time overrides | CLI policy | Lifecycle and malformed-policy E2E | Adoption evidence | verified |
| INT-4 | Keep the candidate planner outside semantic verdict ownership | Plan contract | Plan-then-analyze E2E | Plan protocol tests | verified |
| INT-5 | Keep the controller outside semantic verdict ownership and physical hardware command | Control contract | Control-then-analyze E2E including PIL and loopback HIL | Control protocol and safety tests | verified |
| INT-6 | Keep the software GN&C plane outside semantic verdict ownership, physical hardware command, and flight-navigation claims | Nav/guidance/control contract | Profile-track control-then-analyze plus independent NIS and allocation oracles | Control protocol and safety tests | verified |
| PERF-1 | Analyse one million samples under registered time and memory targets | Kernel/CLI | Frozen benchmark | Benchmark report | verified |
| REL-1 | Run repository-owned local CI as the authoritative private-repo gate | Maintainers | `./scripts/ci-local.sh` | Recorded PR result | verified |
| REL-2 | Require explicit user approval before hosted CI activation | Owner | Repository audit | Release checklist | planned |
| REL-3 | Use curated release notes and rendered desktop/narrow preview | Release owner | Preview and live inspection | Release URL/screenshots | implemented |
| REL-4 | Reject Unicode U+2014 across tracked text and release metadata | Local CI | Repository scan | CI log | verified |
| UX-1 | Synchronise all views by sample identity with bounded finding navigation | Viewer | Public-workflow bundle tests and browser interaction | Visual workflow evidence | verified |
| UX-2 | Distinguish raw, derived, repaired, representation, and physical states | Viewer | Viewer layer and caption tests | UI evidence | verified |
| UX-3 | Never use colour as the sole result signal | Viewer | Text state, contrast checks, and browser accessibility snapshot | Accessibility report | verified |
| NOV-1 | Test the closed diagnostic-contract differentiation hypothesis | Research | Systematic comparison | Updated prior-art matrix | planned |
| NOV-2 | Test the combined evidence/visual/repair workflow hypothesis | Research | Matched prototype comparison | Validation report | planned |
| NOV-3 | Keep independent validation optional and claims conditional | Product/research | Gate and copy audit | Release checklist | verified |

## Completion rule

Every release-critical row must be implemented and verified with its named evidence, or explicitly removed from the supported release scope with owner approval and compatibility review. A document, scaffold, unchecked test, or unavailable hosted check is not evidence of implementation.

M5 owner disposition: `INT-2` and `INT-3` are verified for the shipped adapter crate and adoption-policy CLI. `REL-2` remains planned because hosted CI is a distinct unauthorised gate. `NOV-1` and `NOV-2` remain planned research hypotheses and do not block this private release.
