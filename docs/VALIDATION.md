# Validation

## Research questions

- RQ1: Can the supported rule set distinguish raw quaternion sign discontinuities from physical orientation discontinuities without false physical-motion claims?
- RQ2: Can Quatopsy localise and explain supported defects more completely than matched library primitives and generic trajectory viewers at the same claim scope?
- RQ3: Do repair candidates preserve declared physical orientation and provenance under independent mathematical oracles?
- RQ4: Are results deterministic, bounded, portable, and understandable in the supported workflow?

## Hypotheses and metrics

| Hypothesis | Primary metric | Falsification |
| --- | --- | --- |
| V1 rules are semantically correct | Exact rule/state/interval match on audited corpus | Any unexplained mismatch |
| Sign repair preserves orientation | Maximum independent matrix error | Error above registered tolerance |
| Aggregation fails closed | State under injected refusal/error | Any `pass` |
| Reports are deterministic | Byte digest across repeated/platform runs | Any unexplained digest difference |
| Diagnostic layer adds value over primitives | Complete supported defect explanation rate | No material gain at matched scope |

## Corpus

Before implementation evaluation, freeze inclusion rules and SHA-256 digests for: analytic constant-axis paths; random seeded paths; alternating-sign representations; near-zero and near-pi boundaries; norm drift; timestamp defects; convention mutations; commanded multi-turn and shortest paths; hostile parser cases; and redistributable public spacecraft examples. The TUBIN star-tracker excerpt (`fixtures/public/tubin_str/`, CC BY 4.0, 10.5281/zenodo.19708907) is the in-repo public-flight evaluation slice. Crashes, timeouts, refusals, and excluded rows remain in results.

Mutation tests invert named comparisons (norm, near-zero, time, sign, aggregator, omega, convention). Seeded fuzz (256 paths) asserts that refused, error, and findings states never become pass. Authoritative local CI runs on the development host; Linux-like reproduction is optional via `scripts/linux-conformance.sh` when a compatible container image is already present.

## Baselines

Matched baselines are SciPy `Rotation` and `Slerp`, ROS `tf2` shortest-path primitives, pytransform3d quaternion smoothing/checks, evo trajectory checks, and generic 3D replay where applicable. Baselines are compared only on overlapping claims. A viewer is not penalised for lacking a diagnostic contract it never claims.

## Oracles

Use independently encoded rotation matrices, analytic axis-angle cases, arbitrary-precision calculations near conditioning boundaries, and manually audited source intervals. Fixture generators cannot share the branch logic they validate. Visual inspection is supporting evidence only.

## Pre-registration

Freeze corpus hashes, seeds, rule versions, tolerances, timeouts, memory and finding limits, supported platforms, primary metrics, missing-data treatment, refusal accounting, and stopping rules before the benchmark run. Report all deviations and negative results.

## Statistical treatment

Conformance metrics are exact counts with confidence intervals for sampled fuzz domains. Performance reports median, p95, maximum memory, and input distribution across repeated isolated runs. Human comprehension or debugging-time studies, if later authorised, pre-register tasks, counterbalancing, exclusions, and analysis before collection.

## Claim discipline

Internal reproducible evidence can support implementation and release claims within the tested scope. External independent reproduction and expert challenge are optional and do not gate implementation, product completion, or release. Without them, public copy must not claim independent validation, broad external reproducibility, or expert consensus.

## Viewer browser evidence

The 2026-08-16 forensic gap-closure review generated the `sign_alternating` viewer through the public CLI and loaded it from a local HTTP server in the in-app browser. The accessibility snapshot exposed the skip link, six named regions, text verdict, labelled range input, descriptive canvas alternatives, keyboard-operable finding buttons, and separate raw, derived, and proposed state text. Clicking a finding selected the linked source-row geometry, the page emitted no console errors, and a 375 by 812 viewport had no horizontal overflow. The local server observed requests only for `index.html`, `viewer.js`, and `viewer.css`. Automated tests retain the source, CSP, contrast, keyboard wiring, bounded geometry, and finding-link assertions; this browser inspection is supporting evidence rather than a substitute for those tests.

The 2026-08-16 forensic-console refinement regenerated the `sign_alternating` bundle through the public CLI and inspected it at 1440 by 1000 and 375 by 812. Desktop visual QA confirmed the two linked attitude surfaces, full-width timeline, case metrics, and canonical evidence hierarchy. Narrow visual QA confirmed a single-column layout, readable canvas labels, 76/152/76 pixel transport controls, a 345 pixel panel inside the 375 pixel viewport, and no horizontal overflow (`scrollWidth` 360). Selecting the second finding moved to source row 3, stepping moved to row 4, and playback reached sample 4 of 4 and stopped. The accessibility tree exposed the advisory claim, canonical status, named regions, descriptive canvases, evidence buttons, sample values, and playback controls. The console emitted no browser logs and the local server received only `index.html`, `viewer.css`, and `viewer.js`. This remains supporting browser evidence; automated tests own the persistent source, contrast, CSP, responsive, reduced-motion, keyboard, and refusal checks.

## Optional post-release evidence

Independent reproduction, practitioner interviews, pilots, adoption cohorts, and ecosystem ranking may be pursued after release. Their absence prohibits demand, preference, adoption, and independently validated claims only.
