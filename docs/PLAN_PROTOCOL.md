# Plan protocol

`quatopsy plan` reads `quatopsy.plan-problem/1` and writes three files: `input.csv`, `manifest.json`, and `plan.json`. The CSV and manifest are ordinary kernel inputs. `plan.json` uses schema `quatopsy.plan/1`.

## Ownership

The planner does not assign `pass`, `findings`, `refused`, or `error`. Those states exist only on `quatopsy.report/1` after a separate `analyze` invocation. `plan.json` is refused if it would contain a `result` field.

## Problem document

Required fields: `schema`, `component_order`, `rotation_sense`, `frame_from`, `frame_to`, `time_unit`, `q_initial`, `q_final`, `omega_initial`, `omega_final`, `inertia`, `torque_limit_nm`, `sample_count`, `objective`. Optional `slew_rate_limit_rad_s` caps body rate. Optional `actuators`, `keep_out_zones`, `campaign`, and `solver` select the bounded shooting path.

Unknown fields are refused. Supported `rotation_sense` is `active` and `time_unit` is `s`. Boundary rates must be rest-to-rest. Inertia is `spherical`, `diagonal`, or a symmetric positive-definite `tensor` with products of inertia. `torque_limit_nm` is a positive scalar or a three-component box. Objective is `minimum-time` or a `weighted` object with non-negative `minimum_time`, `control_effort`, `energy_proxy`, `pointing`, `smoothness`, and `momentum` weights. Zero-angle and near-pi geodesics are refused.

Actuators may declare reaction wheels (axis, torque, momentum, optional power), torque thrusters, and a four-CMG pyramid. Keep-out zones are body-axis versus inertial-axis cones. A campaign requests a bounded open-loop perturbation study. `solver` may force `eigenaxis-bang-coast-bang` or `direct-shooting`; the legacy inputs `multiple-shooting` and `scvx-collocation` select the same bounded direct-shooting path and are retained only for input compatibility.

## Plan document

`status` is `feasible-candidate` when the independent oracle accepts the emitted samples. `optimality_class` is always `not-claimed`. Residuals are computed by `quatopsy-oracle` from rotation-matrix geodesics and Euler's equation `J ω̇ + ω × (Jω + h) = τ` on every adjacent pair, including switch samples. Torque discontinuities compare the inferred torque to the nearer endpoint. Kinematic residual tolerance matches `QAT-OMEGA-001` (`1e-3` rad/s). Rest-to-rest boundary residuals are `1e-4` to admit discrete shooting; the closed-form eigenaxis generator is typically far inside that bound. `infeasibility` is omitted on a feasible candidate. An optional `campaign` object records perturbed-model trials and is not a report result.

## Algorithms

`eigenaxis-bang-coast-bang` version `1` is used for minimum-time rest-to-rest problems without actuators, keep-out, or weighted trade-offs. It rotates about the shortest-path eigenaxis.

`direct-shooting-lm` version `1` is used when actuators, keep-out zones, weighted objectives, or a numerical solver is declared. Decision variables are duration and piecewise controls. Attitude is reconstructed with the SO(3) exponential map; quaternion components are never decision variables. Convergence is bounded by compiled iteration, decision, duration, and sample caps. A locally converged candidate is not globally optimal. This is not multiple shooting, a collocation transcription, or sequential convexification.

The kernel manifest declares quaternion and angular-velocity columns; torque and optional momentum columns are present for the oracle and ignored by analysis. `sample_count` is a lower bound; density is raised when needed so interval kinematics stay within the kernel omega tolerance.

This is not a controller. It does not command actuators. Optional `quatopsy control` `guidance.csv_text` may replay the emitted CSV as a time-tagged reference. That ingest does not make the plan a guidance solution or a report result.
