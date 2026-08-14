use quatopsy_core::identity::sha256_hex;
use quatopsy_core::limits::Limits;
use quatopsy_core::repair::{render_repaired_csv, sign_lift_plan};
use quatopsy_core::{AnalyzeRequest, analyze};
use quatopsy_oracle::{RefQuat, matrices_close, rotation_matrix};

fn fixture(name: &str) -> (Vec<u8>, Vec<u8>) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("fixtures/conformance").join(name);
    (
        std::fs::read(dir.join("input.csv")).unwrap(),
        std::fs::read(dir.join("manifest.json")).unwrap(),
    )
}

#[test]
fn sign_lift_preserves_independent_rotation_matrices() {
    let (csv, manifest) = fixture("sign_alternating");
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    let parsed = quatopsy_core::ingest::ingest_bytes(
        &csv,
        &manifest,
        Limits::defaults(),
        quatopsy_core::cancel::Cancel {
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(5),
            flag: None,
        },
    )
    .unwrap();
    let plan = sign_lift_plan(&parsed.samples, &report.analysis_id).expect("sign repair");
    assert!(plan.repair.physical_orientation_equivalent);
    for (raw, repaired) in parsed.samples.iter().zip(plan.quaternions.iter()) {
        let a = RefQuat {
            w: raw.raw.w,
            x: raw.raw.x,
            y: raw.raw.y,
            z: raw.raw.z,
        };
        let b = RefQuat {
            w: repaired.w,
            x: repaired.x,
            y: repaired.y,
            z: repaired.z,
        };
        assert!(
            matrices_close(rotation_matrix(a), rotation_matrix(b), 1e-12),
            "matrix mismatch for {a:?} vs {b:?}"
        );
    }
    let rendered = render_repaired_csv(
        &csv,
        &parsed.declarations,
        &parsed.samples,
        &plan.quaternions,
    )
    .unwrap();
    assert_ne!(sha256_hex(&rendered), sha256_hex(&csv));
    assert!(std::str::from_utf8(&rendered).unwrap().contains("1"));
}

#[test]
fn near_pi_does_not_invent_a_unique_sign_repair() {
    let (csv, manifest) = fixture("near_pi");
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: &manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    assert!(report.repairs.is_empty());
    assert_eq!(report.findings[0].repair_disposition.as_str(), "unsafe");
}
