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
        "quatopsy-control-{}-{}-{}",
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
fn control_then_analyze_keeps_verdict_ownership_in_the_kernel() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let controlled = tmp.join("controlled");
    let status = Command::new(bin())
        .args([
            "control",
            "--problem",
            root.join("fixtures/control/so3_rest_to_rest/problem.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            controlled.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let body = fs::read_to_string(controlled.join("control.json")).unwrap();
    assert!(!body.contains("\"result\""));
    assert!(body.contains("geometric-pd-so3"));
    assert!(body.contains("tracked-candidate"));
    assert!(body.contains("\"execution\":\"sil\""));
    assert!(body.contains("in-process"));
    let nav_body = fs::read_to_string(controlled.join("nav.json")).unwrap();
    assert!(!nav_body.contains("\"result\""));
    let guidance_body = fs::read_to_string(controlled.join("guidance.json")).unwrap();
    assert!(!guidance_body.contains("\"result\""));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            controlled.join("input.csv").to_str().unwrap(),
            "--manifest",
            controlled.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let report_body = fs::read_to_string(&report).unwrap();
    assert!(report_body.contains("\"result\":\"pass\""));
    assert!(report_body.contains("omega-consistent"));
}

#[test]
fn pil_then_analyze_keeps_verdict_ownership_in_the_kernel() {
    closed_loop_then_analyze(
        "fixtures/control/so3_rest_to_rest_pil/problem.json",
        "\"execution\":\"pil\"",
        "isolated-controller-process",
    );
}

#[test]
fn hil_then_analyze_keeps_verdict_ownership_in_the_kernel() {
    closed_loop_then_analyze(
        "fixtures/control/so3_rest_to_rest_hil/problem.json",
        "\"execution\":\"hil\"",
        "loopback-actuator-emulator",
    );
}

fn closed_loop_then_analyze(problem: &str, execution: &str, isolation: &str) {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let controlled = tmp.join("controlled");
    let status = Command::new(bin())
        .args([
            "control",
            "--problem",
            root.join(problem).to_str().unwrap(),
            "--output-dir",
            controlled.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let body = fs::read_to_string(controlled.join("control.json")).unwrap();
    assert!(!body.contains("\"result\""));
    assert!(body.contains("geometric-pd-so3"));
    assert!(body.contains("tracked-candidate"));
    assert!(body.contains(execution));
    assert!(body.contains(isolation));
    assert!(body.contains("loopback-emulator"));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            controlled.join("input.csv").to_str().unwrap(),
            "--manifest",
            controlled.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let report_body = fs::read_to_string(&report).unwrap();
    assert!(report_body.contains("\"result\":\"pass\""));
    assert!(report_body.contains("omega-consistent"));
}

#[test]
fn physical_hardware_does_not_create_output_directory() {
    let tmp = tempfile_dir();
    let problem = tmp.join("problem.json");
    fs::write(
        &problem,
        r#"{"schema":"quatopsy.control-problem/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","execution":"hil","latency_class":"bounded-software","q_initial":[1,0,0,0],"q_desired":[1,0,0,0],"omega_initial":[0,0,0],"inertia":{"model":"spherical","j":1.0},"torque_limit_nm":0.2,"cycle_dt_s":0.02,"duration_s":1.0,"max_estimate_age_s":0.05,"max_covariance_trace":1.0,"gains":{"kp":1.0,"kd":1.0},"hardware":{"class":"physical"}}"#,
    )
    .unwrap();
    let out = tmp.join("controlled");
    let status = Command::new(bin())
        .args([
            "control",
            "--problem",
            problem.to_str().unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!out.exists());
}

#[test]
fn declared_plant_hil_then_analyze_keeps_verdict_ownership() {
    closed_loop_then_analyze(
        "fixtures/control/so3_rest_to_rest_plant/problem.json",
        "\"execution\":\"hil\"",
        "loopback-actuator-emulator",
    );
}

#[test]
fn profile_track_then_analyze_keeps_verdict_ownership_in_the_kernel() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let controlled = tmp.join("controlled");
    let status = Command::new(bin())
        .args([
            "control",
            "--problem",
            root.join("fixtures/control/so3_profile_track/problem.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            controlled.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let body = fs::read_to_string(controlled.join("control.json")).unwrap();
    assert!(!body.contains("\"result\""));
    assert!(body.contains("geometric-pd-so3"));
    assert!(body.contains("tracked-candidate"));
    assert!(body.contains("sequential-deterministic"));
    let nav_body = fs::read_to_string(controlled.join("nav.json")).unwrap();
    assert!(!nav_body.contains("\"result\""));
    assert!(nav_body.contains("mekf"));
    let guidance_body = fs::read_to_string(controlled.join("guidance.json")).unwrap();
    assert!(!guidance_body.contains("\"result\""));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            controlled.join("input.csv").to_str().unwrap(),
            "--manifest",
            controlled.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let report_body = fs::read_to_string(&report).unwrap();
    assert!(report_body.contains("\"result\":\"pass\""));
    assert!(report_body.contains("omega-consistent"));
}
