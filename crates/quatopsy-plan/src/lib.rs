//! Offline torque-limited rest-to-rest candidate generator.
//!
//! Generated samples are checked by `quatopsy-oracle` using rotation matrices
//! and Euler's equation. This crate never assigns a report `result`.

mod actuators;
mod campaign;
mod geom;
mod scvx;

use actuators::{Actuators, body_euler_torque};
use campaign::{CampaignReport, CampaignSpec, run_campaign};
use geom::{Quat, apply_tensor, from_declared, is_spd, rest, torque_excess, unit3};
use libm::{acos, fabs, sqrt};
use quatopsy_oracle::{KeepOutCone, PlanDynamics, PlanResiduals, PlanSample, plan_residuals_ex};
use quatopsy_schema::{
    ComponentOrder, MANIFEST_SCHEMA, OMEGA_ABS_TOLERANCE, PI_TIE_ABS_DOT, RotationSense, TimeUnit,
};
use scvx::{CollocationPath, SOLVER_NODES, SolverProblem, Weights, sample_seed, solve};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use thiserror::Error;

const PROBLEM_SCHEMA: &str = "quatopsy.plan-problem/1";
const PLAN_SCHEMA: &str = "quatopsy.plan/1";
const BANG_ALGORITHM: &str = "eigenaxis-bang-coast-bang";
const SCVX_ALGORITHM: &str = "direct-shooting-lm";
const ALGORITHM_VERSION: &str = "1";
const MAX_SAMPLES: u64 = 100_000;
const MIN_SAMPLES: u64 = 3;
const REST_RATE_ABS: f64 = 1.0e-12;
const MAX_KEEP_OUT: usize = 8;

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("{0}")]
    Refused(String),
    #[error("infeasible: {0}")]
    Infeasible(String),
}

