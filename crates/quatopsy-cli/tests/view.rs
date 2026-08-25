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
        "quatopsy-view-test-{}-{}-{}",
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

fn analyze(case: &str, tmp: &std::path::Path) -> PathBuf {
    let root = workspace_root();
    let report = tmp.join(format!("{case}.json"));
    let status = Command::new(bin())
        .args([
            "analyze",
            "--input",
            root.join(format!("fixtures/conformance/{case}/input.csv"))
                .to_str()
                .unwrap(),
            "--manifest",
            root.join(format!("fixtures/conformance/{case}/manifest.json"))
                .to_str()
                .unwrap(),
            "--report",
            report.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.code().is_some());
    report
}

fn view_bundle(case: &str, report: &std::path::Path, tmp: &std::path::Path) -> String {
    let root = workspace_root();
    let out = tmp.join(format!("{case}-view"));
    let status = Command::new(bin())
        .args([
            "view",
            "--report",
            report.to_str().unwrap(),
            "--input",
            root.join(format!("fixtures/conformance/{case}/input.csv"))
                .to_str()
                .unwrap(),
            "--manifest",
            root.join(format!("fixtures/conformance/{case}/manifest.json"))
                .to_str()
                .unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0), "view {case}");
    fs::read_to_string(out.join("index.html")).unwrap()
}

#[test]
fn five_initial_defect_stories_appear_in_the_viewer_bundle() {
    let tmp = tempfile_dir();
    let stories = [
        ("sign_alternating", "findings", "sign-discontinuity"),
        ("near_pi", "findings", "near-pi-ambiguity"),
        ("norm_drift", "findings", "off-unit-sample"),
        ("time_decreasing", "refused", "decreasing-timestamp"),
        ("rate_quarter_turn", "pass", "QAT-RATE-001"),
    ];
    for (case, result, needle) in stories {
        let report = analyze(case, &tmp);
        let html = view_bundle(case, &report, &tmp);
        assert!(
            html.contains(&format!("\"result\":\"{result}\"")),
            "{case} result"
        );
        assert!(html.contains(needle), "{case} missing {needle}");
        assert!(html.contains("source_row"), "{case} sample identity");
        assert!(
            html.contains("projection artefact"),
            "{case} projection label"
        );
        assert!(html.contains("Measured raw samples"), "{case} raw layer");
        assert!(html.contains("Derived body axes"), "{case} derived layer");
        assert!(
            html.contains("Proposed data is a candidate overlay"),
            "{case} proposed layer"
        );
        assert!(html.contains("content=\"default-src 'none'"));
        assert!(html.contains("connect-src 'none'"));
    }
}

#[test]
fn unknown_report_schema_is_refused_by_the_viewer_bundle() {
    let tmp = tempfile_dir();
    let report = tmp.join("bad.json");
    fs::write(
        &report,
        r#"{"schema":"quatopsy.report/99","result":"pass"}"#,
    )
    .unwrap();
    let out = tmp.join("bad-view");
    let status = Command::new(bin())
        .args([
            "view",
            "--report",
            report.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    let html = fs::read_to_string(out.join("index.html")).unwrap();
    let js = fs::read_to_string(out.join("viewer.js")).unwrap();
    assert!(js.contains("Viewer refused unknown report schema"));
    assert!(html.contains("quatopsy.report/99"));
    assert!(!js.contains("http://"));
    assert!(!js.contains("https://"));
    assert!(!js.contains("fetch("));
}

#[test]
fn viewer_assets_meet_keyboard_contrast_and_text_state_requirements() {
    let root = workspace_root();
    let html = fs::read_to_string(root.join("viewer/index.html")).unwrap();
    let css = fs::read_to_string(root.join("viewer/viewer.css")).unwrap();
    let js = fs::read_to_string(root.join("viewer/viewer.js")).unwrap();
    assert!(html.contains("aria-label=\"Quatopsy\""));
    assert!(html.contains("data-brand-version=\"quatopsy.brand/2\""));
    assert!(html.contains("See where rotations go wrong."));
    assert!(html.contains("brand-ribbon-a"));
    assert!(html.contains("#8b5cf6") && html.contains("#d946ef"));
    assert!(html.contains("#58104f") && html.contains("#ec48c6"));
    assert!(html.contains("#f5c96a") && html.contains("#fff8e8"));
    assert!(html.contains("viewBox=\"0 0 32 32\""));
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("aria-label=\"Trajectory playback controls\""));
    assert!(html.contains("id=\"play\""));
    assert!(html.contains("id=\"timeline\"") && html.contains("tabindex=\"0\""));
    assert!(html.contains("Evidence queue"));
    assert!(html.contains("Non-physical view"));
    assert!(css.contains("prefers-reduced-motion"));
    assert!(css.contains("forced-colors"));
    assert!(css.contains("max-width: 480px"));
    assert!(js.contains("ArrowRight"));
    assert!(js.contains("ArrowLeft"));
    assert!(js.contains("timeline.addEventListener(\"click\""));
    assert!(js.contains("window.setInterval"));
    assert!(js.contains("if (reduced || samples.length < 2)"));
    assert!(js.contains("the viewer did not recompute rules"));
    assert!(!js.contains("alert("));
    assert!(!js.contains("confirm("));
    assert!(!js.contains("prompt("));
    assert!(contrast(0xff, 0xf1, 0xd6, 0x09, 0x07, 0x0d) >= 4.5);
    assert!(contrast(0xff, 0x77, 0x70, 0x09, 0x07, 0x0d) >= 4.5);
    assert!(contrast(0xb9, 0xae, 0xbf, 0x09, 0x07, 0x0d) >= 4.5);
}

fn lin(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f64 {
    let l1 = 0.2126 * lin(r1) + 0.7152 * lin(g1) + 0.0722 * lin(b1);
    let l2 = 0.2126 * lin(r2) + 0.7152 * lin(g2) + 0.0722 * lin(b2);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}
