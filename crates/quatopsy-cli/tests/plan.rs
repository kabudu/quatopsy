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
        "quatopsy-plan-{}-{}-{}",
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
fn plan_then_analyze_keeps_verdict_ownership_in_the_kernel() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let planned = tmp.join("planned");
    let status = Command::new(bin())
        .args([
            "plan",
            "--problem",
            root.join("fixtures/plan/spherical_rest_to_rest/problem.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            planned.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let plan_body = fs::read_to_string(planned.join("plan.json")).unwrap();
    assert!(!plan_body.contains("\"result\""));
    assert!(plan_body.contains("feasible-candidate"));
    assert!(plan_body.contains("not-claimed"));
    assert!(!plan_body.contains("\"infeasibility\""));
    let csv = fs::read_to_string(planned.join("input.csv")).unwrap();
    assert_eq!(
        csv.lines().next().unwrap(),
        "t,qw,qx,qy,qz,wx,wy,wz,tx,ty,tz"
    );

    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            planned.join("input.csv").to_str().unwrap(),
            "--manifest",
            planned.join("manifest.json").to_str().unwrap(),
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
fn wheel_plan_then_analyze_keeps_verdict_in_the_kernel() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let planned = tmp.join("planned");
    let status = Command::new(bin())
        .args([
            "plan",
            "--problem",
            root.join("fixtures/plan/wheels_rest_to_rest/problem.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            planned.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let plan_body = fs::read_to_string(planned.join("plan.json")).unwrap();
    assert!(!plan_body.contains("\"result\""));
    assert!(plan_body.contains("direct-shooting-lm"));
    let report = tmp.join("report.json");
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            planned.join("input.csv").to_str().unwrap(),
            "--manifest",
            planned.join("manifest.json").to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let report_body = fs::read_to_string(&report).unwrap();
    assert!(report_body.contains("\"result\":\"pass\""));
}

#[test]
fn unknown_plan_field_does_not_create_output_directory() {
    let tmp = tempfile_dir();
    let problem = tmp.join("problem.json");
    fs::write(
        &problem,
        r#"{"schema":"quatopsy.plan-problem/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","q_initial":[1,0,0,0],"q_final":[0.7071067811865476,0.7071067811865476,0,0],"omega_initial":[0,0,0],"omega_final":[0,0,0],"inertia":{"model":"spherical","j":1.0},"torque_limit_nm":0.05,"sample_count":8,"objective":"minimum-time","wheels":true}"#,
    )
    .unwrap();
    let planned = tmp.join("planned");
    let status = Command::new(bin())
        .args([
            "plan",
            "--problem",
            problem.to_str().unwrap(),
            "--output-dir",
            planned.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!planned.exists());
}

#[test]
fn infeasible_plan_does_not_create_output_directory() {
    let tmp = tempfile_dir();
    let problem = tmp.join("problem.json");
    fs::write(
        &problem,
        r#"{"schema":"quatopsy.plan-problem/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","q_initial":[1,0,0,0],"q_final":[0.7071067811865476,0.5,0.5,0],"omega_initial":[0,0,0],"omega_final":[0,0,0],"inertia":{"model":"diagonal","jxx":1.0,"jyy":40.0,"jzz":0.2},"torque_limit_nm":[0.02,0.02,0.02],"sample_count":8,"objective":"minimum-time"}"#,
    )
    .unwrap();
    let planned = tmp.join("planned");
    let status = Command::new(bin())
        .args([
            "plan",
            "--problem",
            problem.to_str().unwrap(),
            "--output-dir",
            planned.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!planned.exists());
}

#[cfg(unix)]
#[test]
fn symlink_output_parent_is_refused() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let real = tmp.join("real");
    let linked = tmp.join("linked");
    fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    let status = Command::new(bin())
        .args([
            "plan",
            "--problem",
            root.join("fixtures/plan/spherical_rest_to_rest/problem.json")
                .to_str()
                .unwrap(),
            "--output-dir",
            linked.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
    assert!(!real.join("input.csv").exists());
}
