//! Boundary cases that would pass if a named comparison were inverted.

use quatopsy_core::limits::Limits;
use quatopsy_core::{AnalyzeRequest, analyze};
use quatopsy_schema::{NEAR_ZERO_NORM, NORM_ABS_TOLERANCE, ResultState};

fn req<'a>(csv: &'a [u8], manifest: &'a [u8]) -> AnalyzeRequest<'a> {
    AnalyzeRequest {
        csv_bytes: csv,
        manifest_bytes: manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    }
}

fn manifest() -> &'static [u8] {
    br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","columns":{"time":"t","quaternion":["qw","qx","qy","qz"]}}"#
}

#[test]
fn mutant_norm_tolerance_must_still_flag_1e_5_error() {
    let off = 1.0 + NORM_ABS_TOLERANCE * 10.0;
    let csv = format!("t,qw,qx,qy,qz\n0,{off},0,0,0\n1,{off},0,0,0\n").into_bytes();
    let report = analyze(req(&csv, manifest()));
    assert_eq!(report.result, ResultState::Findings);
    assert!(
        report
            .findings
            .iter()
            .any(|item| item.reason_code == "off-unit-sample")
    );
}

#[test]
fn mutant_near_zero_must_refuse_below_threshold() {
    let n = NEAR_ZERO_NORM / 10.0;
    let csv = format!("t,qw,qx,qy,qz\n0,{n},0,0,0\n").into_bytes();
    let report = analyze(req(&csv, manifest()));
    assert_eq!(report.result, ResultState::Refused);
}

#[test]
fn mutant_time_must_refuse_equal_timestamps() {
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n0,1,0,0,0\n";
    let report = analyze(req(csv, manifest()));
    assert_eq!(report.result, ResultState::Refused);
}

#[test]
fn mutant_sign_must_not_treat_negative_dot_as_physical_ok() {
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n1,-1,0,0,0\n";
    let report = analyze(req(csv, manifest()));
    assert_eq!(report.result, ResultState::Findings);
    assert!(
        report
            .findings
            .iter()
            .any(|item| item.reason_code == "sign-discontinuity")
    );
}

#[test]
fn mutant_aggregator_error_must_dominate_pass() {
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n";
    let mut limits = Limits::defaults();
    limits.max_samples = 0;
    let report = analyze(AnalyzeRequest {
        csv_bytes: csv,
        manifest_bytes: manifest(),
        engine_version: "0.1.0",
        limits,
        cancelled: None,
    });
    assert_eq!(report.result, ResultState::Error);
    assert_ne!(report.result, ResultState::Pass);
}

#[test]
fn mutant_omega_must_flag_wrong_body_rate() {
    let csv = b"t,qw,qx,qy,qz,wx,wy,wz\n0,1,0,0,0,0,0,0\n1,0.995004165,0,0,0.0998334166,9,0,0\n";
    let manifest = br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","columns":{"time":"t","quaternion":["qw","qx","qy","qz"],"angular_velocity":["wx","wy","wz"]}}"#;
    let report = analyze(req(csv, manifest));
    assert_eq!(report.result, ResultState::Findings);
    assert!(
        report
            .findings
            .iter()
            .any(|item| item.reason_code == "omega-inconsistent")
    );
}

#[test]
fn mutant_conv_must_flag_identity_matrix_on_non_identity_quat() {
    let csv = b"t,qw,qx,qy,qz,r00,r01,r02,r10,r11,r12,r20,r21,r22\n0,0,1,0,0,1,0,0,0,1,0,0,0,1\n1,0,1,0,0,1,0,0,0,1,0,0,0,1\n";
    let manifest = br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","columns":{"time":"t","quaternion":["qw","qx","qy","qz"],"rotation_matrix":["r00","r01","r02","r10","r11","r12","r20","r21","r22"]}}"#;
    let report = analyze(req(csv, manifest));
    assert_eq!(report.result, ResultState::Findings);
    assert!(
        report
            .findings
            .iter()
            .any(|item| item.rule == "QAT-CONV-001")
    );
}
