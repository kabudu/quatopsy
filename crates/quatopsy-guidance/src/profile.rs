use libm::{acos, sin, sqrt};
use quatopsy_oracle::{KeepOutCone, RefQuat, keep_out_violation};
use thiserror::Error;

pub const MAX_PROFILE_SAMPLES: usize = 100_000;

#[derive(Debug, Error)]
pub enum GuideError {
    #[error("{0}")]
    Refused(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidanceMode {
    Slew,
    Track,
    Hold,
    Safe,
}

#[derive(Debug, Clone, Copy)]
pub struct SunPoint {
    pub body_axis: [f64; 3],
    pub min_angle_rad: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileSample {
    pub t: f64,
    pub q: [f64; 4],
    pub omega: [f64; 3],
    pub alpha: [f64; 3],
}

#[derive(Debug, Clone)]
pub struct Profile {
    samples: Vec<ProfileSample>,
    pub sun_point: Option<SunPoint>,
    pub keep_out: Vec<KeepOutCone>,
}

impl Profile {
    pub fn setpoint(q: [f64; 4], duration_s: f64) -> Result<Self, GuideError> {
        let q = normalize(q)?;
        Self::from_samples(vec![
            ProfileSample {
                t: 0.0,
                q,
                omega: [0.0; 3],
                alpha: [0.0; 3],
            },
            ProfileSample {
                t: duration_s.max(1.0e-9),
                q,
                omega: [0.0; 3],
                alpha: [0.0; 3],
            },
        ])
    }

    pub fn from_samples(mut samples: Vec<ProfileSample>) -> Result<Self, GuideError> {
        if !(2..=MAX_PROFILE_SAMPLES).contains(&samples.len()) {
            return Err(GuideError::Refused(format!(
                "guidance profile needs 2..={MAX_PROFILE_SAMPLES} samples"
            )));
        }
        for sample in &samples {
            if !sample.t.is_finite()
                || !sample.omega.iter().all(|value| value.is_finite())
                || !sample.alpha.iter().all(|value| value.is_finite())
            {
                return Err(GuideError::Refused(
                    "guidance time, angular rate, and acceleration must be finite".to_string(),
                ));
            }
        }
        samples.sort_by(|a, b| a.t.total_cmp(&b.t));
        for window in samples.windows(2) {
            if !window[1].t.is_finite() || window[1].t <= window[0].t {
                return Err(GuideError::Refused(
                    "guidance times must increase".to_string(),
                ));
            }
        }
        for sample in &mut samples {
            sample.q = normalize(sample.q)?;
        }
        for i in 1..samples.len() {
            if dot(samples[i - 1].q, samples[i].q) < 0.0 {
                samples[i].q = [
                    -samples[i].q[0],
                    -samples[i].q[1],
                    -samples[i].q[2],
                    -samples[i].q[3],
                ];
            }
            let dt = samples[i].t - samples[i - 1].t;
            samples[i - 1].alpha = [
                (samples[i].omega[0] - samples[i - 1].omega[0]) / dt,
                (samples[i].omega[1] - samples[i - 1].omega[1]) / dt,
                (samples[i].omega[2] - samples[i - 1].omega[2]) / dt,
            ];
        }
        let last = samples.len() - 1;
        samples[last].alpha = samples[last - 1].alpha;
        Ok(Self {
            samples,
            sun_point: None,
            keep_out: Vec::new(),
        })
    }

    pub fn from_plan_csv(csv: &str) -> Result<Self, GuideError> {
        let mut samples = Vec::new();
        for (index, line) in csv.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if index == 0 && line.starts_with('t') {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                return Err(GuideError::Refused(
                    "guidance CSV needs t,q,omega columns".to_string(),
                ));
            }
            let parsed: Result<Vec<f64>, _> =
                parts.iter().take(8).map(|item| item.parse()).collect();
            let row = parsed.map_err(|_| {
                GuideError::Refused("guidance CSV contains a non-numeric field".to_string())
            })?;
            samples.push(ProfileSample {
                t: row[0],
                q: [row[1], row[2], row[3], row[4]],
                omega: [row[5], row[6], row[7]],
                alpha: [0.0; 3],
            });
        }
        Self::from_samples(samples)
    }

    pub fn sample_at(&self, t: f64) -> Result<ProfileSample, GuideError> {
        if !t.is_finite() {
            return Err(GuideError::Refused(
                "guidance query time is not finite".to_string(),
            ));
        }
        if t <= self.samples[0].t {
            return Ok(self.samples[0]);
        }
        let last = *self.samples.last().unwrap();
        if t >= last.t {
            return Ok(last);
        }
        let upper = self.samples.partition_point(|sample| sample.t < t);
        let left = self.samples[upper - 1];
        let right = self.samples[upper];
        let span = right.t - left.t;
        let u = (t - left.t) / span;
        Ok(ProfileSample {
            t,
            q: slerp(left.q, right.q, u),
            omega: lerp3(left.omega, right.omega, u),
            alpha: left.alpha,
        })
    }

    pub fn validate_terminal_rest(
        &self,
        q_des: [f64; 4],
        attitude_tolerance_rad: f64,
        rate_tolerance_rad_s: f64,
    ) -> Result<(), GuideError> {
        let last = *self.samples.last().expect("validated profile is non-empty");
        if geodesic(last.q, normalize(q_des)?) > attitude_tolerance_rad
            || norm3(last.omega) > rate_tolerance_rad_s
        {
            return Err(GuideError::Refused(
                "guidance profile must terminate at q_desired with zero body rate".to_string(),
            ));
        }
        Ok(())
    }

    pub fn mode(&self, sample: ProfileSample, q_des: [f64; 4]) -> GuidanceMode {
        if self.terminal_rest(sample.q, sample.omega, q_des) {
            GuidanceMode::Hold
        } else if norm3(sample.omega) > 1.0e-3 {
            GuidanceMode::Slew
        } else {
            GuidanceMode::Track
        }
    }

    pub fn terminal_rest(&self, q: [f64; 4], omega: [f64; 3], q_des: [f64; 4]) -> bool {
        geodesic(q, q_des) < 0.05 && norm3(omega) < 0.05
    }

    pub fn sun_violation(&self, q: [f64; 4], sun: [f64; 3]) -> Result<bool, GuideError> {
        let Some(cone) = self.sun_point else {
            return Ok(false);
        };
        keep_out_violation(
            RefQuat {
                w: q[0],
                x: q[1],
                y: q[2],
                z: q[3],
            },
            KeepOutCone {
                body_axis: cone.body_axis,
                inertial_axis: sun,
                min_angle_rad: cone.min_angle_rad,
            },
        )
        .map(|value| value > 1.0e-3)
        .map_err(|err| GuideError::Refused(err.to_string()))
    }
}

fn normalize(q: [f64; 4]) -> Result<[f64; 4], GuideError> {
    let n = sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
    if !n.is_finite() || n < 1.0e-15 {
        return Err(GuideError::Refused(
            "guidance quaternion is near zero".to_string(),
        ));
    }
    Ok([q[0] / n, q[1] / n, q[2] / n, q[3] / n])
}

fn dot(a: [f64; 4], b: [f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

fn slerp(a: [f64; 4], b: [f64; 4], u: f64) -> [f64; 4] {
    let mut b = b;
    let mut cos_om = dot(a, b).clamp(-1.0, 1.0);
    if cos_om < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        cos_om = -cos_om;
    }
    if cos_om > 0.9995 {
        return normalize([
            a[0] + u * (b[0] - a[0]),
            a[1] + u * (b[1] - a[1]),
            a[2] + u * (b[2] - a[2]),
            a[3] + u * (b[3] - a[3]),
        ])
        .unwrap_or(a);
    }
    let omega = acos(cos_om);
    let so = sin(omega);
    let w0 = sin((1.0 - u) * omega) / so;
    let w1 = sin(u * omega) / so;
    [
        w0 * a[0] + w1 * b[0],
        w0 * a[1] + w1 * b[1],
        w0 * a[2] + w1 * b[2],
        w0 * a[3] + w1 * b[3],
    ]
}

fn lerp3(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    [
        a[0] + u * (b[0] - a[0]),
        a[1] + u * (b[1] - a[1]),
        a[2] + u * (b[2] - a[2]),
    ]
}

fn geodesic(a: [f64; 4], b: [f64; 4]) -> f64 {
    quatopsy_oracle::geodesic_angle(
        RefQuat {
            w: a[0],
            x: a[1],
            y: a[2],
            z: a[3],
        },
        RefQuat {
            w: b[0],
            x: b[1],
            y: b[2],
            z: b[3],
        },
    )
}

fn norm3(v: [f64; 3]) -> f64 {
    sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
}
