//! Independent plan residuals from rotation matrices and Euler's equation.
//!
//! These functions must not share the planner's Hamilton-product generator.

use crate::{Matrix3, RefQuat, geodesic_angle, matmul, rotation_matrix, transpose};

/// Matches `quatopsy-schema::OMEGA_ABS_TOLERANCE`. Duplicated so this crate
/// stays independent of the production schema and kernel.
pub const PLAN_KINEMATICS_TOLERANCE: f64 = 1.0e-3;
pub const PLAN_EULER_TOLERANCE: f64 = 5.0e-3;
pub const PLAN_BOUNDARY_TOLERANCE: f64 = 1.0e-4;
pub const PLAN_TORQUE_EXCESS_TOLERANCE: f64 = 1.0e-12;
pub const PLAN_KEEP_OUT_TOLERANCE: f64 = 1.0e-4;

#[derive(Debug, Clone, Copy)]
pub struct PlanSample {
    pub t: f64,
    pub q: RefQuat,
    pub omega: [f64; 3],
    pub torque: [f64; 3],
    pub h: [f64; 3],
}

impl PlanSample {
    pub fn new(t: f64, q: RefQuat, omega: [f64; 3], torque: [f64; 3]) -> Self {
        Self {
            t,
            q,
            omega,
            torque,
            h: [0.0; 3],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlanDynamics {
    pub inertia: [[f64; 3]; 3],
    pub torque_limit_nm: [f64; 3],
}

impl PlanDynamics {
    pub fn diagonal(diag: [f64; 3], torque_limit_nm: [f64; 3]) -> Self {
        Self {
            inertia: [
                [diag[0], 0.0, 0.0],
                [0.0, diag[1], 0.0],
                [0.0, 0.0, diag[2]],
            ],
            torque_limit_nm,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeepOutCone {
    pub body_axis: [f64; 3],
    pub inertial_axis: [f64; 3],
    pub min_angle_rad: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PlanResiduals {
    pub max_kinematics_residual: f64,
    pub max_euler_residual: f64,
    pub max_torque_excess: f64,
    pub boundary_attitude_error: f64,
    pub boundary_rate_error: f64,
    pub max_keep_out_violation: f64,
}

impl PlanResiduals {
    pub fn within_tolerance(self) -> bool {
        self.max_kinematics_residual <= PLAN_KINEMATICS_TOLERANCE
            && self.max_euler_residual <= PLAN_EULER_TOLERANCE
            && self.max_torque_excess <= PLAN_TORQUE_EXCESS_TOLERANCE
            && self.boundary_attitude_error <= PLAN_BOUNDARY_TOLERANCE
            && self.boundary_rate_error <= PLAN_BOUNDARY_TOLERANCE
            && self.max_keep_out_violation <= PLAN_KEEP_OUT_TOLERANCE
    }
}

/// Residuals from emitted samples only. Kinematics use matrix geodesics, not Hamilton logs.
pub fn plan_residuals(
    samples: &[PlanSample],
    q_final: RefQuat,
    dynamics: PlanDynamics,
) -> Result<PlanResiduals, &'static str> {
    plan_residuals_ex(samples, q_final, dynamics, &[])
}

pub fn plan_residuals_ex(
    samples: &[PlanSample],
    q_final: RefQuat,
    dynamics: PlanDynamics,
    keep_out: &[KeepOutCone],
) -> Result<PlanResiduals, &'static str> {
    if samples.len() < 3 {
        return Err("plan oracle requires at least three samples");
    }
    let mut max_kin = 0.0;
    let mut max_euler = 0.0;
    let mut max_torque = 0.0;
    let mut max_keep = 0.0;
    for sample in samples {
        for zone in keep_out {
            let violation = keep_out_violation(sample.q, *zone)?;
            if violation > max_keep {
                max_keep = violation;
            }
        }
    }
    for window in samples.windows(2) {
        let prev = window[0];
        let next = window[1];
        let dt = next.t - prev.t;
        if dt <= 0.0 || !dt.is_finite() {
            return Err("plan oracle times are not increasing");
        }
        let predicted = body_rate_from_matrices(prev.q, next.q, dt)?;
        let kin = max_abs_diff(predicted, next.omega);
        if kin > max_kin {
            max_kin = kin;
        }
        let w_dot = [
            (next.omega[0] - prev.omega[0]) / dt,
            (next.omega[1] - prev.omega[1]) / dt,
            (next.omega[2] - prev.omega[2]) / dt,
        ];
        let w_mid = [
            0.5 * (prev.omega[0] + next.omega[0]),
            0.5 * (prev.omega[1] + next.omega[1]),
            0.5 * (prev.omega[2] + next.omega[2]),
        ];
        let h_mid = [
            0.5 * (prev.h[0] + next.h[0]),
            0.5 * (prev.h[1] + next.h[1]),
            0.5 * (prev.h[2] + next.h[2]),
        ];
        let inferred = euler_torque(dynamics.inertia, w_dot, w_mid, h_mid);
        let euler = max_abs_diff(inferred, prev.torque).min(max_abs_diff(inferred, next.torque));
        if euler > max_euler {
            max_euler = euler;
        }
        for torque in [prev.torque, next.torque, inferred] {
            for (component, limit) in torque.iter().zip(dynamics.torque_limit_nm) {
                let excess = component.abs() - limit;
                if excess > max_torque {
                    max_torque = excess;
                }
            }
        }
    }
    let first = samples[0];
    let last = samples[samples.len() - 1];
    Ok(PlanResiduals {
        max_kinematics_residual: max_kin,
        max_euler_residual: max_euler,
        max_torque_excess: max_torque.max(0.0),
        boundary_attitude_error: geodesic_angle(last.q, q_final),
        boundary_rate_error: first
            .omega
            .iter()
            .chain(last.omega.iter())
            .fold(0.0_f64, |acc, item| acc.max(item.abs())),
        max_keep_out_violation: max_keep,
    })
}

pub fn keep_out_violation(q: RefQuat, zone: KeepOutCone) -> Result<f64, &'static str> {
    let body = unit3(zone.body_axis)?;
    let inertial = unit3(zone.inertial_axis)?;
    if !zone.min_angle_rad.is_finite() || zone.min_angle_rad < 0.0 {
        return Err("plan oracle keep-out angle is invalid");
    }
    let rotated = matvec(rotation_matrix(q), body);
    let dotted = (rotated[0] * inertial[0] + rotated[1] * inertial[1] + rotated[2] * inertial[2])
        .clamp(-1.0, 1.0);
    let angle = dotted.acos();
    Ok((zone.min_angle_rad - angle).max(0.0))
}

fn body_rate_from_matrices(
    prev: RefQuat,
    next: RefQuat,
    dt: f64,
) -> Result<[f64; 3], &'static str> {
    let rel = matmul(transpose(rotation_matrix(prev)), rotation_matrix(next));
    let axis_sin = vee_skew(rel);
    let n = norm3(axis_sin);
    let theta = geodesic_angle(prev, next);
    if theta < 1.0e-18 || n < 1.0e-18 {
        return Ok([0.0, 0.0, 0.0]);
    }
    let scale = theta / (n * dt);
    if !scale.is_finite() {
        return Err("plan oracle produced a non-finite body rate");
    }
    Ok([
        axis_sin[0] * scale,
        axis_sin[1] * scale,
        axis_sin[2] * scale,
    ])
}

fn euler_torque(inertia: [[f64; 3]; 3], w_dot: [f64; 3], w: [f64; 3], h: [f64; 3]) -> [f64; 3] {
    let jw = apply_tensor(inertia, w);
    let jw_dot = apply_tensor(inertia, w_dot);
    let angular = [jw[0] + h[0], jw[1] + h[1], jw[2] + h[2]];
    let gyro = cross(w, angular);
    [
        jw_dot[0] + gyro[0],
        jw_dot[1] + gyro[1],
        jw_dot[2] + gyro[2],
    ]
}

fn apply_tensor(inertia: [[f64; 3]; 3], w: [f64; 3]) -> [f64; 3] {
    [
        inertia[0][0] * w[0] + inertia[0][1] * w[1] + inertia[0][2] * w[2],
        inertia[1][0] * w[0] + inertia[1][1] * w[1] + inertia[1][2] * w[2],
        inertia[2][0] * w[0] + inertia[2][1] * w[1] + inertia[2][2] * w[2],
    ]
}

fn matvec(m: Matrix3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn vee_skew(rel: Matrix3) -> [f64; 3] {
    [
        rel[2][1] - rel[1][2],
        rel[0][2] - rel[2][0],
        rel[1][0] - rel[0][1],
    ]
}

fn unit3(v: [f64; 3]) -> Result<[f64; 3], &'static str> {
    let n = norm3(v);
    if n < 1.0e-18 || !n.is_finite() {
        return Err("plan oracle keep-out axis is near zero");
    }
    Ok([v[0] / n, v[1] / n, v[2] / n])
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn max_abs_diff(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0])
        .abs()
        .max((a[1] - b[1]).abs())
        .max((a[2] - b[2]).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> RefQuat {
        RefQuat {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn about_x(angle: f64) -> RefQuat {
        RefQuat {
            w: (angle * 0.5).cos(),
            x: (angle * 0.5).sin(),
            y: 0.0,
            z: 0.0,
        }
    }

    fn about_z(angle: f64) -> RefQuat {
        RefQuat {
            w: (angle * 0.5).cos(),
            x: 0.0,
            y: 0.0,
            z: (angle * 0.5).sin(),
        }
    }

    #[test]
    fn matrix_rate_matches_constant_x_spin() {
        let dt = 0.01;
        let w = 0.2;
        let prev = about_x(0.0);
        let next = about_x(w * dt);
        let rate = body_rate_from_matrices(prev, next, dt).unwrap();
        assert!((rate[0] - w).abs() < 1e-9);
        assert!(rate[1].abs() < 1e-9);
        assert!(rate[2].abs() < 1e-9);
    }

    #[test]
    fn euler_includes_gyroscopic_term() {
        let inertia = PlanDynamics::diagonal([2.0, 3.0, 4.0], [1.0; 3]).inertia;
        let w = [0.5, 0.0, 0.5];
        let tau = euler_torque(inertia, [0.0; 3], w, [0.0; 3]);
        assert!(tau[1].abs() > 0.1);
        assert!(tau[0].abs() < 1e-15);
        assert!(tau[2].abs() < 1e-15);
    }

    #[test]
    fn euler_includes_stored_momentum() {
        let inertia = PlanDynamics::diagonal([1.0, 1.0, 1.0], [1.0; 3]).inertia;
        let tau = euler_torque(inertia, [0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
        assert!((tau[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn product_of_inertia_appears_in_euler() {
        let mut inertia = PlanDynamics::diagonal([2.0, 3.0, 4.0], [1.0; 3]).inertia;
        inertia[0][1] = 0.5;
        inertia[1][0] = 0.5;
        let tau = euler_torque(inertia, [1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]);
        assert!((tau[0] - 2.0).abs() < 1e-12);
        assert!((tau[1] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn mutant_zero_omega_on_moving_path_exceeds_kinematics_tolerance() {
        let dt = 0.05;
        let samples = [
            PlanSample::new(0.0, about_x(0.0), [0.0; 3], [0.0; 3]),
            PlanSample::new(dt, about_x(0.2 * dt), [0.0; 3], [0.0; 3]),
            PlanSample::new(2.0 * dt, about_x(0.4 * dt), [0.0; 3], [0.0; 3]),
        ];
        let residuals = plan_residuals(
            &samples,
            about_x(0.4 * dt),
            PlanDynamics::diagonal([1.0; 3], [1.0; 3]),
        )
        .unwrap();
        assert!(residuals.max_kinematics_residual > PLAN_KINEMATICS_TOLERANCE);
        assert!(!residuals.within_tolerance());
    }

    #[test]
    fn mutant_wrong_declared_torque_exceeds_euler_tolerance() {
        let dt = 0.01;
        let alpha = 0.2;
        let samples = [
            PlanSample::new(0.0, about_x(0.0), [0.0; 3], [0.0; 3]),
            PlanSample::new(
                dt,
                about_x(0.5 * alpha * dt * dt),
                [alpha * dt, 0.0, 0.0],
                [0.0; 3],
            ),
            PlanSample::new(
                2.0 * dt,
                about_x(2.0 * alpha * dt * dt),
                [2.0 * alpha * dt, 0.0, 0.0],
                [0.0; 3],
            ),
        ];
        let residuals = plan_residuals(
            &samples,
            samples[2].q,
            PlanDynamics::diagonal([1.0; 3], [1.0; 3]),
        )
        .unwrap();
        assert!(residuals.max_euler_residual > PLAN_EULER_TOLERANCE);
    }

    #[test]
    fn identity_samples_are_within_tolerance() {
        let samples = [
            PlanSample::new(0.0, identity(), [0.0; 3], [0.0; 3]),
            PlanSample::new(1.0, identity(), [0.0; 3], [0.0; 3]),
            PlanSample::new(2.0, identity(), [0.0; 3], [0.0; 3]),
        ];
        let residuals = plan_residuals(
            &samples,
            identity(),
            PlanDynamics::diagonal([1.0; 3], [1.0; 3]),
        )
        .unwrap();
        assert!(residuals.within_tolerance());
    }

    #[test]
    fn bang_coast_switch_uses_nearest_endpoint_torque() {
        let dt = 0.05;
        let alpha = 0.2;
        let t_switch = 0.4;
        let omega_peak = alpha * t_switch;
        let bang = [alpha, 0.0, 0.0];
        let coast = [0.0, 0.0, 0.0];
        let samples = [
            PlanSample::new(
                t_switch - dt,
                about_x(0.5 * alpha * (t_switch - dt) * (t_switch - dt)),
                [alpha * (t_switch - dt), 0.0, 0.0],
                bang,
            ),
            PlanSample::new(
                t_switch,
                about_x(0.5 * alpha * t_switch * t_switch),
                [omega_peak, 0.0, 0.0],
                coast,
            ),
            PlanSample::new(
                t_switch + dt,
                about_x(0.5 * alpha * t_switch * t_switch + omega_peak * dt),
                [omega_peak, 0.0, 0.0],
                coast,
            ),
        ];
        let residuals = plan_residuals(
            &samples,
            samples[2].q,
            PlanDynamics::diagonal([1.0; 3], [1.0; 3]),
        )
        .unwrap();
        assert!(residuals.max_euler_residual <= PLAN_EULER_TOLERANCE);
        let w_dot = (omega_peak - alpha * (t_switch - dt)) / dt;
        let inferred = euler_torque(
            PlanDynamics::diagonal([1.0; 3], [1.0; 3]).inertia,
            [w_dot, 0.0, 0.0],
            [0.5 * (alpha * (t_switch - dt) + omega_peak), 0.0, 0.0],
            [0.0; 3],
        );
        let averaged = [
            0.5 * (bang[0] + coast[0]),
            0.5 * (bang[1] + coast[1]),
            0.5 * (bang[2] + coast[2]),
        ];
        assert!(max_abs_diff(inferred, averaged) > PLAN_EULER_TOLERANCE);
    }

    #[test]
    fn keep_out_flags_body_axis_through_forbidden_cone() {
        let zone = KeepOutCone {
            body_axis: [1.0, 0.0, 0.0],
            inertial_axis: [
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            min_angle_rad: 0.4,
        };
        let q = about_z(std::f64::consts::FRAC_PI_4);
        let violation = keep_out_violation(q, zone).unwrap();
        assert!(violation > 0.3);
        let clear = keep_out_violation(identity(), zone).unwrap();
        assert!(clear < 1e-12);
    }
}
