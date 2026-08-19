//! Navigator adapter. Filter math lives in `quatopsy-nav`.

use crate::geom::Quat;
use quatopsy_nav::{NavConfig, NavEstimate, Navigator};

pub(crate) struct Estimator {
    nav: Navigator,
}

impl Estimator {
    pub(crate) fn new(q0: Quat, t0: f64, config: NavConfig) -> Result<Self, crate::ControlError> {
        Ok(Self {
            nav: Navigator::new([q0.w, q0.x, q0.y, q0.z], t0, config)
                .map_err(|err| crate::ControlError::Refused(err.to_string()))?,
        })
    }

    pub(crate) fn predict(
        &mut self,
        now_s: f64,
        gyro_t_s: f64,
        omega: [f64; 3],
    ) -> Result<NavEstimate, crate::ControlError> {
        self.nav
            .predict(
                quatopsy_nav::GyroSample {
                    t_s: gyro_t_s,
                    omega,
                },
                now_s,
            )
            .map_err(|err| crate::ControlError::Refused(err.to_string()))
    }

    pub(crate) fn update_star(
        &mut self,
        t_s: f64,
        q: Quat,
        valid: bool,
    ) -> Result<(NavEstimate, bool), crate::ControlError> {
        self.nav
            .update_star(
                quatopsy_nav::StarSample {
                    t_s,
                    q: [q.w, q.x, q.y, q.z],
                },
                valid,
            )
            .map_err(|err| crate::ControlError::Refused(err.to_string()))
    }

    pub(crate) fn estimate(&self) -> NavEstimate {
        self.nav.estimate()
    }

    pub(crate) fn covariance(&self) -> quatopsy_nav::Matrix6 {
        self.nav.covariance()
    }
}
