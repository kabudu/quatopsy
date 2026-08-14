use std::fs;
use std::path::PathBuf;

use quatopsy_core::limits::Limits;
use quatopsy_core::{AnalyzeRequest, analyze, report_bytes};
use quatopsy_schema::{RULE_RATE, ResultState};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Expected {
    result: String,
    rules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    finding_reasons: Vec<String>,
    #[serde(default)]
    finding_rows: Vec<[u64; 2]>,
    #[serde(default)]
    rate_interval_count: Option<u64>,
    #[serde(default)]
    expected_max_angle_rad: Option<f64>,
    #[serde(default)]
    expected_max_rate_rad_s: Option<f64>,
    #[serde(default)]
    repairs: Vec<ExpectedRepair>,
    #[serde(default)]
    finding_dispositions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedRepair {
    algorithm: String,
    disposition: String,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_case(name: &str) -> (Vec<u8>, Vec<u8>, Expected) {
    let dir = workspace_root().join("fixtures/conformance").join(name);
    let csv = fs::read(dir.join("input.csv")).unwrap();
    let manifest = fs::read(dir.join("manifest.json")).unwrap();
    let expected: Expected =
        serde_json::from_str(&fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
    (csv, manifest, expected)
}

fn run_case(name: &str) {
    let (csv, manifest, expected) = load_case(name);
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    assert_eq!(
        report.result.as_str(),
        expected.result,
        "case {name} result"
    );
    assert_eq!(
        report.result.exit_code(),
        match report.result {
            ResultState::Pass => 0,
            ResultState::Findings => 1,
            ResultState::Refused => 2,
            ResultState::Error => 3,
        }
    );
    for rule in &report.rule_results {
        let want = expected
            .rules
            .get(&rule.rule)
            .unwrap_or_else(|| panic!("case {name} missing expected state for {}", rule.rule));
        assert_eq!(rule.state.as_str(), want, "case {name} rule {}", rule.rule);
    }
    let reasons: Vec<_> = report
        .findings
        .iter()
        .map(|finding| finding.reason_code.as_str())
        .collect();
    assert_eq!(
        reasons, expected.finding_reasons,
        "case {name} finding reasons"
    );
    if !expected.finding_rows.is_empty() {
        let rows: Vec<[u64; 2]> = report
            .findings
            .iter()
            .map(|finding| [finding.source_row_start, finding.source_row_end])
            .collect();
        assert_eq!(rows, expected.finding_rows, "case {name} finding rows");
    }
    if !expected.finding_dispositions.is_empty() {
        let got: Vec<_> = report
            .findings
            .iter()
            .map(|finding| finding.repair_disposition.as_str().to_string())
            .collect();
        assert_eq!(
            got, expected.finding_dispositions,
            "case {name} dispositions"
        );
    }
    let got_repairs: Vec<_> = report
        .repairs
        .iter()
        .map(|repair| (repair.algorithm.as_str(), repair.disposition.as_str()))
        .collect();
    let want_repairs: Vec<_> = expected
        .repairs
        .iter()
        .map(|repair| (repair.algorithm.as_str(), repair.disposition.as_str()))
        .collect();
    assert_eq!(got_repairs, want_repairs, "case {name} repairs");
    if let Some(count) = expected.rate_interval_count {
        let summary = report
            .diagnostics
            .rate_summary
            .as_ref()
            .expect("rate summary");
        assert_eq!(summary.interval_count, count);
    }
    if let Some(angle) = expected.expected_max_angle_rad {
        let summary = report
            .diagnostics
            .rate_summary
            .as_ref()
            .expect("rate summary");
        assert!((summary.max_angle_rad.get() - angle).abs() < 1e-9);
        if let Some(rate) = expected.expected_max_rate_rad_s {
            assert!((summary.max_rate_rad_s.get() - rate).abs() < 1e-9);
        }
        assert!(
            report
                .rule_results
                .iter()
                .any(|item| item.rule == RULE_RATE && item.reason_code == "rates-derived")
        );
    }
    let encoded = report_bytes(&report).expect("canonical json");
    let again = report_bytes(&report).expect("canonical json repeat");
    assert_eq!(encoded, again, "case {name} canonical bytes must be stable");
}

#[test]
fn qat_norm_001_off_unit() {
    run_case("norm_drift");
}

#[test]
fn qat_norm_001_zero_refuses() {
    run_case("zero_quat");
}

#[test]
fn qat_time_001_duplicate() {
    run_case("time_duplicate");
}

#[test]
fn qat_time_001_decreasing() {
    run_case("time_decreasing");
}

#[test]
fn qat_lift_and_sign_001_alternating() {
    run_case("sign_alternating");
}

#[test]
fn qat_pi_001_half_turn() {
    run_case("near_pi");
}

#[test]
fn qat_rate_001_quarter_turn() {
    run_case("rate_quarter_turn");
}

#[test]
fn clean_slew_has_no_release_critical_findings() {
    run_case("clean_slew");
}

#[test]
fn xyzw_declaration_is_honoured() {
    run_case("xyzw_identity");
}

#[test]
fn missing_manifest_field_is_refused() {
    let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n";
    let manifest = br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz"}"#;
    let report = analyze(AnalyzeRequest {
        csv_bytes: csv,
        manifest_bytes: manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    assert_eq!(report.result, ResultState::Refused);
    assert_ne!(report.result, ResultState::Pass);
}

#[test]
fn analysis_id_is_stable_and_input_sensitive() {
    let dir = workspace_root().join("fixtures/conformance/clean_slew");
    let csv = fs::read(dir.join("input.csv")).unwrap();
    let manifest = fs::read(dir.join("manifest.json")).unwrap();
    let a = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    let b = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    assert_eq!(a.analysis_id, b.analysis_id);
    let mut csv2 = csv.clone();
    csv2.extend_from_slice(b"\n");
    let c = analyze(AnalyzeRequest {
        csv_bytes: &csv2,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    assert_ne!(a.analysis_id, c.analysis_id);
}
