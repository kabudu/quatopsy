#![allow(clippy::needless_range_loop)]

use crate::mat::{add, cholesky, identity6, invert3, scale, trace};
use crate::quat::{self, finite};
use crate::{GyroSample, Matrix6, NavConfig, NavError, NavEstimate, StarSample, Vec6};

const N: f64 = 6.0;
/// Scaled unscented weights with `κ = 3` so mean and covariance weights are positive and sum to 1.
const ALPHA: f64 = 1.0;
const BETA: f64 = 0.0;
const KAPPA: f64 = 3.0;

pub(crate) struct Ukf {
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

impl Ukf {
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
            let (wm, wc, lambda) = weights();
            let l = cholesky(add(self.p, scale(identity6(), 1.0e-12)))
                .map_err(|err| NavError::Refused(err.to_string()))?;
            let scale_l = (N + lambda).sqrt();
            let mut sigmas = [[0.0; 6]; 13];
            sigmas[0] = [0.0; 6];
            for i in 0..6 {
                let mut col = [0.0; 6];
                for r in 0..6 {
                    col[r] = scale_l * l[r][i];
                }
                sigmas[i + 1] = col;
                sigmas[i + 7] = [-col[0], -col[1], -col[2], -col[3], -col[4], -col[5]];
            }
            let mut qs = [[0.0; 4]; 13];
            let mut biases = [[0.0; 3]; 13];
            for (i, sigma) in sigmas.iter().enumerate() {
                let qi = quat::normalize(quat::mul(
                    self.q,
                    quat::exp_so3([sigma[0], sigma[1], sigma[2]]),
                ))
                .map_err(|err| NavError::Refused(err.to_string()))?;
                let bi = [
                    self.bias[0] + sigma[3],
                    self.bias[1] + sigma[4],
                    self.bias[2] + sigma[5],
                ];
                let omega = [
                    0.5 * (prev_omega[0] + gyro.omega[0]) - bi[0],
                    0.5 * (prev_omega[1] + gyro.omega[1]) - bi[1],
                    0.5 * (prev_omega[2] + gyro.omega[2]) - bi[2],
                ];
                let steps = ((dt / 7.5e-4).ceil() as usize).clamp(1, 32);
                let h = dt / steps as f64;
                let mut q_i = qi;
                for _ in 0..steps {
                    q_i = quat::integrate(q_i, omega, h)
                        .map_err(|err| NavError::Refused(err.to_string()))?;
                }
                qs[i] = q_i;
                biases[i] = bi;
            }
            let q_ref = self.q;
            let b_ref = self.bias;
            let mut mean_err = [0.0; 6];
            for (i, (q_i, b_i)) in qs.iter().zip(biases.iter()).enumerate() {
                let err = error_from(q_ref, *q_i, b_ref, *b_i);
                let w = if i == 0 { wm[0] } else { wm[1] };
                for k in 0..6 {
                    mean_err[k] += w * err[k];
                }
            }
            self.q = quat::normalize(quat::mul(
                q_ref,
                quat::exp_so3([mean_err[0], mean_err[1], mean_err[2]]),
            ))
            .map_err(|err| NavError::Refused(err.to_string()))?;
            self.bias = [
                b_ref[0] + mean_err[3],
                b_ref[1] + mean_err[4],
                b_ref[2] + mean_err[5],
            ];
            let mut p = crate::mat::zeros6();
            for i in 0..13 {
                let err = error_from(self.q, qs[i], self.bias, biases[i]);
                let w = if i == 0 { wc[0] } else { wc[1] };
                p = add(p, scale(outer(err, err), w));
            }
            self.p = crate::mat::floor_diag(add(p, process_q(self.config, dt)), 1.0e-18);
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
        if !star.t_s.is_finite() || star.t_s > self.t_s + 1.0e-9 {
            return Err(NavError::Refused(
                "star-tracker sample time is invalid or in the future".to_string(),
            ));
        }
        if !finite(star.q) {
            return Err(NavError::Refused(
                "star-tracker measurement is not finite".to_string(),
            ));
        }
        let qm = quat::normalize(star.q).map_err(|err| NavError::Refused(err.to_string()))?;
        let (wm, wc, lambda) = weights();
        let l = cholesky(add(self.p, scale(identity6(), 1.0e-12)))
            .map_err(|err| NavError::Refused(err.to_string()))?;
        let scale_l = (N + lambda).sqrt();
        let mut sigmas = [[0.0; 6]; 13];
        sigmas[0] = [0.0; 6];
        for i in 0..6 {
            let mut col = [0.0; 6];
            for r in 0..6 {
                col[r] = scale_l * l[r][i];
            }
            sigmas[i + 1] = col;
            sigmas[i + 7] = [-col[0], -col[1], -col[2], -col[3], -col[4], -col[5]];
        }
        let mut zs = [[0.0; 3]; 13];
        for (i, sigma) in sigmas.iter().enumerate() {
            let qi = quat::normalize(quat::mul(
                self.q,
                quat::exp_so3([sigma[0], sigma[1], sigma[2]]),
            ))
            .map_err(|err| NavError::Refused(err.to_string()))?;
            zs[i] = quat::attitude_error(qi, qm);
        }
        let mut z_mean = [0.0; 3];
        for (i, z_i) in zs.iter().enumerate() {
            let w = if i == 0 { wm[0] } else { wm[1] };
            z_mean[0] += w * z_i[0];
            z_mean[1] += w * z_i[1];
            z_mean[2] += w * z_i[2];
        }
        let r = self.config.sigma_star_rad * self.config.sigma_star_rad;
        let mut pzz = [[r, 0.0, 0.0], [0.0, r, 0.0], [0.0, 0.0, r]];
        let mut pxz = [[0.0; 3]; 6];
        for i in 0..13 {
            let w = if i == 0 { wc[0] } else { wc[1] };
            let dz = [
                zs[i][0] - z_mean[0],
                zs[i][1] - z_mean[1],
                zs[i][2] - z_mean[2],
            ];
            for a in 0..3 {
                for b in 0..3 {
                    pzz[a][b] += w * dz[a] * dz[b];
                }
            }
            for a in 0..6 {
                for b in 0..3 {
                    // attitude_error(q⊗exp(x), q_meas) has Jacobian -I. Flip so K matches MEKF H=[I 0].
                    pxz[a][b] += w * sigmas[i][a] * (-dz[b]);
                }
            }
        }
        let pzz_inv = invert3(pzz).map_err(|err| NavError::Refused(err.to_string()))?;
        let nis = z_mean[0]
            * (pzz_inv[0][0] * z_mean[0] + pzz_inv[0][1] * z_mean[1] + pzz_inv[0][2] * z_mean[2])
            + z_mean[1]
                * (pzz_inv[1][0] * z_mean[0]
                    + pzz_inv[1][1] * z_mean[1]
                    + pzz_inv[1][2] * z_mean[2])
            + z_mean[2]
                * (pzz_inv[2][0] * z_mean[0]
                    + pzz_inv[2][1] * z_mean[1]
                    + pzz_inv[2][2] * z_mean[2]);
        self.last_nis = nis;
        self.last_z = z_mean;
        self.last_s = pzz;
        if nis > self.config.chi2_gate {
            self.rejected += 1;
            return Ok((self.estimate(), false));
        }
        let mut k = [[0.0; 3]; 6];
        for i in 0..6 {
            for j in 0..3 {
                k[i][j] = pxz[i][0] * pzz_inv[0][j]
                    + pxz[i][1] * pzz_inv[1][j]
                    + pxz[i][2] * pzz_inv[2][j];
            }
        }
        let dx = [
            k[0][0] * z_mean[0] + k[0][1] * z_mean[1] + k[0][2] * z_mean[2],
            k[1][0] * z_mean[0] + k[1][1] * z_mean[1] + k[1][2] * z_mean[2],
            k[2][0] * z_mean[0] + k[2][1] * z_mean[1] + k[2][2] * z_mean[2],
            k[3][0] * z_mean[0] + k[3][1] * z_mean[1] + k[3][2] * z_mean[2],
            k[4][0] * z_mean[0] + k[4][1] * z_mean[1] + k[4][2] * z_mean[2],
            k[5][0] * z_mean[0] + k[5][1] * z_mean[1] + k[5][2] * z_mean[2],
        ];
        self.q = quat::normalize(quat::mul(self.q, quat::exp_so3([dx[0], dx[1], dx[2]])))
            .map_err(|err| NavError::Refused(err.to_string()))?;
        self.bias = [
            self.bias[0] + dx[3],
            self.bias[1] + dx[4],
            self.bias[2] + dx[5],
        ];
        let mut k_pzz_kt = crate::mat::zeros6();
        for i in 0..6 {
            for j in 0..6 {
                let mut acc = 0.0;
                for a in 0..3 {
                    let mut row = 0.0;
                    for b in 0..3 {
                        row += pzz[a][b] * k[j][b];
                    }
                    acc += k[i][a] * row;
                }
                k_pzz_kt[i][j] = acc;
            }
        }
        let mut p = crate::mat::floor_diag(add(self.p, scale(k_pzz_kt, -1.0)), 1.0e-18);
        for i in 0..6 {
            for j in i + 1..6 {
                let mean = 0.5 * (p[i][j] + p[j][i]);
                p[i][j] = mean;
                p[j][i] = mean;
            }
        }
        self.p = p;
        Ok((self.estimate(), true))
    }
}

