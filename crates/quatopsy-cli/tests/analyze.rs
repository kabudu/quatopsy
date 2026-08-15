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

#[test]
fn sign_repair_is_a_new_file_and_clears_sign_findings() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let input = root.join("fixtures/conformance/sign_alternating/input.csv");
    let manifest = root.join("fixtures/conformance/sign_alternating/manifest.json");
    let report = tmp.join("report.json");
    let repaired = tmp.join("repaired.csv");
    let repaired_report = tmp.join("repaired.json");
    assert_eq!(
        Command::new(bin())
            .args([
                "analyze",
                "--input",
                input.to_str().unwrap(),
                "--manifest",
                manifest.to_str().unwrap(),
                "--report",
                report.to_str().unwrap(),
                "--repro-dir",
                tmp.join("repro").to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .code(),
        Some(1)
    );
    let provenance = fs::read_to_string(tmp.join("repro/provenance.json")).unwrap();
    assert!(!provenance.contains(input.to_str().unwrap()));
    assert!(tmp.join("repro/slice.csv").exists());
    assert_eq!(
        Command::new(bin())
            .args([
                "repair",
                "--report",
                report.to_str().unwrap(),
                "--input",
                input.to_str().unwrap(),
                "--manifest",
                manifest.to_str().unwrap(),
                "--repair-id",
                "repair:sign-lift:1",
                "--output",
                repaired.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .code(),
        Some(0)
    );
    assert_ne!(fs::read(&repaired).unwrap(), fs::read(&input).unwrap());
    assert!(
        Command::new(bin())
            .args([
                "analyze",
                "--input",
                repaired.to_str().unwrap(),
                "--manifest",
                manifest.to_str().unwrap(),
                "--report",
                repaired_report.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .success()
    );
    let body = fs::read_to_string(&repaired_report).unwrap();
    assert!(body.contains("\"result\":\"pass\""));
    assert!(!body.contains("sign-discontinuity"));
}

#[test]
fn clean_removes_sibling_tmp() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let stale = tmp.join("report.json.tmp");
    fs::write(&stale, b"stale").unwrap();
    assert!(
        Command::new(bin())
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
                "--clean",
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(!stale.exists());
}

#[cfg(unix)]
#[test]
fn symlink_output_is_refused() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let target = tmp.join("target.json");
    let report = tmp.join("link.json");
    fs::write(&target, b"nope").unwrap();
    std::os::unix::fs::symlink(&target, &report).unwrap();
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
            "--overwrite",
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
    assert_eq!(fs::read(&target).unwrap(), b"nope");
}

#[test]
fn repair_refuses_input_that_does_not_match_the_report() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let output = tmp.join("out.csv");
    assert_eq!(
        Command::new(bin())
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
            .unwrap()
            .code(),
        Some(1)
    );
    let status = Command::new(bin())
        .args([
            "repair",
            "--report",
            report.to_str().unwrap(),
            "--input",
            root.join("fixtures/conformance/clean_slew/input.csv")
                .to_str()
                .unwrap(),
            "--manifest",
            root.join("fixtures/conformance/sign_alternating/manifest.json")
                .to_str()
                .unwrap(),
            "--repair-id",
            "repair:sign-lift:1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(!output.exists());
}

#[test]
fn default_stderr_does_not_echo_sample_payload() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let report = tmp.join("report.json");
    let output = Command::new(bin())
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
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let csv = fs::read_to_string(root.join("fixtures/conformance/clean_slew/input.csv")).unwrap();
    let payload = csv.lines().nth(1).unwrap();
    assert!(
        !stderr.contains(payload),
        "stderr leaked sample row {payload:?}: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn unwritable_report_directory_leaves_no_committed_file() {
    use std::os::unix::fs::PermissionsExt;
    let root = workspace_root();
    let tmp = tempfile_dir();
    let locked = tmp.join("locked");
    fs::create_dir_all(&locked).unwrap();
    let report = locked.join("report.json");
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&locked, perms).unwrap();
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
        .unwrap();
    let mut restore = fs::metadata(&locked).unwrap().permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked, restore).unwrap();
    assert_eq!(status.code(), Some(3));
    assert!(!report.exists());
}

#[cfg(unix)]
#[test]
fn cancelled_repair_write_leaves_source_and_output_untouched() {
    use std::os::unix::fs::PermissionsExt;
    let root = workspace_root();
    let tmp = tempfile_dir();
    let input = root.join("fixtures/conformance/sign_alternating/input.csv");
    let manifest = root.join("fixtures/conformance/sign_alternating/manifest.json");
    let report = tmp.join("report.json");
    assert_eq!(
        Command::new(bin())
            .args([
                "analyze",
                "--input",
                input.to_str().unwrap(),
                "--manifest",
                manifest.to_str().unwrap(),
                "--report",
                report.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .code(),
        Some(1)
    );
    let locked = tmp.join("locked-repair");
    fs::create_dir_all(&locked).unwrap();
    let output = locked.join("repaired.csv");
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&locked, perms).unwrap();
    let before = fs::read(&input).unwrap();
    let status = Command::new(bin())
        .args([
            "repair",
            "--report",
            report.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--repair-id",
            "repair:sign-lift:1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    let mut restore = fs::metadata(&locked).unwrap().permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked, restore).unwrap();
    assert_eq!(status.code(), Some(3));
    assert!(!output.exists());
    assert_eq!(fs::read(&input).unwrap(), before);
}

fn tempfile_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "quatopsy-cli-test-{}-{}-{}",
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
