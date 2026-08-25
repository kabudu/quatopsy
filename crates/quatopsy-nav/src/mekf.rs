#![allow(clippy::needless_range_loop)]

use crate::mat::{add, floor_diag, identity6, invert3, mul, scale, skew, trace, transpose};
use crate::quat::{self, finite};
use crate::{GyroSample, Matrix6, NavConfig, NavError, NavEstimate, StarSample};

pub(crate) struct Mekf {
    pub q: [f64; 4],
    pub bias: [f64; 3],
    pub p: Matrix6,
    pub t_s: f64,
    pub last_nis: f64,
    pub rejected: u64,
    last_omega_meas: [f64; 3],
    last_z: [f64; 3],
    last_s: [[f64; 3]; 3],
    config: NavConfig,
}

impl Mekf {
    pub(crate) fn new(q0: [f64; 4], t0: f64, config: NavConfig) -> Result<Self, NavError> {
        if !finite(q0) || !t0.is_finite() {
            return Err(NavError::Refused(
                "navigator initial state is not finite".to_string(),
            ));
        }
        let q = quat::normalize(q0).map_err(|err| NavError::Refused(err.to_string()))?;
        Ok(Self {
            q,
            bias: [0.0; 3],
            p: initial_p(),
            t_s: t0,
            last_nis: 0.0,
            rejected: 0,
            last_omega_meas: [0.0; 3],
            last_z: [0.0; 3],
            last_s: [[0.0; 3]; 3],
            config,
        })
    }

    pub(crate) fn estimate(&self) -> NavEstimate {
        NavEstimate {
            t_s: self.t_s,
            q: self.q,
            omega: [
                self.last_omega_meas[0] - self.bias[0],
                self.last_omega_meas[1] - self.bias[1],
                self.last_omega_meas[2] - self.bias[2],
            ],
            bias: self.bias,
            covariance_trace: trace(self.p),
            nis: self.last_nis,
            rejected: self.rejected,
            innovation: self.last_z,
            innovation_s: self.last_s,
        }
    }

    pub(crate) fn predict(
        &mut self,
        gyro: GyroSample,
        now_s: f64,
    ) -> Result<NavEstimate, NavError> {
        if !now_s.is_finite() || now_s < self.t_s - 1.0e-15 {
            return Err(NavError::Refused(
                "navigator predict time is invalid".to_string(),
            ));
        }
        if gyro.omega.iter().any(|item| !item.is_finite()) {
            return Err(NavError::Refused(
                "gyro measurement is not finite".to_string(),
            ));
        }
        if gyro.t_s > now_s + 1.0e-9 {
            return Err(NavError::Refused(
                "gyro sample is in the future of the cycle clock".to_string(),
            ));
        }
        let prev_omega = self.last_omega_meas;
        self.last_omega_meas = gyro.omega;
        let dt = now_s - self.t_s;
        if dt > 0.0 {
            let omega = [
                0.5 * (prev_omega[0] + gyro.omega[0]) - self.bias[0],
                0.5 * (prev_omega[1] + gyro.omega[1]) - self.bias[1],
                0.5 * (prev_omega[2] + gyro.omega[2]) - self.bias[2],
            ];
            let steps = ((dt / 7.5e-4).ceil() as usize).clamp(1, 32);
            let h = dt / steps as f64;
            for _ in 0..steps {
                self.q = quat::integrate(self.q, omega, h)
                    .map_err(|err| NavError::Refused(err.to_string()))?;
            }
            let phi = discrete_phi(omega, dt);
            self.p = add(
                mul(mul(phi, self.p), transpose(phi)),
                process_q(self.config, dt),
            );
            self.t_s = now_s;
        }
        Ok(self.estimate())
    }

