# Control protocol

`quatopsy control` reads `quatopsy.control-problem/1` and writes three files: `input.csv`, `manifest.json`, and `control.json`. The CSV and manifest are ordinary kernel inputs. `control.json` uses schema `quatopsy.control/1`.

## Ownership

The controller does not assign `pass`, `findings`, `refused`, or `error`. Those states exist only on `quatopsy.report/1` after a separate `analyze` invocation. `control.json` is refused if it would contain a `result` field. Command inhibition is decided by `quatopsy-oracle`, not by the PD law.

## Problem document

Required fields: `schema`, `component_order`, `rotation_sense`, `frame_from`, `frame_to`, `time_unit`, `execution`, `latency_class`, `q_initial`, `q_desired`, `omega_initial`, `inertia`, `torque_limit_nm`, `cycle_dt_s`, `duration_s`, `max_estimate_age_s`, `max_covariance_trace`, `gains`. Optional `slew_rate_limit_rad_s`, `momentum_limit_nms`, `actuators`, `sensor`, `keep_out_zones`, `campaign`, `hardware`, and `plant`.

Unknown fields are refused. Supported `rotation_sense` is `active` and `time_unit` is `s`. Initial rate must be rest. `execution` may be `sil`, `pil`, or `hil`. `latency_class` must be `bounded-software`. `hard-real-time` is refused; this crate is not an RTOS. `hardware.class` defaults to `loopback-emulator`. `physical` is refused by the systems-safety programme; there is no qualification record that can authorize hardware use. See `docs/CONTROL_SAFETY.md`.

The plant is a rigid body with a symmetric positive-definite inertia tensor and a torque box. Optional declared models add first-order torque lag, residual dipole torque, and gravity-gradient torque. That pair plus the optional models is the operating envelope together with optional slew, momentum, freshness, covariance, and keep-out cones.

## Execution classes

`sil` runs the cycle law and the plant in-process. `pil` runs the cycle law, including the independent monitor, in a child `control-cycle-worker` process on the host CPU; the plant stays in the parent. `hil` sends torque commands to a child `control-loopback-worker` process that emulates the plant; physical actuators are refused. Library calls without a worker binary exercise the same packet ABI in-process. Campaigns always run in-process SIL trials.

## Control law

`geometric-pd-so3` version `1` uses the rotation-matrix error `e_R = 1/2 vee(R_d^T R - R^T R_d)`, rate error `e_ω = ω - R^T R_d ω_d`, optional integral on `e_R` frozen under saturation, gyroscopic compensation `ω × (Jω + h)`, and feed-forward `J α_d`. Quaternion components are not the attitude-error coordinates, so `q` and `-q` command the same torque.

## Estimator contract

Each cycle requires an attitude estimate with timestamp, body rate, frame names, and covariance trace. Estimates older than `max_estimate_age_s`, from the future, with mismatched frames, non-finite values, or covariance above the envelope are inhibited. The SIL sensor is a delayed, optionally noisy plant measurement. `delay_s` is gyro delay. `star_tracker_delay_s` is attitude delay. Zero in either field means zero; star-tracker delay does not fall back to gyro delay. Campaign `delay_s` delays both. Optional `gyro_arw_rad_s_sqrt_s` adds a discrete angular-random-walk bias to the rate measurement. It is not a flight Kalman filter.

## Declared plant models

Optional `plant` fields are software models on the same loopback bus used by SIL, PIL, and HIL. They do not open a device.

- `wheel_lag_s`: first-order command-to-torque lag. Zero is instantaneous. This is not wheel-speed dynamics.
- `magnetic_residual.dipole_am2` and `field_t`: body dipole crossed with a declared frozen inertial field expressed in the body frame, `m × R^T B`. The field is a constant, not an orbit.
- `gravity_gradient.orbital_rate_rad_s` and `nadir_inertial`: `3 n^2 û × J û` with a declared frozen inertial nadir expressed in the body frame. Nadir is a constant, not an orbit.

Independent oracle functions compute lag, magnetic residual, and gravity-gradient torque. Unknown plant fields are refused. These models are not hardware, not a disturbance catalogue, and not a flight-environment certificate.

Motor torque (the lagged command) updates stored wheel momentum `h`. Magnetic residual and gravity-gradient torque enter Euler's equation only.

## Modes and inhibition

Modes are idle, track, hold, inhibit, and safe. Safe rate-damping is the fail-closed fallback after a monitor trip and does not return to track in the same run. The independent monitor reviews the controller command against the envelope before the plant integrates. Excess torque, stale data, keep-out violation, and numerical faults inhibit.

## Control document

`status` is `tracked-candidate` when the run stays inside the envelope and meets the compiled rest tolerance. `inhibited-candidate` records a monitor trip. `open-loop-candidate` means the run finished without inhibition but missed the rest tolerance. None of these is a report result. `execution`, `isolation`, and `hardware_class` record the class that actually ran. Optional `campaign` records deterministic SIL trials under inertia error, sensor noise, delay, disturbance, actuator failure, and numerical faults. It is not a robustness certificate.

The kernel manifest declares quaternion and angular-velocity columns; torque and optional momentum columns are present for inspection and ignored by analysis. Logged `tx,ty,tz` is the plant-applied body torque after command-to-torque lag and declared environmental models. The initial sample is zero because no plant step has run yet. The control cycle still holds the PD command constant across plant substeps. Logged sample density is raised so interval kinematics stay within the kernel omega tolerance.
