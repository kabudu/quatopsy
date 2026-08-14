//! Closed V1 rule registry, lift construction, and fail-closed aggregation.

use std::cmp::Ordering;

use quatopsy_schema::{
    Confidence, Evidence, Finding, FindingClass, FiniteF64, NEAR_ZERO_NORM, NORM_ABS_TOLERANCE,
    RULE_LIFT, RULE_NORM, RULE_PI, RULE_RATE, RULE_SIGN, RULE_TIME, RULE_UNWIND, RULE_VERSION,
    RateSummary, ResultState, RuleResult, RuleState, Severity, UNWIND_ABS_TOLERANCE,
};

use crate::cancel::Cancel;
use crate::ingest::Sample;
use crate::limits::Limits;
use crate::math::{Quaternion, covering_angle, lift_next, quotient_angle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormKind {
    Ok,
    OffUnit,
    NearZero,
    Zero,
    NonFinite,
}

#[derive(Debug, Clone)]
struct PreparedSample {
    sample: Sample,
    norm: f64,
    kind: NormKind,
    unit: Option<Quaternion>,
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub result: ResultState,
    pub rule_results: Vec<RuleResult>,
    pub findings: Vec<Finding>,
    pub rate_summary: Option<RateSummary>,
    pub conditioning: Vec<String>,
    pub complete: bool,
    pub reason_code: String,
    pub message: String,
}

pub fn evaluate(samples: &[Sample], limits: Limits, cancel: Cancel<'_>) -> Analysis {
    let mut findings: Vec<Finding> = Vec::new();
    let mut conditioning = Vec::new();
    let mut truncated_error = false;

    if cancel.check().is_err() {
        let reason = if cancel.is_cancelled() {
            "cancelled"
        } else {
            "timeout"
        };
        return incomplete(reason, "analysis did not complete");
    }

    let prepared: Vec<PreparedSample> = samples.iter().cloned().map(prepare_sample).collect();

    let time = evaluate_time(&prepared, limits, &mut findings, &mut truncated_error);
    let norm = evaluate_norm(
        &prepared,
        limits,
        &mut findings,
        &mut truncated_error,
        &mut conditioning,
    );

    let rotation_ready = prepared.iter().all(|item| item.unit.is_some())
        && prepared
            .iter()
            .all(|item| item.sample.timestamp_finite && !item.sample.timestamp_overflow);

    let (lift, sign, pi) = if rotation_ready && !prepared.is_empty() {
        evaluate_lift_family(&prepared, limits, &mut findings, &mut truncated_error)
    } else {
        let refused = |rule: &str, reason: &str| RuleResult {
            rule: rule.to_string(),
            version: RULE_VERSION.to_string(),
            state: RuleState::Refused,
            finding_count: 0,
            truncated: false,
            reason_code: reason.to_string(),
        };
        (
            refused(RULE_LIFT, "sequence-not-liftable"),
            refused(RULE_SIGN, "sequence-not-liftable"),
            refused(RULE_PI, "sequence-not-liftable"),
        )
    };

    let (rate, rate_summary) = evaluate_rate(&prepared, rotation_ready);
    let unwind = evaluate_unwind(&prepared, limits, &mut findings, &mut truncated_error);

    let mut rule_results = vec![norm, time, lift, sign, rate, pi, unwind];
    if truncated_error {
        for result in &mut rule_results {
            if result.truncated {
                result.state = RuleState::Error;
                result.reason_code = "finding-cap-exceeded".to_string();
            }
        }
    }

    findings.sort_by(|a, b| {
        a.source_row_start
            .cmp(&b.source_row_start)
            .then_with(|| a.rule.cmp(&b.rule))
            .then_with(|| a.id.cmp(&b.id))
    });

    let result = quatopsy_schema::aggregate_result(rule_results.iter().map(|item| item.state));
    Analysis {
        result,
        rule_results,
        findings,
        rate_summary,
        conditioning,
        complete: !truncated_error,
        reason_code: result.as_str().to_string(),
        message: match result {
            ResultState::Pass => "all enabled rules passed".to_string(),
            ResultState::Findings => "one or more obligations were violated".to_string(),
            ResultState::Refused => "declared semantics or values are unsupported".to_string(),
            ResultState::Error => "analysis could not complete".to_string(),
        },
    }
}

fn incomplete(reason_code: &'static str, message: &str) -> Analysis {
    let refused = |rule: &str| RuleResult {
        rule: rule.to_string(),
        version: RULE_VERSION.to_string(),
        state: RuleState::Error,
        finding_count: 0,
        truncated: false,
        reason_code: reason_code.to_string(),
    };
    Analysis {
        result: ResultState::Error,
        rule_results: quatopsy_schema::enabled_rules()
            .into_iter()
            .map(refused)
            .collect(),
        findings: Vec::new(),
        rate_summary: None,
        conditioning: Vec::new(),
        complete: false,
        reason_code: reason_code.to_string(),
        message: message.to_string(),
    }
}

fn prepare_sample(sample: Sample) -> PreparedSample {
    if !sample.raw.is_finite() {
        return PreparedSample {
            sample,
            norm: f64::NAN,
            kind: NormKind::NonFinite,
            unit: None,
        };
    }
    let norm = sample.raw.norm();
    let kind = if norm == 0.0 {
        NormKind::Zero
    } else if norm < NEAR_ZERO_NORM {
        NormKind::NearZero
    } else if libm::fabs(norm - 1.0) > NORM_ABS_TOLERANCE {
        NormKind::OffUnit
    } else {
        NormKind::Ok
    };
    let unit = match kind {
        NormKind::Ok | NormKind::OffUnit => sample.raw.normalized(),
        NormKind::Zero | NormKind::NearZero | NormKind::NonFinite => None,
    };
    PreparedSample {
        sample,
        norm,
        kind,
        unit,
    }
}

fn evaluate_time(
    samples: &[PreparedSample],
    limits: Limits,
    findings: &mut Vec<Finding>,
    truncated: &mut bool,
) -> RuleResult {
    let mut state = RuleState::Pass;
    let mut count = 0_u64;
    for (idx, item) in samples.iter().enumerate() {
        if !item.sample.timestamp_finite || item.sample.timestamp_overflow {
            let reason = if item.sample.timestamp_overflow {
                "timestamp-overflow"
            } else {
                "non-finite-timestamp"
            };
            if push_finding(
                findings,
                limits,
                truncated,
                &mut count,
                time_finding(
                    item,
                    item,
                    reason,
                    "timestamp is not a finite nanosecond value",
                ),
            ) {
                state = RuleState::Refused;
            }
        }
        if idx == 0 {
            continue;
        }
        let prev = &samples[idx - 1];
        if !prev.sample.timestamp_finite
            || !item.sample.timestamp_finite
            || prev.sample.timestamp_overflow
            || item.sample.timestamp_overflow
        {
            continue;
        }
        match item.sample.timestamp_ns.cmp(&prev.sample.timestamp_ns) {
            Ordering::Less => {
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    time_finding(
                        prev,
                        item,
                        "decreasing-timestamp",
                        "timestamps are not monotonically increasing",
                    ),
                ) {
                    state = RuleState::Refused;
                }
            }
            Ordering::Equal => {
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    time_finding(
                        prev,
                        item,
                        "duplicate-timestamp",
                        "timestamps contain a duplicate value",
                    ),
                ) {
                    state = RuleState::Refused;
                }
            }
            Ordering::Greater => {}
        }
    }
    rule_result(
        RULE_TIME,
        state,
        count,
        *truncated && state != RuleState::Pass,
    )
}