    pub(crate) fn update_star(
        &mut self,
        star: StarSample,
        valid: bool,
    ) -> Result<(NavEstimate, bool), NavError> {
        if !valid {
            return Ok((self.estimate(), false));
        }
        if !star.t_s.is_finite() || (star.t_s - self.t_s).abs() > 1.0e-9 {
            return Err(NavError::Refused(
                "star-tracker sample must be time-aligned with the current estimate".to_string(),
            ));
        }
        if !finite(star.q) {
            return Err(NavError::Refused(
                "star-tracker measurement is not finite".to_string(),
            ));
        }
        let qm = quat::normalize(star.q).map_err(|err| NavError::Refused(err.to_string()))?;
        let z = quat::attitude_error(self.q, qm);
        let r = self.config.sigma_star_rad * self.config.sigma_star_rad;
        let s = [
            [self.p[0][0] + r, self.p[0][1], self.p[0][2]],
            [self.p[1][0], self.p[1][1] + r, self.p[1][2]],
            [self.p[2][0], self.p[2][1], self.p[2][2] + r],
        ];
        let s_inv = invert3(s).map_err(|err| NavError::Refused(err.to_string()))?;
        let nis = z[0] * (s_inv[0][0] * z[0] + s_inv[0][1] * z[1] + s_inv[0][2] * z[2])
            + z[1] * (s_inv[1][0] * z[0] + s_inv[1][1] * z[1] + s_inv[1][2] * z[2])
            + z[2] * (s_inv[2][0] * z[0] + s_inv[2][1] * z[1] + s_inv[2][2] * z[2]);
        self.last_nis = nis;
        self.last_z = z;
        self.last_s = s;
        if nis > self.config.chi2_gate {
            self.rejected += 1;
            return Ok((self.estimate(), false));
        }
        let mut k = [[0.0; 3]; 6];
        for i in 0..6 {
            for j in 0..3 {
                k[i][j] = self.p[i][0] * s_inv[0][j]
                    + self.p[i][1] * s_inv[1][j]
                    + self.p[i][2] * s_inv[2][j];
            }
        }
        let dx = [
            k[0][0] * z[0] + k[0][1] * z[1] + k[0][2] * z[2],
            k[1][0] * z[0] + k[1][1] * z[1] + k[1][2] * z[2],
            k[2][0] * z[0] + k[2][1] * z[1] + k[2][2] * z[2],
            k[3][0] * z[0] + k[3][1] * z[1] + k[3][2] * z[2],
            k[4][0] * z[0] + k[4][1] * z[1] + k[4][2] * z[2],
            k[5][0] * z[0] + k[5][1] * z[1] + k[5][2] * z[2],
        ];
        self.q = quat::normalize(quat::mul(self.q, quat::exp_so3([dx[0], dx[1], dx[2]])))
            .map_err(|err| NavError::Refused(err.to_string()))?;
        self.bias = [
            self.bias[0] + dx[3],
            self.bias[1] + dx[4],
            self.bias[2] + dx[5],
        ];
        let mut kh = identity6();
        for i in 0..6 {
            for j in 0..3 {
                kh[i][j] -= k[i][j];
            }
        }
        let imkh_p = mul(kh, self.p);
        let mut krk = crate::mat::zeros6();
        for i in 0..6 {
            for j in 0..6 {
                krk[i][j] = r * (k[i][0] * k[j][0] + k[i][1] * k[j][1] + k[i][2] * k[j][2]);
            }
        }
        self.p = floor_diag(add(mul(imkh_p, transpose(kh)), krk), 1.0e-18);
        Ok((self.estimate(), true))
    }
}

fn initial_p() -> Matrix6 {
    let mut p = crate::mat::zeros6();
    for i in 0..3 {
        p[i][i] = 0.01;
        p[i + 3][i + 3] = 0.0025;
    }
    p
}

fn discrete_phi(omega: [f64; 3], dt: f64) -> Matrix6 {
    let w = skew(omega);
    let mut f = crate::mat::zeros6();
    for i in 0..3 {
        for j in 0..3 {
            f[i][j] = -w[i][j];
            f[i][j + 3] = if i == j { -1.0 } else { 0.0 };
        }
    }
    add(identity6(), scale(f, dt))
}

fn process_q(config: NavConfig, dt: f64) -> Matrix6 {
    let sv2 = config.sigma_arw * config.sigma_arw;
    let su2 = config.sigma_rrw * config.sigma_rrw;
    let mut q = crate::mat::zeros6();
    for i in 0..3 {
        q[i][i] = sv2 * dt + su2 * dt * dt * dt / 3.0;
        q[i][i + 3] = -0.5 * su2 * dt * dt;
        q[i + 3][i] = q[i][i + 3];
        q[i + 3][i + 3] = su2 * dt;
    }
    q
}
