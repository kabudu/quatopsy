//! Declared actuator maps. Axes are body-fixed.

use crate::PlanError;
use crate::geom::{add3, apply_tensor, cross, euler_lhs, invert3, scale3, unit3};
use libm::{cos, fabs, sin};
use serde::Deserialize;

const MAX_WHEELS: usize = 8;
const MAX_THRUSTERS: usize = 12;

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct Actuators {
    #[serde(default)]
    pub wheels: Vec<Wheel>,
    #[serde(default)]
    pub thrusters: Vec<Thruster>,
    #[serde(default)]
    pub cmgs: Option<CmgPyramid>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wheel {
    pub axis: [f64; 3],
    pub max_torque_nm: f64,
    pub max_momentum_nms: f64,
    #[serde(default)]
    pub max_power_w: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Thruster {
    pub torque_axis: [f64; 3],
    pub max_torque_nm: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct CmgPyramid {
    pub skew_rad: f64,
    pub wheel_momentum_nms: f64,
    pub max_gimbal_rate_rad_s: f64,
    #[serde(default = "default_singularity")]
    pub singularity_eps: f64,
}

fn default_singularity() -> f64 {
    1.0e-3
}

#[derive(Debug, Clone)]
pub(crate) struct ActuatorMap {
    pub wheels: Vec<PreparedWheel>,
    pub thrusters: Vec<PreparedThruster>,
    pub cmgs: Option<CmgPyramid>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedWheel {
    pub axis: [f64; 3],
    pub max_torque_nm: f64,
    pub max_momentum_nms: f64,
    pub max_power_w: Option<f64>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedThruster {
    pub axis: [f64; 3],
    pub max_torque_nm: f64,
}

impl Actuators {
    pub(crate) fn prepare(self) -> Result<ActuatorMap, PlanError> {
        if self.wheels.len() > MAX_WHEELS {
            return Err(PlanError::Refused(
                "planner supports at most 8 reaction wheels".to_string(),
            ));
        }
        if self.thrusters.len() > MAX_THRUSTERS {
            return Err(PlanError::Refused(
                "planner supports at most 12 torque thrusters".to_string(),
            ));
        }
        let mut wheels = Vec::new();
        for wheel in self.wheels {
            if !wheel.max_torque_nm.is_finite() || wheel.max_torque_nm <= 0.0 {
                return Err(PlanError::Refused(
                    "wheel max_torque_nm must be finite and positive".to_string(),
                ));
            }
            if !wheel.max_momentum_nms.is_finite() || wheel.max_momentum_nms <= 0.0 {
                return Err(PlanError::Refused(
                    "wheel max_momentum_nms must be finite and positive".to_string(),
                ));
            }
            if let Some(power) = wheel.max_power_w {
                if !power.is_finite() || power <= 0.0 {
                    return Err(PlanError::Refused(
                        "wheel max_power_w must be finite and positive when declared".to_string(),
                    ));
                }
            }
            wheels.push(PreparedWheel {
                axis: unit3(wheel.axis)?,
                max_torque_nm: wheel.max_torque_nm,
                max_momentum_nms: wheel.max_momentum_nms,
                max_power_w: wheel.max_power_w,
            });
        }
        let mut thrusters = Vec::new();
        for thruster in self.thrusters {
            if !thruster.max_torque_nm.is_finite() || thruster.max_torque_nm <= 0.0 {
                return Err(PlanError::Refused(
                    "thruster max_torque_nm must be finite and positive".to_string(),
                ));
            }
            thrusters.push(PreparedThruster {
                axis: unit3(thruster.torque_axis)?,
                max_torque_nm: thruster.max_torque_nm,
            });
        }
        if let Some(cmg) = &self.cmgs {
            if !cmg.skew_rad.is_finite() || cmg.skew_rad <= 0.0 || cmg.skew_rad >= 1.6 {
                return Err(PlanError::Refused(
                    "CMG skew_rad must be finite and in (0, 1.6)".to_string(),
                ));
            }
            if !cmg.wheel_momentum_nms.is_finite() || cmg.wheel_momentum_nms <= 0.0 {
                return Err(PlanError::Refused(
                    "CMG wheel_momentum_nms must be finite and positive".to_string(),
                ));
            }
            if !cmg.max_gimbal_rate_rad_s.is_finite() || cmg.max_gimbal_rate_rad_s <= 0.0 {
                return Err(PlanError::Refused(
                    "CMG max_gimbal_rate_rad_s must be finite and positive".to_string(),
                ));
            }
            if !cmg.singularity_eps.is_finite() || cmg.singularity_eps <= 0.0 {
                return Err(PlanError::Refused(
                    "CMG singularity_eps must be finite and positive".to_string(),
                ));
            }
        }
        Ok(ActuatorMap {
            wheels,
            thrusters,
            cmgs: self.cmgs,
        })
    }
}

impl ActuatorMap {
    pub(crate) fn is_empty(&self) -> bool {
        self.wheels.is_empty() && self.thrusters.is_empty() && self.cmgs.is_none()
    }

    pub(crate) fn control_dim(&self) -> usize {
        let mut n = 0;
        if self.is_empty() {
            return 3;
        }
        n += self.wheels.len();
        n += self.thrusters.len();
        if self.cmgs.is_some() {
            n += 4;
        }
        n
    }

    pub(crate) fn body_torque(
        &self,
        u: &[f64],
        omega: [f64; 3],
        h: [f64; 3],
        delta: [f64; 4],
    ) -> Result<[f64; 3], PlanError> {
        if self.is_empty() {
            if u.len() < 3 {
                return Err(PlanError::Refused(
                    "body-torque control has the wrong width".to_string(),
                ));
            }
            return Ok([u[0], u[1], u[2]]);
        }
        let mut tau = [0.0; 3];
        let mut offset = 0;
        if !self.wheels.is_empty() {
            let mut eta = [0.0; 3];
            for (wheel, &ui) in self
                .wheels
                .iter()
                .zip(&u[offset..offset + self.wheels.len()])
            {
                eta = add3(eta, scale3(wheel.axis, ui));
            }
            tau = add3(tau, scale3(eta, -1.0));
            tau = add3(tau, scale3(cross(omega, h), -1.0));
            offset += self.wheels.len();
        }
        if !self.thrusters.is_empty() {
            for (thruster, &ui) in self
                .thrusters
                .iter()
                .zip(&u[offset..offset + self.thrusters.len()])
            {
                tau = add3(tau, scale3(thruster.axis, ui));
            }
            offset += self.thrusters.len();
        }
        if let Some(cmg) = &self.cmgs {
            let a = pyramid_a(cmg.skew_rad, delta);
            let gd = [u[offset], u[offset + 1], u[offset + 2], u[offset + 3]];
            let mut a_gd = [0.0; 3];
            for i in 0..3 {
                a_gd[i] = a[i][0] * gd[0] + a[i][1] * gd[1] + a[i][2] * gd[2] + a[i][3] * gd[3];
            }
            tau = add3(tau, scale3(a_gd, -cmg.wheel_momentum_nms));
        }
        Ok(tau)
    }

    pub(crate) fn wheel_h_dot(&self, u: &[f64]) -> [f64; 3] {
        let mut hdot = [0.0; 3];
        for (wheel, &ui) in self.wheels.iter().zip(u.iter()) {
            hdot = add3(hdot, scale3(wheel.axis, ui));
        }
        hdot
    }

    pub(crate) fn momentum_excess(&self, h: [f64; 3]) -> f64 {
        let mut excess = 0.0_f64;
        for wheel in &self.wheels {
            let hi = h[0] * wheel.axis[0] + h[1] * wheel.axis[1] + h[2] * wheel.axis[2];
            excess = excess.max(hi.abs() - wheel.max_momentum_nms);
        }
        excess
    }

    pub(crate) fn power_excess(&self, u: &[f64], omega: [f64; 3]) -> f64 {
        let mut excess = 0.0_f64;
        for (i, wheel) in self.wheels.iter().enumerate() {
            let Some(limit) = wheel.max_power_w else {
                continue;
            };
            let speed =
                omega[0] * wheel.axis[0] + omega[1] * wheel.axis[1] + omega[2] * wheel.axis[2];
            excess = excess.max(fabs(u[i] * speed) - limit);
        }
        excess
    }

    pub(crate) fn project_controls(&self, u: &mut [f64], torque_limit: [f64; 3]) {
        if self.is_empty() {
            for k in 0..3 {
                u[k] = u[k].clamp(-torque_limit[k], torque_limit[k]);
            }
            return;
        }
        let mut offset = 0;
        for wheel in &self.wheels {
            u[offset] = u[offset].clamp(-wheel.max_torque_nm, wheel.max_torque_nm);
            offset += 1;
        }
        for thruster in &self.thrusters {
            u[offset] = u[offset].clamp(-thruster.max_torque_nm, thruster.max_torque_nm);
            offset += 1;
        }
        if let Some(cmg) = &self.cmgs {
            for item in u.iter_mut().skip(offset).take(4) {
                *item = item.clamp(-cmg.max_gimbal_rate_rad_s, cmg.max_gimbal_rate_rad_s);
            }
        }
    }

    pub(crate) fn singularity(&self, delta: [f64; 4]) -> f64 {
        let Some(cmg) = &self.cmgs else {
            return f64::INFINITY;
        };
        let a = pyramid_a(cmg.skew_rad, delta);
        let mut aat = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                aat[i][j] =
                    a[i][0] * a[j][0] + a[i][1] * a[j][1] + a[i][2] * a[j][2] + a[i][3] * a[j][3];
            }
        }
        crate::geom::det3(aat)
    }
}

/// Four-CMG pyramid Jacobian `A(δ)` such that `τ = -h A(δ) δ̇`.
pub(crate) fn pyramid_a(skew: f64, delta: [f64; 4]) -> [[f64; 4]; 3] {
    let cg = cos(skew);
    let sg = sin(skew);
    let (s0, c0) = (sin(delta[0]), cos(delta[0]));
    let (s1, c1) = (sin(delta[1]), cos(delta[1]));
    let (s2, c2) = (sin(delta[2]), cos(delta[2]));
    let (s3, c3) = (sin(delta[3]), cos(delta[3]));
    [
        [-cg * s0, -c1, cg * s2, c3],
        [c0, -cg * s1, -c2, cg * s3],
        [sg * c0, sg * c1, sg * c2, sg * c3],
    ]
}

pub(crate) fn allocate_body_torque(
    map: &ActuatorMap,
    tau: [f64; 3],
    n_u: usize,
) -> Result<Vec<f64>, PlanError> {
    let mut u = vec![0.0; n_u];
    if map.is_empty() {
        u[0] = tau[0];
        u[1] = tau[1];
        u[2] = tau[2];
        return Ok(u);
    }
    if !map.wheels.is_empty()
        && map.wheels.len() >= 3
        && map.thrusters.is_empty()
        && map.cmgs.is_none()
    {
        let a = [
            [
                map.wheels[0].axis[0],
                map.wheels[1].axis[0],
                map.wheels[2].axis[0],
            ],
            [
                map.wheels[0].axis[1],
                map.wheels[1].axis[1],
                map.wheels[2].axis[1],
            ],
            [
                map.wheels[0].axis[2],
                map.wheels[1].axis[2],
                map.wheels[2].axis[2],
            ],
        ];
        let inv = invert3(a)?;
        let uw = apply_tensor(inv, scale3(tau, -1.0));
        u[0] = uw[0];
        u[1] = uw[1];
        u[2] = uw[2];
        return Ok(u);
    }
    if map.wheels.is_empty()
        && !map.thrusters.is_empty()
        && map.cmgs.is_none()
        && map.thrusters.len() >= 3
    {
        let a = [
            [
                map.thrusters[0].axis[0],
                map.thrusters[1].axis[0],
                map.thrusters[2].axis[0],
            ],
            [
                map.thrusters[0].axis[1],
                map.thrusters[1].axis[1],
                map.thrusters[2].axis[1],
            ],
            [
                map.thrusters[0].axis[2],
                map.thrusters[1].axis[2],
                map.thrusters[2].axis[2],
            ],
        ];
        let inv = invert3(a)?;
        let ut = apply_tensor(inv, tau);
        u[0] = ut[0];
        u[1] = ut[1];
        u[2] = ut[2];
        return Ok(u);
    }
    if let Some(cmg) = &map.cmgs {
        if map.wheels.is_empty() && map.thrusters.is_empty() {
            let a = pyramid_a(cmg.skew_rad, [0.0; 4]);
            let aat = [
                [
                    a[0][0] * a[0][0] + a[0][1] * a[0][1] + a[0][2] * a[0][2] + a[0][3] * a[0][3],
                    a[0][0] * a[1][0] + a[0][1] * a[1][1] + a[0][2] * a[1][2] + a[0][3] * a[1][3],
                    a[0][0] * a[2][0] + a[0][1] * a[2][1] + a[0][2] * a[2][2] + a[0][3] * a[2][3],
                ],
                [
                    a[1][0] * a[0][0] + a[1][1] * a[0][1] + a[1][2] * a[0][2] + a[1][3] * a[0][3],
                    a[1][0] * a[1][0] + a[1][1] * a[1][1] + a[1][2] * a[1][2] + a[1][3] * a[1][3],
                    a[1][0] * a[2][0] + a[1][1] * a[2][1] + a[1][2] * a[2][2] + a[1][3] * a[2][3],
                ],
                [
                    a[2][0] * a[0][0] + a[2][1] * a[0][1] + a[2][2] * a[0][2] + a[2][3] * a[0][3],
                    a[2][0] * a[1][0] + a[2][1] * a[1][1] + a[2][2] * a[1][2] + a[2][3] * a[1][3],
                    a[2][0] * a[2][0] + a[2][1] * a[2][1] + a[2][2] * a[2][2] + a[2][3] * a[2][3],
                ],
            ];
            let inv = invert3(aat)?;
            let h0 = cmg.wheel_momentum_nms;
            let y = apply_tensor(inv, scale3(tau, -1.0 / h0));
            for (slot, column) in u.iter_mut().take(4).zip(0..4) {
                *slot = a[0][column] * y[0] + a[1][column] * y[1] + a[2][column] * y[2];
            }
            return Ok(u);
        }
    }
    Err(PlanError::Refused(
        "planner needs three independent wheels, three thrusters, or a CMG pyramid to allocate torque"
            .to_string(),
    ))
}

pub(crate) fn body_euler_torque(
    inertia: [[f64; 3]; 3],
    axis: [f64; 3],
    alpha: f64,
    omega: f64,
    h: [f64; 3],
) -> [f64; 3] {
    let w_dot = scale3(axis, alpha);
    let w = scale3(axis, omega);
    euler_lhs(inertia, w_dot, w, h)
}