fn evaluate_norm(
    samples: &[PreparedSample],
    limits: Limits,
    findings: &mut Vec<Finding>,
    truncated: &mut bool,
    conditioning: &mut Vec<String>,
) -> RuleResult {
    let mut state = RuleState::Pass;
    let mut count = 0_u64;
    for item in samples {
        match item.kind {
            NormKind::Ok => {}
            NormKind::OffUnit => {
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    norm_finding(
                        item,
                        FindingClass::InvalidData,
                        Severity::Medium,
                        Confidence::NumericTolerance,
                        "off-unit-sample",
                        "quaternion sample is finite and non-zero but off unit",
                    ),
                ) {
                    promote(&mut state, RuleState::Finding);
                }
            }
            NormKind::NearZero => {
                conditioning.push(format!("near-zero:row-{}", item.sample.source_row));
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    norm_finding(
                        item,
                        FindingClass::InvalidData,
                        Severity::High,
                        Confidence::NumericTolerance,
                        "near-zero-sample",
                        "quaternion sample is numerically near zero",
                    ),
                ) {
                    promote(&mut state, RuleState::Refused);
                }
            }
            NormKind::Zero => {
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    norm_finding(
                        item,
                        FindingClass::InvalidData,
                        Severity::High,
                        Confidence::Exact,
                        "zero-sample",
                        "quaternion sample has zero norm",
                    ),
                ) {
                    promote(&mut state, RuleState::Refused);
                }
            }
            NormKind::NonFinite => {
                if push_finding(
                    findings,
                    limits,
                    truncated,
                    &mut count,
                    norm_finding(
                        item,
                        FindingClass::InvalidData,
                        Severity::High,
                        Confidence::Exact,
                        "non-finite-sample",
                        "quaternion sample contains a non-finite component",
                    ),
                ) {
                    promote(&mut state, RuleState::Refused);
                }
            }
        }
    }
    rule_result(RULE_NORM, state, count, false)
}

