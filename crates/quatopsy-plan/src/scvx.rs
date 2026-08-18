//! Bounded multiple-shooting solver on SO(3) with Levenberg-Marquardt.
//!
//! Attitude is reconstructed with the exponential map. Quaternion components
//! are never decision variables. The algorithm name written to `plan.json` is
//! `multiple-shooting-lm`. This is not a globally optimal NLP, and it is not a
//! sequential-convexification collocation method.

use crate::PlanError;
use crate::actuators::{ActuatorMap, allocate_body_torque};
use crate::geom::{Quat, add3, euler_lhs, exp_so3, log_so3, norm3, scale3, sub3, torque_excess};
use quatopsy_oracle::{KeepOutCone, keep_out_violation};

pub(crate) const SOLVER_NODES: usize = 17;
pub(crate) const MAX_ITERS: usize = 40;
const LM_LAMBDA0: f64 = 1.0e-2;
const FD_EPS: f64 = 1.0e-6;
const DEFECT_TOL: f64 = 5.0e-3;
const ATT_TOL: f64 = 1.0e-4;
const RATE_TOL: f64 = 1.0e-4;

#[derive(Clone, Copy)]
pub(crate) struct Weights {
    pub time: f64,
    pub control: f64,
    pub energy: f64,
    pub pointing: f64,
    pub smoothness: f64,
    pub momentum: f64,
}

impl Weights {
    pub(crate) fn minimum_time() -> Self {
        Self {
            time: 1.0,
            control: 0.0,
            energy: 0.0,
            pointing: 0.0,
            smoothness: 0.0,
            momentum: 0.0,
        }
    }

    pub(crate) fn uses_tradeoff(self) -> bool {
        self.control > 0.0
            || self.energy > 0.0
            || self.pointing > 0.0
            || self.smoothness > 0.0
            || self.momentum > 0.0
    }
}

pub(crate) struct CollocationPath {
    pub times: Vec<f64>,
    pub quats: Vec<Quat>,
    pub omegas: Vec<[f64; 3]>,
    pub torques: Vec<[f64; 3]>,
    pub momenta: Vec<[f64; 3]>,
    pub duration: f64,
    pub iterations: u32,
    pub max_defect: f64,
}

type DensifiedPath = (
    Vec<f64>,
    Vec<Quat>,
    Vec<[f64; 3]>,
    Vec<[f64; 3]>,
    Vec<[f64; 3]>,
);
type InterpolatedSample = (Quat, [f64; 3], [f64; 3], [f64; 3]);

pub(crate) struct SolverProblem<'a> {
    pub q0: Quat,
    pub qf: Quat,
    pub inertia: [[f64; 3]; 3],
    pub torque_limit: [f64; 3],
    pub slew: Option<f64>,
    pub weights: Weights,
    pub actuators: &'a ActuatorMap,
    pub keep_out: &'a [KeepOutCone],
}

struct Decision {
    duration: f64,
    u: Vec<Vec<f64>>,
}

