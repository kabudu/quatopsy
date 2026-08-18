//! Independently encoded reference oracles.
//!
//! This crate must not depend on `quatopsy-core`. Production verdicts never
//! call these functions. The candidate planner may use the plan residual
//! oracle; that path still cannot assign a report `result`.

mod plan;

pub use plan::{
    KeepOutCone, PLAN_BOUNDARY_TOLERANCE, PLAN_EULER_TOLERANCE, PLAN_KEEP_OUT_TOLERANCE,
    PLAN_KINEMATICS_TOLERANCE, PLAN_TORQUE_EXCESS_TOLERANCE, PlanDynamics, PlanResiduals,
    PlanSample, keep_out_violation, plan_residuals, plan_residuals_ex,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefQuat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub type Matrix3 = [[f64; 3]; 3];

const FIXED_SCALE: i128 = 1_i128 << 60;

/// Rotation matrix from the outer-product / skew form:
/// `R = (w^2 - u·u) I + 2 u u^T + 2 w [u]_x`.
pub fn rotation_matrix(q: RefQuat) -> Matrix3 {
    let uu = q.x * q.x + q.y * q.y + q.z * q.z;
    let ww = q.w * q.w;
    let s = ww - uu;
    let x = q.x;
    let y = q.y;
    let z = q.z;
    let w = q.w;
    [
        [
            s + 2.0 * x * x,
            2.0 * x * y - 2.0 * w * z,
            2.0 * x * z + 2.0 * w * y,
        ],
        [
            2.0 * y * x + 2.0 * w * z,
            s + 2.0 * y * y,
            2.0 * y * z - 2.0 * w * x,
        ],
        [
            2.0 * z * x - 2.0 * w * y,
            2.0 * z * y + 2.0 * w * x,
            s + 2.0 * z * z,
        ],
    ]
}

pub fn transpose(m: Matrix3) -> Matrix3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

pub fn matmul(a: Matrix3, b: Matrix3) -> Matrix3 {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

/// Geodesic angle from `trace(R_p^T R_q) = 1 + 2 cos θ`.
pub fn geodesic_angle(p: RefQuat, q: RefQuat) -> f64 {
    let rel = matmul(transpose(rotation_matrix(p)), rotation_matrix(q));
    let trace = rel[0][0] + rel[1][1] + rel[2][2];
    let cos_theta = clamp((trace - 1.0) * 0.5, -1.0, 1.0);
    cos_theta.acos()
}

/// Fixed-point dot product using 2^-60 units. Independent of the kernel adder.
pub fn high_precision_dot(p: RefQuat, q: RefQuat) -> f64 {
    let pw = to_fixed(p.w);
    let px = to_fixed(p.x);
    let py = to_fixed(p.y);
    let pz = to_fixed(p.z);
    let qw = to_fixed(q.w);
    let qx = to_fixed(q.x);
    let qy = to_fixed(q.y);
    let qz = to_fixed(q.z);
    let acc = pw * qw + px * qx + py * qy + pz * qz;
    (acc as f64) / ((FIXED_SCALE * FIXED_SCALE) as f64)
}

fn to_fixed(value: f64) -> i128 {
    (value * FIXED_SCALE as f64).round() as i128
}

pub fn matrices_close(a: Matrix3, b: Matrix3, abs_tol: f64) -> bool {
    for i in 0..3 {
        for j in 0..3 {
            if (a[i][j] - b[i][j]).abs() > abs_tol {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_antipode_share_a_matrix() {
        let p = RefQuat {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let q = RefQuat {
            w: -1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        assert!(matrices_close(
            rotation_matrix(p),
            rotation_matrix(q),
            1e-15
        ));
        assert!(geodesic_angle(p, q).abs() < 1e-12);
    }
}
