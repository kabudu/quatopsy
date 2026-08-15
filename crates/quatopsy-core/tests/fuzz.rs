//! Seeded trajectory generator. Crashes, refusals, and errors must never become pass.

use quatopsy_core::limits::Limits;
use quatopsy_core::{AnalyzeRequest, analyze};
use quatopsy_schema::ResultState;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }
    fn f64(&mut self) -> f64 {
        (self.next() as f64) / (u64::MAX as f64)
    }
}

#[test]
fn seeded_random_paths_never_promote_failure_to_pass() {
    let manifest = br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","columns":{"time":"t","quaternion":["qw","qx","qy","qz"]}}"#;
    let mut rng = Rng(0xC0FFEE);
    for case in 0..256 {
        let n = 3 + (rng.next() % 12) as usize;
        let mut csv = String::from("t,qw,qx,qy,qz\n");
        let mut t = 0.0;
        for _ in 0..n {
            t += 0.1 + rng.f64();
            let w = rng.f64() * 2.0 - 1.0;
            let x = rng.f64() * 2.0 - 1.0;
            let y = rng.f64() * 2.0 - 1.0;
            let z = rng.f64() * 2.0 - 1.0;
            csv.push_str(&format!("{t},{w},{x},{y},{z}\n"));
        }
        let report = analyze(AnalyzeRequest {
            csv_bytes: csv.as_bytes(),
            manifest_bytes: manifest,
            engine_version: "0.1.0",
            limits: Limits::defaults(),
            cancelled: None,
        });
        if report.result == ResultState::Pass {
            assert!(report.findings.is_empty(), "case {case} pass with findings");
            assert!(report.diagnostics.complete);
        } else {
            assert_ne!(
                report.result,
                ResultState::Pass,
                "case {case} non-success must stay non-pass"
            );
        }
        assert_eq!(
            report.rule_results.len(),
            quatopsy_schema::enabled_rules().len()
        );
    }
}