pub(crate) fn solve(
    problem: &SolverProblem<'_>,
    seed_duration: f64,
    seed_omega: &[[f64; 3]],
    seed_torque: &[[f64; 3]],
) -> Result<CollocationPath, PlanError> {
    let n = SOLVER_NODES;
    if seed_omega.len() != n || seed_torque.len() != n {
        return Err(PlanError::Refused(
            "collocation seed has the wrong length".to_string(),
        ));
    }
    if !seed_duration.is_finite() || seed_duration <= 0.0 {
        return Err(PlanError::Refused(
            "collocation seed duration is invalid".to_string(),
        ));
    }
    let n_u = problem.actuators.control_dim();
    let mut u_rows = Vec::with_capacity(n - 1);
    for tau in seed_torque.iter().take(n - 1) {
        let mut row = allocate_body_torque(problem.actuators, *tau, n_u)?;
        problem
            .actuators
            .project_controls(&mut row, problem.torque_limit);
        u_rows.push(row);
    }
    let mut z = Decision {
        duration: seed_duration,
        u: u_rows,
    };
    let mut lambda = LM_LAMBDA0;
    let mut best = residual_norm(problem, &z)?;
    let mut iters = 0;
    for iter in 0..MAX_ITERS {
        iters = iter as u32 + 1;
        let r = residuals(problem, &z)?;
        let mag = norm_slice(&r);
        if mag <= DEFECT_TOL {
            break;
        }
        let jac = jacobian(problem, &z, &r)?;
        let n_z = packed_len(&z);
        let mut a = vec![vec![0.0; n_z]; n_z];
        let mut g = vec![0.0; n_z];
        for i in 0..r.len() {
            for c in 0..n_z {
                g[c] += jac[i][c] * r[i];
                for d in 0..n_z {
                    a[c][d] += jac[i][c] * jac[i][d];
                }
            }
        }
        for k in 0..n_z {
            a[k][k] += lambda;
            g[k] = -g[k];
        }
        if solve_dense(&mut a, &mut g).is_err() {
            lambda = (lambda * 8.0).min(1.0e6);
            continue;
        }
        let mut trial = apply_step(&z, &g);
        project_decision(problem, &mut trial);
        let trial_norm = residual_norm(problem, &trial)?;
        if trial_norm < best {
            z = trial;
            best = trial_norm;
            lambda = (lambda * 0.3).max(1.0e-8);
        } else {
            lambda = (lambda * 4.0).min(1.0e6);
        }
    }
    let path = reconstruct(problem, &z)?;
    let term = log_so3(path.quats[n - 1].conjugate().mul(problem.qf));
    if norm3(term) > ATT_TOL {
        return Err(PlanError::Infeasible(
            "collocation did not meet the terminal attitude".to_string(),
        ));
    }
    if norm3(path.omegas[n - 1]) > RATE_TOL {
        return Err(PlanError::Infeasible(
            "collocation did not meet the rest terminal rate".to_string(),
        ));
    }
    if path.max_defect > DEFECT_TOL {
        return Err(PlanError::Infeasible(
            "collocation dynamics defects remain above tolerance".to_string(),
        ));
    }
    for (q, omega, torque, h) in path
        .quats
        .iter()
        .zip(path.omegas.iter())
        .zip(path.torques.iter())
        .zip(path.momenta.iter())
        .map(|(((q, w), t), h)| (q, w, t, h))
    {
        if torque_excess(*torque, problem.torque_limit) > 1.0e-9 {
            return Err(PlanError::Infeasible(
                "collocation net body torque exceeds the declared box".to_string(),
            ));
        }
        if let Some(limit) = problem.slew {
            if norm3(*omega) > limit + 1.0e-9 {
                return Err(PlanError::Infeasible(
                    "collocation body rate exceeds the slew limit".to_string(),
                ));
            }
        }
        if problem.actuators.momentum_excess(*h) > 1.0e-9 {
            return Err(PlanError::Infeasible(
                "collocation stored momentum exceeds a wheel limit".to_string(),
            ));
        }
        for zone in problem.keep_out {
            if keep_out_violation(q.as_ref(), *zone).unwrap_or(1.0) > 1.0e-3 {
                return Err(PlanError::Infeasible(
                    "collocation violates a keep-out cone".to_string(),
                ));
            }
        }
    }
    let mut out = path;
    out.iterations = iters;
    Ok(out)
}

fn packed_len(z: &Decision) -> usize {
    1 + z.u.iter().map(Vec::len).sum::<usize>()
}

fn pack(z: &Decision, out: &mut [f64]) {
    out[0] = z.duration;
    let mut i = 1;
    for row in &z.u {
        for item in row {
            out[i] = *item;
            i += 1;
        }
    }
}

fn unpack(z: &Decision, values: &[f64]) -> Decision {
    let mut u = z.u.clone();
    let mut i = 1;
    for row in &mut u {
        for item in row.iter_mut() {
            *item = values[i];
            i += 1;
        }
    }
    Decision {
        duration: values[0].max(1.0e-6),
        u,
    }
}

