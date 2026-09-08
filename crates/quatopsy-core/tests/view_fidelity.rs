use quatopsy_core::{
    cancel::Cancel,
    ingest::Sample,
    math::Quaternion,
    view::{build_view, build_view_cancellable},
};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn sample(row: u64, time: i64, q: Quaternion) -> Sample {
    Sample {
        source_row: row,
        timestamp_ns: time,
        raw: q,
        commanded: None,
        omega: None,
        rotation_matrix: None,
        timestamp_finite: true,
        timestamp_overflow: false,
    }
}
#[test]
fn wide_time_geometry_preserves_exact_identity_and_rate() {
    let rows = [
        sample(
            2,
            -9_000_000_000_000_000_000,
            Quaternion::new(1.0, 0.0, 0.0, 0.0),
        ),
        sample(
            3,
            9_000_000_000_000_000_000,
            Quaternion::new(0.0, 1.0, 0.0, 0.0),
        ),
    ];
    let view = build_view(&rows, &[], "id", None, 8);
    assert_eq!(view.samples[1].timestamp_ns_exact, "9000000000000000000");
    assert_eq!(view.samples[1].elapsed_s.unwrap().get(), 18_000_000_000.0);
    assert!(
        (view.samples[1].rate_rad_s.unwrap().get() - std::f64::consts::PI / 18_000_000_000.0).abs()
            < 1e-24
    );
    assert!(view.samples[0].angle_rad.is_none());
}
#[test]
fn omitted_invalid_samples_still_break_retained_geometry() {
    let mut rows = (0..1000)
        .map(|i| sample(i + 2, i as i64, Quaternion::new(1.0, 0.0, 0.0, 0.0)))
        .collect::<Vec<_>>();
    rows[501].raw = Quaternion::new(0.0, 0.0, 0.0, 0.0);
    let view = build_view(&rows, &[], "id", None, 8);
    assert!(view.samples.len() <= 8);
    assert_ne!(
        view.samples[0].geometry_segment,
        view.samples.last().unwrap().geometry_segment
    );
    assert_ne!(
        view.samples[0].stereo_segment,
        view.samples.last().unwrap().stereo_segment
    );
}
#[test]
fn candidate_values_match_repair_plans_including_unchanged_near_unit_rows() {
    let rows = [
        sample(2, 0, Quaternion::new(1.0000001, 0.0, 0.0, 0.0)),
        sample(3, 1, Quaternion::new(-1.2, 0.0, 0.0, 0.0)),
    ];
    let view = build_view(&rows, &[], "id", None, 8);
    let sign = quatopsy_core::repair::sign_lift_plan(&rows, "id").unwrap();
    let norm = quatopsy_core::repair::normalise_plan(&rows, "id").unwrap();
    for (i, point) in view.samples.iter().enumerate() {
        assert_eq!(point.sign_lift.unwrap()[0].get(), sign.quaternions[i].w);
        assert_eq!(point.normalised.unwrap()[0].get(), norm.quaternions[i].w);
    }
}
#[test]
fn cancelled_view_never_returns_a_partial_payload() {
    let flag = AtomicBool::new(true);
    let cancel = Cancel {
        deadline: Instant::now() + Duration::from_secs(1),
        flag: Some(&flag),
    };
    assert!(build_view_cancellable(&[], &[], "id", None, 8, cancel).is_err());
}
