# End-to-end testing

## Public workflow

Tests invoke the released CLI against files, inspect exit status and canonical report, optionally apply a repair into a new file, reopen it through the CLI, then write a static viewer bundle with `quatopsy view` and inspect that packaged HTML entry. Direct library calls supplement but do not replace this lifecycle.

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
| E2E-10 | Unknown report major version | Viewer bundle with actionable message and refusal exit 2 | Protocol fixture |
| E2E-11 | Plan then analyze | Candidate files with no `result`; kernel `pass` including `omega-consistent` | Independent plan residual oracle |
| E2E-12 | Wheel rest-to-rest plan then analyze | `multiple-shooting-lm` candidate; kernel `pass`; no plan `result` | Independent Euler residual with stored momentum |
| E2E-13 | SO(3) rest-to-rest control then analyze | `geometric-pd-so3` SIL candidate; kernel `pass`; no control `result` | Independent SO(3) error and command monitor |
| E2E-14 | Host-CPU PIL then analyze | Isolated controller process; kernel `pass`; no control `result` | Independent SO(3) error and command monitor |
| E2E-16 | Declared-plant loopback HIL then analyze | Command-to-torque lag, residual dipole, gravity-gradient, gyro ARW; kernel `pass`; no control `result` | Independent lag, magnetic, and gravity-gradient oracles; motor vs environmental `h` |
| E2E-17 | Profile-track control then analyze | Time-varying `ω_d`; MEKF audit; kernel `pass`; no `result` on control/nav/guidance | Independent reference kinematics, NIS, and allocation oracles |

## Determinism and portability

Run identical corpus hashes, configuration, and engine versions on supported macOS and Linux-like targets. Compare canonical report bytes, exit states, repairs, and finding order. Platform-specific diagnostic metadata is excluded from the canonical digest and tested separately.

## Specification governance

Mutation tests alter each rule comparison, sign branch, aggregator precedence, tolerance boundary, and serializer field. At least one conformance case must fail for every mutation. Schema changes run backward-reader fixtures and require explicit major-version refusals where incompatible.

## Adoption lifecycle

Test local binary copy/installation and removal, repeated-analysis compatibility, clean analysis, and unknown-major refusal. Cross-version executable upgrade, downgrade, and rollback testing begins when a second supported binary version exists. `INT-3` policy modes, strict rule names, and canonical expiring overrides are covered by CLI lifecycle tests. Adapters are tested as conversion lifecycles with canonical provenance, then analysed by the core; they are not alternative rule engines.

## Privacy sinks

Automated tests capture stdout, stderr, reports, and temporary files. Default operation must not echo CSV payload rows to stderr and the generated viewer must contain a deny-by-default CSP, no remote URLs, and no network or storage APIs. Browser execution verifies the static bundle requests only its three local files. Crash-diagnostic capture remains outside V1 because the CLI installs no crash reporter or persistent logger.

## Chaos and hostile cases

Inject permission failure, cancellation, malformed UTF-8, CSV formula text, path traversal names, symlinks, oversized fields, NaN/infinity encodings, timestamp overflow, and finding floods. Unwritable output directories must leave no committed report or repair file. No partial, unsupported, or timed-out operation may become success.

## Flake policy

Deterministic tests have zero retry allowance. Browser tests may retry only a separately diagnosed environment startup, never a semantic assertion. Any intermittent semantic result is release-blocking until explained and fixed.
