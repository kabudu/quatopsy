//! Isolated controller and actuator-loopback workers.
//!
//! PIL runs the control cycle in a child process. HIL runs the plant behind a
//! command bus in a child process. Neither worker opens a physical device.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::actuators::{fail_axis, momentum_dump, saturate};
use crate::geom::Quat;
use crate::law::{BodyState, Gains, LawState, Reference, geometric_pd};
use crate::modes::{Mode, arbitrate, hold_metrics};
use crate::{ControlError, KD_SAFE};
use quatopsy_oracle::{
    AppliedTorque, KeepOutCone, MonitorEnvelope, MonitorSample, first_order_lag,
    gravity_gradient_torque, magnetic_residual_torque, monitor_command, rigid_body_step,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CycleConfig {
    pub inertia: [[f64; 3]; 3],
    pub torque_limit_nm: [f64; 3],
    pub slew_rate_limit_rad_s: Option<f64>,
    pub momentum_limit_nms: Option<f64>,
    pub max_estimate_age_s: f64,
    pub max_covariance_trace: f64,
    pub dt: f64,
    pub kp: f64,
    pub kd: f64,
    pub ki: f64,
    pub q_desired: [f64; 4],
    pub dump_gain: f64,
    pub keep_out: Vec<ConeDoc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ConeDoc {
    pub body_axis: [f64; 3],
    pub inertial_axis: [f64; 3],
    pub min_angle_rad: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CycleIn {
    pub t: f64,
    pub q: [f64; 4],
    pub omega: [f64; 3],
    pub h: [f64; 3],
    pub estimate_t_s: f64,
    pub covariance_trace: f64,
    pub frames_ok: bool,
    pub fail_axis: Option<u8>,
    pub disturbance_nm: [f64; 3],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CycleOut {
    pub torque: [f64; 3],
    pub mode: String,
    pub inhibited: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PlantConfig {
    pub inertia: [[f64; 3]; 3],
    pub wheels: bool,
    pub q: [f64; 4],
    pub omega: [f64; 3],
    pub h: [f64; 3],
    /// Command-to-torque first-order lag. Not wheel-speed dynamics.
    #[serde(default)]
    pub wheel_lag_s: f64,
    #[serde(default)]
    pub magnetic_dipole_am2: [f64; 3],
    #[serde(default)]
    pub magnetic_field_t: [f64; 3],
    #[serde(default)]
    pub orbital_rate_rad_s: f64,
    #[serde(default)]
    pub nadir_inertial: [f64; 3],
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PlantIn {
    pub torque: [f64; 3],
    pub dt_sub: f64,
    pub substeps: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct PlantSample {
    pub q: [f64; 4],
    pub omega: [f64; 3],
    pub h: [f64; 3],
    pub torque: [f64; 3],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlantOut {
    pub samples: Vec<PlantSample>,
}

pub(crate) struct CycleEngine {
    config: CycleConfig,
    law: LawState,
    mode: Mode,
}

impl CycleEngine {
    pub(crate) fn new(config: CycleConfig) -> Self {
        Self {
            config,
            law: LawState::new(),
            mode: Mode::Idle,
        }
    }

    pub(crate) fn step(&mut self, input: CycleIn) -> Result<CycleOut, ControlError> {
        let q = Quat::new(input.q[0], input.q[1], input.q[2], input.q[3]);
        let q_des = Quat::new(
            self.config.q_desired[0],
            self.config.q_desired[1],
            self.config.q_desired[2],
            self.config.q_desired[3],
        );
        let keep_out: Vec<KeepOutCone> = self
            .config
            .keep_out
            .iter()
            .map(|zone| KeepOutCone {
                body_axis: zone.body_axis,
                inertial_axis: zone.inertial_axis,
                min_angle_rad: zone.min_angle_rad,
            })
            .collect();
        let mut requested = geometric_pd(
            self.config.inertia,
            BodyState {
                q,
                omega: input.omega,
                h: input.h,
            },
            Reference {
                q: q_des,
                omega: [0.0; 3],
                alpha: [0.0; 3],
            },
            Gains {
                kp: self.config.kp,
                kd: self.config.kd,
                ki: self.config.ki,
            },
            self.config.dt,
            &mut self.law,
        );
        if let Some(limit) = self.config.momentum_limit_nms {
            requested = momentum_dump(requested, input.h, limit, self.config.dump_gain);
        }
        requested = saturate(requested, self.config.torque_limit_nm, &mut self.law);
        requested = fail_axis(requested, input.fail_axis.map(usize::from));
        requested = [
            requested[0] + input.disturbance_nm[0],
            requested[1] + input.disturbance_nm[1],
            requested[2] + input.disturbance_nm[2],
        ];
        let envelope = MonitorEnvelope {
            torque_limit_nm: self.config.torque_limit_nm,
            slew_rate_limit_rad_s: self.config.slew_rate_limit_rad_s,
            momentum_limit_nms: self.config.momentum_limit_nms,
            max_estimate_age_s: self.config.max_estimate_age_s,
            max_covariance_trace: self.config.max_covariance_trace,
        };
        let (decision, reason) = monitor_command(
            envelope,
            MonitorSample {
                now_s: input.t,
                estimate_t_s: input.estimate_t_s,
                q: q.as_ref(),
                omega: input.omega,
                h: input.h,
                covariance_trace: input.covariance_trace,
                frames_match: input.frames_ok,
                command_nm: requested,
            },
            &keep_out,
        )
        .map_err(|err| ControlError::Refused(err.to_string()))?;
        let metrics = hold_metrics(crate::geom::so3_error(q, q_des), input.omega);
        let arbitration = arbitrate(
            self.mode,
            requested,
            input.omega,
            KD_SAFE,
            decision.allowed(),
            metrics.0,
            metrics.1,
        );
        self.mode = arbitration.mode;
        Ok(CycleOut {
            torque: arbitration.torque,
            mode: mode_name(self.mode).to_string(),
            inhibited: !decision.allowed(),
            reason: if decision.allowed() {
                None
            } else {
                Some(reason.to_string())
            },
        })
    }
}

pub(crate) struct PlantEngine {
    config: PlantConfig,
    q: Quat,
    omega: [f64; 3],
    h: [f64; 3],
    tau_act: [f64; 3],
}

impl PlantEngine {
    pub(crate) fn new(config: PlantConfig) -> Result<Self, ControlError> {
        Ok(Self {
            q: Quat::new(config.q[0], config.q[1], config.q[2], config.q[3]).normalized()?,
            omega: config.omega,
            h: config.h,
            tau_act: [0.0; 3],
            config,
        })
    }

    pub(crate) fn step(&mut self, input: PlantIn) -> Result<PlantOut, ControlError> {
        if input.substeps == 0 {
            return Err(ControlError::Refused(
                "loopback worker requires a positive substep count".to_string(),
            ));
        }
        let mut samples = Vec::with_capacity(input.substeps as usize);
        for _ in 0..input.substeps {
            self.tau_act = first_order_lag(
                self.tau_act,
                input.torque,
                input.dt_sub,
                self.config.wheel_lag_s,
            )
            .map_err(|err| ControlError::Refused(err.to_string()))?;
            let magnetic = magnetic_residual_torque(
                self.q.as_ref(),
                self.config.magnetic_dipole_am2,
                self.config.magnetic_field_t,
            )
            .map_err(|err| ControlError::Refused(err.to_string()))?;
            let gravity = gravity_gradient_torque(
                self.q.as_ref(),
                self.config.inertia,
                self.config.orbital_rate_rad_s,
                self.config.nadir_inertial,
            )
            .map_err(|err| ControlError::Refused(err.to_string()))?;
            let body_torque = [
                self.tau_act[0] + magnetic[0] + gravity[0],
                self.tau_act[1] + magnetic[1] + gravity[1],
                self.tau_act[2] + magnetic[2] + gravity[2],
            ];
            let (q_next, w_next, h_next) = rigid_body_step(
                self.config.inertia,
                self.q.as_ref(),
                self.omega,
                self.h,
                AppliedTorque {
                    body: body_torque,
                    motor: self.tau_act,
                },
                input.dt_sub,
                self.config.wheels,
            )
            .map_err(|err| ControlError::Refused(err.to_string()))?;
            self.q = Quat::from_ref(q_next).normalized()?;
            self.omega = w_next;
            self.h = h_next;
            samples.push(PlantSample {
                q: [self.q.w, self.q.x, self.q.y, self.q.z],
                omega: self.omega,
                h: self.h,
                torque: body_torque,
            });
        }
        Ok(PlantOut { samples })
    }
}

pub fn run_cycle_worker() -> Result<(), ControlError> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut lines = stdin.lock().lines();
    let first = lines
        .next()
        .ok_or_else(|| ControlError::Refused("cycle worker missing configuration".to_string()))??;
    let config: CycleConfig = serde_json::from_str(&first)
        .map_err(|err| ControlError::Refused(format!("cycle worker config: {err}")))?;
    let mut engine = CycleEngine::new(config);
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let input: CycleIn = serde_json::from_str(&line)
            .map_err(|err| ControlError::Refused(format!("cycle worker input: {err}")))?;
        let output = engine.step(input)?;
        writeln!(
            stdout,
            "{}",
            serde_json::to_string(&output)
                .map_err(|err| ControlError::Refused(format!("cycle worker output: {err}")))?
        )?;
        stdout.flush()?;
    }
    Ok(())
}

pub fn run_loopback_worker() -> Result<(), ControlError> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut lines = stdin.lock().lines();
    let first = lines.next().ok_or_else(|| {
        ControlError::Refused("loopback worker missing configuration".to_string())
    })??;
    let config: PlantConfig = serde_json::from_str(&first)
        .map_err(|err| ControlError::Refused(format!("loopback worker config: {err}")))?;
    let mut engine = PlantEngine::new(config)?;
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let input: PlantIn = serde_json::from_str(&line)
            .map_err(|err| ControlError::Refused(format!("loopback worker input: {err}")))?;
        let output = engine.step(input)?;
        writeln!(
            stdout,
            "{}",
            serde_json::to_string(&output)
                .map_err(|err| ControlError::Refused(format!("loopback worker output: {err}")))?
        )?;
        stdout.flush()?;
    }
    Ok(())
}

pub(crate) enum CycleBackend {
    Local(CycleEngine),
    Child(ChildLines),
}

impl CycleBackend {
    pub(crate) fn open(config: CycleConfig, bin: Option<&Path>) -> Result<Self, ControlError> {
        if let Some(bin) = bin {
            let mut child = ChildLines::spawn(bin, "control-cycle-worker")?;
            child.send_config(&config)?;
            Ok(Self::Child(child))
        } else {
            Ok(Self::Local(CycleEngine::new(config)))
        }
    }

    pub(crate) fn step(&mut self, input: CycleIn) -> Result<CycleOut, ControlError> {
        match self {
            Self::Local(engine) => engine.step(input),
            Self::Child(child) => child.send_json(&input),
        }
    }
}

pub(crate) enum PlantBackend {
    Local(Box<PlantEngine>),
    Child(ChildLines),
}

impl PlantBackend {
    pub(crate) fn open(config: PlantConfig, bin: Option<&Path>) -> Result<Self, ControlError> {
        if let Some(bin) = bin {
            let mut child = ChildLines::spawn(bin, "control-loopback-worker")?;
            child.send_config(&config)?;
            Ok(Self::Child(child))
        } else {
            Ok(Self::Local(Box::new(PlantEngine::new(config)?)))
        }
    }

    pub(crate) fn step(&mut self, input: PlantIn) -> Result<PlantOut, ControlError> {
        match self {
            Self::Local(engine) => engine.step(input),
            Self::Child(child) => child.send_json(&input),
        }
    }
}

pub(crate) fn static_mode(name: &str) -> &'static str {
    match name {
        "idle" => "idle",
        "track" => "track",
        "hold" => "hold",
        "inhibit" => "inhibit",
        "safe" => "safe",
        _ => "safe",
    }
}

pub(crate) fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Idle => "idle",
        Mode::Track => "track",
        Mode::Hold => "hold",
        Mode::Inhibit => "inhibit",
        Mode::Safe => "safe",
    }
}

pub(crate) struct ChildLines {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl ChildLines {
    pub(crate) fn spawn(bin: &Path, worker: &str) -> Result<Self, ControlError> {
        let mut child = Command::new(bin)
            .arg(worker)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|err| ControlError::Refused(format!("could not spawn {worker}: {err}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ControlError::Refused(format!("{worker} stdin missing")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ControlError::Refused(format!("{worker} stdout missing")))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    fn stdin(&mut self) -> Result<&mut ChildStdin, ControlError> {
        self.stdin
            .as_mut()
            .ok_or_else(|| ControlError::Refused("worker stdin closed".to_string()))
    }

    pub(crate) fn send_json<T: Serialize, R: for<'de> Deserialize<'de>>(
        &mut self,
        value: &T,
    ) -> Result<R, ControlError> {
        writeln!(
            self.stdin()?,
            "{}",
            serde_json::to_string(value)
                .map_err(|err| ControlError::Refused(format!("worker serialize: {err}")))?
        )?;
        self.stdin()?.flush()?;
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line)?;
        if n == 0 {
            return Err(ControlError::Refused("worker closed stdout".to_string()));
        }
        serde_json::from_str(line.trim())
            .map_err(|err| ControlError::Refused(format!("worker deserialize: {err}")))
    }

    pub(crate) fn send_config<T: Serialize>(&mut self, value: &T) -> Result<(), ControlError> {
        writeln!(
            self.stdin()?,
            "{}",
            serde_json::to_string(value)
                .map_err(|err| ControlError::Refused(format!("worker config serialize: {err}")))?
        )?;
        self.stdin()?.flush()?;
        Ok(())
    }
}

impl Drop for ChildLines {
    fn drop(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl From<std::io::Error> for ControlError {
    fn from(err: std::io::Error) -> Self {
        Self::Refused(format!("worker io: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rest_plant(lag: f64) -> PlantConfig {
        PlantConfig {
            inertia: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            wheels: false,
            q: [1.0, 0.0, 0.0, 0.0],
            omega: [0.0; 3],
            h: [0.0; 3],
            wheel_lag_s: lag,
            magnetic_dipole_am2: [0.0; 3],
            magnetic_field_t: [0.0; 3],
            orbital_rate_rad_s: 0.0,
            nadir_inertial: [0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn wheel_lag_matches_exact_discrete_torque() {
        let input = PlantIn {
            torque: [1.0, 0.0, 0.0],
            dt_sub: 0.01,
            substeps: 1,
        };
        let lagged = PlantEngine::new(rest_plant(1.0))
            .unwrap()
            .step(input)
            .unwrap();
        let expected = 1.0 - (-0.01_f64).exp();
        assert!((lagged.samples[0].torque[0] - expected).abs() < 1e-12);
        assert!((lagged.samples[0].omega[0] - expected * 0.01).abs() < 1e-9);
        let instant = PlantEngine::new(rest_plant(0.0))
            .unwrap()
            .step(input)
            .unwrap();
        assert!((instant.samples[0].torque[0] - 1.0).abs() < 1e-15);
        assert!(lagged.samples[0].omega[0].abs() < instant.samples[0].omega[0].abs());
    }

    #[test]
    fn magnetic_residual_at_rest_produces_the_oracle_rate() {
        let mut cfg = rest_plant(0.0);
        cfg.magnetic_dipole_am2 = [1.0, 0.0, 0.0];
        cfg.magnetic_field_t = [0.0, 1.0, 0.0];
        let out = PlantEngine::new(cfg)
            .unwrap()
            .step(PlantIn {
                torque: [0.0; 3],
                dt_sub: 0.01,
                substeps: 1,
            })
            .unwrap();
        assert!((out.samples[0].torque[2] - 1.0).abs() < 1e-12);
        assert!((out.samples[0].omega[2] - 0.01).abs() < 1e-9);
        assert!(out.samples[0].h.iter().all(|item| item.abs() < 1e-15));
    }

    #[test]
    fn wheels_store_motor_momentum_not_environmental_torque() {
        let mut cfg = rest_plant(0.0);
        cfg.wheels = true;
        cfg.magnetic_dipole_am2 = [1.0, 0.0, 0.0];
        cfg.magnetic_field_t = [0.0, 1.0, 0.0];
        let out = PlantEngine::new(cfg)
            .unwrap()
            .step(PlantIn {
                torque: [0.0; 3],
                dt_sub: 0.01,
                substeps: 1,
            })
            .unwrap();
        assert!((out.samples[0].omega[2] - 0.01).abs() < 1e-9);
        assert!(
            out.samples[0].h.iter().all(|item| item.abs() < 1e-15),
            "environmental torque leaked into h: {:?}",
            out.samples[0].h
        );
    }

    #[test]
    fn gravity_gradient_at_rest_matches_the_principal_axis_formula() {
        let a = std::f64::consts::FRAC_1_SQRT_2;
        let mut cfg = rest_plant(0.0);
        cfg.inertia = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        cfg.orbital_rate_rad_s = 0.1;
        cfg.nadir_inertial = [a, a, 0.0];
        let out = PlantEngine::new(cfg)
            .unwrap()
            .step(PlantIn {
                torque: [0.0; 3],
                dt_sub: 0.01,
                substeps: 1,
            })
            .unwrap();
        let expected = 3.0 * 0.01 * (a * a);
        assert!((out.samples[0].torque[2] - expected).abs() < 1e-12);
        assert!(out.samples[0].torque[2].abs() > 1e-6);
    }

    #[test]
    fn magnetic_residual_at_rotated_attitude_is_not_the_identity_cross() {
        let mut cfg = rest_plant(0.0);
        cfg.q = [
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
        ];
        cfg.magnetic_dipole_am2 = [1.0, 0.0, 0.0];
        cfg.magnetic_field_t = [0.0, 1.0, 0.0];
        let out = PlantEngine::new(cfg)
            .unwrap()
            .step(PlantIn {
                torque: [0.0; 3],
                dt_sub: 0.01,
                substeps: 1,
            })
            .unwrap();
        assert!(
            out.samples[0].torque.iter().all(|item| item.abs() < 1e-12),
            "identity m×B would be [0,0,1]; plant torque was {:?}",
            out.samples[0].torque
        );
    }
}
