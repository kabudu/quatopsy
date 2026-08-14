use std::f64::consts::{FRAC_1_SQRT_2, PI};

use quatopsy_core::math::{Quaternion, quotient_angle};
use quatopsy_oracle::{
    RefQuat, geodesic_angle, high_precision_dot, matrices_close, rotation_matrix,
};

fn q(w: f64, x: f64, y: f64, z: f64) -> Quaternion {
    Quaternion::new(w, x, y, z)
}

fn rq(value: Quaternion) -> RefQuat {
    RefQuat {
        w: value.w,
        x: value.x,
        y: value.y,
        z: value.z,
    }
}

#[test]
fn quotient_angle_matches_independent_matrix_geodesic() {
    let cases = [
        q(1.0, 0.0, 0.0, 0.0),
        q(-1.0, 0.0, 0.0, 0.0),
        q(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2),
        q(0.0, 1.0, 0.0, 0.0),
        q(0.5, 0.5, 0.5, 0.5).normalized().unwrap(),
    ];
    for p in cases {
        for r in cases {
            let kernel = quotient_angle(p, r);
            let oracle = geodesic_angle(rq(p), rq(r));
            assert!(
                (kernel - oracle).abs() < 1e-9,
                "kernel {kernel} oracle {oracle} for {p:?} {r:?}"
            );
        }
    }
}

#[test]
fn antipodal_samples_share_oracle_matrices() {
    let p = q(0.6, 0.8, 0.0, 0.0).normalized().unwrap();
    let n = p.negate();
    assert!(matrices_close(
        rotation_matrix(rq(p)),
        rotation_matrix(rq(n)),
        1e-12
    ));
    assert!(quotient_angle(p, n) < 1e-12);
}

#[test]
fn high_precision_dot_agrees_on_unit_pairs() {
    let p = q(0.9238795325112867, 0.0, 0.0, 0.3826834323650898);
    let r = q(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2);
    let kernel = p.dot(r);
    let oracle = high_precision_dot(rq(p), rq(r));
    assert!((kernel - oracle).abs() < 1e-12);
}

#[test]
fn near_pi_dot_is_numerically_zero_in_both_encodings() {
    let p = q(1.0, 0.0, 0.0, 0.0);
    let r = q(0.0, 1.0, 0.0, 0.0);
    assert!(p.dot(r).abs() < 1e-15);
    assert!(high_precision_dot(rq(p), rq(r)).abs() < 1e-15);
    assert!((quotient_angle(p, r) - PI).abs() < 1e-12);
}