fn weights() -> ([f64; 2], [f64; 2], f64) {
    let lambda = ALPHA * ALPHA * (N + KAPPA) - N;
    let wm0 = lambda / (N + lambda);
    let wc0 = wm0 + (1.0 - ALPHA * ALPHA + BETA);
    let wi = 0.5 / (N + lambda);
    ([wm0, wi], [wc0, wi], lambda)
}

fn error_from(q_ref: [f64; 4], q: [f64; 4], b_ref: [f64; 3], b: [f64; 3]) -> Vec6 {
    let dq = quat::mul(quat::conj(q_ref), q);
    let sign = if dq[0] < 0.0 { -2.0 } else { 2.0 };
    [
        sign * dq[1],
        sign * dq[2],
        sign * dq[3],
        b[0] - b_ref[0],
        b[1] - b_ref[1],
        b[2] - b_ref[2],
    ]
}

fn outer(a: Vec6, b: Vec6) -> Matrix6 {
    let mut m = crate::mat::zeros6();
    for i in 0..6 {
        for j in 0..6 {
            m[i][j] = a[i] * b[j];
        }
    }
    m
}

fn initial_p() -> Matrix6 {
    let mut p = crate::mat::zeros6();
    for i in 0..3 {
        p[i][i] = 0.01;
        p[i + 3][i + 3] = 0.0025;
    }
    p
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
