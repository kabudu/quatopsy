# Prior art matrix

Search snapshot: 2026-08-14. This is an initial opportunity audit, not an exhaustive systematic review, patent opinion, or legal clearance.

| Source/system | Date/status | Subject and mechanism | Trust/assumption boundary | Evaluation/implementation | Overlap | Difference remaining to establish | Inclusion rationale |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Shoemake, quaternion animation and SLERP | Established, 1985 onward | Quaternion interpolation on the rotation sphere | Correct endpoints and conventions | Widely implemented | Core interpolation and sign-path foundation | No diagnostic contract claimed here | Foundational mechanism |
| Bhat and Bernstein, topological obstruction/unwinding line of work | Established | Global attitude stabilisation limits and quaternion unwinding | Controller and rigid-body assumptions | Theory and simulations across later literature | Explains unwinding risk | Quatopsy only diagnoses supported recorded/commanded paths | Core non-claim |
| NASA quaternion attitude-control reports | 1990s to 2024 | Quaternion conventions, sign ambiguity, shortest rotation, control | Mission-specific models and conventions | Public technical reports | Exact sign and control hazards | No reviewed integrated linter/report workflow found | Primary aerospace evidence |
| ROS `tf2::Quaternion` | Current documentation searched 2026-08-14 | Normalisation, nearest quaternion, shortest-path angle, SLERP | Caller supplies correct frames and conventions | Open source | Implements key primitives | Quatopsy candidate adds closed diagnostics, evidence, refusal, repair provenance | Closest library primitive |
| SciPy `Rotation`, `Slerp`, `RotationSpline` | v1.17 docs | Rotation conversion and shortest-path interpolation | Array/API caller declarations | Mature open source | Mathematical operations and smoothing | No closed spacecraft diagnostic result protocol | Baseline |
| pytransform3d | v3.15/3.16 docs | Quaternion difference and trajectory smoothing | Python arrays and documented conventions | Open source | Trajectory repair/smoothing primitives | No complete topological diagnostic contract found | Close repair primitive |
| evo trajectory tools | Current wiki edited 2026 | Pose import, quaternion/timestamp checks, plots, transformations | Supported formats and pose conventions | Open source, ROS-aware | Validation, plots, export, norm/time checks | Candidate difference narrows to quotient-aware explanations, reproducer, repair evidence, linked `S^3` view | Closest general trajectory tool found |
| Foxglove 3D and plot panels | Current docs | Pose, transform, quaternion/RPY visualisation and time plots | Correct schemas, frames, timestamps | Commercial/open ecosystem tool | Rich robotics replay and frame debugging | Does not establish Quatopsy rule semantics or repairs | Incumbent workflow |
| Basilisk Vizard | Current 2.x docs | Spacecraft simulation and attitude visualisation | Basilisk simulation data | Open-source simulator plus Unity viewer | Spacecraft 3D replay | Diagnostic contract and `S^3` evidence remain unestablished | First-vertical incumbent |
| ESA JUICE Cesium Viewer | Current docs | Mission attitude, trajectory, coverage, SPICE-derived scenes | JUICE planning formats and mission context | Operational web tool | Spacecraft attitude visualisation | Not a generic quotient-aware trajectory linter | Product boundary |
| Aptus | Current product page | MATLAB-based attitude representation and animation | MATLAB and supplied profiles | Commercial educational/engineering GUI | Representation and motion display | No evidence-bound lint/repair workflow found | Commercial visualiser |
| Hopf-fibration and `S^3` interactive visualisers | Current public demos | Stereographic/Hopf projections of unit quaternions | Educational geometry | Open web demos | Makes four-component geometry visible | No engineering diagnostic semantics | Reject visualisation-only novelty |
| Deep Bingham orientation uncertainty | ICLR 2020 and later work | Antipodally symmetric distributions over quaternion orientation | Learned uncertainty/model assumptions | Papers and code in related work | `S^3` uncertainty and visualisation | Uncertainty modelling is outside V1 | Adjacent future surface |
| CN113103240A and CN118426403A | Patent publications | Continuous quaternion trajectory generation and smoothing | Robot/tool-path assumptions | Patent documents | Repair/smoothing surface | Legal review needed before repair product claims | Patent collision surface |
| US6138061A | Patent, 2000 | Quaternion normalisation deviation monitoring in onboard propagation | Specific onboard orbit method | Patent document | Norm diagnostic | Expired/status and claim interpretation require counsel; no broad novelty inferred | Patent diagnostic surface |
| WO2024116253A1 | Patent publication | Selection of short/long SLERP for motion correction | 3D keypoint correction workflow | Patent document | Path selection and correction | Potentially adjacent repair claims require counsel | Recent patent surface |

## Initial conclusion

The individual mathematics, checks, interpolation choices, smoothing operations, visualisations, and spacecraft replay workflows are crowded prior art. The only defensible research direction is the narrow integration of a fail-closed diagnostic contract, representation-versus-physical separation, reproducible evidence, and provenance-preserving repair. That direction remains candidate until systematic comparison and matched evaluation.

## Initial source links

- [NASA: Spacecraft Modeling, Attitude Determination, and Control](https://ntrs.nasa.gov/citations/20240009554)
- [NASA: Quaternion-Based Control Architecture](https://ntrs.nasa.gov/api/citations/20120014565/downloads/20120014565.pdf)
- [ROS tf2 Quaternion source documentation](https://docs.ros.org/api/tf2/html/Quaternion_8h_source.html)
- [SciPy Slerp documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.spatial.transform.Slerp.html)
- [pytransform3d API documentation](https://dfki-ric.github.io/pytransform3d/api.html)
- [evo trajectory documentation](https://github.com/MichaelGrupp/evo/wiki/evo_traj)
- [Foxglove 3D panel documentation](https://docs.foxglove.dev/docs/visualization/panels/3d)
- [Basilisk Vizard documentation](https://avslab.github.io/basilisk/Vizard/VizardGUI.html)
- [ESA JUICE Cesium Viewer](https://juicesoc.esac.esa.int/help/40_cesium_viewer.html)
- [Interactive Hopf fibration implementation](https://github.com/wgxli/hopf-fibration)
- [Deep Bingham orientation uncertainty paper](https://tisl.cs.utoronto.ca/publication/202004-iclr-deep_orientation_uncertainty_learning/iclr20-deep_bingham.pdf)
- [CN113103240A](https://patents.google.com/patent/CN113103240A/en)
- [CN118426403A](https://patents.google.com/patent/CN118426403A/en)
- [US6138061A](https://patents.google.com/patent/US6138061A/en)
- [WO2024116253A1](https://patents.google.com/patent/WO2024116253A1/en)