fn evaluate_lift_family(
    samples: &[PreparedSample],
    limits: Limits,
    findings: &mut Vec<Finding>,
    truncated: &mut bool,
) -> (RuleResult, RuleResult, RuleResult) {
    let mut sign_count = 0_u64;
    let mut pi_count = 0_u64;
    let mut sign_state = RuleState::Pass;
    let mut pi_state = RuleState::Pass;
    let Some(first_unit) = samples[0].unit else {
        return (
            rule_result(RULE_LIFT, RuleState::Refused, 0, false),
            rule_result(RULE_SIGN, RuleState::Refused, 0, false),
            rule_result(RULE_PI, RuleState::Refused, 0, false),
        );
    };
    let mut lifted = first_unit;
    for idx in 1..samples.len() {
        let prev = &samples[idx - 1];
        let next = &samples[idx];
        let Some(prev_unit) = prev.unit else {
            continue;
        };
        let Some(next_unit) = next.unit else {
            continue;
        };
        let raw_dot = prev_unit.dot(next_unit);
        let decision = lift_next(lifted, next_unit);
        if decision.near_pi {
            let recorded = push_finding(
                findings,
                limits,
                truncated,
                &mut pi_count,
                pair_finding(
                    prev,
                    next,
                    RULE_PI,
                    FindingClass::PhysicalDiscontinuity,
                    Severity::High,
                    Confidence::NumericTolerance,
                    "near-pi-ambiguity",
                    "adjacent samples are numerically ambiguous at pi radians",
                    vec![
                        evidence("unit-dot", "1", decision.unit_dot),
                        evidence(
                            "quotient-angle-rad",
                            "rad",
                            quotient_angle(lifted, next_unit),
                        ),
                    ],
                ),
            );
            if recorded {
                promote(&mut pi_state, RuleState::Finding);
            }
        } else if raw_dot < 0.0 {
            let recorded = push_finding(
                findings,
                limits,
                truncated,
                &mut sign_count,
                pair_finding(
                    prev,
                    next,
                    RULE_SIGN,
                    FindingClass::RepresentationDiscontinuity,
                    Severity::Medium,
                    Confidence::Exact,
                    "sign-discontinuity",
                    "raw quaternion sign is antipodal to the neighbouring sample",
                    vec![
                        evidence("unit-dot", "1", raw_dot),
                        evidence(
                            "quotient-angle-rad",
                            "rad",
                            quotient_angle(prev_unit, next_unit),
                        ),
                    ],
                ),
            );
            if recorded {
                promote(&mut sign_state, RuleState::Finding);
            }
        }
        lifted = decision.lifted;
    }
    (
        rule_result(RULE_LIFT, RuleState::Pass, 0, false),
        rule_result(RULE_SIGN, sign_state, sign_count, false),
        rule_result(RULE_PI, pi_state, pi_count, false),
    )
}

