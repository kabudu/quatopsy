//! Independent SO(3) error, rigid-body step, and command monitor.
//!
//! These functions must not share the controller's PD law or estimator.

use crate::{
    KeepOutCone, Matrix3, RefQuat, keep_out_violation, matmul, rotation_matrix, transpose,
};

pub const CONTROL_TORQUE_TOLERANCE: f64 = 1.0e-9;
pub const CONTROL_FRESHNESS_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy)]
pub struct MonitorEnvelope {
    pub torque_limit_nm: [f64; 3],
    pub slew_rate_limit_rad_s: Option<f64>,
    pub momentum_limit_nms: Option<f64>,
    pub max_estimate_age_s: f64,
    pub max_covariance_trace: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorSample {
    pub now_s: f64,
    pub estimate_t_s: f64,
    pub q: RefQuat,
    pub omega: [f64; 3],
    pub h: [f64; 3],
    pub covariance_trace: f64,
    pub frames_match: bool,
    pub command_nm: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorDecision {
    Allow,
    Inhibit,
}

impl MonitorDecision {
    pub fn allowed(self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// Geometric attitude error `e_R = 1/2 vee(R_d^T R - R^T R_d)`.
///
/// The map is on `SO(3)`. Antipodal quaternions produce the same error.
pub fn so3_attitude_error(q: RefQuat, q_desired: RefQuat) -> [f64; 3] {
    let r = rotation_matrix(q);
    let rd = rotation_matrix(q_desired);
    let rel = matmul(transpose(rd), r);
    vee_skew_minus_transpose(rel)
}

/// Independent Euler step for software-in-the-loop truth.
pub fn rigid_body_step(
    inertia: [[f64; 3]; 3],
    q: RefQuat,
    omega: [f64; 3],
    h: [f64; 3],
    torque: [f64; 3],
    dt: f64,
    wheels: bool,
) -> Result<(RefQuat, [f64; 3], [f64; 3]), &'static str> {
    if dt <= 0.0 || !dt.is_finite() {
        return Err("control oracle step dt is invalid");
    }
    if !finite3(omega) || !finite3(h) || !finite3(torque) {
        return Err("control oracle step state is not finite");
    }
    let jinv = invert_spd(inertia)?;
    let jw = apply_tensor(inertia, omega);
    let gyro = cross(omega, [jw[0] + h[0], jw[1] + h[1], jw[2] + h[2]]);
    let w_dot = apply_tensor(
        jinv,
        [
            torque[0] - gyro[0],
            torque[1] - gyro[1],
            torque[2] - gyro[2],
        ],
    );
    let w_next = [
        omega[0] + w_dot[0] * dt,
        omega[1] + w_dot[1] * dt,
        omega[2] + w_dot[2] * dt,
    ];
    let w_mid = [
        0.5 * (omega[0] + w_next[0]),
        0.5 * (omega[1] + w_next[1]),
        0.5 * (omega[2] + w_next[2]),
    ];
    let q_next = quat_mul(q, exp_so3(scale3(w_mid, dt)));
    let q_next = quat_normalize(q_next)?;
    let h_next = if wheels {
        [
            h[0] - torque[0] * dt,
            h[1] - torque[1] * dt,
            h[2] - torque[2] * dt,
        ]
    } else {
        h
    };
    Ok((q_next, w_next, h_next))
}

/// Discrete first-order lag `y ← e^{-dt/τ} y + (1 - e^{-dt/τ}) u`.
///
/// `τ = 0` applies the command immediately.
pub fn first_order_lag(
    state: [f64; 3],
    command: [f64; 3],
    dt: f64,
    tau: f64,
) -> Result<[f64; 3], &'static str> {
    if dt <= 0.0 || !dt.is_finite() || !tau.is_finite() || tau < 0.0 {
        return Err("control oracle lag parameters are invalid");
    }
    if !finite3(state) || !finite3(command) {
        return Err("control oracle lag state is not finite");
    }
    if tau == 0.0 {
        return Ok(command);
    }
    let alpha = (-dt / tau).exp();
    Ok([
        alpha * state[0] + (1.0 - alpha) * command[0],
        alpha * state[1] + (1.0 - alpha) * command[1],
        alpha * state[2] + (1.0 - alpha) * command[2],
    ])
}

/// Residual dipole torque `m × R^T B` in the body frame.
pub fn magnetic_residual_torque(
    q: RefQuat,
    dipole_am2: [f64; 3],
    field_t: [f64; 3],
) -> Result<[f64; 3], &'static str> {
    if !finite3(dipole_am2) || !finite3(field_t) {
        return Err("control oracle magnetic residual is not finite");
    }
    let r = rotation_matrix(q);
    let b_body = apply_tensor(transpose(r), field_t);
    Ok(cross(dipole_am2, b_body))
}

/// Gravity-gradient torque `3 n^2 û × J û` with nadir expressed in the body frame.
pub fn gravity_gradient_torque(
    q: RefQuat,
    inertia: [[f64; 3]; 3],
    orbital_rate_rad_s: f64,
    nadir_inertial: [f64; 3],
) -> Result<[f64; 3], &'static str> {
    if !orbital_rate_rad_s.is_finite() || orbital_rate_rad_s < 0.0 {
        return Err("control oracle orbital rate is invalid");
    }
    if !finite3(nadir_inertial) {
        return Err("control oracle nadir is not finite");
    }
    if orbital_rate_rad_s == 0.0 {
        return Ok([0.0; 3]);
    }
    let n = norm3(nadir_inertial);
    if n < 1.0e-12 {
        return Err("control oracle nadir is near zero");
    }
    let r = rotation_matrix(q);
    let u = apply_tensor(transpose(r), scale3(nadir_inertial, 1.0 / n));
    let ju = apply_tensor(inertia, u);
    Ok(scale3(
        cross(u, ju),
        3.0 * orbital_rate_rad_s * orbital_rate_rad_s,
    ))
}

