//! Shared SO(3) and inertia helpers for the candidate planner.

use libm::{acos, cos, fabs, sin, sqrt};
use quatopsy_oracle::RefQuat;
use quatopsy_schema::{ComponentOrder, NORM_ABS_TOLERANCE};

use crate::PlanError;

#[derive(Clone, Copy)]
pub(crate) struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub(crate) fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    pub(crate) fn is_finite(self) -> bool {
        self.w.is_finite() && self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub(crate) fn norm(self) -> f64 {
        sqrt(self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z)
    }

    pub(crate) fn dot(self, other: Self) -> f64 {
        self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub(crate) fn scale(self, factor: f64) -> Self {
        Self::new(
            self.w * factor,
            self.x * factor,
            self.y * factor,
            self.z * factor,
        )
    }

    pub(crate) fn negate(self) -> Self {
        self.scale(-1.0)
    }

    pub(crate) fn conjugate(self) -> Self {
        Self::new(self.w, -self.x, -self.y, -self.z)
    }

    pub(crate) fn mul(self, other: Self) -> Self {
        Self::new(
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        )
    }

    pub(crate) fn normalized(self) -> Result<Self, PlanError> {
        if !self.is_finite() {
            return Err(PlanError::Refused(
                "planner quaternion is not finite".to_string(),
            ));
        }
        let n = self.norm();
        if n == 0.0 || !n.is_finite() {
            return Err(PlanError::Refused(
                "planner quaternion is near zero".to_string(),
            ));
        }
        if fabs(n - 1.0) > NORM_ABS_TOLERANCE {
            return Err(PlanError::Refused(
                "planner quaternion is off-unit".to_string(),
            ));
        }
        Ok(self.scale(1.0 / n))
    }

    pub(crate) fn as_ref(self) -> RefQuat {
        RefQuat {
            w: self.w,
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

pub(crate) fn from_declared(raw: [f64; 4], order: ComponentOrder) -> Result<Quat, PlanError> {
    if raw.iter().any(|item| !item.is_finite()) {
        return Err(PlanError::Refused(
            "planner quaternion components must be finite".to_string(),
        ));
    }
    match order {
        ComponentOrder::Wxyz => Ok(Quat::new(raw[0], raw[1], raw[2], raw[3])),
        ComponentOrder::Xyzw => Ok(Quat::new(raw[3], raw[0], raw[1], raw[2])),
    }
}

pub(crate) fn exp_so3(phi: [f64; 3]) -> Quat {
    let n = norm3(phi);
    if n < 1.0e-16 {
        return Quat::new(1.0, 0.5 * phi[0], 0.5 * phi[1], 0.5 * phi[2]);
    }
    let half = 0.5 * n;
    let s = sin(half) / n;
    Quat::new(cos(half), phi[0] * s, phi[1] * s, phi[2] * s)
}

pub(crate) fn log_so3(q: Quat) -> [f64; 3] {
    let mut qn = q;
    if qn.w < 0.0 {
        qn = qn.negate();
    }
    let v = [qn.x, qn.y, qn.z];
    let n = norm3(v);
    if n < 1.0e-16 {
        return [2.0 * qn.x, 2.0 * qn.y, 2.0 * qn.z];
    }
    let w = qn.w.clamp(-1.0, 1.0);
    let angle = 2.0 * acos(w);
    scale3(v, angle / n)
}

pub(crate) fn apply_tensor(j: [[f64; 3]; 3], w: [f64; 3]) -> [f64; 3] {
    [
        j[0][0] * w[0] + j[0][1] * w[1] + j[0][2] * w[2],
        j[1][0] * w[0] + j[1][1] * w[1] + j[1][2] * w[2],
        j[2][0] * w[0] + j[2][1] * w[1] + j[2][2] * w[2],
    ]
}

pub(crate) fn invert3(j: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], PlanError> {
    let det = det3(j);
    if !det.is_finite() || fabs(det) < 1.0e-18 {
        return Err(PlanError::Refused(
            "3x3 matrix is not invertible".to_string(),
        ));
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

pub(crate) fn invert_spd(j: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], PlanError> {
    if !is_spd(j) {
        return Err(PlanError::Refused(
            "inertia tensor is not symmetric positive definite".to_string(),
        ));
    }
    invert3(j)
}

pub(crate) fn det3(j: [[f64; 3]; 3]) -> f64 {
    j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
        - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
        + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0])
}

pub(crate) fn is_spd(j: [[f64; 3]; 3]) -> bool {
    if (j[0][1] - j[1][0]).abs() > 1.0e-12
        || (j[0][2] - j[2][0]).abs() > 1.0e-12
        || (j[1][2] - j[2][1]).abs() > 1.0e-12
    {
        return false;
    }
    let m00 = j[0][0];
    let m11 = j[0][0] * j[1][1] - j[0][1] * j[1][0];
    let det = det3(j);
    m00 > 0.0 && m11 > 0.0 && det > 0.0 && m00.is_finite() && m11.is_finite() && det.is_finite()
}

pub(crate) fn euler_lhs(j: [[f64; 3]; 3], w_dot: [f64; 3], w: [f64; 3], h: [f64; 3]) -> [f64; 3] {
    let jw = apply_tensor(j, w);
    let jw_dot = apply_tensor(j, w_dot);
    let gyro = cross(w, [jw[0] + h[0], jw[1] + h[1], jw[2] + h[2]]);
    add3(jw_dot, gyro)
}

pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(crate) fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

pub(crate) fn norm3(v: [f64; 3]) -> f64 {
    sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
}

pub(crate) fn unit3(v: [f64; 3]) -> Result<[f64; 3], PlanError> {
    let n = norm3(v);
    if n < 1.0e-18 || !n.is_finite() {
        return Err(PlanError::Refused(
            "planner axis must be finite and non-zero".to_string(),
        ));
    }
    Ok(scale3(v, 1.0 / n))
}

pub(crate) fn torque_excess(torque: [f64; 3], limit: [f64; 3]) -> f64 {
    (0..3)
        .map(|k| torque[k].abs() - limit[k])
        .fold(0.0_f64, f64::max)
}

pub(crate) fn rest(omega: [f64; 3], abs: f64) -> bool {
    omega
        .iter()
        .all(|item| item.is_finite() && item.abs() <= abs)
}
