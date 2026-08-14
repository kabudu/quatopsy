# Requirements traceability

States are `planned`, `implemented`, `verified`, `deferred`, or `not-applicable`. Documentation alone never changes a behavioural requirement from planned.

| ID | Requirement | Design owner | Primary verification | Release evidence | State |
| --- | --- | --- | --- | --- | --- |
| SEM-1 | Validate finite non-zero quaternion samples and explicit tolerances | Kernel | Conformance and fuzz suite | Rule evidence report | planned |
| SEM-2 | Make physical comparisons invariant under quaternion sign | Kernel | Matrix/analytic oracle | Conformance digest | planned |
| SEM-3 | Construct deterministic lifted sequence with near-pi refusal metadata | Kernel | Boundary and mutation tests | Rule evidence report | planned |
| SEM-4 | Fail closed across pass, findings, refused, and error | Aggregator | Fault injection | E2E result matrix | planned |
| SEM-5 | Version rules, report protocol, and numeric policy | Schema | Compatibility fixtures | Protocol manifest | planned |
| SEC-1 | Bound bytes, rows, fields, memory, time, and findings | Ingest/CLI | Hostile-input tests | Limit test report | planned |
| SEC-2 | Prevent input execution, unsafe path writes, and remote viewer loads | CLI/viewer | Security E2E and CSP scan | Security evidence | planned |
| SEC-3 | Bind report and repairs to immutable digests | Model/report | Tamper tests | Provenance fixture | planned |
| OPS-1 | Use atomic no-clobber output and cancellation cleanup | CLI | Filesystem lifecycle E2E | Operations test report | planned |
| OPS-2 | Provide clean, cache-bypassable deterministic analysis | CLI | Repeated clean runs | Digest comparison | planned |
| INT-1 | Support canonical CSV plus explicit manifest without credentials | Ingest | Public workflow E2E | Install/use evidence | planned |
| INT-2 | Keep adapters outside semantic verdict ownership | Adapter contract | Contract tests | Adapter conformance report | planned |
| INT-3 | Support advisory, selective, and required adoption with scoped overrides | CLI policy | Lifecycle E2E | Adoption evidence | planned |
| PERF-1 | Analyse one million samples under registered time and memory targets | Kernel/CLI | Frozen benchmark | Benchmark report | planned |
| REL-1 | Run repository-owned local CI as the authoritative private-repo gate | Maintainers | `./scripts/ci-local.sh` | Recorded PR result | planned |
| REL-2 | Require explicit user approval before hosted CI activation | Owner | Repository audit | Release checklist | planned |
| REL-3 | Use curated release notes and rendered desktop/narrow preview | Release owner | Preview and live inspection | Release URL/screenshots | planned |
| REL-4 | Reject Unicode U+2014 across tracked text and release metadata | Local CI | Repository scan | CI log | planned |
| UX-1 | Synchronise all views by sample identity | Viewer | Browser E2E | Visual workflow evidence | planned |
| UX-2 | Distinguish raw, derived, repaired, representation, and physical states | Viewer | Accessibility/comprehension review | UI evidence | planned |
| UX-3 | Never use colour as the sole result signal | Viewer | Automated accessibility checks | Accessibility report | planned |
| NOV-1 | Test the closed diagnostic-contract differentiation hypothesis | Research | Systematic comparison | Updated prior-art matrix | planned |
| NOV-2 | Test the combined evidence/visual/repair workflow hypothesis | Research | Matched prototype comparison | Validation report | planned |
| NOV-3 | Keep independent validation optional and claims conditional | Product/research | Gate and copy audit | Release checklist | planned |

## Completion rule

Every release-critical row must be implemented and verified with its named evidence, or explicitly removed from the supported release scope with owner approval and compatibility review. A document, scaffold, unchecked test, or unavailable hosted check is not evidence of implementation.

