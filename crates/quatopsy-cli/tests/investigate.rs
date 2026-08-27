use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use quatopsy_core::identity::sha256_hex;
use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_quatopsy"))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn temp_dir(label: &str) -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "quatopsy-investigate-{label}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    path
}

fn run_complete_case(output: &Path, context: &Path) -> std::process::ExitStatus {
    let repository = root();
    Command::new(bin())
        .arg("investigate")
        .arg("--case-id")
        .arg("ops-2026-042")
        .arg("--input")
        .arg(repository.join("fixtures/conformance/sign_alternating/input.csv"))
        .arg("--manifest")
        .arg(repository.join("fixtures/conformance/sign_alternating/manifest.json"))
        .arg("--event-log")
        .arg(context.join("events.log"))
        .arg("--command-log")
        .arg(context.join("commands.log"))
        .arg("--notes")
        .arg(context.join("notes.txt"))
        .arg("--plan-problem")
        .arg(repository.join("fixtures/plan/spherical_rest_to_rest/problem.json"))
        .arg("--control-problem")
        .arg(repository.join("fixtures/control/so3_rest_to_rest/problem.json"))
        .arg("--output-dir")
        .arg(output)
        .status()
        .unwrap()
}

#[test]
fn incident_case_preserves_sources_and_builds_verified_candidates() {
    let temp = temp_dir("complete");
    fs::write(temp.join("events.log"), b"T+1 SAFE_MODE entered\n").unwrap();
    fs::write(temp.join("commands.log"), b"T+0 SLEW_START accepted\n").unwrap();
    fs::write(temp.join("notes.txt"), b"Investigate sign discontinuity.\n").unwrap();
    let output = temp.join("evidence");
    assert!(run_complete_case(&output, &temp).success());

    let encoded = fs::read(output.join("evidence.json")).unwrap();
    let evidence: Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(evidence["schema"], "quatopsy.evidence/1");
    assert_eq!(evidence["case_id"], "ops-2026-042");
    assert_eq!(evidence["observed"]["result"], "findings");
    assert_eq!(evidence["context"]["interpreted"], false);
    assert_eq!(evidence["candidates"].as_array().unwrap().len(), 2);
    assert!(
        evidence["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|candidate| candidate["result"] == "pass")
    );
    assert_eq!(
        fs::read(output.join("source/input.csv")).unwrap(),
        fs::read(root().join("fixtures/conformance/sign_alternating/input.csv")).unwrap()
    );
    assert!(
        output
            .join("observed/repro/finding-0001/slice.csv")
            .exists()
    );
    assert!(
        output
            .join("observed/repairs/repair_sign-lift_1.csv")
            .exists()
    );
    assert!(output.join("observed/viewer/index.html").exists());
    assert!(
        output
            .join("candidates/plan/analysis/viewer/index.html")
            .exists()
    );
    assert!(
        output
            .join("candidates/control/analysis/viewer/index.html")
            .exists()
    );
    assert!(
        !fs::read_to_string(output.join("candidates/plan/generated/plan.json"))
            .unwrap()
            .contains("\"result\"")
    );
    assert!(
        !fs::read_to_string(output.join("candidates/control/generated/control.json"))
            .unwrap()
            .contains("\"result\"")
    );

    let artifacts = evidence["artifacts"].as_array().unwrap();
    let mut canonical = Vec::new();
    canonical.extend_from_slice(evidence["case_id"].as_str().unwrap().as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(evidence["tool"]["version"].as_str().unwrap().as_bytes());
    canonical.push(b'\n');
    for artifact in artifacts {
        let relative = artifact["path"].as_str().unwrap();
        let bytes = fs::read(output.join(relative)).unwrap();
        assert_eq!(artifact["bytes"].as_u64().unwrap(), bytes.len() as u64);
        assert_eq!(artifact["sha256"], sha256_hex(&bytes));
        canonical.extend_from_slice(relative.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(artifact["sha256"].as_str().unwrap().as_bytes());
        canonical.push(b'\n');
    }
    assert_eq!(evidence["bundle_id"], sha256_hex(&canonical));
    assert!(
        !String::from_utf8(encoded)
            .unwrap()
            .contains(temp.to_string_lossy().as_ref())
    );
    assert!(
        Command::new(bin())
            .arg("verify-evidence")
            .arg("--bundle")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    let original_evidence = fs::read(output.join("evidence.json")).unwrap();
    let mut altered_evidence: Value = serde_json::from_slice(&original_evidence).unwrap();
    altered_evidence["boundaries"][0] = serde_json::json!("flight approved");
    fs::write(
        output.join("evidence.json"),
        serde_json::to_vec_pretty(&altered_evidence).unwrap(),
    )
    .unwrap();
    assert!(
        !Command::new(bin())
            .arg("verify-evidence")
            .arg("--bundle")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    fs::write(output.join("evidence.json"), original_evidence).unwrap();
    fs::write(output.join("context/events.log"), b"tampered\n").unwrap();
    assert!(
        !Command::new(bin())
            .arg("verify-evidence")
            .arg("--bundle")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn existing_bundle_is_not_modified() {
    let temp = temp_dir("existing");
    let output = temp.join("evidence");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel"), b"keep").unwrap();
    let repository = root();
    let status = Command::new(bin())
        .arg("investigate")
        .arg("--case-id")
        .arg("existing")
        .arg("--input")
        .arg(repository.join("fixtures/conformance/clean_slew/input.csv"))
        .arg("--manifest")
        .arg(repository.join("fixtures/conformance/clean_slew/manifest.json"))
        .arg("--output-dir")
        .arg(&output)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(64));
    assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep");
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn failed_candidate_removes_the_incomplete_bundle() {
    let temp = temp_dir("rollback");
    let problem = temp.join("impossible.json");
    let repository = root();
    let mut value: Value = serde_json::from_slice(
        &fs::read(repository.join("fixtures/plan/spherical_rest_to_rest/problem.json")).unwrap(),
    )
    .unwrap();
    value["unexpected"] = serde_json::json!(true);
    fs::write(&problem, serde_json::to_vec(&value).unwrap()).unwrap();
    let output = temp.join("evidence");
    let status = Command::new(bin())
        .arg("investigate")
        .arg("--case-id")
        .arg("rollback")
        .arg("--input")
        .arg(repository.join("fixtures/conformance/clean_slew/input.csv"))
        .arg("--manifest")
        .arg(repository.join("fixtures/conformance/clean_slew/manifest.json"))
        .arg("--plan-problem")
        .arg(&problem)
        .arg("--output-dir")
        .arg(&output)
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!output.exists());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn external_telemetry_is_adapted_inside_the_evidence_boundary() {
    let temp = temp_dir("adapter");
    let repository = root();
    let source = repository.join("fixtures/public/tubin_str/source.csv");
    let output = temp.join("evidence");
    let status = Command::new(bin())
        .arg("investigate")
        .arg("--case-id")
        .arg("tubin-str-public-slice")
        .arg("--input")
        .arg(&source)
        .arg("--format")
        .arg("tubin-str")
        .arg("--output-dir")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        fs::read(output.join("source/external-input.bin")).unwrap(),
        fs::read(source).unwrap()
    );
    assert!(output.join("source/adapted/input.csv").exists());
    assert!(output.join("source/adapted/manifest.json").exists());
    let provenance = fs::read_to_string(output.join("source/adapted/provenance.json")).unwrap();
    assert!(!provenance.contains("\"result\""));
    assert!(output.join("observed/report.json").exists());
    assert!(output.join("observed/viewer/index.html").exists());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn identical_cases_have_identical_evidence_manifests() {
    let temp = temp_dir("deterministic");
    let repository = root();
    let first = temp.join("first");
    let second = temp.join("second");
    for output in [&first, &second] {
        let status = Command::new(bin())
            .arg("investigate")
            .arg("--case-id")
            .arg("deterministic-case")
            .arg("--input")
            .arg(repository.join("fixtures/conformance/clean_slew/input.csv"))
            .arg("--manifest")
            .arg(repository.join("fixtures/conformance/clean_slew/manifest.json"))
            .arg("--output-dir")
            .arg(output)
            .status()
            .unwrap();
        assert!(status.success());
    }
    assert_eq!(
        fs::read(first.join("evidence.json")).unwrap(),
        fs::read(second.join("evidence.json")).unwrap()
    );
    fs::remove_dir_all(temp).unwrap();
}
