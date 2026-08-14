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

Before implementation evaluation, freeze inclusion rules and SHA-256 digests for: analytic constant-axis paths; random seeded paths; alternating-sign representations; near-zero and near-pi boundaries; norm drift; timestamp defects; convention mutations; commanded multi-turn and shortest paths; hostile parser cases; and pre-selected public spacecraft-like examples whose licences permit redistribution. Crashes, timeouts, refusals, and excluded rows remain in results.

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

## Optional post-release evidence

Independent reproduction, practitioner interviews, pilots, adoption cohorts, and ecosystem ranking may be pursued after release. Their absence prohibits demand, preference, adoption, and independently validated claims only.