#[derive(Debug, Clone)]
pub struct PlanOutput {
    pub csv: String,
    pub manifest: String,
    pub plan: String,
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
    q_initial: [f64; 4],
    q_final: [f64; 4],
    omega_initial: [f64; 3],
    omega_final: [f64; 3],
    inertia: Inertia,
    torque_limit_nm: TorqueLimit,
    #[serde(default)]
    slew_rate_limit_rad_s: Option<f64>,
    sample_count: u64,
    objective: Objective,
    #[serde(default)]
    actuators: Option<Actuators>,
    #[serde(default)]
    keep_out_zones: Vec<KeepOutZone>,
    #[serde(default)]
    campaign: Option<CampaignSpec>,
    #[serde(default)]
    solver: Option<SolverName>,
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
    fn tensor(self) -> Result<[[f64; 3]; 3], PlanError> {
        let j = match self {
            Self::Spherical { j } => {
                if !j.is_finite() || j <= 0.0 {
                    return Err(PlanError::Refused(
                        "inertia components must be finite and positive".to_string(),
                    ));
                }
                [[j, 0.0, 0.0], [0.0, j, 0.0], [0.0, 0.0, j]]
            }
            Self::Diagonal { jxx, jyy, jzz } => [[jxx, 0.0, 0.0], [0.0, jyy, 0.0], [0.0, 0.0, jzz]],
            Self::Tensor {
                jxx,
                jyy,
                jzz,
                jxy,
                jxz,
                jyz,
            } => [[jxx, jxy, jxz], [jxy, jyy, jyz], [jxz, jyz, jzz]],
        };
        if !is_spd(j) {
            return Err(PlanError::Refused(
                "inertia tensor must be symmetric positive definite".to_string(),
            ));
        }
        Ok(j)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TorqueLimit {
    Scalar(f64),
    Box([f64; 3]),
}

impl TorqueLimit {
    fn box_limit(self) -> Result<[f64; 3], PlanError> {
        let values = match self {
            Self::Scalar(value) => [value, value, value],
            Self::Box(values) => values,
        };
        if values.iter().any(|item| !item.is_finite() || *item <= 0.0) {
            return Err(PlanError::Refused(
                "torque_limit_nm must be finite and positive".to_string(),
            ));
        }
        Ok(values)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum Objective {
    Named(NamedObjective),
    Weighted(WeightedObjective),
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum NamedObjective {
    MinimumTime,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct WeightedObjective {
    kind: WeightedKind,
    #[serde(default)]
    minimum_time: f64,
    #[serde(default)]
    control_effort: f64,
    #[serde(default)]
    energy_proxy: f64,
    #[serde(default)]
    pointing: f64,
    #[serde(default)]
    smoothness: f64,
    #[serde(default)]
    momentum: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum WeightedKind {
    Weighted,
}

impl Objective {
    fn weights(&self) -> Result<Weights, PlanError> {
        match self {
            Self::Named(NamedObjective::MinimumTime) => Ok(Weights::minimum_time()),
            Self::Weighted(item) => {
                let weights = Weights {
                    time: item.minimum_time,
                    control: item.control_effort,
                    energy: item.energy_proxy,
                    pointing: item.pointing,
                    smoothness: item.smoothness,
                    momentum: item.momentum,
                };
                if [
                    weights.time,
                    weights.control,
                    weights.energy,
                    weights.pointing,
                    weights.smoothness,
                    weights.momentum,
                ]
                .iter()
                .any(|item| !item.is_finite() || *item < 0.0)
                {
                    return Err(PlanError::Refused(
                        "objective weights must be finite and non-negative".to_string(),
                    ));
                }
                if weights.time
                    + weights.control
                    + weights.energy
                    + weights.pointing
                    + weights.smoothness
                    + weights.momentum
                    <= 0.0
                {
                    return Err(PlanError::Refused(
                        "weighted objective needs at least one positive weight".to_string(),
                    ));
                }
                Ok(weights)
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum SolverName {
    EigenaxisBangCoastBang,
    #[serde(
        rename = "direct-shooting",
        alias = "multiple-shooting",
        alias = "scvx-collocation"
    )]
    MultipleShooting,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct KeepOutZone {
    body_axis: [f64; 3],
    inertial_axis: [f64; 3],
    min_angle_rad: f64,
}

#[derive(Debug, Serialize)]
struct PlanDocument {
    schema: &'static str,
    algorithm: &'static str,
    algorithm_version: &'static str,
    status: &'static str,
    objective: Objective,
    optimality_class: &'static str,
    notes: &'static str,
    duration_s: f64,
    angle_rad: f64,
    axis: [f64; 3],
    alpha_max_rad_s2: f64,
    omega_peak_rad_s: f64,
    inertia: InertiaExport,
    torque_limit_nm: [f64; 3],
    sample_count: u64,
    solver_iterations: u32,
    problem_sha256: String,
    trajectory_sha256: String,
    manifest_sha256: String,
    residuals: OracleResiduals,
    #[serde(skip_serializing_if = "Option::is_none")]
    campaign: Option<CampaignReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    infeasibility: Option<String>,
    engine_version: String,
}

#[derive(Debug, Serialize)]
struct InertiaExport {
    jxx: f64,
    jyy: f64,
    jzz: f64,
    jxy: f64,
    jxz: f64,
    jyz: f64,
}

#[derive(Debug, Serialize)]
struct OracleResiduals {
    max_kinematics_residual: f64,
    max_euler_residual: f64,
    max_torque_excess: f64,
    boundary_attitude_error: f64,
    boundary_rate_error: f64,
    max_keep_out_violation: f64,
}

#[derive(Clone, Copy)]
struct Profile {
    theta: f64,
    alpha: f64,
    t_acc: f64,
    t_coast: f64,
    duration: f64,
    omega_peak: f64,
    axis: [f64; 3],
    inertia: [[f64; 3]; 3],
    q0: Quat,
}

impl Profile {
    fn angle(&self, t: f64) -> f64 {
        let t_dec = self.t_acc + self.t_coast;
        if t <= self.t_acc {
            0.5 * self.alpha * t * t
        } else if t <= t_dec {
            0.5 * self.alpha * self.t_acc * self.t_acc + self.omega_peak * (t - self.t_acc)
        } else {
            let td = t - t_dec;
            self.theta - 0.5 * self.alpha * (self.t_acc - td) * (self.t_acc - td)
        }
    }

    fn rate(&self, t: f64) -> f64 {
        let t_dec = self.t_acc + self.t_coast;
        if t <= self.t_acc {
            self.alpha * t
        } else if t <= t_dec {
            self.omega_peak
        } else {
            self.omega_peak - self.alpha * (t - t_dec)
        }
    }

    fn alpha_signed(&self, t: f64) -> f64 {
        let t_dec = self.t_acc + self.t_coast;
        if t < self.t_acc {
            self.alpha
        } else if t < t_dec {
            0.0
        } else if t <= self.duration {
            -self.alpha
        } else {
            0.0
        }
    }

    fn omega_vec(&self, t: f64) -> [f64; 3] {
        geom::scale3(self.axis, self.rate(t))
    }

    fn torque_vec(&self, t: f64) -> [f64; 3] {
        body_euler_torque(
            self.inertia,
            self.axis,
            self.alpha_signed(t),
            self.rate(t),
            [0.0; 3],
        )
    }

    fn quaternion(&self, t: f64) -> Quat {
        let phi = self.angle(t);
        let half = 0.5 * phi;
        let s = libm::sin(half);
        let rel = Quat::new(
            libm::cos(half),
            self.axis[0] * s,
            self.axis[1] * s,
            self.axis[2] * s,
        );
        self.q0.mul(rel)
    }
}

pub fn plan(problem_bytes: &[u8], version: &str) -> Result<PlanOutput, PlanError> {
    plan_cancelled(problem_bytes, version, None)
}

pub fn plan_cancelled(
    problem_bytes: &[u8],
    version: &str,
    cancelled: Option<&AtomicBool>,
) -> Result<PlanOutput, PlanError> {
    let problem: ProblemDocument = serde_json::from_slice(problem_bytes)
        .map_err(|err| PlanError::Refused(format!("plan problem parse failed: {err}")))?;
    if problem.schema != PROBLEM_SCHEMA {
        return Err(PlanError::Refused(format!(
            "unsupported plan problem schema {}",
            problem.schema
        )));
    }
    if problem.rotation_sense != RotationSense::Active {
        return Err(PlanError::Refused(
            "planner supports active rotation_sense only".to_string(),
        ));
    }
    if problem.time_unit != TimeUnit::S {
        return Err(PlanError::Refused(
            "planner supports time_unit s only".to_string(),
        ));
    }
    if problem.frame_from.is_empty()
        || problem.frame_to.is_empty()
        || problem.frame_from == problem.frame_to
    {
        return Err(PlanError::Refused(
            "frame_from and frame_to must be distinct and non-empty".to_string(),
        ));
    }
    if !rest(problem.omega_initial, REST_RATE_ABS) || !rest(problem.omega_final, REST_RATE_ABS) {
        return Err(PlanError::Refused(
            "planner supports rest-to-rest boundary rates only".to_string(),
        ));
    }
    if problem.sample_count < MIN_SAMPLES || problem.sample_count > MAX_SAMPLES {
        return Err(PlanError::Refused(
            "planner sample_count is outside the supported bound".to_string(),
        ));
    }
    if problem.keep_out_zones.len() > MAX_KEEP_OUT {
        return Err(PlanError::Refused(
            "planner supports at most 8 keep-out zones".to_string(),
        ));
    }
    let inertia = problem.inertia.tensor()?;
    let torque_limit = problem.torque_limit_nm.box_limit()?;
    let q0 = from_declared(problem.q_initial, problem.component_order)?.normalized()?;
    let qf = from_declared(problem.q_final, problem.component_order)?.normalized()?;
    let weights = problem.objective.weights()?;
    let actuators = problem.actuators.unwrap_or_default().prepare()?;
    let keep_out = prepare_keep_out(&problem.keep_out_zones)?;
    let profile = bang_coast_bang(q0, qf, inertia, torque_limit, problem.slew_rate_limit_rad_s)?;
    let use_scvx = matches!(problem.solver, Some(SolverName::MultipleShooting))
        || !actuators.is_empty()
        || !keep_out.is_empty()
        || weights.uses_tradeoff();
    if matches!(problem.solver, Some(SolverName::EigenaxisBangCoastBang)) && use_scvx {
        return Err(PlanError::Refused(
            "eigenaxis-bang-coast-bang cannot honour actuators, keep-out zones, or weighted trade-offs"
                .to_string(),
        ));
    }
    let (algorithm, rows, solver_iterations, duration, omega_peak) = if use_scvx {
        let (seed_omega, seed_tau) = sample_seed(
            profile.duration,
            |t| profile.omega_vec(t),
            |t| profile.torque_vec(t),
        );
        let path = solve(
            &SolverProblem {
                q0,
                qf,
                inertia,
                torque_limit,
                slew: problem.slew_rate_limit_rad_s,
                weights,
                actuators: &actuators,
                keep_out: &keep_out,
            },
            profile.duration,
            &seed_omega,
            &seed_tau,
            cancelled,
        )?;
        let (times, quats, omegas, torques, momenta) = scvx::densify(
            &SolverProblem {
                q0,
                qf,
                inertia,
                torque_limit,
                slew: problem.slew_rate_limit_rad_s,
                weights,
                actuators: &actuators,
                keep_out: &keep_out,
            },
            &path,
            problem.sample_count as usize,
            profile.alpha,
        )?;
        let rows: Vec<Row> = times
            .into_iter()
            .zip(quats)
            .zip(omegas)
            .zip(torques)
            .zip(momenta)
            .map(|((((t, q), w), tau), h)| Row {
                t,
                q,
                omega: w,
                torque: tau,
                h,
            })
            .collect();
        (
            SCVX_ALGORITHM,
            rows,
            path.iterations,
            path.duration,
            path.omegas
                .iter()
                .map(|item| geom::norm3(*item))
                .fold(0.0_f64, f64::max),
        )
    } else {
        let n = required_sample_count(problem.sample_count as usize, &profile)?;
        let times = sample_times(n, &profile);
        let mut rows = Vec::with_capacity(times.len());
        for t in times {
            let q = profile.quaternion(t).normalized()?;
            let omega = profile.omega_vec(t);
            let torque = profile.torque_vec(t);
            if torque_excess(torque, torque_limit) > 0.0 {
                return Err(PlanError::Infeasible(
                    "eigenaxis torque exceeds the declared box under J omega-dot + omega x J omega"
                        .to_string(),
                ));
            }
            rows.push(Row {
                t,
                q,
                omega,
                torque,
                h: [0.0; 3],
            });
        }
        (
            BANG_ALGORITHM,
            rows,
            0,
            profile.duration,
            profile.omega_peak,
        )
    };
    for row in &rows {
        if torque_excess(row.torque, torque_limit) > 1.0e-9 {
            return Err(PlanError::Infeasible(
                "emitted net body torque exceeds the declared box".to_string(),
            ));
        }
    }
    let csv = render_csv(&rows);
    let oracle_samples = parse_emitted_csv(&csv)?;
    let residuals = plan_residuals_ex(
        &oracle_samples,
        qf.as_ref(),
        PlanDynamics {
            inertia,
            torque_limit_nm: torque_limit,
        },
        &keep_out,
    )
    .map_err(|err| PlanError::Refused(err.to_string()))?;
    if !residuals.within_tolerance() {
        return Err(PlanError::Refused(format!(
            "independent plan oracle rejected the generated trajectory kin={:.4e} euler={:.4e} excess={:.4e} att={:.4e} rate={:.4e} keepout={:.4e}",
            residuals.max_kinematics_residual,
            residuals.max_euler_residual,
            residuals.max_torque_excess,
            residuals.boundary_attitude_error,
            residuals.boundary_rate_error,
            residuals.max_keep_out_violation
        )));
    }
    let campaign = match problem.campaign {
        Some(spec) => {
            let path = rows_to_path(&rows, duration)?;
            Some(run_campaign(
                &spec,
                inertia,
                torque_limit,
                &path,
                qf,
                &keep_out,
                &actuators,
            )?)
        }
        None => None,
    };
    let manifest = serde_json::json!({
        "schema": MANIFEST_SCHEMA,
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": problem.frame_from,
        "frame_to": problem.frame_to,
        "time_unit": "s",
        "columns": {
            "time": "t",
            "quaternion": ["qw", "qx", "qy", "qz"],
            "angular_velocity": ["wx", "wy", "wz"]
        }
    })
    .to_string();
    let notes = if algorithm == SCVX_ALGORITHM {
        "Candidate reference under bounded SO(3) multiple shooting. Not flight approval. Not a report result. Not globally optimal."
    } else {
        "Candidate reference under a torque-limited eigenaxis rigid-body model. Not flight approval. Not a report result."
    };
    let plan = PlanDocument {
        schema: PLAN_SCHEMA,
        algorithm,
        algorithm_version: ALGORITHM_VERSION,
        status: "feasible-candidate",
        objective: problem.objective,
        optimality_class: "not-claimed",
        notes,
        duration_s: duration,
        angle_rad: profile.theta,
        axis: profile.axis,
        alpha_max_rad_s2: profile.alpha,
        omega_peak_rad_s: omega_peak,
        inertia: InertiaExport {
            jxx: inertia[0][0],
            jyy: inertia[1][1],
            jzz: inertia[2][2],
            jxy: inertia[0][1],
            jxz: inertia[0][2],
            jyz: inertia[1][2],
        },
        torque_limit_nm: torque_limit,
        sample_count: rows.len() as u64,
        solver_iterations,
        problem_sha256: digest_hex(problem_bytes),
        trajectory_sha256: digest_hex(csv.as_bytes()),
        manifest_sha256: digest_hex(manifest.as_bytes()),
        residuals: export_residuals(residuals),
        campaign,
        infeasibility: None,
        engine_version: version.to_string(),
    };
    let plan_json = serde_json::to_string(&plan)
        .map_err(|err| PlanError::Refused(format!("plan serialize failed: {err}")))?;
    let parsed: serde_json::Value = serde_json::from_str(&plan_json)
        .map_err(|err| PlanError::Refused(format!("plan serialize roundtrip failed: {err}")))?;
    if parsed.get("result").is_some() {
        return Err(PlanError::Refused(
            "planner output must not contain a result field".to_string(),
        ));
    }
    Ok(PlanOutput {
        csv,
        manifest,
        plan: plan_json,
    })
}

struct Row {
    t: f64,
    q: Quat,
    omega: [f64; 3],
    torque: [f64; 3],
    h: [f64; 3],
}

fn prepare_keep_out(zones: &[KeepOutZone]) -> Result<Vec<KeepOutCone>, PlanError> {
    let mut out = Vec::with_capacity(zones.len());
    for zone in zones {
        if !zone.min_angle_rad.is_finite() || zone.min_angle_rad < 0.0 || zone.min_angle_rad >= 3.2
        {
            return Err(PlanError::Refused(
                "keep-out min_angle_rad must be finite and in [0, 3.2)".to_string(),
            ));
        }
        out.push(KeepOutCone {
            body_axis: unit3(zone.body_axis)?,
            inertial_axis: unit3(zone.inertial_axis)?,
            min_angle_rad: zone.min_angle_rad,
        });
    }
    Ok(out)
}

fn render_csv(rows: &[Row]) -> String {
    let with_h = rows.iter().any(|row| geom::norm3(row.h) > 0.0);
    let mut csv = if with_h {
        String::from("t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz,hx,hy,hz\n")
    } else {
        String::from("t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz\n")
    };
    for row in rows {
        if with_h {
            csv.push_str(&format!(
                "{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17}\n",
                row.t, row.q.w, row.q.x, row.q.y, row.q.z,
                row.omega[0], row.omega[1], row.omega[2],
                row.torque[0], row.torque[1], row.torque[2],
                row.h[0], row.h[1], row.h[2]
            ));
        } else {
            csv.push_str(&format!(
                "{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17}\n",
                row.t,
                row.q.w,
                row.q.x,
                row.q.y,
                row.q.z,
                row.omega[0],
                row.omega[1],
                row.omega[2],
                row.torque[0],
                row.torque[1],
                row.torque[2]
            ));
        }
    }
    csv
}

fn parse_emitted_csv(csv: &str) -> Result<Vec<PlanSample>, PlanError> {
    let mut samples = Vec::new();
    for (index, line) in csv.lines().enumerate() {
        if index == 0 {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 11 && parts.len() != 14 {
            return Err(PlanError::Refused(
                "planner csv roundtrip has the wrong width".to_string(),
            ));
        }
        let nums: Result<Vec<f64>, _> = parts.iter().map(|item| item.parse()).collect();
        let nums = nums
            .map_err(|_| PlanError::Refused("planner csv roundtrip is not numeric".to_string()))?;
        let mut sample = PlanSample::new(
            nums[0],
            quatopsy_oracle::RefQuat {
                w: nums[1],
                x: nums[2],
                y: nums[3],
                z: nums[4],
            },
            [nums[5], nums[6], nums[7]],
            [nums[8], nums[9], nums[10]],
        );
        if nums.len() == 14 {
            sample.h = [nums[11], nums[12], nums[13]];
        }
        samples.push(sample);
    }
    Ok(samples)
}

fn rows_to_path(rows: &[Row], duration: f64) -> Result<CollocationPath, PlanError> {
    if rows.len() < 3 {
        return Err(PlanError::Refused(
            "campaign requires at least three samples".to_string(),
        ));
    }
    let step = (rows.len() - 1) / (SOLVER_NODES - 1).max(1);
    let step = step.max(1);
    let mut times = Vec::new();
    let mut quats = Vec::new();
    let mut omegas = Vec::new();
    let mut torques = Vec::new();
    let mut momenta = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if index % step == 0 || index + 1 == rows.len() {
            times.push(row.t);
            quats.push(row.q);
            omegas.push(row.omega);
            torques.push(row.torque);
            momenta.push(row.h);
        }
    }
    Ok(CollocationPath {
        wheel_momenta: vec![vec![]; times.len()],
        times,
        quats,
        omegas,
        torques,
        momenta,
        duration,
        iterations: 0,
        max_defect: 0.0,
    })
}

fn export_residuals(residuals: PlanResiduals) -> OracleResiduals {
    OracleResiduals {
        max_kinematics_residual: residuals.max_kinematics_residual,
        max_euler_residual: residuals.max_euler_residual,
        max_torque_excess: residuals.max_torque_excess,
        boundary_attitude_error: residuals.boundary_attitude_error,
        boundary_rate_error: residuals.boundary_rate_error,
        max_keep_out_violation: residuals.max_keep_out_violation,
    }
}

fn required_sample_count(requested: usize, profile: &Profile) -> Result<usize, PlanError> {
    let dt_max = (2.0 * OMEGA_ABS_TOLERANCE * 0.75) / profile.alpha;
    if !dt_max.is_finite() || dt_max <= 0.0 {
        return Err(PlanError::Refused(
            "planner could not choose a sample interval".to_string(),
        ));
    }
    let from_rate = (profile.duration / dt_max).ceil() as usize + 1;
    let count = requested.max(from_rate);
    if count as u64 > MAX_SAMPLES {
        return Err(PlanError::Refused(
            "planner would exceed the sample limit to keep rate samples kernel-consistent"
                .to_string(),
        ));
    }
    Ok(count)
}

fn sample_times(count: usize, profile: &Profile) -> Vec<f64> {
    let dt = profile.duration / (count as f64 - 1.0);
    let mut times: Vec<f64> = (0..count).map(|i| i as f64 * dt).collect();
    times.push(0.0);
    times.push(profile.t_acc);
    if profile.t_coast > 0.0 {
        times.push(profile.t_acc + profile.t_coast);
    }
    times.push(profile.duration);
    times.sort_by(|a, b| a.total_cmp(b));
    let mut unique = Vec::with_capacity(times.len());
    for time in times {
        if unique
            .last()
            .is_none_or(|prev: &f64| time - *prev > dt * 1.0e-9)
        {
            unique.push(time);
        }
    }
    unique
}

fn bang_coast_bang(
    q0: Quat,
    qf: Quat,
    inertia: [[f64; 3]; 3],
    torque_limit: [f64; 3],
    slew_limit: Option<f64>,
) -> Result<Profile, PlanError> {
    let mut q_target = qf;
    if q0.dot(q_target) < 0.0 {
        q_target = q_target.negate();
    }
    let rel = q0.conjugate().mul(q_target);
    let vec_norm = sqrt(rel.x * rel.x + rel.y * rel.y + rel.z * rel.z);
    if vec_norm < 1.0e-18 {
        return Err(PlanError::Refused(
            "planner refuses a zero-angle rest-to-rest problem".to_string(),
        ));
    }
    let dotted = rel.w.clamp(-1.0, 1.0);
    if fabs(dotted) <= PI_TIE_ABS_DOT {
        return Err(PlanError::Refused(
            "planner refuses a near-pi geodesic with a non-unique eigenaxis".to_string(),
        ));
    }
    let theta = 2.0 * acos(dotted);
    if !theta.is_finite() || theta <= 0.0 {
        return Err(PlanError::Refused(
            "planner could not form a positive eigenaxis angle".to_string(),
        ));
    }
    let axis = [rel.x / vec_norm, rel.y / vec_norm, rel.z / vec_norm];
    let alpha = alpha_from_box(inertia, axis, torque_limit)?;
    let (t_acc, t_coast, omega_peak) = switch_times(theta, alpha, slew_limit)?;
    let duration = 2.0 * t_acc + t_coast;
    Ok(Profile {
        theta,
        alpha,
        t_acc,
        t_coast,
        duration,
        omega_peak,
        axis,
        inertia,
        q0,
    })
}

fn alpha_from_box(
    inertia: [[f64; 3]; 3],
    axis: [f64; 3],
    torque_limit: [f64; 3],
) -> Result<f64, PlanError> {
    let jn = apply_tensor(inertia, axis);
    let mut alpha = f64::INFINITY;
    for k in 0..3 {
        if fabs(jn[k]) <= 1.0e-15 {
            continue;
        }
        alpha = alpha.min(torque_limit[k] / fabs(jn[k]));
    }
    if !alpha.is_finite() || alpha <= 0.0 {
        return Err(PlanError::Refused(
            "torque box cannot accelerate about the eigenaxis".to_string(),
        ));
    }
    Ok(alpha)
}

fn switch_times(
    theta: f64,
    alpha: f64,
    slew_limit: Option<f64>,
) -> Result<(f64, f64, f64), PlanError> {
    let bang = sqrt(theta / alpha);
    let omega_bang = alpha * bang;
    match slew_limit {
        None => Ok((bang, 0.0, omega_bang)),
        Some(limit) => {
            if !limit.is_finite() || limit <= 0.0 {
                return Err(PlanError::Refused(
                    "slew_rate_limit_rad_s must be finite and positive when declared".to_string(),
                ));
            }
            if omega_bang <= limit {
                Ok((bang, 0.0, omega_bang))
            } else {
                let t_acc = limit / alpha;
                let t_coast = (theta - limit * limit / alpha) / limit;
                if !t_coast.is_finite() || t_coast < 0.0 {
                    return Err(PlanError::Refused(
                        "slew-rate limit produced an invalid coast interval".to_string(),
                    ));
                }
                Ok((t_acc, t_coast, limit))
            }
        }
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn example_problem_bytes() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema": PROBLEM_SCHEMA,
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "BODY",
        "frame_to": "J2000",
        "time_unit": "s",
        "q_initial": [1.0, 0.0, 0.0, 0.0],
        "q_final": [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0, 0.0],
        "omega_initial": [0.0, 0.0, 0.0],
        "omega_final": [0.0, 0.0, 0.0],
        "inertia": {"model": "spherical", "j": 1.0},
        "torque_limit_nm": 0.05,
        "sample_count": 321,
        "objective": "minimum-time"
    }))
    .expect("example problem serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use quatopsy_oracle::plan_residuals;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn plan_document_has_no_result_field() {
        let out = plan(&example_problem_bytes(), "0.1.0").unwrap();
        let value: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert!(value.get("result").is_none());
        assert_eq!(value["schema"], PLAN_SCHEMA);
        assert_eq!(value["status"], "feasible-candidate");
        assert_eq!(value["optimality_class"], "not-claimed");
        assert!(value["residuals"]["max_euler_residual"].as_f64().unwrap() < 5e-3);
        assert!(out.manifest.contains("angular_velocity"));
        assert!(out.csv.starts_with("t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz\n"));
    }

    #[test]
    fn independent_geodesic_matches_closed_form_angle() {
        let out = plan(&example_problem_bytes(), "0.1.0").unwrap();
        let value: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert!((value["angle_rad"].as_f64().unwrap() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(
            value["residuals"]["boundary_attitude_error"]
                .as_f64()
                .unwrap()
                < 1e-8
        );
    }

    #[test]
    fn slew_limit_inserts_a_coast_and_caps_peak_rate() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["slew_rate_limit_rad_s"] = serde_json::json!(0.05);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert!((doc["omega_peak_rad_s"].as_f64().unwrap() - 0.05).abs() < 1e-12);
        assert!(doc["duration_s"].as_f64().unwrap() > 10.0);
        assert!(
            doc["residuals"]["max_kinematics_residual"]
                .as_f64()
                .unwrap()
                <= 1e-3
        );
    }

    #[test]
    fn requested_sample_count_is_raised_for_kernel_omega_consistency() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["sample_count"] = serde_json::json!(8);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert!(doc["sample_count"].as_u64().unwrap() > 8);
        assert!(
            doc["residuals"]["max_kinematics_residual"]
                .as_f64()
                .unwrap()
                <= 1e-3
        );
    }

    #[test]
    fn switch_times_are_present_on_the_emitted_grid() {
        let out = plan(&example_problem_bytes(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        let alpha = doc["alpha_max_rad_s2"].as_f64().unwrap();
        let omega_peak = doc["omega_peak_rad_s"].as_f64().unwrap();
        let duration = doc["duration_s"].as_f64().unwrap();
        let t_acc = omega_peak / alpha;
        let times: Vec<f64> = parse_emitted_csv(&out.csv)
            .unwrap()
            .into_iter()
            .map(|sample| sample.t)
            .collect();
        assert!((times[0] - 0.0).abs() < 1e-12);
        assert!((times[times.len() - 1] - duration).abs() < 1e-12);
        assert!(times.iter().any(|time| (time - t_acc).abs() < 1e-12));
        assert!(
            times
                .iter()
                .any(|time| (time - (duration - t_acc)).abs() < 1e-12)
        );
    }

    #[test]
    fn principal_diagonal_inertia_is_feasible() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["inertia"] = serde_json::json!({"model":"diagonal","jxx":2.0,"jyy":3.0,"jzz":4.0});
        value["torque_limit_nm"] = serde_json::json!([1.0, 1.0, 1.0]);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["status"], "feasible-candidate");
        assert!(doc.get("infeasibility").is_none());
        assert!((doc["alpha_max_rad_s2"].as_f64().unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn mutated_emitted_omega_is_rejected_by_the_oracle() {
        let out = plan(&example_problem_bytes(), "0.1.0").unwrap();
        let mut samples = parse_emitted_csv(&out.csv).unwrap();
        samples[3].omega[0] += 0.5;
        let q_final = samples.last().unwrap().q;
        let residuals = plan_residuals(
            &samples,
            q_final,
            PlanDynamics::diagonal([1.0, 1.0, 1.0], [0.05, 0.05, 0.05]),
        )
        .unwrap();
        assert!(!residuals.within_tolerance());
    }

    #[test]
    fn nonprincipal_axis_with_tight_box_is_infeasible() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["inertia"] = serde_json::json!({"model":"diagonal","jxx":1.0,"jyy":40.0,"jzz":0.2});
        value["q_final"] = serde_json::json!([std::f64::consts::FRAC_1_SQRT_2, 0.5, 0.5, 0.0]);
        value["torque_limit_nm"] = serde_json::json!([0.02, 0.02, 0.02]);
        let err = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap_err();
        assert!(matches!(err, PlanError::Infeasible(_)));
    }

    #[test]
    fn unknown_problem_field_is_refused() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["controller"] = serde_json::json!(true);
        assert!(plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").is_err());
    }

    #[test]
    fn non_rest_boundary_is_refused() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["omega_initial"] = serde_json::json!([0.1, 0.0, 0.0]);
        assert!(plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").is_err());
    }

    #[test]
    fn non_positive_inertia_is_refused() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["inertia"] = serde_json::json!({"model":"spherical","j":0.0});
        assert!(plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").is_err());
    }

    #[test]
    fn full_inertia_tensor_is_feasible_when_spd() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["inertia"] = serde_json::json!({
            "model":"tensor","jxx":2.0,"jyy":3.0,"jzz":4.0,"jxy":0.1,"jxz":0.0,"jyz":0.05
        });
        value["torque_limit_nm"] = serde_json::json!(1.0);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["status"], "feasible-candidate");
        assert!((doc["inertia"]["jxy"].as_f64().unwrap() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn indefinite_inertia_tensor_is_refused() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["inertia"] =
            serde_json::json!({"model":"tensor","jxx":1.0,"jyy":1.0,"jzz":1.0,"jxy":2.0});
        assert!(plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").is_err());
    }

    #[test]
    fn three_wheel_rest_to_rest_is_feasible() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["actuators"] = serde_json::json!({
            "wheels": [
                {"axis":[1.0,0.0,0.0],"max_torque_nm":0.2,"max_momentum_nms":2.0},
                {"axis":[0.0,1.0,0.0],"max_torque_nm":0.2,"max_momentum_nms":2.0},
                {"axis":[0.0,0.0,1.0],"max_torque_nm":0.2,"max_momentum_nms":2.0}
            ]
        });
        value["torque_limit_nm"] = serde_json::json!(0.2);
        value["sample_count"] = serde_json::json!(64);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["algorithm"], "direct-shooting-lm");
        assert!(doc.get("result").is_none());
        assert!(out.csv.contains("hx"));
    }

    #[test]
    fn three_thruster_rest_to_rest_is_feasible() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["actuators"] = serde_json::json!({
            "thrusters": [
                {"torque_axis":[1.0,0.0,0.0],"max_torque_nm":0.2},
                {"torque_axis":[0.0,1.0,0.0],"max_torque_nm":0.2},
                {"torque_axis":[0.0,0.0,1.0],"max_torque_nm":0.2}
            ]
        });
        value["torque_limit_nm"] = serde_json::json!(0.2);
        value["sample_count"] = serde_json::json!(64);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["algorithm"], "direct-shooting-lm");
        assert!(doc.get("result").is_none());
    }

    #[test]
    fn keep_out_on_eigenaxis_is_infeasible_or_avoided() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["q_final"] = serde_json::json!([
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2
        ]);
        value["keep_out_zones"] = serde_json::json!([{
            "body_axis":[1.0,0.0,0.0],
            "inertial_axis":[std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0],
            "min_angle_rad": 0.5
        }]);
        value["sample_count"] = serde_json::json!(64);
        let result = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0");
        match result {
            Ok(out) => {
                let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
                assert_eq!(doc["algorithm"], "direct-shooting-lm");
                assert!(doc["residuals"]["max_keep_out_violation"].as_f64().unwrap() <= 1e-3);
            }
            Err(PlanError::Infeasible(_)) => {}
            Err(other) => panic!("unexpected {other}"),
        }
    }

    #[test]
    fn weighted_control_effort_uses_direct_shooting() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["objective"] = serde_json::json!({
            "kind":"weighted",
            "minimum_time":1.0,
            "control_effort":0.2
        });
        value["sample_count"] = serde_json::json!(64);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["algorithm"], "direct-shooting-lm");
        assert_eq!(doc["optimality_class"], "not-claimed");
    }

    #[test]
    fn pointing_weight_changes_the_direct_shooting_candidate() {
        let mut low: serde_json::Value = serde_json::from_slice(&example_problem_bytes()).unwrap();
        low["objective"] = serde_json::json!({
            "kind":"weighted", "minimum_time":1.0, "pointing":0.01
        });
        low["sample_count"] = serde_json::json!(64);
        let mut high = low.clone();
        high["objective"]["pointing"] = serde_json::json!(100.0);
        let low_out = plan(&serde_json::to_vec(&low).unwrap(), "0.1.0").unwrap();
        let high_out = plan(&serde_json::to_vec(&high).unwrap(), "0.1.0").unwrap();
        assert_ne!(low_out.csv, high_out.csv);
    }

    #[test]
    fn direct_shooting_observes_cancellation() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["objective"] = serde_json::json!({
            "kind":"weighted", "minimum_time":1.0, "control_effort":0.1
        });
        let cancelled = AtomicBool::new(true);
        let err = plan_cancelled(
            &serde_json::to_vec(&value).unwrap(),
            "0.1.0",
            Some(&cancelled),
        )
        .unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn legacy_scvx_collocation_alias_selects_direct_shooting() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["solver"] = serde_json::json!("scvx-collocation");
        value["sample_count"] = serde_json::json!(64);
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert_eq!(doc["algorithm"], "direct-shooting-lm");
    }

    #[test]
    fn campaign_does_not_write_a_result_field() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["campaign"] = serde_json::json!({"trials": 4, "inertia_rel_sigma": 0.02, "seed": 7});
        let out = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
        assert!(doc.get("result").is_none());
        assert_eq!(doc["campaign"]["trials"], 4);
        assert!(
            doc["campaign"]["notes"]
                .as_str()
                .unwrap()
                .contains("Not a report result")
        );
    }

    #[test]
    fn cmg_pyramid_rest_to_rest_is_feasible_or_singular() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&example_problem_bytes()).unwrap();
        value["actuators"] = serde_json::json!({
            "cmgs": {
                "skew_rad": 0.9553,
                "wheel_momentum_nms": 0.5,
                "max_gimbal_rate_rad_s": 2.0
            }
        });
        value["torque_limit_nm"] = serde_json::json!(0.2);
        value["sample_count"] = serde_json::json!(64);
        let result = plan(&serde_json::to_vec(&value).unwrap(), "0.1.0");
        match result {
            Ok(out) => {
                let doc: serde_json::Value = serde_json::from_str(&out.plan).unwrap();
                assert_eq!(doc["algorithm"], "direct-shooting-lm");
            }
            Err(PlanError::Infeasible(_)) => {}
            Err(other) => panic!("unexpected {other}"),
        }
    }
}
