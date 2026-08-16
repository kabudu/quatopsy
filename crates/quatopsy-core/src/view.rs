//! Non-authoritative view geometry derived from a report-bound trajectory.
//!
//! The viewer must display `report.result` unchanged. This payload only supplies
//! labelled geometry for linked plots.

use std::collections::BTreeSet;

use serde::Serialize;

use quatopsy_schema::{Finding, FiniteF64, VIEW_MAX_POINTS, VIEW_SAFE_MAX_POINTS, VIEW_SCHEMA};

use crate::ingest::Sample;
use crate::math::{Quaternion, lift_next, quotient_angle};

pub const VIEW_KIND_DERIVED: &str = "derived-geometry";
const STEREO_POLE_EPS: f64 = 1.0e-12;

#[derive(Debug, Clone, Serialize)]
pub struct ViewPayload {
    pub schema: String,
    pub analysis_id: String,
    pub kind: String,
    pub authoritative: bool,
    pub projection: String,
    pub projection_warning: String,
    pub downsample: DownsampleInfo,
    pub finding_links: Vec<ViewFindingLink>,
    pub samples: Vec<ViewSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewFindingLink {
    pub finding_id: String,
    pub source_row_start: u64,
    pub source_row_end: u64,
    pub geometry_source_row: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownsampleInfo {
    pub source_sample_count: u64,
    pub emitted_sample_count: u64,
    pub max_points: u64,
    pub retained_findings: bool,
    pub retained_extrema: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewSample {
    pub source_row: u64,
    pub timestamp_ns: i64,
    pub raw: Option<[FiniteF64; 4]>,
    pub lifted: Option<[FiniteF64; 4]>,
    pub proposed: Option<[FiniteF64; 4]>,
    pub body_x: Option<[FiniteF64; 3]>,
    pub body_y: Option<[FiniteF64; 3]>,
    pub body_z: Option<[FiniteF64; 3]>,
    pub proposed_body_x: Option<[FiniteF64; 3]>,
    pub stereo: Option<[FiniteF64; 3]>,
    pub stereo_pole: bool,
    pub angle_rad: Option<FiniteF64>,
    pub rate_rad_s: Option<FiniteF64>,
    pub pinned_finding: bool,
    pub pinned_extremum: bool,
}

pub fn build_view(
    samples: &[Sample],
    findings: &[Finding],
    analysis_id: &str,
    proposed: Option<&[Quaternion]>,
    max_points: u64,
) -> ViewPayload {
    let max_points = max_points.clamp(8, VIEW_SAFE_MAX_POINTS);
    let series = geometry_series(samples, proposed);
    let extrema = extrema_indices(&series);
    let finding_pins = finding_indices(samples, findings);
    let selected: BTreeSet<usize> =
        select_indices(series.len(), max_points as usize, &extrema, &finding_pins)
            .into_iter()
            .collect();
    let selected_rows = selected
        .iter()
        .map(|idx| samples[*idx].source_row)
        .collect::<Vec<_>>();
    let finding_links = findings
        .iter()
        .map(|finding| ViewFindingLink {
            finding_id: finding.id.clone(),
            source_row_start: finding.source_row_start,
            source_row_end: finding.source_row_end,
            geometry_source_row: nearest_selected_row(&selected_rows, finding.source_row_start),
        })
        .collect::<Vec<_>>();
    let retained_findings = finding_links.len() == findings.len()
        && finding_links
            .iter()
            .all(|link| link.geometry_source_row.is_some());
    let retained_extrema = extrema.iter().all(|idx| selected.contains(idx));
    let mut emitted = Vec::with_capacity(selected.len());
    for idx in &selected {
        let mut sample = series[*idx].clone();
        sample.pinned_finding = finding_pins.contains(idx);
        sample.pinned_extremum = extrema.contains(idx);
        emitted.push(sample);
    }
    ViewPayload {
        schema: VIEW_SCHEMA.to_string(),
        analysis_id: analysis_id.to_string(),
        kind: VIEW_KIND_DERIVED.to_string(),
        authoritative: false,
        projection: "stereographic-from-w-minus-one".to_string(),
        projection_warning:
            "The S^3 panel is a stereographic projection artefact, not a physical trajectory."
                .to_string(),
        downsample: DownsampleInfo {
            source_sample_count: samples.len() as u64,
            emitted_sample_count: emitted.len() as u64,
            max_points,
            retained_findings,
            retained_extrema,
        },
        finding_links,
        samples: emitted,
    }
}

fn nearest_selected_row(rows: &[u64], target: u64) -> Option<u64> {
    match rows.binary_search(&target) {
        Ok(index) => rows.get(index).copied(),
        Err(0) => rows.first().copied(),
        Err(index) if index >= rows.len() => rows.last().copied(),
        Err(index) => {
            let before = rows[index - 1];
            let after = rows[index];
            Some(if before.abs_diff(target) <= after.abs_diff(target) {
                before
            } else {
                after
            })
        }
    }
}

pub fn empty_view(analysis_id: &str) -> ViewPayload {
    ViewPayload {
        schema: VIEW_SCHEMA.to_string(),
        analysis_id: analysis_id.to_string(),
        kind: VIEW_KIND_DERIVED.to_string(),
        authoritative: false,
        projection: "stereographic-from-w-minus-one".to_string(),
        projection_warning:
            "The S^3 panel is a stereographic projection artefact, not a physical trajectory."
                .to_string(),
        downsample: DownsampleInfo {
            source_sample_count: 0,
            emitted_sample_count: 0,
            max_points: VIEW_MAX_POINTS,
            retained_findings: true,
            retained_extrema: true,
        },
        finding_links: Vec::new(),
        samples: Vec::new(),
    }
}

pub fn select_indices(
    n: usize,
    max_points: usize,
    extrema: &[usize],
    findings: &[usize],
) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    if n <= max_points {
        return (0..n).collect();
    }
    let mut must = BTreeSet::new();
    must.insert(0);
    must.insert(n - 1);
    for idx in extrema {
        if *idx < n {
            must.insert(*idx);
        }
    }
    let mut extra = BTreeSet::new();
    for idx in findings {
        if *idx < n {
            extra.insert(*idx);
        }
    }
    if must.len() >= max_points {
        return stride_set(&must, max_points);
    }
    let mut chosen = must.clone();
    for idx in extra {
        if chosen.len() >= max_points {
            break;
        }
        chosen.insert(idx);
    }
    if chosen.len() >= max_points {
        return chosen.into_iter().collect();
    }
    let remaining = max_points - chosen.len();
    if remaining > 0 {
        let step = (n as f64) / (remaining as f64);
        for k in 0..remaining {
            let idx = ((k as f64) * step).floor() as usize;
            chosen.insert(idx.min(n - 1));
            if chosen.len() >= max_points {
                break;
            }
        }
    }
    chosen.into_iter().collect()
}

fn stride_set(set: &BTreeSet<usize>, max_points: usize) -> Vec<usize> {
    let values: Vec<usize> = set.iter().copied().collect();
    if values.len() <= max_points {
        return values;
    }
    let mut out = BTreeSet::new();
    out.insert(values[0]);
    out.insert(values[values.len() - 1]);
    let inner = max_points.saturating_sub(2);
    if inner > 0 && values.len() > 2 {
        let step = (values.len() - 1) as f64 / (inner as f64 + 1.0);
        for k in 1..=inner {
            let idx = (k as f64 * step).round() as usize;
            out.insert(values[idx.min(values.len() - 1)]);
        }
    }
    out.into_iter().take(max_points).collect()
}

fn geometry_series(samples: &[Sample], proposed: Option<&[Quaternion]>) -> Vec<ViewSample> {
    let mut lifted_state: Option<Quaternion> = None;
    let mut prev_unit: Option<Quaternion> = None;
    let mut prev_time: Option<i64> = None;
    let mut out = Vec::with_capacity(samples.len());
    for (idx, sample) in samples.iter().enumerate() {
        let raw = finite_quat(sample.raw);
        let unit = sample.raw.normalized();
        let lifted = match (lifted_state, unit) {
            (None, Some(next)) => {
                lifted_state = Some(next);
                Some(next)
            }
            (Some(prev), Some(next)) => {
                let decision = lift_next(prev, next);
                lifted_state = Some(decision.lifted);
                Some(decision.lifted)
            }
            _ => None,
        };
        let proposed_q = proposed.and_then(|series| series.get(idx).copied());
        let proposed_unit = proposed_q.and_then(Quaternion::normalized);
        let body_x = unit.map(|q| q.rotate_vector([1.0, 0.0, 0.0]));
        let body_y = unit.map(|q| q.rotate_vector([0.0, 1.0, 0.0]));
        let body_z = unit.map(|q| q.rotate_vector([0.0, 0.0, 1.0]));
        let proposed_body_x = proposed_unit.map(|q| q.rotate_vector([1.0, 0.0, 0.0]));
        let (stereo, stereo_pole) = match lifted {
            Some(q) => stereographic(q),
            None => (None, false),
        };
        let (angle, rate) = match (prev_unit, unit, prev_time) {
            (Some(prev), Some(curr), Some(prev_t)) => {
                let angle = quotient_angle(prev, curr);
                let dt = (sample.timestamp_ns - prev_t) as f64 / 1.0e9;
                let rate = if dt > 0.0 && dt.is_finite() {
                    FiniteF64::new(angle / dt).ok()
                } else {
                    None
                };
                (FiniteF64::new(angle).ok(), rate)
            }
            _ => (None, None),
        };
        prev_unit = unit;
        prev_time = Some(sample.timestamp_ns);
        out.push(ViewSample {
            source_row: sample.source_row,
            timestamp_ns: sample.timestamp_ns,
            raw: raw.map(quat_arr),
            lifted: lifted.and_then(finite_quat).map(quat_arr),
            proposed: proposed_q.and_then(finite_quat).map(quat_arr),
            body_x: body_x.and_then(vec_arr),
            body_y: body_y.and_then(vec_arr),
            body_z: body_z.and_then(vec_arr),
            proposed_body_x: proposed_body_x.and_then(vec_arr),
            stereo: stereo.and_then(vec_arr),
            stereo_pole,
            angle_rad: angle,
            rate_rad_s: rate,
            pinned_finding: false,
            pinned_extremum: false,
        });
    }
    out
}

fn stereographic(q: Quaternion) -> (Option<[f64; 3]>, bool) {
    let denom = 1.0 + q.w;
    if libm::fabs(denom) <= STEREO_POLE_EPS {
        return (None, true);
    }
    (Some([q.x / denom, q.y / denom, q.z / denom]), false)
}

fn extrema_indices(series: &[ViewSample]) -> Vec<usize> {
    let mut min_angle = (f64::INFINITY, None);
    let mut max_angle = (f64::NEG_INFINITY, None);
    let mut min_rate = (f64::INFINITY, None);
    let mut max_rate = (f64::NEG_INFINITY, None);
    for (idx, sample) in series.iter().enumerate() {
        if let Some(angle) = sample.angle_rad {
            let value = angle.get();
            if value < min_angle.0 {
                min_angle = (value, Some(idx));
            }
            if value > max_angle.0 {
                max_angle = (value, Some(idx));
            }
        }
        if let Some(rate) = sample.rate_rad_s {
            let value = rate.get();
            if value < min_rate.0 {
                min_rate = (value, Some(idx));
            }
            if value > max_rate.0 {
                max_rate = (value, Some(idx));
            }
        }
    }
    [min_angle.1, max_angle.1, min_rate.1, max_rate.1]
        .into_iter()
        .flatten()
        .collect()
}

fn finding_indices(samples: &[Sample], findings: &[Finding]) -> Vec<usize> {
    let mut by_row = std::collections::HashMap::new();
    for (idx, sample) in samples.iter().enumerate() {
        by_row.insert(sample.source_row, idx);
    }
    let mut pins = BTreeSet::new();
    for finding in findings {
        if let Some(idx) = nearest_index(&by_row, finding.source_row_start) {
            pins.insert(idx);
        }
        if let Some(idx) = nearest_index(&by_row, finding.source_row_end) {
            pins.insert(idx);
        }
    }
    pins.into_iter().collect()
}

fn nearest_index(by_row: &std::collections::HashMap<u64, usize>, row: u64) -> Option<usize> {
    if let Some(idx) = by_row.get(&row) {
        return Some(*idx);
    }
    by_row
        .iter()
        .min_by_key(|(sample_row, _)| sample_row.abs_diff(row))
        .map(|(_, idx)| *idx)
}

fn finite_quat(q: Quaternion) -> Option<Quaternion> {
    q.is_finite().then_some(q)
}

fn quat_arr(q: Quaternion) -> [FiniteF64; 4] {
    [
        FiniteF64::new(q.w).unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        FiniteF64::new(q.x).unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        FiniteF64::new(q.y).unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
        FiniteF64::new(q.z).unwrap_or_else(|_| FiniteF64::new(0.0).expect("zero")),
    ]
}

fn vec_arr(v: [f64; 3]) -> Option<[FiniteF64; 3]> {
    Some([
        FiniteF64::new(v[0]).ok()?,
        FiniteF64::new(v[1]).ok()?,
        FiniteF64::new(v[2]).ok()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::Sample;
    use quatopsy_schema::{Confidence, FindingClass, RepairDisposition, Severity};

    fn finding(start: u64, end: u64) -> Finding {
        Finding {
            id: format!("f-{start}"),
            rule: "QAT-SIGN-001".to_string(),
            rule_version: "1".to_string(),
            class: FindingClass::RepresentationDiscontinuity,
            severity: Severity::Medium,
            confidence: Confidence::Exact,
            source_row_start: start,
            source_row_end: end,
            timestamp_ns_start: 0,
            timestamp_ns_end: 0,
            evidence: Vec::new(),
            summary: "sign".to_string(),
            reason_code: "sign-discontinuity".to_string(),
            repair_disposition: RepairDisposition::Proposed,
            repair_refs: Vec::new(),
        }
    }

    #[test]
    fn downsample_keeps_finding_and_extrema_within_cap() {
        let mut samples = Vec::new();
        for i in 0..20_000_u64 {
            let w = if i == 999 { 0.0 } else { 1.0 };
            let q = if i == 999 {
                Quaternion::new(0.0, 1.0, 0.0, 0.0)
            } else {
                Quaternion::new(w, 0.0, 0.0, 0.0)
            };
            samples.push(Sample {
                source_row: i + 2,
                timestamp_ns: (i as i64) * 1_000_000,
                raw: q,
                commanded: None,
                omega: None,
                rotation_matrix: None,
                timestamp_finite: true,
                timestamp_overflow: false,
            });
        }
        let findings = vec![finding(12345 + 2, 12345 + 2)];
        let view = build_view(&samples, &findings, "id", None, 256);
        assert!(view.samples.len() <= 256);
        assert!(view.downsample.retained_findings);
        assert!(view.downsample.retained_extrema);
        assert!(
            view.samples
                .iter()
                .any(|sample| sample.source_row == 12347 && sample.pinned_finding)
        );
        assert!(view.samples.iter().any(|sample| sample.pinned_extremum));
        assert!(!view.authoritative);
    }

    #[test]
    fn empty_input_is_empty_view() {
        let view = build_view(&[], &[], "id", None, VIEW_MAX_POINTS);
        assert!(view.samples.is_empty());
    }

    #[test]
    fn finding_links_remain_complete_when_geometry_pin_budget_is_exceeded() {
        let samples = (0..100_u64)
            .map(|i| Sample {
                source_row: i + 2,
                timestamp_ns: i as i64,
                raw: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                commanded: None,
                omega: None,
                rotation_matrix: None,
                timestamp_finite: true,
                timestamp_overflow: false,
            })
            .collect::<Vec<_>>();
        let findings = (0..50_u64)
            .map(|i| finding(i + 2, i + 2))
            .collect::<Vec<_>>();
        let view = build_view(&samples, &findings, "id", None, 8);
        assert_eq!(view.samples.len(), 8);
        assert_eq!(view.finding_links.len(), findings.len());
        assert!(view.downsample.retained_findings);
        assert!(
            view.finding_links
                .iter()
                .all(|link| link.geometry_source_row.is_some())
        );
    }
}