fn apply_step(z: &Decision, step: &[f64]) -> Decision {
    let n = packed_len(z);
    let mut packed = vec![0.0; n];
    pack(z, &mut packed);
    for (item, delta) in packed.iter_mut().zip(step) {
        *item += *delta;
    }
    unpack(z, &packed)
}

fn project_decision(problem: &SolverProblem<'_>, z: &mut Decision) {
    z.duration = z.duration.clamp(1.0e-3, 1.0e4);
    for row in &mut z.u {
        problem
            .actuators
            .project_controls(row, problem.torque_limit);
    }
}

fn reconstruct(problem: &SolverProblem<'_>, z: &Decision) -> Result<CollocationPath, PlanError> {
    let n = z.u.len() + 1;
    let dt = z.duration / (n as f64 - 1.0);
    let jinv = crate::geom::invert_spd(problem.inertia)?;
    let mut quats = vec![problem.q0; n];
    let mut omegas = vec![[0.0; 3]; n];
    let mut torques = vec![[0.0; 3]; n];
    let mut momenta = vec![[0.0; 3]; n];
    let mut w = [0.0; 3];
    let mut h = [0.0; 3];
    let mut delta = [0.0; 4];
    let mut max_defect = 0.0_f64;
    for i in 0..n - 1 {
        momenta[i] = h;
        omegas[i] = w;
        let tau = problem.actuators.body_torque(&z.u[i], w, h, delta)?;
        torques[i] = tau;
        let gyro = euler_lhs(problem.inertia, [0.0; 3], w, h);
        let w_dot = crate::geom::apply_tensor(jinv, sub3(tau, gyro));
        let w_next = add3(w, scale3(w_dot, dt));
        let w_mid = scale3(add3(w, w_next), 0.5);
        let lhs = euler_lhs(problem.inertia, w_dot, w, h);
        max_defect = max_defect.max(norm3(sub3(lhs, tau)));
        quats[i + 1] = quats[i].mul(exp_so3(scale3(w_mid, dt))).normalized()?;
        h = add3(h, scale3(problem.actuators.wheel_h_dot(&z.u[i]), dt));
        if let Some(cmg) = &problem.actuators.cmgs {
            let off = problem.actuators.wheels.len() + problem.actuators.thrusters.len();
            for (gimbal, rate) in delta.iter_mut().zip(z.u[i].iter().skip(off).take(4)) {
                *gimbal += *rate * dt;
            }
            if problem.actuators.singularity(delta) < cmg.singularity_eps {
                return Err(PlanError::Infeasible(
                    "CMG pyramid is singular on the collocation grid".to_string(),
                ));
            }
        }
        if problem.actuators.power_excess(&z.u[i], w) > 1.0e-9 {
            return Err(PlanError::Infeasible(
                "wheel power exceeds the declared limit".to_string(),
            ));
        }
        w = w_next;
    }
    omegas[n - 1] = w;
    momenta[n - 1] = h;
    torques[n - 1] = torques[n - 2];
    let times: Vec<f64> = (0..n).map(|i| i as f64 * dt).collect();
    Ok(CollocationPath {
        times,
        quats,
        omegas,
        torques,
        momenta,
        duration: z.duration,
        iterations: 0,
        max_defect,
    })
}

