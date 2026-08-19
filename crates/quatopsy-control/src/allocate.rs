//! Reaction-wheel allocation. Does not import the planner.

use crate::ControlError;
use crate::geom::{add3, norm3, scale3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WheelArray {
    pub axes: Vec<[f64; 3]>,
    pub inertia_kgm2: f64,
    pub torque_limit_nm: f64,
    pub momentum_limit_nms: f64,
}

pub(crate) struct Allocated {
    pub body: [f64; 3],
    #[allow(dead_code)]
    pub wheels: Vec<f64>,
}

pub(crate) fn allocate(requested: [f64; 3], array: &WheelArray) -> Result<Allocated, ControlError> {
    if array.axes.len() < 3 || array.axes.len() > 4 {
        return Err(ControlError::Refused(
            "wheel array must be a 3-axis triad or 4-wheel pyramid".to_string(),
        ));
    }
    if !array.inertia_kgm2.is_finite() || array.inertia_kgm2 <= 0.0 {
        return Err(ControlError::Refused(
            "wheel inertia_kgm2 must be finite and positive".to_string(),
        ));
    }
    if !array.torque_limit_nm.is_finite() || array.torque_limit_nm <= 0.0 {
        return Err(ControlError::Refused(
            "wheel torque_limit_nm must be finite and positive".to_string(),
        ));
    }
    let mut aat = [[0.0; 3]; 3];
    for axis in &array.axes {
        for i in 0..3 {
            for j in 0..3 {
                aat[i][j] += axis[i] * axis[j];
            }
        }
    }
    let aat_inv = invert3(aat)?;
    let mut u = vec![0.0; array.axes.len()];
    for (slot, axis) in u.iter_mut().zip(array.axes.iter()) {
        let mut tau_axis = 0.0;
        for i in 0..3 {
            tau_axis += axis[i]
                * (aat_inv[i][0] * requested[0]
                    + aat_inv[i][1] * requested[1]
                    + aat_inv[i][2] * requested[2]);
        }
        *slot = tau_axis.clamp(-array.torque_limit_nm, array.torque_limit_nm);
    }
    let mut body = [0.0; 3];
    for (axis, torque) in array.axes.iter().zip(u.iter()) {
        body[0] += axis[0] * *torque;
        body[1] += axis[1] * *torque;
        body[2] += axis[2] * *torque;
    }
    Ok(Allocated { body, wheels: u })
}

pub(crate) fn magnetorquer_dump(
    requested: [f64; 3],
    h: [f64; 3],
    field_t: [f64; 3],
    momentum_limit: f64,
    gain: f64,
) -> [f64; 3] {
    if norm3(h) <= 0.8 * momentum_limit {
        return requested;
    }
    let b2 = field_t[0] * field_t[0] + field_t[1] * field_t[1] + field_t[2] * field_t[2];
    if b2 < 1.0e-24 {
        return add3(requested, scale3(h, -gain));
    }
    let h_dot_b = h[0] * field_t[0] + h[1] * field_t[1] + h[2] * field_t[2];
    let h_par = scale3(field_t, h_dot_b / b2);
    let h_perp = [h[0] - h_par[0], h[1] - h_par[1], h[2] - h_par[2]];
    add3(requested, scale3(h_perp, -gain))
}

fn invert3(j: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], ControlError> {
    let det = j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
        - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
        + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0]);
    if !det.is_finite() || det.abs() < 1.0e-18 {
        return Err(ControlError::Refused(
            "wheel allocation matrix is singular".to_string(),
        ));
    }
    let inv = 1.0 / det;
    Ok([
        [
            (j[1][1] * j[2][2] - j[1][2] * j[2][1]) * inv,
            (j[0][2] * j[2][1] - j[0][1] * j[2][2]) * inv,
            (j[0][1] * j[1][2] - j[0][2] * j[1][1]) * inv,
        ],
        [
            (j[1][2] * j[2][0] - j[1][0] * j[2][2]) * inv,
            (j[0][0] * j[2][2] - j[0][2] * j[2][0]) * inv,
            (j[0][2] * j[1][0] - j[0][0] * j[1][2]) * inv,
        ],
        [
            (j[1][0] * j[2][1] - j[1][1] * j[2][0]) * inv,
            (j[0][1] * j[2][0] - j[0][0] * j[2][1]) * inv,
            (j[0][0] * j[1][1] - j[0][1] * j[1][0]) * inv,
        ],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triad_reproduces_the_requested_torque() {
        let array = WheelArray {
            axes: vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            inertia_kgm2: 0.01,
            torque_limit_nm: 2.0,
            momentum_limit_nms: 4.0,
        };
        let out = allocate([0.3, -0.4, 0.5], &array).unwrap();
        assert!((out.body[0] - 0.3).abs() < 1e-12);
        assert!((out.body[1] + 0.4).abs() < 1e-12);
        assert!((out.body[2] - 0.5).abs() < 1e-12);
        let residual =
            quatopsy_oracle::allocation_residual([0.3, -0.4, 0.5], &array.axes, &out.wheels)
                .unwrap();
        assert!(residual.iter().all(|item| item.abs() < 1e-12));
    }

    #[test]
    fn pyramid_has_a_small_residual_inside_limits() {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let array = WheelArray {
            axes: vec![[s, 0.0, s], [0.0, s, s], [-s, 0.0, s], [0.0, -s, s]],
            inertia_kgm2: 0.01,
            torque_limit_nm: 2.0,
            momentum_limit_nms: 4.0,
        };
        let out = allocate([0.2, 0.1, 0.0], &array).unwrap();
        let residual =
            quatopsy_oracle::allocation_residual([0.2, 0.1, 0.0], &array.axes, &out.wheels)
                .unwrap();
        assert!(norm3(residual) < 1e-9);
    }
}
