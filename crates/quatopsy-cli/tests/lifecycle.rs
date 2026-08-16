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
        "quatopsy-life-test-{}-{}-{}",
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
fn install_repeat_analysis_removal_and_report_compatibility() {
    let root = workspace_root();
    let tmp = tempfile_dir();
    let source_csv = root.join("fixtures/conformance/clean_slew/input.csv");
    let source_manifest = root.join("fixtures/conformance/clean_slew/manifest.json");
    let install = tmp.join("install/bin");
    fs::create_dir_all(&install).unwrap();
    let installed = install.join("quatopsy");
    fs::copy(bin(), &installed).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&installed).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&installed, perms).unwrap();
    }

    let report_v1 = tmp.join("report-install.json");
    assert!(
        Command::new(&installed)
            .args([
                "analyze",
                "--input",
                source_csv.to_str().unwrap(),
                "--manifest",
                source_manifest.to_str().unwrap(),
                "--report",
                report_v1.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .success()
    );
    let installed_body = fs::read_to_string(&report_v1).unwrap();
    assert!(installed_body.contains("\"schema\":\"quatopsy.report/1\""));
    assert!(installed_body.contains("QAT-UNWIND-001"));

    let report_upgrade = tmp.join("report-upgrade.json");
    assert!(
        Command::new(&installed)
            .args([
                "analyze",
                "--input",
                source_csv.to_str().unwrap(),
                "--manifest",
                source_manifest.to_str().unwrap(),
                "--report",
                report_upgrade.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        fs::read_to_string(&report_v1)
            .unwrap()
            .contains("quatopsy.report/1")
    );

    let view_dir = tmp.join("view-compat");
    assert_eq!(
        Command::new(&installed)
            .args([
                "view",
                "--report",
                report_v1.to_str().unwrap(),
                "--output",
                view_dir.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .code(),
        Some(0)
    );
    let html = fs::read_to_string(view_dir.join("index.html")).unwrap();
    assert!(html.contains("\"result\":\"pass\""));

    let future = tmp.join("future.json");
    fs::write(
        &future,
        r#"{"schema":"quatopsy.report/99","result":"pass"}"#,
    )
    .unwrap();
    let future_view = tmp.join("future-view");
    assert_eq!(
        Command::new(&installed)
            .args([
                "view",
                "--report",
                future.to_str().unwrap(),
                "--output",
                future_view.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    let js = fs::read_to_string(future_view.join("viewer.js")).unwrap();
    assert!(js.contains("Viewer refused unknown report schema"));

    fs::remove_dir_all(tmp.join("install")).unwrap();
    assert!(!installed.exists());
    assert!(source_csv.exists());
    assert!(report_v1.exists());
    assert!(report_upgrade.exists());
}
