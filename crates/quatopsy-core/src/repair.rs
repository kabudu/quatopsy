//! Repair candidates. Proposals never overwrite source inputs.

use quatopsy_schema::{
    ALG_NORMALISE, ALG_SIGN_LIFT, ALG_VERSION, ComponentOrder, Declarations, Finding, FiniteF64,
    NEAR_ZERO_NORM, NORM_ABS_TOLERANCE, REPAIR_MATRIX_ABS_TOLERANCE, RULE_NORM, RULE_PI,
    RULE_REPAIR, RULE_SIGN, RULE_TIME, RULE_UNWIND, RULE_VERSION, Repair, RepairDisposition,
    RuleResult, RuleState,
};

use crate::ingest::Sample;
use crate::math::{Quaternion, lift_next};

#[derive(Debug, Clone)]
pub struct RepairPlan {
    pub repair: Repair,
    pub quaternions: Vec<Quaternion>,
}

pub fn attach_repairs(
    samples: &[Sample],
    findings: &mut [Finding],
    analysis_id: &str,
) -> (Vec<Repair>, RuleResult) {
    let mut repairs = Vec::new();
    if let Some(plan) = sign_lift_plan(samples, analysis_id) {
        link_findings(
            findings,
            RULE_SIGN,
            &plan.repair.id,
            RepairDisposition::Proposed,
        );
        repairs.push(plan.repair);
    }
    if let Some(plan) = normalise_plan(samples, analysis_id) {
        link_findings(
            findings,
            RULE_NORM,
            &plan.repair.id,
            RepairDisposition::Proposed,
        );
        repairs.push(plan.repair);
    }
    for finding in findings.iter_mut() {
        if finding.repair_disposition != RepairDisposition::None {
            continue;
        }
        finding.repair_disposition = match finding.rule.as_str() {
            RULE_SIGN | RULE_NORM if finding.reason_code == "off-unit-sample" => {
                RepairDisposition::Inapplicable
            }
            RULE_PI => RepairDisposition::Unsafe,
            RULE_NORM => RepairDisposition::Unsafe,
            RULE_TIME => RepairDisposition::Inapplicable,
            RULE_UNWIND => RepairDisposition::Inapplicable,
            _ => RepairDisposition::Inapplicable,
        };
    }
    (
        repairs,
        RuleResult {
            rule: RULE_REPAIR.to_string(),
            version: RULE_VERSION.to_string(),
            state: RuleState::Pass,
            finding_count: 0,
            truncated: false,
            reason_code: "dispositions-complete".to_string(),
        },
    )
}

pub fn sign_lift_plan(samples: &[Sample], analysis_id: &str) -> Option<RepairPlan> {
    if samples.is_empty() {
        return None;
    }
    let mut quaternions: Vec<Quaternion> = samples.iter().map(|sample| sample.raw).collect();
    let mut affected = Vec::new();
    let first_unit = samples[0].raw.normalized()?;
    let mut lifted = first_unit;
    for idx in 1..samples.len() {
        let next_unit = samples[idx].raw.normalized()?;
        let decision = lift_next(lifted, next_unit);
        if decision.flipped {
            quaternions[idx] = samples[idx].raw.negate();
            affected.push(samples[idx].source_row);
        }
        lifted = decision.lifted;
    }
    if affected.is_empty() {
        return None;
    }
    Some(RepairPlan {
        repair: Repair {
            id: "repair:sign-lift:1".to_string(),
            algorithm: ALG_SIGN_LIFT.to_string(),
            algorithm_version: ALG_VERSION.to_string(),
            source_analysis_id: analysis_id.to_string(),
            disposition: RepairDisposition::Proposed,
            physical_orientation_equivalent: true,
            affected_rows: affected,
            preconditions: vec![
                "finite-non-zero-samples".to_string(),
                "no-unique-claim-on-near-pi-ties".to_string(),
            ],
            numeric_tolerance: FiniteF64::new(REPAIR_MATRIX_ABS_TOLERANCE).expect("tol"),
            max_norm_delta: None,
            output_digest: None,
        },
        quaternions,
    })
}

