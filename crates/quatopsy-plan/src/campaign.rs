//! Deterministic open-loop campaigns under inertia error and saturation.

use crate::geom::{Quat, add3, apply_tensor, euler_lhs, exp_so3, invert_spd, scale3, sub3};
use crate::scvx::CollocationPath;
use serde::Deserialize;
use serde::Serialize;

const MAX_TRIALS: u32 = 32;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct CampaignSpec {
    pub trials: u32,
    #[serde(default = "default_sigma")]
    pub inertia_rel_sigma: f64,
    #[serde(default = "default_true")]
    pub saturate_actuators: bool,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_sigma() -> f64 {
    0.05
}

fn default_true() -> bool {
    true
}

fn default_seed() -> u64 {
    1
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct CampaignReport {
    pub trials: u32,
    pub keep_out_violations: u32,
    pub terminal_attitude_fail: u32,
    pub saturation_events: u32,
    pub notes: &'static str,
}

pub(crate) fn run_campaign(
    spec: &CampaignSpec,
    inertia: [[f64; 3]; 3],
    torque_limit: [f64; 3],
    path: &CollocationPath,
    q_final: Quat,
    keep_out: &[quatopsy_oracle::KeepOutCone],
) -> Result<CampaignReport, crate::PlanError> {
    if spec.trials == 0 || spec.trials > MAX_TRIALS {
        return Err(crate::PlanError::Refused(
            "campaign trials must be in 1..=32".to_string(),
        ));
    }
    if !spec.inertia_rel_sigma.is_finite()
        || spec.inertia_rel_sigma < 0.0
        || spec.inertia_rel_sigma > 0.5
    {
        return Err(crate::PlanError::Refused(
            "campaign inertia_rel_sigma must be in [0, 0.5]".to_string(),
        ));
    }
    let mut rng = spec.seed | 1;
    let mut keep_out_violations = 0;
    let mut terminal_attitude_fail = 0;
    let mut saturation_events = 0;
    for _ in 0..spec.trials {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let perturbed = perturb_inertia(inertia, spec.inertia_rel_sigma, rng);
        let Ok(jinv) = invert_spd(perturbed) else {
            terminal_attitude_fail += 1;
            continue;
        };
        let mut q = path.quats[0];
        let mut w = path.omegas[0];
        let mut sat = false;
        let mut keep = false;
        for i in 0..path.times.len() - 1 {
            let dt = path.times[i + 1] - path.times[i];
            let mut tau = path.torques[i];
            if spec.saturate_actuators {
                for k in 0..3 {
                    let clipped = tau[k].clamp(-torque_limit[k], torque_limit[k]);
                    if (clipped - tau[k]).abs() > 1.0e-12 {
                        sat = true;
                    }
                    tau[k] = clipped;
                }
            }
            let lhs = euler_lhs(perturbed, [0.0; 3], w, path.momenta[i]);
            let gyro = lhs;
            let w_dot = apply_tensor(jinv, sub3(tau, gyro));
            w = add3(w, scale3(w_dot, dt));
            q = q.mul(exp_so3(scale3(w, dt))).normalized()?;
            for zone in keep_out {
                if quatopsy_oracle::keep_out_violation(q.as_ref(), *zone).unwrap_or(1.0) > 1.0e-3 {
                    keep = true;
                }
            }
        }
        let att = crate::geom::log_so3(q.conjugate().mul(q_final));
        if crate::geom::norm3(att) > 0.05 {
            terminal_attitude_fail += 1;
        }
        if keep {
            keep_out_violations += 1;
        }
        if sat {
            saturation_events += 1;
        }
    }
    Ok(CampaignReport {
        trials: spec.trials,
        keep_out_violations,
        terminal_attitude_fail,
        saturation_events,
        notes: "Open-loop perturbed-model simulation. Not a report result. Not a robustness certificate.",
    })
}

fn perturb_inertia(j: [[f64; 3]; 3], sigma: f64, seed: u64) -> [[f64; 3]; 3] {
    let mut out = j;
    let noise = |k: u32| {
        let shifted = seed.wrapping_add(u64::from(k).wrapping_mul(0x9E3779B97F4A7C15));
        let unit = (shifted >> 11) as f64 / ((1u64 << 53) as f64);
        (unit * 2.0 - 1.0) * sigma
    };
    for (i, row) in out.iter_mut().enumerate() {
        row[i] *= 1.0 + noise(i as u32);
        if row[i] <= 1.0e-9 {
            row[i] = 1.0e-9;
        }
    }
    let p01 = j[0][1] + noise(3) * (j[0][0].abs() + j[1][1].abs()) * 0.5;
    let p02 = j[0][2] + noise(4) * (j[0][0].abs() + j[2][2].abs()) * 0.5;
    let p12 = j[1][2] + noise(5) * (j[1][1].abs() + j[2][2].abs()) * 0.5;
    out[0][1] = p01;
    out[1][0] = p01;
    out[0][2] = p02;
    out[2][0] = p02;
    out[1][2] = p12;
    out[2][1] = p12;
    out
}
