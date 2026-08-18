//! Deterministic robustness trials. Not a flight Monte Carlo certificate.

use serde::Deserialize;
use serde::Serialize;

pub(crate) const MAX_TRIALS: u32 = 16;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct CampaignSpec {
    pub trials: u32,
    #[serde(default = "default_sigma")]
    pub inertia_rel_sigma: f64,
    #[serde(default)]
    pub quat_noise: f64,
    #[serde(default)]
    pub rate_noise: f64,
    #[serde(default)]
    pub delay_s: f64,
    #[serde(default)]
    pub disturbance_nm: f64,
    #[serde(default)]
    pub actuator_fail_axis: Option<u8>,
    #[serde(default)]
    pub numerical_fault: bool,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_sigma() -> f64 {
    0.05
}

fn default_seed() -> u64 {
    1
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct CampaignReport {
    pub trials: u32,
    pub inhibit_events: u32,
    pub terminal_attitude_fail: u32,
    pub notes: &'static str,
}

pub(crate) fn validate(spec: &CampaignSpec) -> Result<(), crate::ControlError> {
    if spec.trials == 0 || spec.trials > MAX_TRIALS {
        return Err(crate::ControlError::Refused(
            "campaign trials must be in 1..=16".to_string(),
        ));
    }
    for (name, value, max) in [
        ("inertia_rel_sigma", spec.inertia_rel_sigma, 0.5),
        ("quat_noise", spec.quat_noise, 0.2),
        ("rate_noise", spec.rate_noise, 0.5),
        ("delay_s", spec.delay_s, 1.0),
        ("disturbance_nm", spec.disturbance_nm, 1.0),
    ] {
        if !value.is_finite() || value < 0.0 || value > max {
            return Err(crate::ControlError::Refused(format!(
                "campaign {name} must be in [0, {max}]"
            )));
        }
    }
    if let Some(axis) = spec.actuator_fail_axis
        && axis > 2
    {
        return Err(crate::ControlError::Refused(
            "campaign actuator_fail_axis must be 0, 1, or 2".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn next_rng(rng: u64) -> u64 {
    rng.wrapping_mul(6364136223846793005).wrapping_add(1)
}

pub(crate) fn unit_noise(seed: u64, lane: u32) -> f64 {
    let shifted = seed.wrapping_add(u64::from(lane).wrapping_mul(0x9E3779B97F4A7C15));
    let unit = (shifted >> 11) as f64 / ((1u64 << 53) as f64);
    unit * 2.0 - 1.0
}

pub(crate) fn perturb_inertia(j: [[f64; 3]; 3], sigma: f64, seed: u64) -> [[f64; 3]; 3] {
    let mut out = j;
    for (i, row) in out.iter_mut().enumerate() {
        row[i] *= 1.0 + unit_noise(seed, i as u32) * sigma;
        if row[i] <= 1.0e-9 {
            row[i] = 1.0e-9;
        }
    }
    out
}
