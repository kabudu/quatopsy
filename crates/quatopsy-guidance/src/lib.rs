//! Time-tagged attitude guidance. Never assigns a report `result`.

mod orbit;
mod profile;

pub use orbit::{Geometry, TwoBody};
pub use profile::{
    GuidanceMode, GuideError, MAX_PROFILE_SAMPLES, Profile, ProfileSample, SunPoint,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_setpoint_holds_zero_rate() {
        let profile = Profile::setpoint([1.0, 0.0, 0.0, 0.0], 8.0).unwrap();
        let sample = profile.sample_at(1.0).unwrap();
        assert_eq!(sample.omega, [0.0; 3]);
        assert_eq!(sample.alpha, [0.0; 3]);
        assert!(profile.terminal_rest(sample.q, sample.omega, [1.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn interpolated_profile_has_nonzero_rate() {
        let a = std::f64::consts::FRAC_1_SQRT_2;
        let rate = std::f64::consts::FRAC_PI_2;
        let profile = Profile::from_samples(vec![
            ProfileSample {
                t: 0.0,
                q: [1.0, 0.0, 0.0, 0.0],
                omega: [rate, 0.0, 0.0],
                alpha: [0.0; 3],
            },
            ProfileSample {
                t: 1.0,
                q: [a, a, 0.0, 0.0],
                omega: [rate, 0.0, 0.0],
                alpha: [0.0; 3],
            },
        ])
        .unwrap();
        let mid = profile.sample_at(0.5).unwrap();
        assert!(mid.omega[0].abs() > 0.5);
        let residual = quatopsy_oracle::reference_rate_residual(
            quatopsy_oracle::RefQuat {
                w: 1.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            quatopsy_oracle::RefQuat {
                w: mid.q[0],
                x: mid.q[1],
                y: mid.q[2],
                z: mid.q[3],
            },
            mid.omega,
            0.5,
        )
        .unwrap();
        assert!(residual < 0.2);
    }

    #[test]
    fn two_body_field_and_nadir_change_with_time() {
        let orbit = TwoBody {
            n: 0.001,
            phase: 0.0,
            mu: 3.986e14,
            earth_radius_m: 6.371e6,
        };
        let a = orbit.geometry(0.0).unwrap();
        let b = orbit.geometry(1000.0).unwrap();
        assert!((a.nadir[0] - b.nadir[0]).abs() > 1e-4);
        assert!(
            (a.field_t[0] - b.field_t[0]).abs() > 1e-8
                || (a.field_t[2] - b.field_t[2]).abs() > 1e-8
        );
    }

    #[test]
    fn plan_csv_ingest_derives_alpha() {
        let csv =
            "t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz\n0,1,0,0,0,0,0,0,0,0,0\n1,1,0,0,0,0.1,0,0,0,0,0\n";
        let profile = Profile::from_plan_csv(csv).unwrap();
        let sample = profile.sample_at(0.5).unwrap();
        assert!((sample.alpha[0] - 0.1).abs() < 1e-12);
    }

    #[test]
    fn named_sun_point_can_violate() {
        let mut profile = Profile::setpoint([1.0, 0.0, 0.0, 0.0], 1.0).unwrap();
        profile.sun_point = Some(SunPoint {
            body_axis: [1.0, 0.0, 0.0],
            min_angle_rad: 1.0,
        });
        assert!(
            profile
                .sun_violation([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
                .unwrap()
        );
        assert!(
            !profile
                .sun_violation([1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
                .unwrap()
        );
    }

    #[test]
    fn non_finite_rate_is_refused() {
        let err = Profile::from_samples(vec![
            ProfileSample {
                t: 0.0,
                q: [1.0, 0.0, 0.0, 0.0],
                omega: [f64::NAN, 0.0, 0.0],
                alpha: [0.0; 3],
            },
            ProfileSample {
                t: 1.0,
                q: [1.0, 0.0, 0.0, 0.0],
                omega: [0.0; 3],
                alpha: [0.0; 3],
            },
        ])
        .unwrap_err();
        assert!(err.to_string().contains("must be finite"));
    }

    #[test]
    fn terminal_rest_contract_is_enforced() {
        let profile = Profile::from_samples(vec![
            ProfileSample {
                t: 0.0,
                q: [1.0, 0.0, 0.0, 0.0],
                omega: [0.0; 3],
                alpha: [0.0; 3],
            },
            ProfileSample {
                t: 1.0,
                q: [1.0, 0.0, 0.0, 0.0],
                omega: [0.1, 0.0, 0.0],
                alpha: [0.0; 3],
            },
        ])
        .unwrap();
        assert!(
            profile
                .validate_terminal_rest([1.0, 0.0, 0.0, 0.0], 1e-4, 1e-4)
                .is_err()
        );
    }
}
