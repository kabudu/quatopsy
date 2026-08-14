use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bin() -> PathBuf {
    env!("CARGO_BIN_EXE_quatopsy").into()
}

#[test]
fn analyze_clean_slew_exits_zero_and_writes_report() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
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
            report.to_str().unwrap(),
        ])
        .status()
        .expect("run quatopsy");
    assert!(status.success());
    let body = fs::read_to_string(&report).unwrap();
    assert!(body.contains("\"result\":\"pass\""));
    assert!(body.contains("\"schema\":\"quatopsy.report/1\""));
}

#[test]
fn analyze_sign_fixture_exits_one() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
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
        .expect("run quatopsy");
    assert_eq!(status.code(), Some(1));
}

#[test]
fn no_clobber_default_is_usage_error() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    fs::write(&report, b"existing").unwrap();
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
            report.to_str().unwrap(),
        ])
        .status()
        .expect("run quatopsy");
    assert_eq!(status.code(), Some(64));
    assert_eq!(fs::read(&report).unwrap(), b"existing");
}

#[test]
fn usage_error_is_exit_64() {
    let status = Command::new(bin()).arg("analyze").status().unwrap();
    assert_eq!(status.code(), Some(64));
}

fn tempfile_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "quatopsy-cli-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
