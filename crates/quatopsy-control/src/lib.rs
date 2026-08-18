//! Software-in-the-loop, processor-in-the-loop, and loopback hardware-in-the-loop
//! attitude control.
//!
//! Geometric PD commands are inhibited by an independent oracle monitor. This
//! crate never assigns a report `result` and never opens a physical actuator.

mod actuators;
mod campaign;
mod estimate;
mod geom;
mod isolation;
mod law;
mod modes;

pub use isolation::{run_cycle_worker, run_loopback_worker};

use campaign::{CampaignReport, CampaignSpec, perturb_inertia, unit_noise, validate};
use estimate::{Estimator, Measurement};
use geom::{Quat, add3, from_declared, invert_spd, norm3, rest};
use isolation::{
    ConeDoc, CycleBackend, CycleConfig, CycleIn, PlantBackend, PlantConfig, PlantIn, static_mode,
};
use law::Gains;
use quatopsy_oracle::{KeepOutCone, geodesic_angle};
use quatopsy_schema::{ComponentOrder, MANIFEST_SCHEMA, RotationSense, TimeUnit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;

const PROBLEM_SCHEMA: &str = "quatopsy.control-problem/1";
const CONTROL_SCHEMA: &str = "quatopsy.control/1";
const ALGORITHM: &str = "geometric-pd-so3";
const ALGORITHM_VERSION: &str = "1";
const MAX_SAMPLES: u64 = 100_000;
const MIN_SAMPLES: u64 = 3;
const REST_RATE_ABS: f64 = 1.0e-12;
const MAX_KEEP_OUT: usize = 8;
const KD_SAFE: f64 = 1.0;
const KERNEL_DT_MAX: f64 = 7.5e-4;

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("{0}")]
    Refused(String),
}

