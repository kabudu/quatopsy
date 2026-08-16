//! Minimal reproducible slices with provenance and path redaction.

use serde::Serialize;

use quatopsy_schema::Finding;

#[derive(Debug, Clone, Serialize)]
pub struct ReproProvenance {
    pub analysis_id: String,
    pub csv_sha256: String,
    pub manifest_sha256: String,
    pub source_row_start: u64,
    pub source_row_end: u64,
    pub finding_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_path: Option<String>,
}

pub fn finding_repro_bounds(finding: &Finding) -> (u64, u64) {
    (
        finding.source_row_start.saturating_sub(1).max(2),
        finding.source_row_end.saturating_add(1),
    )
}

pub fn slice_csv(csv_bytes: &[u8], start_row: u64, end_row: u64) -> Result<Vec<u8>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(csv_bytes);
    let headers = reader.headers().map_err(|err| err.to_string())?.clone();
    let mut out = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new().from_writer(&mut out);
        writer
            .write_record(&headers)
            .map_err(|err| err.to_string())?;
        for record in reader.records() {
            let record = record.map_err(|err| err.to_string())?;
            let line = record.position().map(|pos| pos.line()).unwrap_or(0);
            if line >= start_row && line <= end_row {
                writer
                    .write_record(&record)
                    .map_err(|err| err.to_string())?;
            }
        }
        writer.flush().map_err(|err| err.to_string())?;
    }
    Ok(out)
}

pub fn provenance(
    analysis_id: String,
    csv_sha256: String,
    manifest_sha256: String,
    start_row: u64,
    end_row: u64,
    findings: &[Finding],
    input_path: Option<String>,
) -> ReproProvenance {
    ReproProvenance {
        analysis_id,
        csv_sha256,
        manifest_sha256,
        source_row_start: start_row,
        source_row_end: end_row,
        finding_ids: findings.iter().map(|finding| finding.id.clone()).collect(),
        input_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quatopsy_schema::{Confidence, FindingClass, RepairDisposition, Severity};

    #[test]
    fn finding_slice_is_bounded_to_one_context_row() {
        let finding = Finding {
            id: "finding:test".to_string(),
            rule: "QAT-SIGN-001".to_string(),
            rule_version: "1".to_string(),
            class: FindingClass::RepresentationDiscontinuity,
            severity: Severity::Medium,
            confidence: Confidence::Exact,
            source_row_start: 100,
            source_row_end: 101,
            timestamp_ns_start: 0,
            timestamp_ns_end: 1,
            evidence: Vec::new(),
            summary: "test".to_string(),
            reason_code: "test".to_string(),
            repair_disposition: RepairDisposition::None,
            repair_refs: Vec::new(),
        };
        assert_eq!(finding_repro_bounds(&finding), (99, 102));
    }
}
