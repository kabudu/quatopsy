# End-to-end testing

## Public workflow

Tests invoke the released CLI against files, inspect exit status and canonical report, optionally apply a repair into a new file, reopen it through the CLI, then load the generated report in the packaged static viewer through its normal browser entry point. Direct library calls supplement but do not replace this lifecycle.

## Scenario inventory

| ID | Scenario | Expected public outcome | Oracle |
| --- | --- | --- | --- |
| E2E-01 | Clean constant-rate slew | `pass` under selected rules | Independent matrix sequence |
| E2E-02 | Alternating `q` and `-q` | Sign findings, zero physical jump, preserving repair | Matrix equivalence oracle |
| E2E-03 | Long-way commanded path | Bounded commanded-path finding when enabled | Analytic axis-angle path |
| E2E-04 | Near-pi transition | Ambiguity metadata, no fabricated unique repair | High-precision dot/angle oracle |
| E2E-05 | Mixed component order | Refusal or convention mismatch with redundant evidence | Hand-audited fixture |
| E2E-06 | Norm drift and zero quaternion | Findings then refusal at zero sample | Exact norm oracle |
| E2E-07 | Duplicate/decreasing time | Refusal, never pass | Integer timestamp oracle |
| E2E-08 | Huge, hostile, or malformed input | Bounded error/refusal and cleanup | Resource and filesystem assertions |
| E2E-09 | Cancelled repair write | Original intact, no committed partial output | Filesystem snapshot |
| E2E-10 | Unknown report major version | Viewer refusal with actionable message | Protocol fixture |

## Determinism and portability

Run identical corpus hashes, configuration, and engine versions on supported macOS and Linux-like targets. Compare canonical report bytes, exit states, repairs, and finding order. Platform-specific diagnostic metadata is excluded from the canonical digest and tested separately.

## Specification governance

Mutation tests alter each rule comparison, sign branch, aggregator precedence, tolerance boundary, and serializer field. At least one conformance case must fail for every mutation. Schema changes run backward-reader fixtures and require explicit major-version refusals where incompatible.

## Adoption lifecycle

Test advisory output, selective enforcement, expiring overrides, brownfield baselines, monorepo units, clean analysis, version upgrade, supported downgrade, rollback, cache removal, and complete uninstall. Adapters are tested as conversion lifecycles with canonical provenance, not as alternative rule engines.

## Privacy sinks

Capture stdout, stderr, reports, logs, crash diagnostics, temporary files, browser storage, and network activity. Default operation must emit no sample values to logs beyond bounded evidence in the user-requested report and must make no network requests.

## Chaos and hostile cases

Inject disk-full, permission failure, cancellation, malformed UTF-8, CSV formula text, path traversal names, symlinks, oversized fields, NaN/infinity encodings, timestamp overflow, finding floods, and corrupted cache entries. No partial, unsupported, or timed-out operation may become success.

## Flake policy

Deterministic tests have zero retry allowance. Browser tests may retry only a separately diagnosed environment startup, never a semantic assertion. Any intermittent semantic result is release-blocking until explained and fixed.