fn evaluate_rate(
    samples: &[PreparedSample],
    rotation_ready: bool,
) -> (RuleResult, Option<RateSummary>) {
    if !rotation_ready {
        return (rule_result(RULE_RATE, RuleState::Refused, 0, false), None);
    }
    let mut min_angle = f64::INFINITY;
    let mut max_angle = 0.0_f64;
    let mut min_rate = f64::INFINITY;
    let mut max_rate = 0.0_f64;
    let mut count = 0_u64;
    for idx in 1..samples.len() {
        let prev = &samples[idx - 1];
        let next = &samples[idx];
        let dt_ns = next
            .sample
            .timestamp_ns
            .saturating_sub(prev.sample.timestamp_ns);
        if dt_ns <= 0 {
            return (rule_result(RULE_RATE, RuleState::Refused, 0, false), None);
        }
        let Some(p) = prev.unit else {
            return (rule_result(RULE_RATE, RuleState::Refused, 0, false), None);
        };
        let Some(q) = next.unit else {
            return (rule_result(RULE_RATE, RuleState::Refused, 0, false), None);
        };
        let angle = quotient_angle(p, q);
        let dt_s = dt_ns as f64 / 1_000_000_000.0;
        let rate = angle / dt_s;
        min_angle = min_angle.min(angle);
        max_angle = max_angle.max(angle);
        min_rate = min_rate.min(rate);
        max_rate = max_rate.max(rate);
        count += 1;
    }
    if count == 0 {
        return (rule_result(RULE_RATE, RuleState::Refused, 0, false), None);
    }
    let summary = RateSummary {
        interval_count: count,
        min_angle_rad: FiniteF64::new(min_angle)
            .unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        max_angle_rad: FiniteF64::new(max_angle)
            .unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        min_rate_rad_s: FiniteF64::new(min_rate)
            .unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        max_rate_rad_s: FiniteF64::new(max_rate)
            .unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
    };
    (
        RuleResult {
            rule: RULE_RATE.to_string(),
            version: RULE_VERSION.to_string(),
            state: RuleState::Pass,
            finding_count: 0,
            truncated: false,
            reason_code: "rates-derived".to_string(),
        },
        Some(summary),
    )
}

fn evaluate_unwind(
    prepared: &[PreparedSample],
    limits: Limits,
    findings: &mut Vec<Finding>,
    truncated: &mut bool,
) -> RuleResult {
    let supplied = prepared.iter().any(|item| item.sample.commanded.is_some());
    if !supplied {
        return RuleResult {
            rule: RULE_UNWIND.to_string(),
            version: RULE_VERSION.to_string(),
            state: RuleState::Pass,
            finding_count: 0,
            truncated: false,
            reason_code: "commanded-path-absent".to_string(),
        };
    }
    if prepared.iter().any(|item| item.sample.commanded.is_none()) {
        return rule_result(RULE_UNWIND, RuleState::Refused, 0, false);
    }
    let mut count = 0_u64;
    let mut local_trunc = false;
    for window in prepared.windows(2) {
        let Some(prev) = window[0].sample.commanded.and_then(Quaternion::normalized) else {
            return rule_result(RULE_UNWIND, RuleState::Refused, 0, false);
        };
        let Some(next) = window[1].sample.commanded.and_then(Quaternion::normalized) else {
            return rule_result(RULE_UNWIND, RuleState::Refused, 0, false);
        };
        let shortest = quotient_angle(prev, next);
        let covering = covering_angle(prev, next);
        if covering > shortest + UNWIND_ABS_TOLERANCE
            && !push_finding(
                findings,
                limits,
                &mut local_trunc,
                &mut count,
                pair_finding(
                    &window[0],
                    &window[1],
                    RULE_UNWIND,
                    FindingClass::DynamicThreshold,
                    Severity::Medium,
                    Confidence::NumericTolerance,
                    "commanded-long-way",
                    "commanded adjacent covering exceeds the quotient-shortest rotation",
                    vec![
                        evidence("commanded_covering_rad", "rad", covering),
                        evidence("shortest_angle_rad", "rad", shortest),
                    ],
                ),
            )
        {
            break;
        }
    }
    *truncated = *truncated || local_trunc;
    let state = if local_trunc {
        RuleState::Error
    } else if count > 0 {
        RuleState::Finding
    } else {
        RuleState::Pass
    };
    RuleResult {
        rule: RULE_UNWIND.to_string(),
        version: RULE_VERSION.to_string(),
        state,
        finding_count: count,
        truncated: local_trunc,
        reason_code: if count > 0 {
            "commanded-long-way".to_string()
        } else {
            "commanded-path-shortest".to_string()
        },
    }
}

