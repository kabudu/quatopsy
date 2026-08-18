//! Saturation, anti-windup bookkeeping, and stored-momentum dump.

use crate::geom::{add3, norm3, scale3};
use crate::law::LawState;

pub(crate) fn saturate(torque: [f64; 3], limit: [f64; 3], state: &mut LawState) -> [f64; 3] {
    let mut out = [0.0; 3];
    let mut sat = false;
    for k in 0..3 {
        out[k] = torque[k].clamp(-limit[k], limit[k]);
        if (out[k] - torque[k]).abs() > 1.0e-12 {
            sat = true;
        }
    }
    state.saturated = sat;
    out
}

pub(crate) fn momentum_dump(
    requested: [f64; 3],
    h: [f64; 3],
    momentum_limit: f64,
    gain: f64,
) -> [f64; 3] {
    let n = norm3(h);
    if n <= 0.8 * momentum_limit {
        return requested;
    }
    add3(requested, scale3(h, -gain))
}

pub(crate) fn fail_axis(torque: [f64; 3], axis: Option<usize>) -> [f64; 3] {
    let Some(index) = axis else {
        return torque;
    };
    if index > 2 {
        return torque;
    }
    let mut out = torque;
    out[index] = 0.0;
    out
}
