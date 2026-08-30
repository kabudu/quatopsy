//! Software-in-the-loop, processor-in-the-loop, and loopback hardware-in-the-loop
//! attitude control.
//!
//! Geometric PD commands are inhibited by an independent oracle monitor. This
//! crate never assigns a report `result` and never opens a physical actuator.

mod actuators;
mod allocate;
mod campaign;
mod estimate;
mod geom;
mod isolation;
mod law;
mod modes;

pub use isolation::{run_cycle_worker, run_loopback_worker};

use campaign::{CampaignReport, CampaignSpec, perturb_inertia, unit_noise, validate};
use estimate::Estimator;
use geom::{Quat, add3, from_declared, invert_spd, norm3, rest};
use isolation::{
    ConeDoc, CycleBackend, CycleConfig, CycleIn, PlantBackend, PlantConfig, PlantIn, static_mode,
};
use law::Gains;
use quatopsy_guidance::{Profile, ProfileSample, SunPoint, TwoBody};
use quatopsy_nav::{FilterKind, NavConfig};
use quatopsy_oracle::{KeepOutCone, geodesic_angle};
use quatopsy_schema::{ComponentOrder, MANIFEST_SCHEMA, RotationSense, TimeUnit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

const PROBLEM_SCHEMA: &str = "quatopsy.control-problem/1";
const CONTROL_SCHEMA: &str = "quatopsy.control/1";
const ALGORITHM: &str = "geometric-pd-so3";
const ALGORITHM_VERSION: &str = "1";
const MAX_SAMPLES: u64 = 100_000;
const MAX_GAIN_BREAKS: usize = 1_024;
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
    pub nav: String,
    pub guidance: String,
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
    #[serde(default)]
    navigation: NavigationDoc,
    #[serde(default)]
    guidance: GuidanceDoc,
    #[serde(default)]
    orbit: Option<OrbitDoc>,
    #[serde(default)]
    gain_schedule: Vec<GainBreak>,
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

const NAV_SCHEMA: &str = "quatopsy.nav/1";
const GUIDANCE_SCHEMA: &str = "quatopsy.guidance/1";

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
struct NavigationDoc {
    #[serde(default)]
    filter: FilterDoc,
    #[serde(default)]
    sigma_star_rad: f64,
    #[serde(default)]
    sigma_rrw_rad_s_sqrt_s: f64,
    #[serde(default)]
    chi2_gate: f64,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum FilterDoc {
    #[default]
    Mekf,
    Ukf,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct GuidanceDoc {
    #[serde(default)]
    profile: Vec<ProfileRow>,
    #[serde(default)]
    csv_text: Option<String>,
    #[serde(default)]
    sun_point: Option<SunPointDoc>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct ProfileRow {
    t: f64,
    q: [f64; 4],
    #[serde(default)]
    omega: [f64; 3],
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct SunPointDoc {
    body_axis: [f64; 3],
    min_angle_rad: f64,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct OrbitDoc {
    n: f64,
    #[serde(default)]
    phase: f64,
    #[serde(default = "default_mu")]
    mu: f64,
    #[serde(default = "default_re")]
    earth_radius_m: f64,
}

fn default_mu() -> f64 {
    3.986_004_418e14
}

fn default_re() -> f64 {
    6.371e6
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct GainBreak {
    error_rad: f64,
    kp: f64,
    kd: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct WheelsArrayDoc {
    #[serde(default)]
    axes: Vec<[f64; 3]>,
    #[serde(default = "default_wheel_j")]
    inertia_kgm2: f64,
    #[serde(default)]
    torque_limit_nm: f64,
    #[serde(default)]
    momentum_limit_nms: f64,
}

fn default_wheel_j() -> f64 {
    0.01
}

#[derive(Debug, Serialize)]
struct NavDocument {
    schema: &'static str,
    filter: &'static str,
    last_nis: f64,
    last_nees: Option<f64>,
    rejected: u64,
    covariance_trace: f64,
    bias: [f64; 3],
    updates: Vec<NavAuditRow>,
    notes: &'static str,
}

#[derive(Debug, Serialize)]
struct NavAuditRow {
    t: f64,
    measurement_valid: bool,
    accepted: bool,
    nis: f64,
    nees: Option<f64>,
}

#[derive(Debug, Serialize)]
struct GuidanceDocument {
    schema: &'static str,
    mode: &'static str,
    sample_count: u64,
    notes: &'static str,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Actuators {
    #[serde(default)]
    wheels: bool,
    #[serde(default = "default_dump")]
    momentum_dump_gain: f64,
    #[serde(default)]
    wheels_array: Option<WheelsArrayDoc>,
}

impl Default for Actuators {
    fn default() -> Self {
        Self {
            wheels: false,
            momentum_dump_gain: default_dump(),
            wheels_array: None,
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
    #[serde(default)]
    status: SensorStatus,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum SensorStatus {
    #[default]
    Ok,
    Failed,
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
            status: SensorStatus::Ok,
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
    nav_sha256: String,
    guidance_sha256: String,
    runtime_partition: &'static str,
    /// Retained for `quatopsy.control/1` compatibility. Canonical timing is unavailable.
    nav_phase_ns: u64,
    guidance_phase_ns: u64,
    control_phase_ns: u64,
    plant_phase_ns: u64,
    nav_phase_calls: u64,
    guidance_phase_calls: u64,
    control_phase_calls: u64,
    plant_phase_calls: u64,
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
    nav: NavConfig,
    profile: Profile,
    orbit: Option<TwoBody>,
    gain_schedule: Vec<GainBreak>,
    wheels_array: Option<allocate::WheelArray>,
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
    sensor_fault: bool,
    dt_jitter_s: f64,
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
    nav: String,
    guidance: String,
    nav_phase_calls: u64,
    guidance_phase_calls: u64,
    control_phase_calls: u64,
    plant_phase_calls: u64,
}

pub fn control(problem_bytes: &[u8], version: &str) -> Result<ControlOutput, ControlError> {
    control_with_workers(problem_bytes, version, None)
}

pub fn control_with_workers(
    problem_bytes: &[u8],
    version: &str,
    worker_bin: Option<&Path>,
) -> Result<ControlOutput, ControlError> {
    control_with_workers_cancelled(problem_bytes, version, worker_bin, None)
}

pub fn control_with_workers_cancelled(
    problem_bytes: &[u8],
    version: &str,
    worker_bin: Option<&Path>,
    cancelled: Option<&AtomicBool>,
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
        sensor_fault: false,
        dt_jitter_s: 0.0,
    };
    let run = run_closed_loop(&prepared, &nominal, worker_bin, cancelled)?;
    let campaign = match &problem.campaign {
        Some(spec) => Some(run_campaign(&prepared, spec, cancelled)?),
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
        nav_sha256: digest_hex(run.nav.as_bytes()),
        guidance_sha256: digest_hex(run.guidance.as_bytes()),
        runtime_partition: "sequential-deterministic",
        nav_phase_ns: 0,
        guidance_phase_ns: 0,
        control_phase_ns: 0,
        plant_phase_ns: 0,
        nav_phase_calls: run.nav_phase_calls,
        guidance_phase_calls: run.guidance_phase_calls,
        control_phase_calls: run.control_phase_calls,
        plant_phase_calls: run.plant_phase_calls,
    };
    let control_json = serde_json::to_string(&document)
        .map_err(|err| ControlError::Refused(format!("control serialize failed: {err}")))?;
    for (name, body) in [
        ("control", control_json.as_str()),
        ("nav", run.nav.as_str()),
        ("guidance", run.guidance.as_str()),
    ] {
        let parsed: serde_json::Value = serde_json::from_str(body).map_err(|err| {
            ControlError::Refused(format!("{name} serialize roundtrip failed: {err}"))
        })?;
        if parsed.get("result").is_some() {
            return Err(ControlError::Refused(format!(
                "{name} output must not contain a result field"
            )));
        }
    }
    Ok(ControlOutput {
        csv: run.csv,
        manifest,
        control: control_json,
        nav: run.nav,
        guidance: run.guidance,
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
    if problem.gain_schedule.len() > MAX_GAIN_BREAKS {
        return Err(ControlError::Refused(format!(
            "gain_schedule exceeds the {MAX_GAIN_BREAKS}-entry limit"
        )));
    }
    for item in &problem.gain_schedule {
        if !item.error_rad.is_finite()
            || item.error_rad < 0.0
            || !item.kp.is_finite()
            || item.kp <= 0.0
            || !item.kd.is_finite()
            || item.kd <= 0.0
        {
            return Err(ControlError::Refused(
                "gain_schedule breakpoints must be finite with non-negative error and positive gains"
                    .to_string(),
            ));
        }
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
        nav: prepare_nav(problem)?,
        profile: prepare_profile(problem)?,
        orbit: prepare_orbit(problem)?,
        gain_schedule: problem.gain_schedule.clone(),
        wheels_array: prepare_wheels(problem)?,
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

fn prepare_nav(problem: &ProblemDocument) -> Result<NavConfig, ControlError> {
    let mut config = NavConfig {
        filter: match problem.navigation.filter {
            FilterDoc::Mekf => FilterKind::Mekf,
            FilterDoc::Ukf => FilterKind::Ukf,
        },
        ..NavConfig::default()
    };
    if problem.navigation.sigma_star_rad > 0.0 {
        config.sigma_star_rad = problem.navigation.sigma_star_rad;
    }
    if problem.sensor.gyro_arw_rad_s_sqrt_s > 0.0 {
        config.sigma_arw = problem.sensor.gyro_arw_rad_s_sqrt_s;
    }
    if problem.navigation.sigma_rrw_rad_s_sqrt_s > 0.0 {
        config.sigma_rrw = problem.navigation.sigma_rrw_rad_s_sqrt_s;
    }
    if problem.navigation.chi2_gate > 0.0 {
        config.chi2_gate = problem.navigation.chi2_gate;
    }
    config
        .validated()
        .map_err(|err| ControlError::Refused(err.to_string()))
}

fn prepare_profile(problem: &ProblemDocument) -> Result<Profile, ControlError> {
    let mut profile = if let Some(csv) = &problem.guidance.csv_text {
        Profile::from_plan_csv(csv).map_err(|err| ControlError::Refused(err.to_string()))?
    } else if problem.guidance.profile.len() >= 2 {
        let samples = problem
            .guidance
            .profile
            .iter()
            .map(|row| ProfileSample {
                t: row.t,
                q: row.q,
                omega: row.omega,
                alpha: [0.0; 3],
            })
            .collect();
        Profile::from_samples(samples).map_err(|err| ControlError::Refused(err.to_string()))?
    } else {
        let q = from_declared(problem.q_desired, problem.component_order)?.normalized()?;
        Profile::setpoint([q.w, q.x, q.y, q.z], problem.duration_s)
            .map_err(|err| ControlError::Refused(err.to_string()))?
    };
    let q_des = from_declared(problem.q_desired, problem.component_order)?.normalized()?;
    profile
        .validate_terminal_rest([q_des.w, q_des.x, q_des.y, q_des.z], 1.0e-4, 1.0e-4)
        .map_err(|err| ControlError::Refused(err.to_string()))?;
    if let Some(sun) = problem.guidance.sun_point {
        if !sun.body_axis.iter().all(|value| value.is_finite())
            || norm3(sun.body_axis) < 1.0e-12
            || !sun.min_angle_rad.is_finite()
            || !(0.0..=std::f64::consts::PI).contains(&sun.min_angle_rad)
        {
            return Err(ControlError::Refused(
                "sun_point must declare a finite non-zero body axis and angle in [0, pi]"
                    .to_string(),
            ));
        }
        profile.sun_point = Some(SunPoint {
            body_axis: sun.body_axis,
            min_angle_rad: sun.min_angle_rad,
        });
    }
    profile.keep_out = prepare_keep_out(&problem.keep_out_zones)?;
    Ok(profile)
}

fn prepare_orbit(problem: &ProblemDocument) -> Result<Option<TwoBody>, ControlError> {
    let Some(orbit) = problem.orbit else {
        return Ok(None);
    };
    TwoBody {
        n: orbit.n,
        phase: orbit.phase,
        mu: orbit.mu,
        earth_radius_m: orbit.earth_radius_m,
    }
    .validated()
    .map(Some)
    .map_err(|err| ControlError::Refused(err.to_string()))
}

fn prepare_wheels(problem: &ProblemDocument) -> Result<Option<allocate::WheelArray>, ControlError> {
    let Some(array) = &problem.actuators.wheels_array else {
        return Ok(None);
    };
    if array.axes.is_empty() {
        return Ok(None);
    }
    if !matches!(array.axes.len(), 3 | 4) {
        return Err(ControlError::Refused(
            "wheels_array requires exactly 3 or 4 wheel axes".to_string(),
        ));
    }
    Ok(Some(allocate::WheelArray {
        axes: array.axes.clone(),
        inertia_kgm2: if array.inertia_kgm2 > 0.0 {
            array.inertia_kgm2
        } else {
            default_wheel_j()
        },
        torque_limit_nm: if array.torque_limit_nm > 0.0 {
            array.torque_limit_nm
        } else {
            problem.torque_limit_nm.box_limit()?[0]
        },
        momentum_limit_nms: if array.momentum_limit_nms > 0.0 {
            array.momentum_limit_nms
        } else {
            problem.momentum_limit_nms.unwrap_or(4.0)
        },
    }))
}

fn scheduled_gains(base: Gains, schedule: &[GainBreak], error_rad: f64) -> (f64, f64) {
    let mut kp = base.kp;
    let mut kd = base.kd;
    for item in schedule {
        if error_rad >= item.error_rad {
            kp = item.kp;
            kd = item.kd;
        }
    }
    (kp, kd)
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
        wheels_array: prepared.wheels_array.clone(),
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
    cancelled: Option<&AtomicBool>,
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
    let mut estimator = Estimator::new(prepared.q0, 0.0, prepared.nav)?;
    let mut inhibit_count = 0_u64;
    let mut last_reason: Option<String> = None;
    let mut last_mode = "idle";
    let mut rows = Vec::with_capacity(prepared.steps);
    let mut arw_bias = [0.0; 3];
    let mut last_nis = 0.0;
    let mut last_nees = None;
    let mut rejected = 0_u64;
    let mut nav_updates = Vec::with_capacity(prepared.steps.saturating_sub(1));
    let mut nav_phase_calls = 0_u64;
    let mut guidance_phase_calls = 0_u64;
    let mut control_phase_calls = 0_u64;
    let mut plant_phase_calls = 0_u64;
    let mut last_guide_mode = "track";
    let quat_delay = faults.star_tracker_delay_s;
    for i in 0..prepared.steps.saturating_sub(1) {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(ControlError::Refused(
                "controller was cancelled".to_string(),
            ));
        }
        let mut t = prepared.dt * i as f64;
        if faults.dt_jitter_s > 0.0 {
            t += faults.dt_jitter_s * 0.5 * (unit_noise(faults.seed, 9000 + i as u32) + 1.0);
        }
        history.push((t, q, w));
        let geom_now = if let Some(orbit) = prepared.orbit {
            Some(
                orbit
                    .geometry(t)
                    .map_err(|err| ControlError::Refused(err.to_string()))?,
            )
        } else {
            None
        };
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
        if delayed_q.0 < t {
            meas_q = meas_q
                .mul(geom::exp_so3(geom::scale3(delayed_w.2, t - delayed_q.0)))
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
        estimator.predict(t, delayed_w.0, meas_w)?;
        let eclipse = geom_now.map(|item| item.eclipse).unwrap_or(false);
        let star_valid = !eclipse
            && !faults.sensor_fault
            && prepared.sensor.status != SensorStatus::Failed
            && meas_q.is_finite();
        let (estimate, accepted) = estimator.update_star(t, meas_q, star_valid)?;
        last_nis = estimate.nis;
        let mut update_nees = None;
        if accepted {
            let err = quatopsy_nav::attitude_error_state(
                estimate.q,
                [q.w, q.x, q.y, q.z],
                estimate.bias,
                [0.0; 3],
            );
            update_nees = quatopsy_oracle::error_nees(err, estimator.covariance()).ok();
            last_nees = update_nees;
        }
        nav_updates.push(NavAuditRow {
            t,
            measurement_valid: star_valid,
            accepted,
            nis: estimate.nis,
            nees: update_nees,
        });
        rejected = estimate.rejected;
        nav_phase_calls += 1;
        let reference = prepared
            .profile
            .sample_at(t)
            .map_err(|err| ControlError::Refused(err.to_string()))?;
        last_guide_mode = match prepared.profile.mode(
            reference,
            [
                prepared.q_des.w,
                prepared.q_des.x,
                prepared.q_des.y,
                prepared.q_des.z,
            ],
        ) {
            quatopsy_guidance::GuidanceMode::Slew => "slew",
            quatopsy_guidance::GuidanceMode::Track => "track",
            quatopsy_guidance::GuidanceMode::Hold => "hold",
            quatopsy_guidance::GuidanceMode::Safe => "safe",
        };
        if let Some(geo) = geom_now
            && prepared
                .profile
                .sun_violation(estimate.q, geo.sun)
                .map_err(|err| ControlError::Refused(err.to_string()))?
        {
            last_guide_mode = "safe";
        }
        guidance_phase_calls += 1;
        let att_err = geodesic_angle(
            quatopsy_oracle::RefQuat {
                w: estimate.q[0],
                x: estimate.q[1],
                y: estimate.q[2],
                z: estimate.q[3],
            },
            quatopsy_oracle::RefQuat {
                w: reference.q[0],
                x: reference.q[1],
                y: reference.q[2],
                z: reference.q[3],
            },
        );
        let (kp, kd) = scheduled_gains(prepared.gains, &prepared.gain_schedule, att_err);
        let commanded = cycle.step(CycleIn {
            t,
            q: estimate.q,
            omega: estimate.omega,
            h,
            estimate_t_s: estimate.t_s,
            covariance_trace: estimate.covariance_trace,
            frames_ok: true,
            fail_axis: faults.fail_axis.and_then(|axis| u8::try_from(axis).ok()),
            disturbance_nm: faults.disturbance,
            q_ref: reference.q,
            omega_ref: reference.omega,
            alpha_ref: reference.alpha,
            kp,
            kd,
            field_t: geom_now
                .map(|item| item.field_t)
                .or(Some(prepared.plant.magnetic_residual.field_t)),
        })?;
        last_mode = static_mode(&commanded.mode);
        if commanded.inhibited {
            inhibit_count += 1;
            last_reason = commanded.reason;
        }
        control_phase_calls += 1;
        let applied = commanded.torque;
        if i == 0 {
            rows.push((t, q, w, [0.0; 3], h));
        }
        let next = plant.step(PlantIn {
            torque: applied,
            dt_sub: prepared.dt_sub,
            substeps: prepared.substeps as u32,
            field_t: geom_now.map(|item| item.field_t),
            nadir: geom_now.map(|item| item.nadir),
        })?;
        plant_phase_calls += 1;
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
    let nav_json = serde_json::to_string(&NavDocument {
        schema: NAV_SCHEMA,
        filter: match prepared.nav.filter {
            FilterKind::Mekf => "mekf",
            FilterKind::Ukf => "ukf",
        },
        last_nis,
        last_nees,
        rejected,
        covariance_trace: estimator.estimate().covariance_trace,
        bias: estimator.estimate().bias,
        updates: nav_updates,
        notes: "Software attitude navigator. Not a flight filter. Not a report result.",
    })
    .map_err(|err| ControlError::Refused(format!("nav serialize failed: {err}")))?;
    let guidance_json = serde_json::to_string(&GuidanceDocument {
        schema: GUIDANCE_SCHEMA,
        mode: last_guide_mode,
        sample_count: rows.len() as u64,
        notes: "Time-tagged guidance profile. Not flight approval. Not a report result.",
    })
    .map_err(|err| ControlError::Refused(format!("guidance serialize failed: {err}")))?;
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
        nav: nav_json,
        guidance: guidance_json,
        nav_phase_calls,
        guidance_phase_calls,
        control_phase_calls,
        plant_phase_calls,
    })
}

fn delayed_state(history: &[(f64, Quat, [f64; 3])], now: f64, delay: f64) -> (f64, Quat, [f64; 3]) {
    let target = now - delay;
    let upper = history.partition_point(|item| item.0 <= target);
    history[upper.saturating_sub(1)]
}

fn run_campaign(
    prepared: &Prepared,
    spec: &CampaignSpec,
    cancelled: Option<&AtomicBool>,
) -> Result<CampaignReport, ControlError> {
    validate(spec)?;
    let mut rng = spec.seed | 1;
    let mut inhibit_events = 0;
    let mut terminal_attitude_fail = 0;
    for trial in 0..spec.trials {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(ControlError::Refused(
                "controller was cancelled".to_string(),
            ));
        }
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
            sensor_fault: spec.sensor_fault,
            dt_jitter_s: spec.dt_jitter_s.min(prepared.dt * 0.5),
        };
        let run = run_closed_loop(prepared, &faults, None, cancelled)?;
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
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
        let nav: serde_json::Value = serde_json::from_str(&out.nav).unwrap();
        assert!(
            nav["rejected"].as_u64().unwrap() < 40,
            "honest P must not chi-square-reject the slew: {}",
            nav["rejected"]
        );
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
    fn monitor_still_inhibits_a_stale_estimate() {
        let envelope = MonitorEnvelope {
            torque_limit_nm: [1.0, 1.0, 1.0],
            slew_rate_limit_rad_s: None,
            momentum_limit_nms: None,
            max_estimate_age_s: 0.05,
            max_covariance_trace: 1.0,
        };
        let (decision, _) = monitor_command(
            envelope,
            MonitorSample {
                now_s: 1.0,
                estimate_t_s: 0.0,
                q: Quat::new(1.0, 0.0, 0.0, 0.0).as_ref(),
                omega: [0.0; 3],
                h: [0.0; 3],
                covariance_trace: 1e-6,
                frames_match: true,
                command_nm: [0.0; 3],
            },
            &[],
        )
        .unwrap();
        assert!(!decision.allowed());
    }

    #[test]
    fn delayed_star_and_gyro_do_not_stale_inhibit_with_mekf() {
        let mut value = example_problem();
        value["sensor"]["delay_s"] = serde_json::json!(0.2);
        value["sensor"]["star_tracker_delay_s"] = serde_json::json!(0.2);
        value["max_estimate_age_s"] = serde_json::json!(0.02);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_ne!(doc["status"], "inhibited-candidate");
        assert_eq!(doc["inhibit_count"], 0);
        assert!(doc.get("result").is_none());
        assert!(
            serde_json::from_str::<serde_json::Value>(&out.nav)
                .unwrap()
                .get("result")
                .is_none()
        );
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
    fn star_tracker_delay_does_not_stale_inhibit_when_gyro_propagates() {
        let mut value = example_problem();
        value["sensor"]["star_tracker_delay_s"] = serde_json::json!(0.2);
        value["max_estimate_age_s"] = serde_json::json!(0.02);
        value["duration_s"] = serde_json::json!(1.0);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_ne!(doc["status"], "inhibited-candidate");
        assert_eq!(doc["inhibit_count"], 0);
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

    #[test]
    fn nav_and_guidance_documents_have_no_result() {
        let out = run(&example_problem()).unwrap();
        for body in [&out.control, &out.nav, &out.guidance] {
            let doc: serde_json::Value = serde_json::from_str(body).unwrap();
            assert!(doc.get("result").is_none());
        }
        let control: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(control["runtime_partition"], "sequential-deterministic");
        assert!(control["nav_phase_calls"].as_u64().unwrap() > 0);
        assert!(control["guidance_phase_calls"].as_u64().unwrap() > 0);
        assert!(control["control_phase_calls"].as_u64().unwrap() > 0);
        assert!(control["plant_phase_calls"].as_u64().unwrap() > 0);
        let nav: serde_json::Value = serde_json::from_str(&out.nav).unwrap();
        assert_eq!(nav["filter"], "mekf");
        assert!(
            nav["notes"]
                .as_str()
                .unwrap()
                .contains("Not a flight filter")
        );
    }

    #[test]
    fn canonical_control_outputs_are_reproducible() {
        let first = run(&example_problem()).unwrap();
        let second = run(&example_problem()).unwrap();
        assert_eq!(first.csv, second.csv);
        assert_eq!(first.control, second.control);
        assert_eq!(first.nav, second.nav);
        assert_eq!(first.guidance, second.guidance);
    }

    #[test]
    fn digest_hex_remains_canonical_across_sha2_api_versions() {
        assert_eq!(
            digest_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn control_observes_cancellation_inside_the_cycle_loop() {
        let cancelled = AtomicBool::new(true);
        let problem = serde_json::to_vec(&example_problem()).unwrap();
        let err =
            control_with_workers_cancelled(&problem, "0.1.0", None, Some(&cancelled)).unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn nav_nis_matches_the_independent_oracle() {
        let mut nav = quatopsy_nav::Navigator::new(
            [1.0, 0.0, 0.0, 0.0],
            0.0,
            quatopsy_nav::NavConfig::default(),
        )
        .unwrap();
        nav.predict(
            quatopsy_nav::GyroSample {
                t_s: 0.01,
                omega: [0.0; 3],
            },
            0.01,
        )
        .unwrap();
        let (est, accepted) = nav
            .update_star(
                quatopsy_nav::StarSample {
                    t_s: 0.01,
                    q: [1.0, 0.0, 0.0, 0.0],
                },
                true,
            )
            .unwrap();
        assert!(accepted);
        let oracle = quatopsy_oracle::innovation_nis(est.innovation, est.innovation_s).unwrap();
        assert!((oracle - est.nis).abs() < 1e-12);
    }

    #[test]
    fn ukf_filter_selection_still_tracks() {
        let mut value = example_problem();
        value["navigation"] = serde_json::json!({ "filter": "ukf" });
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "tracked-candidate");
        let nav: serde_json::Value = serde_json::from_str(&out.nav).unwrap();
        assert_eq!(nav["filter"], "ukf");
        assert!(nav.get("result").is_none());
    }

    #[test]
    fn profile_with_nonzero_rate_is_sampled_into_pd() {
        let mut value = example_problem();
        let a = std::f64::consts::FRAC_1_SQRT_2;
        value["guidance"] = serde_json::json!({
            "profile": [
                {"t": 0.0, "q": [1.0, 0.0, 0.0, 0.0], "omega": [0.0, 0.0, 0.0]},
                {"t": 2.0, "q": [0.9238795325112867, 0.3826834323650898, 0.0, 0.0], "omega": [0.4, 0.0, 0.0]},
                {"t": 6.0, "q": [a, a, 0.0, 0.0], "omega": [0.0, 0.0, 0.0]},
                {"t": 8.0, "q": [a, a, 0.0, 0.0], "omega": [0.0, 0.0, 0.0]}
            ]
        });
        value["gain_schedule"] = serde_json::json!([
            {"error_rad": 0.0, "kp": 4.0, "kd": 4.0},
            {"error_rad": 0.5, "kp": 6.0, "kd": 5.0}
        ]);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "tracked-candidate");
        assert!(doc.get("result").is_none());
        let mid = csv_data(&out.csv)
            .into_iter()
            .find(|row| (row[0] - 2.0).abs() < 0.03)
            .unwrap();
        assert!(
            mid[5].abs() > 0.05,
            "profile tracking must command a nonzero rate, wx={}",
            mid[5]
        );
    }

    #[test]
    fn wheel_allocation_tracks_and_has_a_small_oracle_residual() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let mut value = example_problem();
        value["actuators"]["wheels"] = serde_json::json!(true);
        value["actuators"]["wheels_array"] = serde_json::json!({
            "axes": [[s, 0.0, s], [0.0, s, s], [-s, 0.0, s], [0.0, -s, s]],
            "inertia_kgm2": 0.01,
            "torque_limit_nm": 1.0,
            "momentum_limit_nms": 4.0
        });
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "tracked-candidate");
        let residual = quatopsy_oracle::allocation_residual(
            [0.2, 0.1, 0.0],
            &[[s, 0.0, s], [0.0, s, s], [-s, 0.0, s], [0.0, -s, s]],
            &allocate::allocate(
                [0.2, 0.1, 0.0],
                &allocate::WheelArray {
                    axes: vec![[s, 0.0, s], [0.0, s, s], [-s, 0.0, s], [0.0, -s, s]],
                    inertia_kgm2: 0.01,
                    torque_limit_nm: 1.0,
                    momentum_limit_nms: 4.0,
                },
            )
            .unwrap()
            .wheels,
        )
        .unwrap();
        assert!(norm3(residual) < 1e-9);
    }

    #[test]
    fn two_body_field_changes_logged_magnetic_torque() {
        let mut value = identity_hold();
        value["plant"] = serde_json::json!({
            "magnetic_residual": {
                "dipole_am2": [1.0, 0.0, 0.0],
                "field_t": [0.0, 0.0, 0.0]
            }
        });
        value["orbit"] = serde_json::json!({
            "n": 0.05,
            "phase": 0.0
        });
        let out = run(&value).unwrap();
        let rows = csv_data(&out.csv);
        let early = [rows[1][8], rows[1][9], rows[1][10]];
        let late = [
            rows[rows.len() - 1][8],
            rows[rows.len() - 1][9],
            rows[rows.len() - 1][10],
        ];
        let delta =
            (early[0] - late[0]).abs() + (early[1] - late[1]).abs() + (early[2] - late[2]).abs();
        assert!(delta > 1e-8, "orbit B(t) must change plant magnetic torque");
        assert!(
            serde_json::from_str::<serde_json::Value>(&out.control)
                .unwrap()
                .get("result")
                .is_none()
        );
    }

    #[test]
    fn eclipse_without_star_updates_can_inhibit_on_covariance() {
        let mut value = identity_hold();
        value["duration_s"] = serde_json::json!(1.0);
        value["max_covariance_trace"] = serde_json::json!(0.045);
        value["sensor"]["gyro_arw_rad_s_sqrt_s"] = serde_json::json!(0.08);
        value["orbit"] = serde_json::json!({
            "n": 0.001,
            "phase": std::f64::consts::PI
        });
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "inhibited-candidate");
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn failed_sensor_status_is_propagate_only_and_can_inhibit() {
        let mut value = identity_hold();
        value["duration_s"] = serde_json::json!(1.0);
        value["max_covariance_trace"] = serde_json::json!(0.045);
        value["sensor"]["status"] = serde_json::json!("failed");
        value["sensor"]["gyro_arw_rad_s_sqrt_s"] = serde_json::json!(0.08);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_eq!(doc["status"], "inhibited-candidate");
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn rejected_star_outliers_are_not_a_monitor_trip() {
        let mut value = identity_hold();
        value["duration_s"] = serde_json::json!(1.0);
        value["sensor"]["quat_noise"] = serde_json::json!(0.2);
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert_ne!(doc["status"], "inhibited-candidate");
        assert_eq!(doc["inhibit_count"], 0);
        let nav: serde_json::Value = serde_json::from_str(&out.nav).unwrap();
        assert!(nav["rejected"].as_u64().unwrap() > 0);
        assert!(nav.get("result").is_none());
    }

    #[test]
    fn sensor_fault_and_jitter_campaign_does_not_write_result() {
        let mut value = example_problem();
        value["duration_s"] = serde_json::json!(1.0);
        value["campaign"] = serde_json::json!({
            "trials": 2,
            "sensor_fault": true,
            "dt_jitter_s": 0.005,
            "inertia_rel_sigma": 0.02,
            "seed": 5
        });
        let out = run(&value).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.control).unwrap();
        assert!(doc.get("result").is_none());
        assert_eq!(doc["campaign"]["trials"], 2);
    }
}
