//! Software attitude navigator. Never assigns a report `result`.

mod mat;
mod mekf;
mod quat;
mod ukf;

use crate::mekf::Mekf;
use crate::ukf::Ukf;
use thiserror::Error;

pub type Matrix6 = [[f64; 6]; 6];
pub type Vec6 = [f64; 6];

#[derive(Debug, Error)]
pub enum NavError {
    #[error("{0}")]
    Refused(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Mekf,
    Ukf,
}

#[derive(Debug, Clone, Copy)]
pub struct NavConfig {
    pub filter: FilterKind,
    pub sigma_star_rad: f64,
    pub sigma_arw: f64,
    pub sigma_rrw: f64,
    pub chi2_gate: f64,
}

impl NavConfig {
    pub fn validated(self) -> Result<Self, NavError> {
        if self.sigma_star_rad <= 0.0
            || !self.sigma_star_rad.is_finite()
            || self.sigma_star_rad > 1.0
            || self.sigma_arw < 0.0
            || !self.sigma_arw.is_finite()
            || self.sigma_rrw < 0.0
            || !self.sigma_rrw.is_finite()
            || self.chi2_gate <= 0.0
            || !self.chi2_gate.is_finite()
        {
            return Err(NavError::Refused(
                "navigation noise and chi-square gate must be finite and positive within compiled bounds"
                    .to_string(),
            ));
        }
        Ok(self)
    }
}

impl Default for NavConfig {
    fn default() -> Self {
        Self {
            filter: FilterKind::Mekf,
            sigma_star_rad: 1.0e-4,
            sigma_arw: 1.0e-4,
            sigma_rrw: 1.0e-6,
            chi2_gate: 11.345,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NavEstimate {
    pub t_s: f64,
    pub q: [f64; 4],
    pub omega: [f64; 3],
    pub bias: [f64; 3],
    pub covariance_trace: f64,
    pub nis: f64,
    pub rejected: u64,
    pub innovation: [f64; 3],
    pub innovation_s: [[f64; 3]; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct GyroSample {
    pub t_s: f64,
    pub omega: [f64; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct StarSample {
    pub t_s: f64,
    pub q: [f64; 4],
}

pub struct Navigator {
    kind: FilterKind,
    mekf: Mekf,
    ukf: Ukf,
}

impl Navigator {
    pub fn new(q0: [f64; 4], t0: f64, config: NavConfig) -> Result<Self, NavError> {
        let config = config.validated()?;
        Ok(Self {
            kind: config.filter,
            mekf: Mekf::new(q0, t0, config)?,
            ukf: Ukf::new(q0, t0, config)?,
        })
    }

    pub fn predict(&mut self, gyro: GyroSample, now_s: f64) -> Result<NavEstimate, NavError> {
        match self.kind {
            FilterKind::Mekf => self.mekf.predict(gyro, now_s),
            FilterKind::Ukf => self.ukf.predict(gyro, now_s),
        }
    }

    pub fn update_star(
        &mut self,
        star: StarSample,
        valid: bool,
    ) -> Result<(NavEstimate, bool), NavError> {
        match self.kind {
            FilterKind::Mekf => self.mekf.update_star(star, valid),
            FilterKind::Ukf => self.ukf.update_star(star, valid),
        }
    }

    pub fn estimate(&self) -> NavEstimate {
        match self.kind {
            FilterKind::Mekf => self.mekf.estimate(),
            FilterKind::Ukf => self.ukf.estimate(),
        }
    }

    pub fn covariance(&self) -> Matrix6 {
        match self.kind {
            FilterKind::Mekf => self.mekf.p,
            FilterKind::Ukf => self.ukf.p,
        }
    }
}

/// Error-state vector for independent NEES: `[δθ; δb]`.
pub fn attitude_error_state(
    q_hat: [f64; 4],
    q_true: [f64; 4],
    bias_hat: [f64; 3],
    bias_true: [f64; 3],
) -> Vec6 {
    let dq = crate::quat::mul(crate::quat::conj(q_hat), q_true);
    let sign = if dq[0] < 0.0 { -2.0 } else { 2.0 };
    [
        sign * dq[1],
        sign * dq[2],
        sign * dq[3],
        bias_hat[0] - bias_true[0],
        bias_hat[1] - bias_true[1],
        bias_hat[2] - bias_true[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> [f64; 4] {
        [1.0, 0.0, 0.0, 0.0]
    }

    fn config(kind: FilterKind) -> NavConfig {
        NavConfig {
            filter: kind,
            sigma_star_rad: 1.0e-3,
            sigma_arw: 1.0e-4,
            sigma_rrw: 1.0e-6,
            chi2_gate: 11.345,
        }
    }

    fn recover_bias(kind: FilterKind) {
        let mut nav = Navigator::new(identity(), 0.0, config(kind)).unwrap();
        let true_bias = [0.02, 0.0, 0.0];
        for k in 0..400 {
            let t = 0.01 * (k + 1) as f64;
            let gyro = GyroSample {
                t_s: t,
                omega: true_bias,
            };
            nav.predict(gyro, t).unwrap();
            if k % 10 == 9 {
                nav.update_star(
                    StarSample {
                        t_s: t,
                        q: identity(),
                    },
                    true,
                )
                .unwrap();
            }
        }
        let est = nav.estimate();
        assert!(
            (est.bias[0] - true_bias[0]).abs() < 5.0e-3,
            "bias {} for {kind:?}",
            est.bias[0]
        );
        assert!(
            est.omega[0].abs() < 5.0e-3,
            "corrected rate {} for {kind:?} must not copy the raw gyro",
            est.omega[0]
        );
        assert!(est.covariance_trace > 0.0);
    }

    #[test]
    fn mekf_recovers_a_constant_gyro_bias() {
        recover_bias(FilterKind::Mekf);
    }

    #[test]
    fn ukf_recovers_a_constant_gyro_bias() {
        recover_bias(FilterKind::Ukf);
    }

    #[test]
    fn outlier_star_sample_is_rejected_and_does_not_jump() {
        let mut nav = Navigator::new(identity(), 0.0, config(FilterKind::Mekf)).unwrap();
        nav.predict(
            GyroSample {
                t_s: 0.01,
                omega: [0.0; 3],
            },
            0.01,
        )
        .unwrap();
        let before = nav.estimate();
        let (after, accepted) = nav
            .update_star(
                StarSample {
                    t_s: 0.01,
                    q: [
                        std::f64::consts::FRAC_1_SQRT_2,
                        std::f64::consts::FRAC_1_SQRT_2,
                        0.0,
                        0.0,
                    ],
                },
                true,
            )
            .unwrap();
        assert!(!accepted);
        assert_eq!(after.rejected, 1);
        assert!((after.q[0] - before.q[0]).abs() < 1e-12);
        assert!((after.q[1] - before.q[1]).abs() < 1e-12);
    }

    #[test]
    fn identity_pass_through_would_copy_the_raw_gyro() {
        let gyro = [0.02_f64, 0.0, 0.0];
        let copied = gyro;
        assert!((copied[0] - 0.02).abs() < 1e-15);
        recover_bias(FilterKind::Mekf);
        recover_bias(FilterKind::Ukf);
    }

    #[test]
    fn matching_star_update_has_small_nis() {
        let mut nav = Navigator::new(identity(), 0.0, config(FilterKind::Mekf)).unwrap();
        nav.predict(
            GyroSample {
                t_s: 0.01,
                omega: [0.0; 3],
            },
            0.01,
        )
        .unwrap();
        let (est, accepted) = nav
            .update_star(
                StarSample {
                    t_s: 0.01,
                    q: identity(),
                },
                true,
            )
            .unwrap();
        assert!(accepted);
        assert!(est.nis < 1.0);
        assert!(est.covariance_trace > 0.0);
        assert!(est.covariance_trace < 0.04);
    }

    #[test]
    fn predict_only_grows_covariance_with_process_noise() {
        let mut mekf_config = config(FilterKind::Mekf);
        mekf_config.sigma_arw = 0.05;
        let mut mekf = Navigator::new(identity(), 0.0, mekf_config).unwrap();
        mekf.predict(
            GyroSample {
                t_s: 0.01,
                omega: [0.0; 3],
            },
            0.01,
        )
        .unwrap();
        let mekf_start = mekf.estimate().covariance_trace;
        let mut quiet = config(FilterKind::Ukf);
        quiet.sigma_arw = 0.0;
        quiet.sigma_rrw = 0.0;
        let mut noisy = config(FilterKind::Ukf);
        noisy.sigma_arw = 0.05;
        let mut ukf_quiet = Navigator::new(identity(), 0.0, quiet).unwrap();
        let mut ukf_noisy = Navigator::new(identity(), 0.0, noisy).unwrap();
        for k in 1..80 {
            let t = 0.01 * k as f64;
            let gyro = GyroSample {
                t_s: t,
                omega: [0.0; 3],
            };
            mekf.predict(gyro, t).unwrap();
            ukf_quiet.predict(gyro, t).unwrap();
            ukf_noisy.predict(gyro, t).unwrap();
        }
        assert!(
            mekf.estimate().covariance_trace > mekf_start + 1.0e-4,
            "MEKF process noise must grow P when stars are absent"
        );
        assert!(
            ukf_noisy.estimate().covariance_trace > ukf_quiet.estimate().covariance_trace + 1.0e-4,
            "UKF process noise must be visible against a Q=0 run"
        );
    }

    #[test]
    fn delayed_star_is_applied_future_star_is_refused() {
        let mut nav = Navigator::new(identity(), 0.0, config(FilterKind::Mekf)).unwrap();
        nav.predict(
            GyroSample {
                t_s: 0.01,
                omega: [0.0; 3],
            },
            0.05,
        )
        .unwrap();
        let (est, accepted) = nav
            .update_star(
                StarSample {
                    t_s: 0.01,
                    q: identity(),
                },
                true,
            )
            .unwrap();
        assert!(accepted);
        assert!(est.t_s > 0.04);
        assert!(
            nav.update_star(
                StarSample {
                    t_s: 1.0,
                    q: identity(),
                },
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn ukf_measurement_update_is_not_a_linear_mekf_copy() {
        let mut mekf = Navigator::new(identity(), 0.0, config(FilterKind::Mekf)).unwrap();
        let mut ukf = Navigator::new(identity(), 0.0, config(FilterKind::Ukf)).unwrap();
        let gyro = GyroSample {
            t_s: 0.05,
            omega: [0.1, 0.0, 0.0],
        };
        mekf.predict(gyro, 0.05).unwrap();
        ukf.predict(gyro, 0.05).unwrap();
        let star = StarSample {
            t_s: 0.05,
            q: [0.999987500078125, 0.004999979166692708, 0.0, 0.0],
        };
        let (m, m_ok) = mekf.update_star(star, true).unwrap();
        let (u, u_ok) = ukf.update_star(star, true).unwrap();
        assert_eq!(m_ok, u_ok);
        let p_diff = (m.covariance_trace - u.covariance_trace).abs();
        assert!(
            p_diff > 1e-12 || (m.q[1] - u.q[1]).abs() > 1e-12,
            "UKF must not reproduce the MEKF Joseph update identically"
        );
    }
}
