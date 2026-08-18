use crate::GuideError;
use libm::sqrt;

#[derive(Debug, Clone, Copy)]
pub struct TwoBody {
    pub n: f64,
    pub phase: f64,
    pub mu: f64,
    pub earth_radius_m: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub r: [f64; 3],
    pub v: [f64; 3],
    pub nadir: [f64; 3],
    pub sun: [f64; 3],
    pub field_t: [f64; 3],
    pub eclipse: bool,
}

impl TwoBody {
    pub fn validated(self) -> Result<Self, GuideError> {
        if self.n <= 0.0
            || !self.n.is_finite()
            || self.n > 0.1
            || !self.phase.is_finite()
            || self.mu <= 0.0
            || !self.mu.is_finite()
            || self.earth_radius_m <= 0.0
            || !self.earth_radius_m.is_finite()
        {
            return Err(GuideError::Refused(
                "two-body geometry parameters must be finite and positive within compiled bounds"
                    .to_string(),
            ));
        }
        Ok(self)
    }

    pub fn geometry(self, t: f64) -> Result<Geometry, GuideError> {
        let body = self.validated()?;
        if !t.is_finite() {
            return Err(GuideError::Refused(
                "two-body epoch is not finite".to_string(),
            ));
        }
        let a = libm::cbrt(body.mu / (body.n * body.n));
        let theta = body.phase + body.n * t;
        let (s, c) = (theta.sin(), theta.cos());
        let r = [a * c, a * s, 0.0];
        let v = [-body.n * a * s, body.n * a * c, 0.0];
        let rn = sqrt(r[0] * r[0] + r[1] * r[1] + r[2] * r[2]);
        let nadir = [-r[0] / rn, -r[1] / rn, -r[2] / rn];
        let sun = [1.0, 0.0, 0.0];
        let scale = 3.0e-5 * (body.earth_radius_m / rn).powi(3);
        let field_t = [scale * s, 0.0, scale * c];
        let r_dot_sun = r[0] * sun[0] + r[1] * sun[1] + r[2] * sun[2];
        let cross_n = sqrt(
            (r[1] * sun[2] - r[2] * sun[1]).powi(2)
                + (r[2] * sun[0] - r[0] * sun[2]).powi(2)
                + (r[0] * sun[1] - r[1] * sun[0]).powi(2),
        );
        let eclipse = r_dot_sun < 0.0 && cross_n < body.earth_radius_m;
        Ok(Geometry {
            r,
            v,
            nadir,
            sun,
            field_t,
            eclipse,
        })
    }
}
