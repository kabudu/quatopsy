//! Estimator interface with explicit frame, timestamp, covariance, and freshness.

use crate::geom::Quat;

#[derive(Clone, Copy)]
pub(crate) struct AttitudeEstimate {
    pub t_s: f64,
    pub q: Quat,
    pub omega: [f64; 3],
    pub covariance_trace: f64,
    pub frame_from_ok: bool,
    pub frame_to_ok: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Measurement {
    pub t_s: f64,
    pub q: Quat,
    pub omega: [f64; 3],
    pub covariance_trace: f64,
    pub frame_from_ok: bool,
    pub frame_to_ok: bool,
}

pub(crate) struct Estimator {
    estimate: Option<AttitudeEstimate>,
}

impl Estimator {
    pub(crate) fn new() -> Self {
        Self { estimate: None }
    }

    pub(crate) fn ingest(&mut self, measurement: Measurement) -> AttitudeEstimate {
        let estimate = AttitudeEstimate {
            t_s: measurement.t_s,
            q: measurement.q,
            omega: measurement.omega,
            covariance_trace: measurement.covariance_trace,
            frame_from_ok: measurement.frame_from_ok,
            frame_to_ok: measurement.frame_to_ok,
        };
        self.estimate = Some(estimate);
        estimate
    }
}