fn promote(current: &mut RuleState, candidate: RuleState) {
    let next = quatopsy_schema::aggregate_result([*current, candidate]);
    *current = match next {
        ResultState::Pass => RuleState::Pass,
        ResultState::Findings => RuleState::Finding,
        ResultState::Refused => RuleState::Refused,
        ResultState::Error => RuleState::Error,
    };
}

fn rule_result(rule: &str, state: RuleState, finding_count: u64, truncated: bool) -> RuleResult {
    RuleResult {
        rule: rule.to_string(),
        version: RULE_VERSION.to_string(),
        state,
        finding_count,
        truncated,
        reason_code: state.as_str().to_string(),
    }
}

fn push_finding(
    findings: &mut Vec<Finding>,
    limits: Limits,
    truncated: &mut bool,
    count: &mut u64,
    finding: Finding,
) -> bool {
    if *count >= limits.max_findings_per_rule {
        *truncated = true;
        return false;
    }
    *count += 1;
    findings.push(finding);
    true
}

fn time_finding(
    start: &PreparedSample,
    end: &PreparedSample,
    reason: &str,
    summary: &str,
) -> Finding {
    pair_finding(
        start,
        end,
        RULE_TIME,
        FindingClass::InvalidData,
        Severity::High,
        Confidence::Exact,
        reason,
        summary,
        vec![
            evidence("t0_ns", "ns", start.sample.timestamp_ns as f64),
            evidence("t1_ns", "ns", end.sample.timestamp_ns as f64),
        ],
    )
}

fn norm_finding(
    item: &PreparedSample,
    class: FindingClass,
    severity: Severity,
    confidence: Confidence,
    reason: &str,
    summary: &str,
) -> Finding {
    let norm_value = if item.norm.is_finite() {
        item.norm
    } else {
        0.0
    };
    pair_finding(
        item,
        item,
        RULE_NORM,
        class,
        severity,
        confidence,
        reason,
        summary,
        vec![evidence("norm", "1", norm_value)],
    )
}

#[allow(clippy::too_many_arguments)]
fn pair_finding(
    start: &PreparedSample,
    end: &PreparedSample,
    rule: &str,
    class: FindingClass,
    severity: Severity,
    confidence: Confidence,
    reason: &str,
    summary: &str,
    evidence_values: Vec<Evidence>,
) -> Finding {
    Finding {
        id: format!(
            "finding:{rule}:{}:{}:{reason}",
            start.sample.source_row, end.sample.source_row
        ),
        rule: rule.to_string(),
        rule_version: RULE_VERSION.to_string(),
        class,
        severity,
        confidence,
        source_row_start: start.sample.source_row,
        source_row_end: end.sample.source_row,
        timestamp_ns_start: start.sample.timestamp_ns,
        timestamp_ns_end: end.sample.timestamp_ns,
        evidence: evidence_values,
        summary: summary.to_string(),
        reason_code: reason.to_string(),
        repair_disposition: quatopsy_schema::RepairDisposition::None,
        repair_refs: Vec::new(),
    }
}

fn evidence(name: &str, unit: &str, number: f64) -> Evidence {
    Evidence {
        name: name.to_string(),
        unit: unit.to_string(),
        number: FiniteF64::new(number)
            .unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero finite")),
    }
}
