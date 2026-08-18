//! Geometric PD on SO(3) with gyroscopic compensation and angular-acceleration feed-forward.

use crate::geom::{
    Quat, add3, apply_matrix, apply_tensor, cross, matmul, rotation_matrix, scale3, so3_error,
    sub3, transpose,
};

#[derive(Clone, Copy)]
pub(crate) struct Gains {
    pub kp: f64,
    pub kd: f64,
    pub ki: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct Reference {
    pub q: Quat,
    pub omega: [f64; 3],
    pub alpha: [f64; 3],
}

#[derive(Clone, Copy)]
pub(crate) struct LawState {
    pub integral: [f64; 3],
    pub saturated: bool,
}

impl LawState {
    pub(crate) fn new() -> Self {
        Self {
            integral: [0.0; 3],
            saturated: false,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BodyState {
    pub q: Quat,
    pub omega: [f64; 3],
    pub h: [f64; 3],
}

pub(crate) fn geometric_pd(
    inertia: [[f64; 3]; 3],
    body: BodyState,
    reference: Reference,
    gains: Gains,
    dt: f64,
    state: &mut LawState,
) -> [f64; 3] {
    let e_r = so3_error(body.q, reference.q);
    let r = rotation_matrix(body.q);
    let rd = rotation_matrix(reference.q);
    let omega_d_body = apply_matrix(matmul(transpose(r), rd), reference.omega);
    let e_w = sub3(body.omega, omega_d_body);
    if !state.saturated {
        state.integral = add3(state.integral, scale3(e_r, dt));
        for slot in &mut state.integral {
            *slot = slot.clamp(-10.0, 10.0);
        }
    }
    let pd = add3(
        add3(scale3(e_r, -gains.kp), scale3(e_w, -gains.kd)),
        scale3(state.integral, -gains.ki),
    );
    let jw = apply_tensor(inertia, body.omega);
    let gyro = cross(
        body.omega,
        [jw[0] + body.h[0], jw[1] + body.h[1], jw[2] + body.h[2]],
    );
    let alpha_body = apply_matrix(matmul(transpose(r), rd), reference.alpha);
    let feedforward = add3(gyro, apply_tensor(inertia, alpha_body));
    add3(pd, feedforward)
}
