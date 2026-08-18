//! Mode transitions, command arbitration, and fail-closed safe fallback.

use crate::geom::{norm3, scale3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Idle,
    Track,
    Hold,
    Inhibit,
    Safe,
}

#[derive(Clone, Copy)]
pub(crate) struct Arbitration {
    pub mode: Mode,
    pub torque: [f64; 3],
}

pub(crate) fn arbitrate(
    mode: Mode,
    tracked_torque: [f64; 3],
    omega: [f64; 3],
    kd_safe: f64,
    monitor_allow: bool,
    attitude_err: f64,
    rate_err: f64,
) -> Arbitration {
    if !monitor_allow || mode == Mode::Safe || mode == Mode::Inhibit {
        return Arbitration {
            mode: Mode::Safe,
            torque: scale3(omega, -kd_safe),
        };
    }
    if attitude_err < 1.0e-2 && rate_err < 1.0e-2 {
        return Arbitration {
            mode: Mode::Hold,
            torque: tracked_torque,
        };
    }
    Arbitration {
        mode: Mode::Track,
        torque: tracked_torque,
    }
}

pub(crate) fn hold_metrics(e_r: [f64; 3], omega: [f64; 3]) -> (f64, f64) {
    (norm3(e_r), norm3(omega))
}
