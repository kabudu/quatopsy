use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn license_and_claims_freeze_exist() {
    let root = workspace_root();
    for relative in [
        "LICENSE",
        "NOTICE",
        "CHANGELOG.md",
        "docs/CLAIMS.md",
        "docs/RELEASE_GATE.md",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        let path = root.join(relative);
        assert!(path.is_file(), "missing {relative}");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.is_empty(), "{relative} is empty");
    }
}

#[test]
fn cargo_publish_refuses_without_authorization() {
    let script = workspace_root().join("scripts/publish-crates.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg("--publish")
        .env_remove("QUATOPSY_RELEASE_AUTHORIZE")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("QUATOPSY_RELEASE_AUTHORIZE=1"),
        "stderr={stderr}"
    );
}
