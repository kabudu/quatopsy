# Investigation fidelity increment

## Scope and acceptance

This owner-requested increment addresses the first-principles review of exact time, evidence presentation, investigation usability and bounded local work. The kernel remains the sole verdict owner. Planning, control and navigation remain candidate producers with unchanged hardware refusal boundaries.

| Requirement | Acceptance evidence | State |
| --- | --- | --- |
| Exact decimal timestamp parsing, checked bounds and wide interval subtraction | Adjacent nanoseconds above 2^53, decimal seconds, negative epochs, half-nanosecond rounding, both i64 endpoints and overflow regressions | implemented |
| Exact displayed timestamp identity | Additive string identity in geometry and DOM selection assertion | implemented |
| Truthful missing-data and downsample presentation | Segment identity survives omitted invalid samples; missing timeline values produce no zero point; links distinguish exact endpoints from approximate navigation | implemented |
| Complete finding navigation and source context | Paginated/filterable queue reaches finding 2501; canonical summary, severity, confidence, evidence units and source endpoint context | implemented |
| Named candidate comparison | Sign-lift and normalisation values equal canonical repair-plan outputs; changed components carry text labels as well as colour | implemented |
| Physical and representation controls | Axis selection, camera rotation/reset, declared frames, raw/lift projection, fit/reset, pole information and component traces | implemented |
| Time and sample playback | Time-proportional timeline; recorded-time and retained-sample modes; reduced-motion stepping; page-hide cancellation | implemented |
| Bounded generation and interaction | Streaming geometry passes, borrowed kernel samples, ordered source lookup, cooperative cancellation, cached plot layers and paginated DOM | implemented |
| Workflow performance | Nominal million-sample analyze/view/investigate/verify, dense-finding view, optional rate/matrix columns, 60-second stage deadline, 512 MiB child peak RSS and 32 MiB viewer payload | implemented |
| Desktop/narrow browser appearance and accessibility | Actual browser execution, screenshot inspection, keyboard/focus, reduced motion and no overflow at 1440, 1920 and 390 pixels | pending |
| Practitioner outcome evidence | Pre-registered evaluation protocol below; recruitment and actual observations require practitioners | pending |

## Compatibility

Report JSON retains `quatopsy.report/1` and integer nanosecond fields. Parsing now implements the existing checked decimal-time intent without an intermediate binary64 rounding step. Exact timestamp inputs that older versions incorrectly collapsed can change verdicts and report digests. Reproduce historical reports with their recorded tool version; do not substitute a newer engine for an old analysis.

The view payload remains `quatopsy.view/1`. Existing numeric `timestamp_ns` remains for old readers; the new UI uses `timestamp_ns_exact`. Additive `elapsed_s` is a plotting coordinate, not an exact identity. Geometry and stereo segment counters preserve gaps even when invalid samples are omitted by downsampling. `retained_findings` continues to mean that every finding has a navigation link; `exact_finding_endpoints` and per-link `exact_geometry` describe actual retained endpoints. Context stores up to six endpoint-neighbour rows per finding, deduplicated by source identity, rather than copying complete long intervals.

The viewport renders only retained samples and may connect distant source samples. These connections are explicitly approximate, not inferred physical interpolation. Global angle and rate extrema are retained; no claim covers every local extremum. Invalid or nonmonotonic time disables the elapsed-time axis and recorded-time playback. No browser rule evaluation is introduced.

## Validation boundary

`node scripts/test-viewer.mjs` runs the shipped viewer JavaScript against a simulated DOM/canvas. It checks interaction behaviour and drawing operations. It does not execute a real browser, establish CSS layout, measure accessibility-tree output or substitute for screenshot inspection.

In this task, the in-app browser rejected the original generated `file://` viewer under URL security policy and explicitly prohibited alternate routes around the block. No browser workaround was attempted. The original screenshots supplied by the owner guided the redesign; they are not evidence of the new layout. Rendered QA and merge readiness remain open until actual screenshots and interaction checks are obtained through an allowed route.

## Manual visual QA

Generate fresh bundles with `python3 scripts/prepare-viewer-qa.py`. Open the printed HTML paths manually. Inspect sign discontinuity, norm drift, moving attitude, decreasing time, exact epoch timestamps a dense queue and irregular +X rotation at 1440x900, 1920x1080 and 390x844.

- Verify that findings, plots and evidence are legible; controls do not overlap; tables scroll inside their panels; no document-wide horizontal overflow occurs.
- Select findings and inspect exact source context, canonical evidence, approximate-link notice and raw/lift/candidate values.
- Change body axis and camera; verify reset. Switch raw/lift stereographic layers; inspect poles, fit and reset.
- Toggle angle/rate and elapsed/index axes. Click points and compare the selected source row. Step, play and pause in both playback modes.
- Navigate using Tab and timeline arrow/Home/End keys. Focus must stay visible and arrow navigation must not scroll the page.
- Enable reduced motion before loading and while playback runs; playback must stop and subsequent activation must step once.
- Search and paginate the dense queue. Select the last finding. No finding should disappear solely because it is after the initial page.
- Check empty/report-only and refused states without implying a pass. Record screenshots, browser/version, dimensions, defects and resolutions here before marking visual acceptance complete.

## Practitioner evaluation protocol

Before expanding GN&C breadth, evaluate the core task: can an engineer explain and reproduce a supported orientation incident? Recruit spacecraft attitude-data practitioners; use only redistributable synthetic/public fixtures or separately authorised data. Freeze fixture digests, task wording and scoring before sessions. Counterbalance the order of Quatopsy and the participant's usual component-plot workflow.

Tasks: locate an injected sign discontinuity; distinguish it from physical motion; identify a norm defect; explain a refusal; describe candidate effects; hand the evidence to a second engineer for reproduction. Record completion time, correct source interval, correct representation/physical distinction, false-finding burden, repair understanding and successful independent reproduction. Include failures and refusals in results. Report paired observations and uncertainty without claiming independent validation from in-repository tests. Recruitment, observations and debugging-time gains are not claimed by this implementation.

## Local measurements (2026-09-08, macOS development host)

Released-CLI workflow run: nominal analyze 0.302 s, view 0.377 s, investigate 0.738 s and verify-evidence 0.034 s. Maximum cumulative child peak RSS was 397.9 MiB; nominal viewer payload was 2.12 MiB. Dense view took 0.065 s with a 3.95 MiB payload. These are a single local run, not portable timing guarantees. Skipping unnecessary repair re-ingest for reports without proposed repairs reduced the observed nominal investigation peak from 502.3 MiB to 397.9 MiB in these runs.