/// Independent envelope and freshness checks. The PD law cannot override this.
pub fn monitor_command(
    envelope: MonitorEnvelope,
    sample: MonitorSample,
    keep_out: &[KeepOutCone],
) -> Result<(MonitorDecision, &'static str), &'static str> {
    if !envelope.max_estimate_age_s.is_finite() || envelope.max_estimate_age_s < 0.0 {
        return Err("control oracle envelope age is invalid");
    }
    if !envelope.max_covariance_trace.is_finite() || envelope.max_covariance_trace <= 0.0 {
        return Err("control oracle envelope covariance is invalid");
    }
    if !finite3(envelope.torque_limit_nm)
        || envelope.torque_limit_nm.iter().any(|item| *item <= 0.0)
    {
        return Err("control oracle torque envelope is invalid");
    }
    if !sample.q.w.is_finite()
        || !sample.q.x.is_finite()
        || !sample.q.y.is_finite()
        || !sample.q.z.is_finite()
        || !finite3(sample.omega)
        || !finite3(sample.h)
        || !finite3(sample.command_nm)
        || !sample.now_s.is_finite()
        || !sample.estimate_t_s.is_finite()
        || !sample.covariance_trace.is_finite()
    {
        return Ok((MonitorDecision::Inhibit, "non-finite estimate or command"));
    }
    if !sample.frames_match {
        return Ok((MonitorDecision::Inhibit, "estimate frames do not match"));
    }
    let age = sample.now_s - sample.estimate_t_s;
    if age < -CONTROL_FRESHNESS_TOLERANCE || age > envelope.max_estimate_age_s {
        return Ok((
            MonitorDecision::Inhibit,
            "estimate is stale or from the future",
        ));
    }
    if sample.covariance_trace > envelope.max_covariance_trace {
        return Ok((
            MonitorDecision::Inhibit,
            "estimate covariance exceeds envelope",
        ));
    }
    for (command, limit) in sample
        .command_nm
        .iter()
        .zip(envelope.torque_limit_nm.iter())
    {
        if command.abs() > *limit + CONTROL_TORQUE_TOLERANCE {
            return Ok((MonitorDecision::Inhibit, "command exceeds torque envelope"));
        }
    }
    if let Some(slew) = envelope.slew_rate_limit_rad_s {
        if !slew.is_finite() || slew <= 0.0 {
            return Err("control oracle slew envelope is invalid");
        }
        if norm3(sample.omega) > slew + 1.0e-9 {
            return Ok((MonitorDecision::Inhibit, "body rate exceeds slew envelope"));
        }
    }
    if let Some(limit) = envelope.momentum_limit_nms {
        if !limit.is_finite() || limit <= 0.0 {
            return Err("control oracle momentum envelope is invalid");
        }
        if norm3(sample.h) > limit + 1.0e-9 {
            return Ok((MonitorDecision::Inhibit, "stored momentum exceeds envelope"));
        }
    }
    for zone in keep_out {
        if keep_out_violation(sample.q, *zone)? > 1.0e-3 {
            return Ok((
                MonitorDecision::Inhibit,
                "estimate violates a keep-out cone",
            ));
        }
    }
    Ok((MonitorDecision::Allow, "allow"))
}

fn vee_skew_minus_transpose(rel: Matrix3) -> [f64; 3] {
    [
        0.5 * (rel[2][1] - rel[1][2]),
        0.5 * (rel[0][2] - rel[2][0]),
        0.5 * (rel[1][0] - rel[0][1]),
    ]
}

fn invert_spd(j: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], &'static str> {
    if (j[0][1] - j[1][0]).abs() > 1.0e-12
        || (j[0][2] - j[2][0]).abs() > 1.0e-12
        || (j[1][2] - j[2][1]).abs() > 1.0e-12
    {
        return Err("control oracle inertia is not symmetric");
    }
    let det = det3(j);
    let m00 = j[0][0];
    let m11 = j[0][0] * j[1][1] - j[0][1] * j[1][0];
    if m00 <= 0.0 || m11 <= 0.0 || det <= 0.0 || !det.is_finite() {
        return Err("control oracle inertia is not positive definite");
    }
    let inv_det = 1.0 / det;
    Ok([
        [
            (j[1][1] * j[2][2] - j[1][2] * j[2][1]) * inv_det,
            (j[0][2] * j[2][1] - j[0][1] * j[2][2]) * inv_det,
            (j[0][1] * j[1][2] - j[0][2] * j[1][1]) * inv_det,
        ],
        [
            (j[1][2] * j[2][0] - j[1][0] * j[2][2]) * inv_det,
            (j[0][0] * j[2][2] - j[0][2] * j[2][0]) * inv_det,
            (j[0][2] * j[1][0] - j[0][0] * j[1][2]) * inv_det,
        ],
        [
            (j[1][0] * j[2][1] - j[1][1] * j[2][0]) * inv_det,
            (j[0][1] * j[2][0] - j[0][0] * j[2][1]) * inv_det,
            (j[0][0] * j[1][1] - j[0][1] * j[1][0]) * inv_det,
        ],
    ])
}