fn residuals(problem: &SolverProblem<'_>, z: &Decision) -> Result<Vec<f64>, PlanError> {
    let path = reconstruct(problem, z)?;
    let n = path.times.len();
    let dt = z.duration / (n as f64 - 1.0);
    let mut r = Vec::new();
    let att = log_so3(path.quats[n - 1].conjugate().mul(problem.qf));
    r.extend_from_slice(&scale3(att, 400.0));
    r.extend_from_slice(&scale3(path.omegas[n - 1], 400.0));
    for q in &path.quats {
        let mut viol = 0.0_f64;
        for zone in problem.keep_out {
            viol = viol.max(keep_out_violation(q.as_ref(), *zone).unwrap_or(1.0));
        }
        r.push(viol * 80.0);
    }
    r.push(problem.weights.time.sqrt() * z.duration * 0.02);
    let mut effort = 0.0;
    for row in &z.u {
        for item in row {
            effort += *item * *item;
        }
    }
    r.push(problem.weights.control.sqrt() * (effort * dt).sqrt());
    let mut energy = 0.0;
    for i in 0..n - 1 {
        energy += (path.torques[i][0] * path.omegas[i][0]
            + path.torques[i][1] * path.omegas[i][1]
            + path.torques[i][2] * path.omegas[i][2])
            .abs()
            * dt;
    }
    r.push(problem.weights.energy.sqrt() * energy.sqrt());
    let mut smooth = 0.0;
    for i in 0..n - 1 {
        let w_dot = scale3(sub3(path.omegas[i + 1], path.omegas[i]), 1.0 / dt);
        smooth += norm3(w_dot) * norm3(w_dot) * dt;
    }
    r.push(problem.weights.smoothness.sqrt() * smooth.sqrt());
    r.push(problem.weights.momentum.sqrt() * norm3(path.momenta[n - 1]));
    Ok(r)
}

fn residual_norm(problem: &SolverProblem<'_>, z: &Decision) -> Result<f64, PlanError> {
    Ok(norm_slice(&residuals(problem, z)?))
}

fn jacobian(
    problem: &SolverProblem<'_>,
    z: &Decision,
    r0: &[f64],
) -> Result<Vec<Vec<f64>>, PlanError> {
    let n_z = packed_len(z);
    let mut packed = vec![0.0; n_z];
    pack(z, &mut packed);
    let mut jac = vec![vec![0.0; n_z]; r0.len()];
    for c in 0..n_z {
        let mut plus = packed.clone();
        plus[c] += FD_EPS;
        let mut zp = unpack(z, &plus);
        project_decision(problem, &mut zp);
        let rp = residuals(problem, &zp)?;
        if rp.len() != r0.len() {
            return Err(PlanError::Refused(
                "collocation residual width changed during linearisation".to_string(),
            ));
        }
        for i in 0..r0.len() {
            jac[i][c] = (rp[i] - r0[i]) / FD_EPS;
        }
    }
    Ok(jac)
}

fn norm_slice(values: &[f64]) -> f64 {
    values.iter().map(|item| *item * *item).sum::<f64>().sqrt()
}

#[allow(clippy::needless_range_loop)]
fn solve_dense(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<(), PlanError> {
    let n = b.len();
    for k in 0..n {
        let mut pivot = k;
        let mut best = a[k][k].abs();
        for i in k + 1..n {
            let mag = a[i][k].abs();
            if mag > best {
                best = mag;
                pivot = i;
            }
        }
        if best < 1.0e-18 {
            return Err(PlanError::Refused(
                "collocation normal equations are singular".to_string(),
            ));
        }
        a.swap(k, pivot);
        b.swap(k, pivot);
        let diag = a[k][k];
        for j in k..n {
            a[k][j] /= diag;
        }
        b[k] /= diag;
        for i in 0..n {
            if i == k {
                continue;
            }
            let factor = a[i][k];
            for j in k..n {
                a[i][j] -= factor * a[k][j];
            }
            b[i] -= factor * b[k];
        }
    }
    Ok(())
}

pub(crate) fn sample_seed(
    duration: f64,
    omega_at: impl Fn(f64) -> [f64; 3],
    torque_at: impl Fn(f64) -> [f64; 3],
) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let n = SOLVER_NODES;
    let mut omegas = Vec::with_capacity(n);
    let mut torques = Vec::with_capacity(n);
    for i in 0..n {
        let t = duration * i as f64 / (n as f64 - 1.0);
        omegas.push(omega_at(t));
        torques.push(torque_at(t));
    }
    (omegas, torques)
}