#[derive(Debug, Clone)]
pub struct ControlOutput {
    pub csv: String,
    pub manifest: String,
    pub control: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProblemDocument {
    schema: String,
    component_order: ComponentOrder,
    rotation_sense: RotationSense,
    frame_from: String,
    frame_to: String,
    time_unit: TimeUnit,
    execution: Execution,
    latency_class: LatencyClass,
    q_initial: [f64; 4],
    q_desired: [f64; 4],
    omega_initial: [f64; 3],
    inertia: Inertia,
    torque_limit_nm: TorqueLimit,
    #[serde(default)]
    slew_rate_limit_rad_s: Option<f64>,
    #[serde(default)]
    momentum_limit_nms: Option<f64>,
    cycle_dt_s: f64,
    duration_s: f64,
    max_estimate_age_s: f64,
    max_covariance_trace: f64,
    gains: GainsDoc,
    #[serde(default)]
    actuators: Actuators,
    #[serde(default)]
    sensor: Sensor,
    #[serde(default)]
    keep_out_zones: Vec<KeepOutZone>,
    #[serde(default)]
    campaign: Option<CampaignSpec>,
    #[serde(default)]
    hardware: Hardware,
    #[serde(default)]
    plant: PlantModels,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Execution {
    Sil,
    Pil,
    Hil,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum LatencyClass {
    BoundedSoftware,
    HardRealTime,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
struct Hardware {
    #[serde(default)]
    class: HardwareClass,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum HardwareClass {
    #[default]
    LoopbackEmulator,
    Physical,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
struct PlantModels {
    /// Command-to-torque first-order lag. Not wheel-speed dynamics.
    #[serde(default)]
    wheel_lag_s: f64,
    #[serde(default)]
    magnetic_residual: MagneticResidual,
    #[serde(default)]
    gravity_gradient: GravityGradient,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
struct MagneticResidual {
    #[serde(default)]
    dipole_am2: [f64; 3],
    #[serde(default)]
    field_t: [f64; 3],
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
struct GravityGradient {
    #[serde(default)]
    orbital_rate_rad_s: f64,
    #[serde(default = "default_nadir")]
    nadir_inertial: [f64; 3],
}

fn default_nadir() -> [f64; 3] {
    [0.0, 0.0, 1.0]
}

#[derive(Debug, Deserialize)]
#[serde(tag = "model", rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
enum Inertia {
    Spherical {
        j: f64,
    },
    Diagonal {
        jxx: f64,
        jyy: f64,
        jzz: f64,
    },
    Tensor {
        jxx: f64,
        jyy: f64,
        jzz: f64,
        #[serde(default)]
        jxy: f64,
        #[serde(default)]
        jxz: f64,
        #[serde(default)]
        jyz: f64,
    },
}

impl Inertia {
    fn tensor(&self) -> Result<[[f64; 3]; 3], ControlError> {
        let j = match self {
            Self::Spherical { j } => {
                if !j.is_finite() || *j <= 0.0 {
                    return Err(ControlError::Refused(
                        "spherical inertia must be finite and positive".to_string(),
                    ));
                }
                [[*j, 0.0, 0.0], [0.0, *j, 0.0], [0.0, 0.0, *j]]
            }
            Self::Diagonal { jxx, jyy, jzz } => {
                [[*jxx, 0.0, 0.0], [0.0, *jyy, 0.0], [0.0, 0.0, *jzz]]
            }
            Self::Tensor {
                jxx,
                jyy,
                jzz,
                jxy,
                jxz,
                jyz,
            } => [[*jxx, *jxy, *jxz], [*jxy, *jyy, *jyz], [*jxz, *jyz, *jzz]],
        };
        invert_spd(j)?;
        Ok(j)
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
enum TorqueLimit {
    Scalar(f64),
    Box([f64; 3]),
}

impl TorqueLimit {
    fn box_limit(self) -> Result<[f64; 3], ControlError> {
        let limit = match self {
            Self::Scalar(value) => [value, value, value],
            Self::Box(value) => value,
        };
        if limit.iter().any(|item| !item.is_finite() || *item <= 0.0) {
            return Err(ControlError::Refused(
                "torque_limit_nm must be finite and positive".to_string(),
            ));
        }
        Ok(limit)
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct GainsDoc {
    kp: f64,
    kd: f64,
    #[serde(default)]
    ki: f64,
}

impl GainsDoc {
    fn validated(self) -> Result<Gains, ControlError> {
        if ![self.kp, self.kd, self.ki]
            .iter()
            .all(|item| item.is_finite())
            || self.kp <= 0.0
            || self.kd <= 0.0
            || self.ki < 0.0
            || self.kp > 1.0e4
            || self.kd > 1.0e4
            || self.ki > 1.0e3
        {
            return Err(ControlError::Refused(
                "controller gains must be finite, kp and kd positive, and ki non-negative within compiled bounds"
                    .to_string(),
            ));
        }
        Ok(Gains {
            kp: self.kp,
            kd: self.kd,
            ki: self.ki,
        })
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Actuators {
    #[serde(default)]
    wheels: bool,
    #[serde(default = "default_dump")]
    momentum_dump_gain: f64,
}

impl Default for Actuators {
    fn default() -> Self {
        Self {
            wheels: false,
            momentum_dump_gain: default_dump(),
        }
    }
}

fn default_dump() -> f64 {
    0.5
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Sensor {
    /// Gyro measurement delay. Zero means zero.
    #[serde(default)]
    delay_s: f64,
    #[serde(default)]
    quat_noise: f64,
    #[serde(default)]
    rate_noise: f64,
    /// Attitude measurement delay. Zero means zero, not a fallback to `delay_s`.
    #[serde(default)]
    star_tracker_delay_s: f64,
    #[serde(default)]
    gyro_arw_rad_s_sqrt_s: f64,
    #[serde(default = "default_cov")]
    covariance_trace: f64,
}

impl Default for Sensor {
    fn default() -> Self {
        Self {
            delay_s: 0.0,
            quat_noise: 0.0,
            rate_noise: 0.0,
            star_tracker_delay_s: 0.0,
            gyro_arw_rad_s_sqrt_s: 0.0,
            covariance_trace: default_cov(),
        }
    }
}

fn default_cov() -> f64 {
    1.0e-6
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct KeepOutZone {
    body_axis: [f64; 3],
    inertial_axis: [f64; 3],
    min_angle_rad: f64,
}

#[derive(Debug, Serialize)]
struct ControlDocument {
    schema: &'static str,
    algorithm: &'static str,
    algorithm_version: &'static str,
    status: &'static str,
    execution: &'static str,
    latency_class: &'static str,
    hardware_class: &'static str,
    isolation: &'static str,
    notes: &'static str,
    duration_s: f64,
    cycle_dt_s: f64,
    sample_count: u64,
    inhibit_count: u64,
    last_inhibit_reason: Option<String>,
    terminal_attitude_error_rad: f64,
    terminal_rate_error_rad_s: f64,
    mode: &'static str,
    problem_sha256: String,
    trajectory_sha256: String,
    manifest_sha256: String,
    campaign: Option<CampaignReport>,
    engine_version: String,
}

struct Prepared {
    q0: Quat,
    q_des: Quat,
    omega0: [f64; 3],
    inertia: [[f64; 3]; 3],
    torque_limit: [f64; 3],
    slew: Option<f64>,
    momentum_limit: Option<f64>,
    dt: f64,
    steps: usize,
    max_age: f64,
    max_cov: f64,
    gains: Gains,
    wheels: bool,
    dump_gain: f64,
    sensor: Sensor,
    keep_out: Vec<KeepOutCone>,
    frame_from: String,
    frame_to: String,
    substeps: usize,
    dt_sub: f64,
    execution: Execution,
    plant: PlantModels,
}

struct Faults {
    inertia: [[f64; 3]; 3],
    quat_noise: f64,
    rate_noise: f64,
    delay_s: f64,
    star_tracker_delay_s: f64,
    disturbance: [f64; 3],
    fail_axis: Option<usize>,
    nan_step: Option<usize>,
    seed: u64,
}

struct SilRun {
    csv: String,
    status: &'static str,
    inhibit_count: u64,
    last_inhibit_reason: Option<String>,
    terminal_attitude_error_rad: f64,
    terminal_rate_error_rad_s: f64,
    mode: &'static str,
    sample_count: u64,
    inhibited: bool,
}

pub fn control(problem_bytes: &[u8], version: &str) -> Result<ControlOutput, ControlError> {
    control_with_workers(problem_bytes, version, None)
}

pub fn control_with_workers(
    problem_bytes: &[u8],
    version: &str,
    worker_bin: Option<&Path>,
) -> Result<ControlOutput, ControlError> {
    let problem: ProblemDocument = serde_json::from_slice(problem_bytes)
        .map_err(|err| ControlError::Refused(format!("control problem parse failed: {err}")))?;
    let prepared = prepare(&problem)?;
    let nominal = Faults {
        inertia: prepared.inertia,
        quat_noise: prepared.sensor.quat_noise,
        rate_noise: prepared.sensor.rate_noise,
        delay_s: prepared.sensor.delay_s,
        star_tracker_delay_s: prepared.sensor.star_tracker_delay_s,
        disturbance: [0.0; 3],
        fail_axis: None,
        nan_step: None,
        seed: 1,
    };
    let run = run_closed_loop(&prepared, &nominal, worker_bin)?;
    let campaign = match &problem.campaign {
        Some(spec) => Some(run_campaign(&prepared, spec)?),
        None => None,
    };
    let isolated = worker_bin.is_some() && !matches!(prepared.execution, Execution::Sil);
    let (execution, isolation, notes) = match prepared.execution {
        Execution::Sil => (
            "sil",
            "in-process",
            "Software-in-the-loop geometric PD on SO(3). Not flight approval. Not a report result. Not hard real-time. Not a hardware command.",
        ),
        Execution::Pil => (
            "pil",
            if isolated {
                "isolated-controller-process"
            } else {
                "in-process-packet-abi"
            },
            "Processor-in-the-loop geometric PD. Controller cycle is isolated from the plant. Host CPU, not a qualified flight processor. Not flight approval.",
        ),
        Execution::Hil => (
            "hil",
            if isolated {
                "loopback-actuator-emulator"
            } else {
                "in-process-packet-abi"
            },
            "Hardware-in-the-loop command bus against a loopback actuator emulator. Physical actuators are refused. Not flight approval.",
        ),
    };
    let manifest = serde_json::json!({
        "schema": MANIFEST_SCHEMA,
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": prepared.frame_from,
        "frame_to": prepared.frame_to,
        "time_unit": "s",
        "columns": {
            "time": "t",
            "quaternion": ["qw", "qx", "qy", "qz"],
            "angular_velocity": ["wx", "wy", "wz"]
        }
    })
    .to_string();
    let document = ControlDocument {
        schema: CONTROL_SCHEMA,
        algorithm: ALGORITHM,
        algorithm_version: ALGORITHM_VERSION,
        status: run.status,
        execution,
        latency_class: "bounded-software",
        hardware_class: "loopback-emulator",
        isolation,
        notes,
        duration_s: prepared.dt * (prepared.steps as f64 - 1.0),
        cycle_dt_s: prepared.dt,
        sample_count: run.sample_count,
        inhibit_count: run.inhibit_count,
        last_inhibit_reason: run.last_inhibit_reason,
        terminal_attitude_error_rad: run.terminal_attitude_error_rad,
        terminal_rate_error_rad_s: run.terminal_rate_error_rad_s,
        mode: run.mode,
        problem_sha256: digest_hex(problem_bytes),
        trajectory_sha256: digest_hex(run.csv.as_bytes()),
        manifest_sha256: digest_hex(manifest.as_bytes()),
        campaign,
        engine_version: version.to_string(),
    };
    let control_json = serde_json::to_string(&document)
        .map_err(|err| ControlError::Refused(format!("control serialize failed: {err}")))?;
    let parsed: serde_json::Value = serde_json::from_str(&control_json).map_err(|err| {
        ControlError::Refused(format!("control serialize roundtrip failed: {err}"))
    })?;
    if parsed.get("result").is_some() {
        return Err(ControlError::Refused(
            "controller output must not contain a result field".to_string(),
        ));
    }
    Ok(ControlOutput {
        csv: run.csv,
        manifest,
        control: control_json,
    })
}

fn prepare(problem: &ProblemDocument) -> Result<Prepared, ControlError> {
    if problem.schema != PROBLEM_SCHEMA {
        return Err(ControlError::Refused(format!(
            "unsupported control problem schema {}",
            problem.schema
        )));
    }
    match problem.execution {
        Execution::Sil | Execution::Pil | Execution::Hil => {}
    }
    if problem.hardware.class == HardwareClass::Physical {
        return Err(ControlError::Refused(
            "physical actuators are refused; the safety programme has no qualification record"
                .to_string(),
        ));
    }
    if matches!(problem.latency_class, LatencyClass::HardRealTime) {
        return Err(ControlError::Refused(
            "hard real-time execution is refused; only bounded-software latency is implemented"
                .to_string(),
        ));
    }
    if problem.rotation_sense != RotationSense::Active {
        return Err(ControlError::Refused(
            "controller supports active rotation_sense only".to_string(),
        ));
    }
    if problem.time_unit != TimeUnit::S {
        return Err(ControlError::Refused(
            "controller supports time_unit s only".to_string(),
        ));
    }
    if problem.frame_from.is_empty()
        || problem.frame_to.is_empty()
        || problem.frame_from == problem.frame_to
    {
        return Err(ControlError::Refused(
            "frame_from and frame_to must be distinct and non-empty".to_string(),
        ));
    }
    if !rest(problem.omega_initial, REST_RATE_ABS) {
        return Err(ControlError::Refused(
            "controller supports rest initial rates only".to_string(),
        ));
    }
    if !problem.cycle_dt_s.is_finite() || problem.cycle_dt_s < 1.0e-4 || problem.cycle_dt_s > 0.1 {
        return Err(ControlError::Refused(
            "cycle_dt_s must be in [1e-4, 0.1]".to_string(),
        ));
    }
    if !problem.duration_s.is_finite() || problem.duration_s <= 0.0 || problem.duration_s > 3_600.0
    {
        return Err(ControlError::Refused(
            "duration_s must be in (0, 3600]".to_string(),
        ));
    }
    let steps = (problem.duration_s / problem.cycle_dt_s).round() as u64 + 1;
    if !(MIN_SAMPLES..=MAX_SAMPLES).contains(&steps) {
        return Err(ControlError::Refused(
            "controller sample count is outside the supported bound".to_string(),
        ));
    }
    let substeps = ((problem.cycle_dt_s / KERNEL_DT_MAX).ceil() as u64).max(1);
    let logged = 1 + (steps - 1) * substeps;
    if logged > MAX_SAMPLES {
        return Err(ControlError::Refused(
            "controller would exceed the sample limit to keep rate samples kernel-consistent"
                .to_string(),
        ));
    }
    if !problem.max_estimate_age_s.is_finite() || problem.max_estimate_age_s < 0.0 {
        return Err(ControlError::Refused(
            "max_estimate_age_s must be finite and non-negative".to_string(),
        ));
    }
    if !problem.max_covariance_trace.is_finite() || problem.max_covariance_trace <= 0.0 {
        return Err(ControlError::Refused(
            "max_covariance_trace must be finite and positive".to_string(),
        ));
    }
    if problem.keep_out_zones.len() > MAX_KEEP_OUT {
        return Err(ControlError::Refused(
            "controller supports at most 8 keep-out zones".to_string(),
        ));
    }
    if !problem.actuators.momentum_dump_gain.is_finite()
        || problem.actuators.momentum_dump_gain < 0.0
        || problem.actuators.momentum_dump_gain > 10.0
    {
        return Err(ControlError::Refused(
            "momentum_dump_gain must be in [0, 10]".to_string(),
        ));
    }
    if !problem.sensor.delay_s.is_finite()
        || problem.sensor.delay_s < 0.0
        || problem.sensor.delay_s > 1.0
        || !problem.sensor.quat_noise.is_finite()
        || problem.sensor.quat_noise < 0.0
        || !problem.sensor.rate_noise.is_finite()
        || problem.sensor.rate_noise < 0.0
        || !problem.sensor.star_tracker_delay_s.is_finite()
        || problem.sensor.star_tracker_delay_s < 0.0
        || problem.sensor.star_tracker_delay_s > 1.0
        || !problem.sensor.gyro_arw_rad_s_sqrt_s.is_finite()
        || problem.sensor.gyro_arw_rad_s_sqrt_s < 0.0
        || problem.sensor.gyro_arw_rad_s_sqrt_s > 1.0
        || !problem.sensor.covariance_trace.is_finite()
        || problem.sensor.covariance_trace < 0.0
    {
        return Err(ControlError::Refused(
            "sensor delay, noise, star-tracker delay, gyro ARW, and covariance must be finite and non-negative within compiled bounds"
                .to_string(),
        ));
    }
    if !problem.plant.wheel_lag_s.is_finite()
        || problem.plant.wheel_lag_s < 0.0
        || problem.plant.wheel_lag_s > 10.0
        || !problem
            .plant
            .magnetic_residual
            .dipole_am2
            .iter()
            .all(|item| item.is_finite())
        || !problem
            .plant
            .magnetic_residual
            .field_t
            .iter()
            .all(|item| item.is_finite())
        || !problem
            .plant
            .gravity_gradient
            .orbital_rate_rad_s
            .is_finite()
        || problem.plant.gravity_gradient.orbital_rate_rad_s < 0.0
        || problem.plant.gravity_gradient.orbital_rate_rad_s > 0.1
        || !problem
            .plant
            .gravity_gradient
            .nadir_inertial
            .iter()
            .all(|item| item.is_finite())
    {
        return Err(ControlError::Refused(
            "declared plant models must be finite and inside compiled bounds".to_string(),
        ));
    }
    if problem.plant.gravity_gradient.orbital_rate_rad_s > 0.0
        && norm3(problem.plant.gravity_gradient.nadir_inertial) < 1.0e-12
    {
        return Err(ControlError::Refused(
            "gravity-gradient nadir_inertial must be non-zero when orbital_rate_rad_s is positive"
                .to_string(),
        ));
    }
    if let Some(slew) = problem.slew_rate_limit_rad_s
        && (!slew.is_finite() || slew <= 0.0)
    {
        return Err(ControlError::Refused(
            "slew_rate_limit_rad_s must be finite and positive".to_string(),
        ));
    }
    if let Some(limit) = problem.momentum_limit_nms
        && (!limit.is_finite() || limit <= 0.0)
    {
        return Err(ControlError::Refused(
            "momentum_limit_nms must be finite and positive".to_string(),
        ));
    }
    Ok(Prepared {
        q0: from_declared(problem.q_initial, problem.component_order)?.normalized()?,
        q_des: from_declared(problem.q_desired, problem.component_order)?.normalized()?,
        omega0: problem.omega_initial,
        inertia: problem.inertia.tensor()?,
        torque_limit: problem.torque_limit_nm.box_limit()?,
        slew: problem.slew_rate_limit_rad_s,
        momentum_limit: problem.momentum_limit_nms,
        dt: problem.cycle_dt_s,
        steps: steps as usize,
        max_age: problem.max_estimate_age_s,
        max_cov: problem.max_covariance_trace,
        gains: problem.gains.validated()?,
        wheels: problem.actuators.wheels,
        dump_gain: problem.actuators.momentum_dump_gain,
        sensor: problem.sensor,
        keep_out: prepare_keep_out(&problem.keep_out_zones)?,
        frame_from: problem.frame_from.clone(),
        frame_to: problem.frame_to.clone(),
        substeps: substeps as usize,
        dt_sub: problem.cycle_dt_s / substeps as f64,
        execution: problem.execution,
        plant: problem.plant,
    })
}

fn prepare_keep_out(zones: &[KeepOutZone]) -> Result<Vec<KeepOutCone>, ControlError> {
    let mut out = Vec::with_capacity(zones.len());
    for zone in zones {
        if !zone.min_angle_rad.is_finite() || zone.min_angle_rad < 0.0 || zone.min_angle_rad >= 3.2
        {
            return Err(ControlError::Refused(
                "keep-out min_angle_rad must be finite and in [0, 3.2)".to_string(),
            ));
        }
        out.push(KeepOutCone {
            body_axis: zone.body_axis,
            inertial_axis: zone.inertial_axis,
            min_angle_rad: zone.min_angle_rad,
        });
    }
    Ok(out)
}

fn cycle_config(prepared: &Prepared, faults: &Faults) -> CycleConfig {
    CycleConfig {
        inertia: faults.inertia,
        torque_limit_nm: prepared.torque_limit,
        slew_rate_limit_rad_s: prepared.slew,
        momentum_limit_nms: prepared.momentum_limit,
        max_estimate_age_s: prepared.max_age,
        max_covariance_trace: prepared.max_cov,
        dt: prepared.dt,
        kp: prepared.gains.kp,
        kd: prepared.gains.kd,
        ki: prepared.gains.ki,
        q_desired: [
            prepared.q_des.w,
            prepared.q_des.x,
            prepared.q_des.y,
            prepared.q_des.z,
        ],
        dump_gain: prepared.dump_gain,
        keep_out: prepared
            .keep_out
            .iter()
            .map(|zone| ConeDoc {
                body_axis: zone.body_axis,
                inertial_axis: zone.inertial_axis,
                min_angle_rad: zone.min_angle_rad,
            })
            .collect(),
    }
}

fn plant_config(prepared: &Prepared, faults: &Faults) -> PlantConfig {
    PlantConfig {
        inertia: faults.inertia,
        wheels: prepared.wheels,
        q: [prepared.q0.w, prepared.q0.x, prepared.q0.y, prepared.q0.z],
        omega: prepared.omega0,
        h: [0.0; 3],
        wheel_lag_s: prepared.plant.wheel_lag_s,
        magnetic_dipole_am2: prepared.plant.magnetic_residual.dipole_am2,
        magnetic_field_t: prepared.plant.magnetic_residual.field_t,
        orbital_rate_rad_s: prepared.plant.gravity_gradient.orbital_rate_rad_s,
        nadir_inertial: prepared.plant.gravity_gradient.nadir_inertial,
    }
}

fn run_closed_loop(
    prepared: &Prepared,
    faults: &Faults,
    worker_bin: Option<&Path>,
) -> Result<SilRun, ControlError> {
    let cycle_bin = match prepared.execution {
        Execution::Pil => worker_bin,
        Execution::Sil | Execution::Hil => None,
    };
    let plant_bin = match prepared.execution {
        Execution::Hil => worker_bin,
        Execution::Sil | Execution::Pil => None,
    };
    let mut cycle = CycleBackend::open(cycle_config(prepared, faults), cycle_bin)?;
    let mut plant = PlantBackend::open(plant_config(prepared, faults), plant_bin)?;
    let mut q = prepared.q0;
    let mut w = prepared.omega0;
    let mut h = [0.0; 3];
    let mut history: Vec<(f64, Quat, [f64; 3])> = Vec::with_capacity(prepared.steps);
    let mut estimator = Estimator::new();
    let mut inhibit_count = 0_u64;
    let mut last_reason: Option<String> = None;
    let mut last_mode = "idle";
    let mut rows = Vec::with_capacity(prepared.steps);
    let mut arw_bias = [0.0; 3];
    let quat_delay = faults.star_tracker_delay_s;
    for i in 0..prepared.steps.saturating_sub(1) {
        let mut t = prepared.dt * i as f64;
        history.push((t, q, w));
        let delayed_q = delayed_state(&history, t, quat_delay);
        let delayed_w = delayed_state(&history, t, faults.delay_s);
        let mut meas_q = delayed_q.1;
        if faults.quat_noise > 0.0 {
            meas_q = meas_q
                .mul(geom::exp_so3([
                    faults.quat_noise * unit_noise(faults.seed, (i as u32) * 3),
                    faults.quat_noise * unit_noise(faults.seed, (i as u32) * 3 + 1),
                    faults.quat_noise * unit_noise(faults.seed, (i as u32) * 3 + 2),
                ]))
                .normalized()?;
        }
        let mut meas_w = delayed_w.2;
        if faults.rate_noise > 0.0 {
            meas_w = add3(
                meas_w,
                [
                    faults.rate_noise * unit_noise(faults.seed, 1000 + i as u32),
                    faults.rate_noise * unit_noise(faults.seed, 2000 + i as u32),
                    faults.rate_noise * unit_noise(faults.seed, 3000 + i as u32),
                ],
            );
        }
        if prepared.sensor.gyro_arw_rad_s_sqrt_s > 0.0 {
            let step = prepared.sensor.gyro_arw_rad_s_sqrt_s * prepared.dt.sqrt();
            arw_bias = add3(
                arw_bias,
                [
                    step * unit_noise(faults.seed, 4000 + i as u32),
                    step * unit_noise(faults.seed, 5000 + i as u32),
                    step * unit_noise(faults.seed, 6000 + i as u32),
                ],
            );
            meas_w = add3(meas_w, arw_bias);
        }
        if faults.nan_step == Some(i) {
            meas_q.w = f64::NAN;
        }
        let estimate = estimator.ingest(Measurement {
            t_s: delayed_q.0,
            q: meas_q,
            omega: meas_w,
            covariance_trace: prepared.sensor.covariance_trace,
            frame_from_ok: true,
            frame_to_ok: true,
        });
        let commanded = cycle.step(CycleIn {
            t,
            q: [estimate.q.w, estimate.q.x, estimate.q.y, estimate.q.z],
            omega: estimate.omega,
            h,
            estimate_t_s: estimate.t_s,
            covariance_trace: estimate.covariance_trace,
            frames_ok: estimate.frame_from_ok && estimate.frame_to_ok,
            fail_axis: faults.fail_axis.and_then(|axis| u8::try_from(axis).ok()),
            disturbance_nm: faults.disturbance,
        })?;
        last_mode = static_mode(&commanded.mode);
        if commanded.inhibited {
            inhibit_count += 1;
            last_reason = commanded.reason;
        }
        let applied = commanded.torque;
        if i == 0 {
            rows.push((t, q, w, [0.0; 3], h));
        }
        let next = plant.step(PlantIn {
            torque: applied,
            dt_sub: prepared.dt_sub,
            substeps: prepared.substeps as u32,
        })?;
        if next.samples.len() != prepared.substeps {
            return Err(ControlError::Refused(
                "plant worker returned the wrong number of substeps".to_string(),
            ));
        }
        for sample in next.samples {
            q = Quat::new(sample.q[0], sample.q[1], sample.q[2], sample.q[3]).normalized()?;
            w = sample.omega;
            h = sample.h;
            t += prepared.dt_sub;
            rows.push((t, q, w, sample.torque, h));
        }
    }
    let terminal_att = geodesic_angle(q.as_ref(), prepared.q_des.as_ref());
    let terminal_rate = norm3(w);
    let status = if inhibit_count == 0 && terminal_att <= 0.05 && terminal_rate <= 0.05 {
        "tracked-candidate"
    } else if inhibit_count > 0 {
        "inhibited-candidate"
    } else {
        "open-loop-candidate"
    };
    Ok(SilRun {
        csv: render_csv(&rows),
        status,
        inhibit_count,
        last_inhibit_reason: last_reason,
        terminal_attitude_error_rad: terminal_att,
        terminal_rate_error_rad_s: terminal_rate,
        mode: last_mode,
        sample_count: rows.len() as u64,
        inhibited: inhibit_count > 0,
    })
}

fn delayed_state(history: &[(f64, Quat, [f64; 3])], now: f64, delay: f64) -> (f64, Quat, [f64; 3]) {
    let target = now - delay;
    let mut chosen = history[0];
    for item in history {
        if item.0 <= target {
            chosen = *item;
        }
    }
    chosen
}

fn run_campaign(prepared: &Prepared, spec: &CampaignSpec) -> Result<CampaignReport, ControlError> {
    validate(spec)?;
    let mut rng = spec.seed | 1;
    let mut inhibit_events = 0;
    let mut terminal_attitude_fail = 0;
    for trial in 0..spec.trials {
        rng = campaign::next_rng(rng);
        let inertia = perturb_inertia(prepared.inertia, spec.inertia_rel_sigma, rng);
        let faults = Faults {
            inertia,
            quat_noise: spec.quat_noise,
            rate_noise: spec.rate_noise,
            delay_s: spec.delay_s,
            star_tracker_delay_s: spec.delay_s,
            disturbance: [
                spec.disturbance_nm * unit_noise(rng, 7),
                spec.disturbance_nm * unit_noise(rng, 8),
                spec.disturbance_nm * unit_noise(rng, 9),
            ],
            fail_axis: spec.actuator_fail_axis.map(usize::from),
            nan_step: if spec.numerical_fault {
                Some(prepared.steps / 2)
            } else {
                None
            },
            seed: rng.wrapping_add(u64::from(trial)),
        };
        let run = run_closed_loop(prepared, &faults, None)?;
        if run.inhibited {
            inhibit_events += 1;
        }
        if run.terminal_attitude_error_rad > 0.05 {
            terminal_attitude_fail += 1;
        }
    }
    Ok(CampaignReport {
        trials: spec.trials,
        inhibit_events,
        terminal_attitude_fail,
        notes: "Deterministic SIL robustness trials. Not a report result. Not a flight robustness certificate.",
    })
}

type CsvRow = (f64, Quat, [f64; 3], [f64; 3], [f64; 3]);

fn render_csv(rows: &[CsvRow]) -> String {
    let mut csv = String::from("t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz,hx,hy,hz\n");
    for (t, q, w, tau, h) in rows {
        csv.push_str(&format!(
            "{t:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}\n",
            q.w, q.x, q.y, q.z, w[0], w[1], w[2], tau[0], tau[1], tau[2], h[0], h[1], h[2]
        ));
    }
    csv
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::so3_error;
    use quatopsy_oracle::{
        KeepOutCone, MonitorEnvelope, MonitorSample, keep_out_violation, monitor_command,
        so3_attitude_error,
    };

    fn identity_hold() -> serde_json::Value {
        let mut value = example_problem();
        value["q_desired"] = serde_json::json!([1.0, 0.0, 0.0, 0.0]);
        value["duration_s"] = serde_json::json!(0.2);
        value
    }

    fn csv_data(csv: &str) -> Vec<Vec<f64>> {
        csv.lines()
            .skip(1)
            .filter(|line| !line.is_empty())
            .map(|line| {
                line.split(',')
                    .map(|item| item.parse::<f64>().unwrap())
                    .collect()
            })
            .collect()
    }

    fn example_problem() -> serde_json::Value {
        serde_json::json!({
            "schema": "quatopsy.control-problem/1",
            "component_order": "wxyz",
            "rotation_sense": "active",
            "frame_from": "BODY",
            "frame_to": "J2000",
            "time_unit": "s",
            "execution": "sil",
            "latency_class": "bounded-software",
            "q_initial": [1.0, 0.0, 0.0, 0.0],
            "q_desired": [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0, 0.0],
            "omega_initial": [0.0, 0.0, 0.0],
            "inertia": {"model": "spherical", "j": 1.0},
            "torque_limit_nm": 1.0,
            "slew_rate_limit_rad_s": 2.0,
            "momentum_limit_nms": 4.0,
            "cycle_dt_s": 0.02,
            "duration_s": 8.0,
            "max_estimate_age_s": 0.05,
            "max_covariance_trace": 1.0,
            "gains": {"kp": 4.0, "kd": 4.0, "ki": 0.0},
            "actuators": {"wheels": false, "momentum_dump_gain": 0.5},
            "sensor": {"delay_s": 0.0, "quat_noise": 0.0, "rate_noise": 0.0, "covariance_trace": 1e-6}
        })
    }

    fn run(value: &serde_json::Value) -> Result<ControlOutput, ControlError> {
        control(&serde_json::to_vec(value).unwrap(), "0.1.0")
    }

    #[test]
    fn sil_rest_to_rest_tracks_without_a_result_field() {
        let out = run(&example_problem()).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert!(doc.get("result").is_none());
        assert_eq!(doc["algorithm"], "geometric-pd-so3");
        assert_eq!(doc["execution"], "sil");
        assert_eq!(doc["status"], "tracked-candidate");
        assert!(doc["terminal_attitude_error_rad"].as_f64().unwrap() <= 0.05);
    }

    #[test]
    fn antipodal_initial_attitude_does_not_unwind() {
        let mut value = example_problem();
        value["q_initial"] = serde_json::json!([-1.0, 0.0, 0.0, 0.0]);
        value["q_desired"] = serde_json::json!([1.0, 0.0, 0.0, 0.0]);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert!(doc["terminal_attitude_error_rad"].as_f64().unwrap() < 1e-6);
        let peak = out
            .csv
            .lines()
            .skip(1)
            .map(|line| {
                let parts: Vec<f64> = line.split(',').map(|item| item.parse().unwrap()).collect();
                (parts[8].abs() + parts[9].abs() + parts[10].abs()) / 3.0
            })
            .fold(0.0_f64, f64::max);
        assert!(peak < 1e-6, "unwinding torque peak {peak}");
    }

    #[test]
    fn law_error_matches_independent_so3_oracle() {
        let q = Quat::new(0.9238795325112867, 0.3826834323650898, 0.0, 0.0);
        let qd = Quat::new(1.0, 0.0, 0.0, 0.0);
        let local = so3_error(q, qd);
        let oracle = so3_attitude_error(q.as_ref(), qd.as_ref());
        for k in 0..3 {
            assert!((local[k] - oracle[k]).abs() < 1e-12);
        }
    }

    #[test]
    fn excess_command_is_inhibited_by_the_oracle_not_the_law() {
        let envelope = MonitorEnvelope {
            torque_limit_nm: [0.2, 0.2, 0.2],
            slew_rate_limit_rad_s: Some(1.0),
            momentum_limit_nms: None,
            max_estimate_age_s: 0.05,
            max_covariance_trace: 1.0,
        };
        let (decision, _) = monitor_command(
            envelope,
            MonitorSample {
                now_s: 0.0,
                estimate_t_s: 0.0,
                q: Quat::new(1.0, 0.0, 0.0, 0.0).as_ref(),
                omega: [0.0; 3],
                h: [0.0; 3],
                covariance_trace: 1e-6,
                frames_match: true,
                command_nm: [2.0, 0.0, 0.0],
            },
            &[],
        )
        .unwrap();
        assert!(!decision.allowed());
    }

    #[test]
    fn stale_measurement_produces_an_inhibited_candidate() {
        let mut value = example_problem();
        value["sensor"]["delay_s"] = serde_json::json!(0.2);
        value["sensor"]["star_tracker_delay_s"] = serde_json::json!(0.2);
        value["max_estimate_age_s"] = serde_json::json!(0.02);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "inhibited-candidate");
        assert!(doc["inhibit_count"].as_u64().unwrap() > 0);
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn numerical_fault_campaign_does_not_write_result() {
        let mut value = example_problem();
        value["duration_s"] = serde_json::json!(1.0);
        value["campaign"] = serde_json::json!({
            "trials": 2,
            "numerical_fault": true,
            "inertia_rel_sigma": 0.02,
            "seed": 3
        });
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert!(doc.get("result").is_none());
        assert_eq!(doc["campaign"]["trials"], 2);
        assert!(
            doc["campaign"]["notes"]
                .as_str()
                .unwrap()
                .contains("Not a report result")
        );
    }

    #[test]
    fn pil_and_hil_packet_abi_do_not_write_result() {
        for execution in ["pil", "hil"] {
            let mut value = example_problem();
            value["execution"] = serde_json::json!(execution);
            let out = run(&value).unwrap();
            let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
            assert!(doc.get("result").is_none());
            assert_eq!(doc["execution"], execution);
            assert_eq!(doc["isolation"], "in-process-packet-abi");
            assert_eq!(doc["hardware_class"], "loopback-emulator");
            assert_eq!(doc["status"], "tracked-candidate");
        }
    }

    #[test]
    fn hard_real_time_and_physical_hardware_are_refused() {
        let mut value = example_problem();
        value["latency_class"] = serde_json::json!("hard-real-time");
        assert!(matches!(run(&value), Err(ControlError::Refused(_))));

        let mut value = example_problem();
        value["hardware"] = serde_json::json!({ "class": "physical" });
        let err = run(&value).unwrap_err();
        assert!(err.to_string().contains("physical actuators are refused"));
    }

    #[test]
    fn unknown_field_is_refused() {
        let mut value = example_problem();
        value["flight_board"] = serde_json::json!(true);
        assert!(run(&value).is_err());
    }

    #[test]
    fn star_tracker_delay_produces_an_inhibited_candidate() {
        let mut value = example_problem();
        value["sensor"]["star_tracker_delay_s"] = serde_json::json!(0.2);
        value["max_estimate_age_s"] = serde_json::json!(0.02);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "inhibited-candidate");
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn gyro_delay_without_star_tracker_delay_is_not_stale_inhibited() {
        let mut value = example_problem();
        value["sensor"]["delay_s"] = serde_json::json!(0.2);
        value["sensor"]["star_tracker_delay_s"] = serde_json::json!(0.0);
        value["max_estimate_age_s"] = serde_json::json!(0.02);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_ne!(doc["status"], "inhibited-candidate");
        assert_eq!(doc["inhibit_count"], 0);
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn logged_torque_is_plant_applied_after_the_first_sample() {
        let mut value = identity_hold();
        value["plant"] = serde_json::json!({
            "magnetic_residual": {
                "dipole_am2": [1.0, 0.0, 0.0],
                "field_t": [0.0, 1.0, 0.0]
            }
        });
        let out = run(&value).unwrap();
        let rows = csv_data(&out.csv);
        assert!(rows[0][8].abs() < 1e-15);
        assert!(rows[0][9].abs() < 1e-15);
        assert!(rows[0][10].abs() < 1e-15);
        assert!(
            (rows[1][10] - 1.0).abs() < 1e-9,
            "plant-applied tz was {}",
            rows[1][10]
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&out.control)
                .unwrap()
                .get("result")
                .is_none()
        );
    }

    #[test]
    fn identity_hold_gravity_gradient_appears_in_logged_plant_torque() {
        let a = std::f64::consts::FRAC_1_SQRT_2;
        let mut value = identity_hold();
        value["inertia"] = serde_json::json!({
            "model": "diagonal",
            "jxx": 1.0,
            "jyy": 2.0,
            "jzz": 3.0
        });
        value["plant"] = serde_json::json!({
            "gravity_gradient": {
                "orbital_rate_rad_s": 0.1,
                "nadir_inertial": [a, a, 0.0]
            }
        });
        let out = run(&value).unwrap();
        let expected = 3.0 * 0.01 * (a * a);
        let tz = csv_data(&out.csv)[1][10];
        assert!((tz - expected).abs() < 1e-9, "plant-applied tz was {tz}");
    }

    #[test]
    fn identity_hold_wheels_do_not_store_magnetic_momentum() {
        let mut value = identity_hold();
        value["actuators"]["wheels"] = serde_json::json!(true);
        value["plant"] = serde_json::json!({
            "magnetic_residual": {
                "dipole_am2": [1.0, 0.0, 0.0],
                "field_t": [0.0, 1.0, 0.0]
            }
        });
        let out = run(&value).unwrap();
        let row = &csv_data(&out.csv)[1];
        assert!((row[10] - 1.0).abs() < 1e-9);
        assert!(
            row[11].abs() < 1e-12 && row[12].abs() < 1e-12 && row[13].abs() < 1e-12,
            "environmental torque leaked into h: {} {} {}",
            row[11],
            row[12],
            row[13]
        );
    }

    #[test]
    fn gyro_arw_changes_the_identity_hold_rates() {
        let quiet = identity_hold();
        let mut noisy = identity_hold();
        noisy["sensor"]["gyro_arw_rad_s_sqrt_s"] = serde_json::json!(0.05);
        let a = csv_data(&run(&quiet).unwrap().csv);
        let b = csv_data(&run(&noisy).unwrap().csv);
        let differ = a.iter().zip(&b).any(|(left, right)| {
            (left[5] - right[5]).abs() > 1e-9
                || (left[6] - right[6]).abs() > 1e-9
                || (left[7] - right[7]).abs() > 1e-9
                || (left[8] - right[8]).abs() > 1e-9
        });
        assert!(differ, "ARW must change logged rates or torque");
    }

    #[test]
    fn hamilton_and_oracle_magnetic_residual_agree_off_identity() {
        let q = Quat::new(
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
        );
        let dipole = [1.0, 0.0, 0.0];
        let field = [0.0, 1.0, 0.0];
        let r = crate::geom::rotation_matrix(q);
        let b_body = crate::geom::apply_matrix(crate::geom::transpose(r), field);
        let hamilton = crate::geom::cross(dipole, b_body);
        let oracle = quatopsy_oracle::magnetic_residual_torque(q.as_ref(), dipole, field).unwrap();
        for k in 0..3 {
            assert!((hamilton[k] - oracle[k]).abs() < 1e-12);
        }
        assert!(
            oracle.iter().all(|item| item.abs() < 1e-12),
            "identity m×B would be [0,0,1]; residual was {oracle:?}"
        );
    }

    #[test]
    fn declared_plant_models_remain_software_and_can_track() {
        let mut value = example_problem();
        value["plant"] = serde_json::json!({
            "wheel_lag_s": 0.05,
            "magnetic_residual": {
                "dipole_am2": [0.02, 0.0, 0.0],
                "field_t": [0.0, 0.0, 3.0e-5]
            },
            "gravity_gradient": {
                "orbital_rate_rad_s": 0.001,
                "nadir_inertial": [0.0, 0.0, 1.0]
            }
        });
        value["sensor"]["gyro_arw_rad_s_sqrt_s"] = serde_json::json!(1.0e-4);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert!(doc.get("result").is_none());
        assert_eq!(doc["status"], "tracked-candidate");
        assert_eq!(doc["hardware_class"], "loopback-emulator");
    }

    #[test]
    fn keep_out_on_the_slew_is_inhibited_or_avoided() {
        let mut value = example_problem();
        value["keep_out_zones"] = serde_json::json!([{
            "body_axis": [1.0, 0.0, 0.0],
            "inertial_axis": [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0],
            "min_angle_rad": 0.4
        }]);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        let inhibited = doc["status"] == "inhibited-candidate";
        let clear = out.csv.lines().skip(1).all(|line| {
            let parts: Vec<f64> = line.split(',').map(|item| item.parse().unwrap()).collect();
            let q = quatopsy_oracle::RefQuat {
                w: parts[1],
                x: parts[2],
                y: parts[3],
                z: parts[4],
            };
            keep_out_violation(
                q,
                KeepOutCone {
                    body_axis: [1.0, 0.0, 0.0],
                    inertial_axis: [
                        std::f64::consts::FRAC_1_SQRT_2,
                        std::f64::consts::FRAC_1_SQRT_2,
                        0.0,
                    ],
                    min_angle_rad: 0.4,
                },
            )
            .unwrap()
                <= 1e-3
        });
        assert!(inhibited || clear);
    }
}
