//! Controller-local geometry. Rotation matrices use the Hamilton product form,
//! not the oracle outer-product encoding.

use libm::{cos, fabs, sin, sqrt};
use quatopsy_oracle::RefQuat;
use quatopsy_schema::{ComponentOrder, NORM_ABS_TOLERANCE};

use crate::ControlError;

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

    pub(crate) fn scale(self, factor: f64) -> Self {
        Self::new(
            self.w * factor,
            self.x * factor,
            self.y * factor,
            self.z * factor,
        )
    }

    pub(crate) fn mul(self, other: Self) -> Self {
        Self::new(
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        )
    }

    pub(crate) fn normalized(self) -> Result<Self, ControlError> {
        if !self.is_finite() {
            return Err(ControlError::Refused(
                "controller quaternion is not finite".to_string(),
            ));
        }
        let n = self.norm();
        if n == 0.0 || !n.is_finite() {
            return Err(ControlError::Refused(
                "controller quaternion is near zero".to_string(),
            ));
        }
        if fabs(n - 1.0) > NORM_ABS_TOLERANCE {
            return Err(ControlError::Refused(
                "controller quaternion is off-unit".to_string(),
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

    pub(crate) fn from_ref(q: RefQuat) -> Self {
        Self::new(q.w, q.x, q.y, q.z)
    }
}

pub(crate) fn from_declared(raw: [f64; 4], order: ComponentOrder) -> Result<Quat, ControlError> {
    if raw.iter().any(|item| !item.is_finite()) {
        return Err(ControlError::Refused(
            "controller quaternion components must be finite".to_string(),
        ));
    }
    match order {
        ComponentOrder::Wxyz => Ok(Quat::new(raw[0], raw[1], raw[2], raw[3])),
        ComponentOrder::Xyzw => Ok(Quat::new(raw[3], raw[0], raw[1], raw[2])),
    }
}

/// Hamilton rotation matrix, independently encoded from the oracle form.
pub(crate) fn rotation_matrix(q: Quat) -> [[f64; 3]; 3] {
    let x = q.x;
    let y = q.y;
    let z = q.z;
    let w = q.w;
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}

pub(crate) fn transpose(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

pub(crate) fn matmul(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

pub(crate) fn apply_matrix(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

pub(crate) fn so3_error(q: Quat, q_desired: Quat) -> [f64; 3] {
    let r = rotation_matrix(q);
    let rd = rotation_matrix(q_desired);
    let rel = matmul(transpose(rd), r);
    [
        0.5 * (rel[2][1] - rel[1][2]),
        0.5 * (rel[0][2] - rel[2][0]),
        0.5 * (rel[1][0] - rel[0][1]),
    ]
}

pub(crate) fn apply_tensor(j: [[f64; 3]; 3], w: [f64; 3]) -> [f64; 3] {
    [
        j[0][0] * w[0] + j[0][1] * w[1] + j[0][2] * w[2],
        j[1][0] * w[0] + j[1][1] * w[1] + j[1][2] * w[2],
        j[2][0] * w[0] + j[2][1] * w[1] + j[2][2] * w[2],
    ]
}

pub(crate) fn invert_spd(j: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], ControlError> {
    if (j[0][1] - j[1][0]).abs() > 1.0e-12
        || (j[0][2] - j[2][0]).abs() > 1.0e-12
        || (j[1][2] - j[2][1]).abs() > 1.0e-12
    {
        return Err(ControlError::Refused(
            "inertia tensor is not symmetric".to_string(),
        ));
    }
    let det = det3(j);
    let m00 = j[0][0];
    let m11 = j[0][0] * j[1][1] - j[0][1] * j[1][0];
    if m00 <= 0.0 || m11 <= 0.0 || det <= 0.0 || !det.is_finite() {
        return Err(ControlError::Refused(
            "inertia tensor is not positive definite".to_string(),
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

fn det3(j: [[f64; 3]; 3]) -> f64 {
    j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
        - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
        + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0])
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

pub(crate) fn rest(omega: [f64; 3], abs: f64) -> bool {
    omega
        .iter()
        .all(|item| item.is_finite() && item.abs() <= abs)
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
