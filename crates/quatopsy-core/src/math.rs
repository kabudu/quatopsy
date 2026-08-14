//! IEEE 754 binary64 quaternion primitives with documented operation order.
//!
//! Transcendental operations use the `libm` software implementations so that
//! macOS and Linux-like hosts share a numeric path.

use libm::{acos, fabs, sqrt};
use quatopsy_schema::PI_TIE_ABS_DOT;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    pub fn is_finite(self) -> bool {
        self.w.is_finite() && self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Squared Euclidean norm with component order w, x, y, z.
    pub fn norm_squared(self) -> f64 {
        let ww = self.w * self.w;
        let xx = self.x * self.x;
        let yy = self.y * self.y;
        let zz = self.z * self.z;
        ww + xx + yy + zz
    }

    pub fn norm(self) -> f64 {
        sqrt(self.norm_squared())
    }

    /// Dot product with component order w, x, y, z.
    pub fn dot(self, other: Self) -> f64 {
        let w = self.w * other.w;
        let x = self.x * other.x;
        let y = self.y * other.y;
        let z = self.z * other.z;
        w + x + y + z
    }

    pub fn negate(self) -> Self {
        Self {
            w: -self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    pub fn scale(self, factor: f64) -> Self {
        Self {
            w: self.w * factor,
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    pub fn normalized(self) -> Option<Self> {
        if !self.is_finite() {
            return None;
        }
        let n = self.norm();
        if n == 0.0 || !n.is_finite() {
            None
        } else {
            Some(self.scale(1.0 / n))
        }
    }

    pub fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Hamilton product with factor order `self * other`.
    pub fn hamilton_mul(self, other: Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Rotate a vector by a unit quaternion: `q v q*`.
    pub fn rotate_vector(self, vector: [f64; 3]) -> [f64; 3] {
        let qv = Self::new(0.0, vector[0], vector[1], vector[2]);
        let rotated = self.hamilton_mul(qv).hamilton_mul(self.conjugate());
        [rotated.x, rotated.y, rotated.z]
    }
}

/// Clamp a dot product to `[-1, 1]` after a proven unit-domain calculation.
pub fn clamp_unit_dot(dot: f64) -> f64 {
    dot.clamp(-1.0, 1.0)
}

/// Physical geodesic angle on `SO(3)`: `2 acos(|p · q|)` for unit quaternions.
pub fn quotient_angle(p: Quaternion, q: Quaternion) -> f64 {
    let dotted = clamp_unit_dot(fabs(p.dot(q)));
    2.0 * acos(dotted)
}

/// Covering angle on `S^3`: `2 acos(p · q)` for unit quaternions, without sign invariance.
pub fn covering_angle(p: Quaternion, q: Quaternion) -> f64 {
    let dotted = clamp_unit_dot(p.dot(q));
    2.0 * acos(dotted)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiftDecision {
    pub lifted: Quaternion,
    pub flipped: bool,
    pub near_pi: bool,
    pub unit_dot: f64,
}

/// Choose the sign of `next_unit` that maximises the dot product with `prev_lifted`.
///
/// At an exact or tolerance-defined tie, the raw sign is retained.
pub fn lift_next(prev_lifted: Quaternion, next_unit: Quaternion) -> LiftDecision {
    let unit_dot = prev_lifted.dot(next_unit);
    if fabs(unit_dot) <= PI_TIE_ABS_DOT {
        LiftDecision {
            lifted: next_unit,
            flipped: false,
            near_pi: true,
            unit_dot,
        }
    } else if unit_dot < 0.0 {
        LiftDecision {
            lifted: next_unit.negate(),
            flipped: true,
            near_pi: false,
            unit_dot,
        }
    } else {
        LiftDecision {
            lifted: next_unit,
            flipped: false,
            near_pi: false,
            unit_dot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covering_angle_detects_long_way_to_same_orientation() {
        let identity = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let short = Quaternion::new(s, 0.0, 0.0, s);
        let long = short.negate();
        assert!((quotient_angle(identity, short) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((quotient_angle(identity, long) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(covering_angle(identity, long) > covering_angle(identity, short) + 1.0);
    }

    #[test]
    fn identity_rotation_leaves_vector() {
        let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        assert_eq!(q.rotate_vector([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn antipodes_have_zero_physical_angle() {
        let p = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let q = Quaternion::new(-1.0, 0.0, 0.0, 0.0);
        assert!(quotient_angle(p, q) < 1.0e-15);
    }

    #[test]
    fn lift_flips_negative_dot_and_retains_tie() {
        let prev = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let flipped = lift_next(prev, Quaternion::new(-1.0, 0.0, 0.0, 0.0));
        assert!(flipped.flipped);
        assert!(!flipped.near_pi);
        assert_eq!(flipped.lifted.w, 1.0);

        let tie = lift_next(prev, Quaternion::new(0.0, 1.0, 0.0, 0.0));
        assert!(tie.near_pi);
        assert!(!tie.flipped);
        assert_eq!(tie.lifted.x, 1.0);
    }
}
