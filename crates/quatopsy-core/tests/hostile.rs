use std::sync::atomic::AtomicBool;

use quatopsy_core::limits::Limits;
use quatopsy_core::{AnalyzeRequest, analyze};
use quatopsy_schema::ResultState;

fn req<'a>(csv: &'a [u8], manifest: &'a [u8], limits: Limits) -> AnalyzeRequest<'a> {
    AnalyzeRequest {
        csv_bytes: csv,
        manifest_bytes: manifest,
        engine_version: "0.1.0",
        limits,
        cancelled: None,
    }
}

fn manifest() -> &'static [u8] {
    br#"{
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "BODY",
        "frame_to": "J2000",
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qw", "qx", "qy", "qz"]}
    }"#
}

#[test]
fn formula_text_is_error() {
    let csv = b"t,qw,qx,qy,qz\n0,=1+1,0,0,0\n";
    let report = analyze(req(csv, manifest(), Limits::defaults()));
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "formula-text");
}

#[test]
fn invalid_utf8_is_error() {
    let csv = b"t,qw,qx,qy,qz\n0,\xff\xfe,0,0,0\n";
    let report = analyze(req(csv, manifest(), Limits::defaults()));
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "invalid-utf8");
}

#[test]
fn nan_sample_is_refused() {
    let csv = b"t,qw,qx,qy,qz\n0,NaN,0,0,0\n1,1,0,0,0\n";
    let report = analyze(req(csv, manifest(), Limits::defaults()));
    assert_eq!(report.result, ResultState::Refused);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.reason_code == "non-finite-sample")
    );
}

#[test]
fn inf_sample_is_refused() {
    let csv = b"t,qw,qx,qy,qz\n0,inf,0,0,0\n";
    let report = analyze(req(csv, manifest(), Limits::defaults()));
    assert_eq!(report.result, ResultState::Refused);
}

#[test]
fn oversized_field_is_error() {
    let mut limits = Limits::defaults();
    limits.max_field_bytes = 4;
    let csv = b"t,qw,qx,qy,qz\n0,1.0000000,0,0,0\n";
    let report = analyze(req(csv, manifest(), limits));
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "field-limit");
}

#[test]
fn sample_limit_is_error() {
    let mut limits = Limits::defaults();
    limits.max_samples = 1;
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n1,1,0,0,0\n";
    let report = analyze(req(csv, manifest(), limits));
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "sample-limit");
}

#[test]
fn finding_cap_prevents_pass() {
    let mut limits = Limits::defaults();
    limits.max_findings_per_rule = 1;
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n1,-1,0,0,0\n2,1,0,0,0\n3,-1,0,0,0\n";
    let report = analyze(req(csv, manifest(), limits));
    assert_ne!(report.result, ResultState::Pass);
    assert_eq!(report.result, ResultState::Error);
}

#[test]
fn cancelled_analysis_is_error() {
    let flag = AtomicBool::new(true);
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n";
    let report = analyze(AnalyzeRequest {
        csv_bytes: csv,
        manifest_bytes: manifest(),
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: Some(&flag),
    });
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "cancelled");
}

#[test]
fn unicode_numeric_field_is_error() {
    let csv = "t,qw,qx,qy,qz\n0,1℃,0,0,0\n".as_bytes();
    let report = analyze(req(csv, manifest(), Limits::defaults()));
    assert_eq!(report.result, ResultState::Error);
    assert_eq!(report.diagnostics.reason_code, "invalid-number");
}

#[test]
fn unknown_manifest_field_is_refused() {
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n";
    let manifest = br#"{
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "BODY",
        "frame_to": "J2000",
        "time_unit": "s",
        "guess": true,
        "columns": {"time": "t", "quaternion": ["qw", "qx", "qy", "qz"]}
    }"#;
    let report = analyze(req(csv, manifest, Limits::defaults()));
    assert_eq!(report.result, ResultState::Refused);
}
