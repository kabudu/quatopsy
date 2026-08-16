use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bin() -> PathBuf {
    env!("CARGO_BIN_EXE_quatopsy").into()
}

fn tempfile_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "quatopsy-policy-{}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn advisory_policy_exits_zero_on_sign_findings() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--policy",
            "advisory",
            "--input",
            root.join("fixtures/conformance/sign_alternating/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/sign_alternating/manifest.json")
                .to_str()
                .unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"result\":\"findings\""));
}

#[test]
fn selective_policy_fails_only_named_rules() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--policy",
            "selective",
            "--fail-on",
            "QAT-NORM-001",
            "--input",
            root.join("fixtures/conformance/sign_alternating/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/sign_alternating/manifest.json")
                .to_str()
                .unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn override_suppresses_selective_failure_without_rewriting_pass() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let overrides = tmp.join("overrides.json");
    fs::write(
        &overrides,
        r#"{"schema":"quatopsy.override/1","overrides":[{"rule":"QAT-SIGN-001","authority":"owner","reason":"known archive representation","created":"2026-01-01T00:00:00Z","expires":"9999-01-01T00:00:00Z"}]}"#,
    )
    .unwrap();
    let status = Command::new(bin())
        .args([
            "analyze",
            "--policy",
            "selective",
            "--fail-on",
            "QAT-SIGN-001",
            "--override-file",
            overrides.to_str().unwrap(),
            "--input",
            root.join("fixtures/conformance/sign_alternating/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/sign_alternating/manifest.json")
                .to_str()
                .unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"result\":\"findings\""));
}

#[test]
fn parent_directory_output_is_refused() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            root.join("fixtures/conformance/clean_slew/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/clean_slew/manifest.json")
                .to_str()
                .unwrap(),
            "--report",
            tmp.join("../sneaky.json").to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
}

#[test]
fn ids_adapter_emits_canonical_files_without_verdicts() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let out = tmp.join("adapted");
    let status = Command::new(bin())
        .args([
            "adapt",
            "--format",
            "ids-jason1",
            "--input",
            root.join("fixtures/public/ids_jason1_format/source.qbody")
                .to_str()
                .unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let provenance = fs::read_to_string(out.join("provenance.json")).unwrap();
    assert!(!provenance.contains("\"result\""));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            out.join("input.csv").to_str().unwrap(),
            "--manifest",
            out.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn ros_adapter_honours_xyzw() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let out = tmp.join("ros");
    let status = Command::new(bin())
        .args([
            "adapt",
            "--format",
            "ros-json",
            "--input",
            root.join("fixtures/adapt/ros_xyzw/source.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            out.join("input.csv").to_str().unwrap(),
            "--manifest",
            out.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn tubin_public_telemetry_adapts_then_analyses() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let out = tmp.join("tubin");
    let status = Command::new(bin())
        .args([
            "adapt",
            "--format",
            "tubin-str",
            "--input",
            root.join("fixtures/public/tubin_str/source.csv")
                .to_str()
                .unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let provenance = fs::read_to_string(out.join("provenance.json")).unwrap();
    assert!(!provenance.contains("\"result\""));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            out.join("input.csv").to_str().unwrap(),
            "--manifest",
            out.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.code().is_some());
    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"schema\":\"quatopsy.report/1\""));
    assert!(body.contains("QAT-OMEGA-001"));
    assert!(!body.contains("\"result\":\"error\""));
}

#[test]
fn expired_override_is_refused() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let overrides = tmp.join("overrides.json");
    fs::write(
        &overrides,
        r#"{"schema":"quatopsy.override/1","overrides":[{"rule":"QAT-SIGN-001","authority":"owner","reason":"stale","created":"2020-01-01T00:00:00Z","expires":"2020-02-01T00:00:00Z"}]}"#,
    )
    .unwrap();
    let status = Command::new(bin())
        .args([
            "analyze",
            "--policy",
            "selective",
            "--fail-on",
            "QAT-SIGN-001",
            "--override-file",
            overrides.to_str().unwrap(),
            "--input",
            root.join("fixtures/conformance/sign_alternating/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/sign_alternating/manifest.json")
                .to_str()
                .unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

fn adapt_then_analyze(format: &str, source: &[u8]) {
    let tmp = tempfile_dir();
    let input = tmp.join("source.bin");
    fs::write(&input, source).unwrap();
    let out = tmp.join("adapted");
    let status = Command::new(bin())
        .args([
            "adapt",
            "--format",
            format,
            "--input",
            input.to_str().unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "{format} adapt failed");
    let provenance = fs::read_to_string(out.join("provenance.json")).unwrap();
    assert!(!provenance.contains("\"result\""));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            out.join("input.csv").to_str().unwrap(),
            "--manifest",
            out.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.code().is_some());
    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"schema\":\"quatopsy.report/1\""));
    assert!(!body.contains("\"result\":\"error\""));
}

#[test]
fn mcap_adapter_emits_canonical_files_without_verdicts() {
    let bytes = quatopsy_adapt::encode_mcap_json_poses(
        "base_link",
        "map",
        &[(0.0, 0.0, 0.0, 0.0, 1.0), (1.0, 0.0, 0.0, 0.0, 1.0)],
    );
    adapt_then_analyze("mcap-json", &bytes);
}

#[test]
fn spice_ck_adapter_emits_canonical_files_without_verdicts() {
    let bytes = quatopsy_adapt::encode_ck_type3(
        -82_000,
        1,
        &[
            (0.0, 1.0, 0.0, 0.0, 0.0),
            (
                1.0,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
                0.0,
                std::f64::consts::FRAC_1_SQRT_2,
            ),
        ],
    );
    adapt_then_analyze("spice-ck", &bytes);
}
