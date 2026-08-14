//! Release-profile budget for one million samples. Ignored in default `cargo test`.

use std::time::Instant;

use quatopsy_core::limits::Limits;
use quatopsy_core::{AnalyzeRequest, analyze};
use quatopsy_schema::ResultState;

#[test]
#[ignore]
fn million_samples_meet_budget() {
    let n = 1_000_000_usize;
    let mut csv = Vec::with_capacity(n * 16);
    csv.extend_from_slice(b"t,qw,qx,qy,qz\n");
    for i in 0..n {
        csv.extend_from_slice(i.to_string().as_bytes());
        csv.extend_from_slice(b",1,0,0,0\n");
    }
    let manifest = br#"{
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "BODY",
        "frame_to": "J2000",
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qw", "qx", "qy", "qz"]}
    }"#;
    let started = Instant::now();
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv,
        manifest_bytes: manifest,
        engine_version: "0.1.0",
        limits: Limits::defaults(),
        cancelled: None,
    });
    let elapsed = started.elapsed().as_secs_f64();
    assert_eq!(report.result, ResultState::Pass);
    assert_eq!(report.input.sample_count, n as u64);
    assert!(
        elapsed < 10.0,
        "1e6 samples took {elapsed} s, budget is 10 s"
    );
    #[cfg(unix)]
    {
        let rss = max_rss_bytes();
        assert!(
            rss < 512 * 1024 * 1024,
            "peak RSS {rss} bytes exceeds 512 MiB"
        );
    }
}

#[cfg(unix)]
fn max_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let rc = unsafe {
        // SAFETY: usage points to uninit rusage; getrusage writes the full struct.
        libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr())
    };
    assert_eq!(rc, 0);
    let rss = unsafe {
        // SAFETY: getrusage succeeded, so usage is initialised.
        usage.assume_init()
    }
    .ru_maxrss;
    if cfg!(target_os = "macos") {
        rss as u64
    } else {
        rss as u64 * 1024
    }
}