fn det3(j: [[f64; 3]; 3]) -> f64 {
    j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
        - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
        + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0])
}

fn apply_tensor(j: [[f64; 3]; 3], w: [f64; 3]) -> [f64; 3] {
    [
        j[0][0] * w[0] + j[0][1] * w[1] + j[0][2] * w[2],
        j[1][0] * w[0] + j[1][1] * w[1] + j[1][2] * w[2],
        j[2][0] * w[0] + j[2][1] * w[1] + j[2][2] * w[2],
    ]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn finite3(v: [f64; 3]) -> bool {
    v.iter().all(|item| item.is_finite())
}

fn exp_so3(phi: [f64; 3]) -> RefQuat {
    let n = norm3(phi);
    if n < 1.0e-16 {
        return RefQuat {
            w: 1.0,
            x: 0.5 * phi[0],
            y: 0.5 * phi[1],
            z: 0.5 * phi[2],
        };
    }
    let half = 0.5 * n;
    let s = half.sin() / n;
    RefQuat {
        w: half.cos(),
        x: phi[0] * s,
        y: phi[1] * s,
        z: phi[2] * s,
    }
}

fn quat_mul(a: RefQuat, b: RefQuat) -> RefQuat {
    RefQuat {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

fn quat_normalize(q: RefQuat) -> Result<RefQuat, &'static str> {
    let n = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
    if n < 1.0e-18 || !n.is_finite() {
        return Err("control oracle quaternion is near zero");
    }
    Ok(RefQuat {
        w: q.w / n,
        x: q.x / n,
        y: q.y / n,
        z: q.z / n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geodesic_angle;

    fn identity() -> RefQuat {
        RefQuat {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn antipode() -> RefQuat {
        RefQuat {
            w: -1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn envelope() -> MonitorEnvelope {
        MonitorEnvelope {
            torque_limit_nm: [0.2, 0.2, 0.2],
            slew_rate_limit_rad_s: Some(1.0),
            momentum_limit_nms: Some(2.0),
            max_estimate_age_s: 0.05,
            max_covariance_trace: 1.0,
        }
    }

    fn sample() -> MonitorSample {
        MonitorSample {
            now_s: 1.0,
            estimate_t_s: 0.99,
            q: identity(),
            omega: [0.0; 3],
            h: [0.0; 3],
            covariance_trace: 1.0e-6,
            frames_match: true,
            command_nm: [0.01, 0.0, 0.0],
        }
    }

    #[test]
    fn antipodal_quaternions_share_so3_error() {
        let target = RefQuat {
            w: std::f64::consts::FRAC_1_SQRT_2,
            x: std::f64::consts::FRAC_1_SQRT_2,
            y: 0.0,
            z: 0.0,
        };
        let a = so3_attitude_error(identity(), target);
        let b = so3_attitude_error(antipode(), target);
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-12);
        }
        assert!(geodesic_angle(identity(), antipode()).abs() < 1e-12);
        let self_err = so3_attitude_error(identity(), antipode());
        assert!(norm3(self_err) < 1e-12);
    }

    #[test]
    fn monitor_allows_a_fresh_in_envelope_command() {
        let (decision, _) = monitor_command(envelope(), sample(), &[]).unwrap();
        assert_eq!(decision, MonitorDecision::Allow);
    }

    #[test]
    fn monitor_inhibits_excess_torque() {
        let mut item = sample();
        item.command_nm = [1.0, 0.0, 0.0];
        let (decision, reason) = monitor_command(envelope(), item, &[]).unwrap();
        assert_eq!(decision, MonitorDecision::Inhibit);
        assert!(reason.contains("torque"));
    }

    #[test]
    fn monitor_inhibits_stale_and_nan_estimates() {
        let mut stale = sample();
        stale.estimate_t_s = 0.0;
        let (decision, _) = monitor_command(envelope(), stale, &[]).unwrap();
        assert_eq!(decision, MonitorDecision::Inhibit);
        let mut nan = sample();
        nan.command_nm[0] = f64::NAN;
        let (decision, reason) = monitor_command(envelope(), nan, &[]).unwrap();
        assert_eq!(decision, MonitorDecision::Inhibit);
        assert!(reason.contains("non-finite"));
    }

    #[test]
    fn rigid_body_step_preserves_rest_under_zero_torque() {
        let inertia = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let (q, w, h) = rigid_body_step(
            inertia,
            identity(),
            [0.0; 3],
            [0.0; 3],
            [0.0; 3],
            0.01,
            false,
        )
        .unwrap();
        assert!(geodesic_angle(q, identity()) < 1e-15);
        assert!(norm3(w) < 1e-15);
        assert!(norm3(h) < 1e-15);
    }

    #[test]
    fn first_order_lag_is_identity_at_zero_tau() {
        let cmd = [1.0, -2.0, 0.5];
        assert_eq!(first_order_lag([0.0; 3], cmd, 0.01, 0.0).unwrap(), cmd);
    }

    #[test]
    fn magnetic_residual_matches_body_cross_product_at_identity() {
        let tau = magnetic_residual_torque(identity(), [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
        assert!((tau[0]).abs() < 1e-15);
        assert!((tau[1]).abs() < 1e-15);
        assert!((tau[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn gravity_gradient_matches_principal_axis_formula() {
        let inertia = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let a = std::f64::consts::FRAC_1_SQRT_2;
        let tau = gravity_gradient_torque(identity(), inertia, 0.1, [a, a, 0.0]).unwrap();
        let expected = 3.0 * 0.01 * (a * a);
        assert!(tau[0].abs() < 1e-12);
        assert!(tau[1].abs() < 1e-12);
        assert!((tau[2] - expected).abs() < 1e-12);
    }
}