pub fn normalise_plan(samples: &[Sample], analysis_id: &str) -> Option<RepairPlan> {
    let mut quaternions: Vec<Quaternion> = samples.iter().map(|sample| sample.raw).collect();
    let mut affected = Vec::new();
    let mut max_delta = 0.0_f64;
    for (idx, sample) in samples.iter().enumerate() {
        if !sample.raw.is_finite() {
            continue;
        }
        let norm = sample.raw.norm();
        if norm == 0.0 || norm < NEAR_ZERO_NORM {
            continue;
        }
        if libm::fabs(norm - 1.0) <= NORM_ABS_TOLERANCE {
            continue;
        }
        let Some(unit) = sample.raw.normalized() else {
            continue;
        };
        quaternions[idx] = unit;
        affected.push(sample.source_row);
        max_delta = max_delta.max(libm::fabs(norm - 1.0));
    }
    if affected.is_empty() {
        return None;
    }
    Some(RepairPlan {
        repair: Repair {
            id: "repair:normalise:1".to_string(),
            algorithm: ALG_NORMALISE.to_string(),
            algorithm_version: ALG_VERSION.to_string(),
            source_analysis_id: analysis_id.to_string(),
            disposition: RepairDisposition::Proposed,
            physical_orientation_equivalent: true,
            affected_rows: affected,
            preconditions: vec!["finite-non-zero-off-unit".to_string()],
            numeric_tolerance: FiniteF64::new(NORM_ABS_TOLERANCE).expect("tol"),
            max_norm_delta: FiniteF64::new(max_delta).ok(),
            output_digest: None,
        },
        quaternions,
    })
}

pub fn plan_by_id(samples: &[Sample], analysis_id: &str, repair_id: &str) -> Option<RepairPlan> {
    match repair_id {
        "repair:sign-lift:1" => sign_lift_plan(samples, analysis_id),
        "repair:normalise:1" => normalise_plan(samples, analysis_id),
        _ => None,
    }
}

pub fn render_repaired_csv(
    csv_bytes: &[u8],
    declarations: &Declarations,
    samples: &[Sample],
    quaternions: &[Quaternion],
) -> Result<Vec<u8>, String> {
    if samples.len() != quaternions.len() {
        return Err("repair series length does not match samples".to_string());
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(csv_bytes);
    let headers = reader.headers().map_err(|err| err.to_string())?.clone();
    let mut by_row = std::collections::HashMap::new();
    for (sample, quat) in samples.iter().zip(quaternions.iter()) {
        by_row.insert(sample.source_row, *quat);
    }
    let time_idx = header_index(&headers, &declarations.time_column)?;
    let quat_idx = [
        header_index(&headers, &declarations.quaternion_columns[0])?,
        header_index(&headers, &declarations.quaternion_columns[1])?,
        header_index(&headers, &declarations.quaternion_columns[2])?,
        header_index(&headers, &declarations.quaternion_columns[3])?,
    ];
    let mut out = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new()
            .flexible(false)
            .from_writer(&mut out);
        writer
            .write_record(&headers)
            .map_err(|err| err.to_string())?;
        for record in reader.records() {
            let record = record.map_err(|err| err.to_string())?;
            let line = record.position().map(|pos| pos.line()).unwrap_or(0);
            let mut fields: Vec<String> = record.iter().map(str::to_string).collect();
            if let Some(quat) = by_row.get(&line) {
                let ordered = order_components(declarations.component_order, *quat);
                fields[time_idx] = record.get(time_idx).unwrap_or("").to_string();
                for (slot, value) in quat_idx.iter().zip(ordered.iter()) {
                    fields[*slot] = format_number(*value);
                }
            }
            writer
                .write_record(&fields)
                .map_err(|err| err.to_string())?;
        }
        writer.flush().map_err(|err| err.to_string())?;
    }
    Ok(out)
}

fn link_findings(
    findings: &mut [Finding],
    rule: &str,
    repair_id: &str,
    disposition: RepairDisposition,
) {
    for finding in findings.iter_mut() {
        let matches =
            finding.rule == rule && (rule != RULE_NORM || finding.reason_code == "off-unit-sample");
        if matches {
            finding.repair_disposition = disposition;
            if !finding.repair_refs.iter().any(|id| id == repair_id) {
                finding.repair_refs.push(repair_id.to_string());
            }
        }
    }
}

fn header_index(headers: &csv::StringRecord, name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|item| item == name)
        .ok_or_else(|| format!("missing column {name}"))
}

fn order_components(order: ComponentOrder, quat: Quaternion) -> [f64; 4] {
    match order {
        ComponentOrder::Wxyz => [quat.w, quat.x, quat.y, quat.z],
        ComponentOrder::Xyzw => [quat.x, quat.y, quat.z, quat.w],
    }
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        format!("{value}")
    }
}