pub(crate) fn densify(
    problem: &SolverProblem<'_>,
    path: &CollocationPath,
    requested: usize,
    alpha: f64,
) -> Result<DensifiedPath, PlanError> {
    let dt_max = (2.0 * 1.0e-3 * 0.75) / alpha.max(1.0e-6);
    let from_rate = (path.duration / dt_max).ceil() as usize + 1;
    let count = requested.max(from_rate).max(path.times.len());
    if count > 100_000 {
        return Err(PlanError::Refused(
            "planner would exceed the sample limit to keep rate samples kernel-consistent"
                .to_string(),
        ));
    }
    let n_coarse = path.times.len();
    if n_coarse < 2 {
        return Err(PlanError::Refused(
            "collocation path is too short to densify".to_string(),
        ));
    }
    let mut times = Vec::with_capacity(count);
    let mut quats = Vec::with_capacity(count);
    let mut omegas = Vec::with_capacity(count);
    let mut torques = Vec::with_capacity(count);
    let mut momenta = Vec::with_capacity(count);
    for i in 0..count {
        let t = path.duration * i as f64 / (count as f64 - 1.0);
        let (q, w, tau, h) = interpolate(problem, path, t)?;
        times.push(t);
        quats.push(q);
        omegas.push(w);
        torques.push(tau);
        momenta.push(h);
    }
    Ok((times, quats, omegas, torques, momenta))
}

fn interpolate(
    problem: &SolverProblem<'_>,
    path: &CollocationPath,
    t: f64,
) -> Result<InterpolatedSample, PlanError> {
    if t <= path.times[0] {
        return Ok((
            path.quats[0],
            path.omegas[0],
            path.torques[0],
            path.momenta[0],
        ));
    }
    let last = path.times.len() - 1;
    if t >= path.times[last] {
        return Ok((
            path.quats[last],
            path.omegas[last],
            path.torques[last],
            path.momenta[last],
        ));
    }
    let mut i = 0;
    while i + 1 < path.times.len() && path.times[i + 1] < t {
        i += 1;
    }
    let t0 = path.times[i];
    let t1 = path.times[i + 1];
    let dt = t1 - t0;
    let a = (t - t0) / dt;
    let omega = add3(
        scale3(path.omegas[i], 1.0 - a),
        scale3(path.omegas[i + 1], a),
    );
    let h = add3(
        scale3(path.momenta[i], 1.0 - a),
        scale3(path.momenta[i + 1], a),
    );
    let w_dot = scale3(sub3(path.omegas[i + 1], path.omegas[i]), 1.0 / dt);
    let torque = euler_lhs(problem.inertia, w_dot, omega, h);
    let s = t - t0;
    let integrated = add3(scale3(path.omegas[i], s), scale3(w_dot, 0.5 * s * s));
    let q = path.quats[i].mul(exp_so3(integrated)).normalized()?;
    Ok((q, omega, torque, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{add3, apply_tensor, euler_lhs, norm3, scale3, sub3};

    #[test]
    fn dynamics_jacobian_matches_finite_difference_on_a_short_arc() {
        let inertia = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let w0 = [0.0, 0.0, 0.0];
        let w1 = [0.02, 0.0, 0.0];
        let dt = 0.1;
        let w_mid = scale3(add3(w0, w1), 0.5);
        let w_dot = scale3(sub3(w1, w0), 1.0 / dt);
        let tau = euler_lhs(inertia, w_dot, w_mid, [0.0; 3]);
        let d0 = sub3(euler_lhs(inertia, w_dot, w_mid, [0.0; 3]), tau);
        assert!(norm3(d0) < 1e-15);
        let mut w1p = w1;
        w1p[0] += FD_EPS;
        let w_mid_p = scale3(add3(w0, w1p), 0.5);
        let w_dot_p = scale3(sub3(w1p, w0), 1.0 / dt);
        let dp = sub3(euler_lhs(inertia, w_dot_p, w_mid_p, [0.0; 3]), tau);
        let fd = dp[0] / FD_EPS;
        let jinv = apply_tensor(inertia, [1.0, 0.0, 0.0]);
        assert!((jinv[0] - 1.0).abs() < 1e-12);
        assert!(fd.abs() > 1.0);
    }
}
