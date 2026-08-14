//! Public analysis entry point.

use std::time::{Duration, Instant};

use quatopsy_schema::{
    Diagnostics, ENGINE_NAME, InputInfo, LimitsUsed, NUMERIC_PROFILE_ID, REPORT_SCHEMA,
    RULE_SET_VERSION, Report, ResultState, RuleState,
};

use crate::identity::{analysis_id, sha256_hex};
use crate::ingest::{IngestError, ingest_bytes};
use crate::kernel::{Analysis, evaluate};
use crate::limits::Limits;

pub mod identity;
pub mod ingest;
pub mod kernel;
pub mod limits;
pub mod math;

pub use limits::Limits as AnalysisLimits;

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeRequest<'a> {
    pub csv_bytes: &'a [u8],
    pub manifest_bytes: &'a [u8],
    pub engine_version: &'a str,
    pub limits: Limits,
}

pub fn analyze(request: AnalyzeRequest<'_>) -> Report {
    let limits = request.limits.clamp_to_safe();
    let started = Instant::now();
    let deadline = started + Duration::from_millis(limits.timeout_ms.max(1));
    let id = analysis_id(
        request.csv_bytes,
        request.manifest_bytes,
        request.engine_version,
        limits,
    );
    let csv_sha256 = sha256_hex(request.csv_bytes);
    let manifest_sha256 = sha256_hex(request.manifest_bytes);

    match ingest_bytes(request.csv_bytes, request.manifest_bytes, limits, deadline) {
        Ok(parsed) => {
            let analysis = evaluate(&parsed.samples, limits, deadline);
            build_report(
                id,
                request.engine_version,
                csv_sha256,
                manifest_sha256,
                request.csv_bytes.len() as u64,
                parsed.samples.len() as u64,
                parsed.declarations,
                limits,
                analysis,
                parsed.bom_stripped,
            )
        }
        Err(error) => ingest_failure_report(
            id,
            request.engine_version,
            csv_sha256,
            manifest_sha256,
            request.csv_bytes.len() as u64,
            limits,
            error,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    analysis_id_hex: String,
    engine_version: &str,
    csv_sha256: String,
    manifest_sha256: String,
    byte_count: u64,
    sample_count: u64,
    declarations: quatopsy_schema::Declarations,
    limits: Limits,
    analysis: Analysis,
    bom_stripped: bool,
) -> Report {
    Report {
        schema: REPORT_SCHEMA.to_string(),
        analysis_id: analysis_id_hex,
        tool: quatopsy_schema::ToolInfo {
            name: ENGINE_NAME.to_string(),
            version: engine_version.to_string(),
            rule_set_version: RULE_SET_VERSION.to_string(),
            numeric_profile: NUMERIC_PROFILE_ID.to_string(),
        },
        input: InputInfo {
            csv_sha256,
            manifest_sha256,
            sample_count,
            byte_count,
        },
        declarations,
        limits: limits_used(limits),
        result: analysis.result,
        rule_results: analysis.rule_results,
        findings: analysis.findings,
        repairs: Vec::new(),
        diagnostics: Diagnostics {
            complete: analysis.complete,
            bom_stripped,
            reason_code: analysis.reason_code,
            message: analysis.message,
            conditioning: analysis.conditioning,
            rate_summary: analysis.rate_summary,
        },
    }
}

fn ingest_failure_report(
    analysis_id_hex: String,
    engine_version: &str,
    csv_sha256: String,
    manifest_sha256: String,
    byte_count: u64,
    limits: Limits,
    error: IngestError,
) -> Report {
    let (result, rule_state) = if error.is_refused() {
        (ResultState::Refused, RuleState::Refused)
    } else {
        (ResultState::Error, RuleState::Error)
    };
    let reason = error.reason_code().to_string();
    let message = error.to_string();
    let rule_results = quatopsy_schema::enabled_rules()
        .into_iter()
        .map(|rule| quatopsy_schema::RuleResult {
            rule: rule.to_string(),
            version: quatopsy_schema::RULE_VERSION.to_string(),
            state: rule_state,
            finding_count: 0,
            truncated: false,
            reason_code: reason.clone(),
        })
        .collect();
    Report {
        schema: REPORT_SCHEMA.to_string(),
        analysis_id: analysis_id_hex,
        tool: quatopsy_schema::ToolInfo {
            name: ENGINE_NAME.to_string(),
            version: engine_version.to_string(),
            rule_set_version: RULE_SET_VERSION.to_string(),
            numeric_profile: NUMERIC_PROFILE_ID.to_string(),
        },
        input: InputInfo {
            csv_sha256,
            manifest_sha256,
            sample_count: 0,
            byte_count,
        },
        declarations: quatopsy_schema::Declarations {
            component_order: quatopsy_schema::ComponentOrder::Wxyz,
            rotation_sense: quatopsy_schema::RotationSense::Active,
            frame_from: String::new(),
            frame_to: String::new(),
            time_unit: quatopsy_schema::TimeUnit::S,
            time_column: String::new(),
            quaternion_columns: [String::new(), String::new(), String::new(), String::new()],
            angular_velocity_columns: None,
        },
        limits: limits_used(limits),
        result,
        rule_results,
        findings: Vec::new(),
        repairs: Vec::new(),
        diagnostics: Diagnostics {
            complete: false,
            bom_stripped: false,
            reason_code: reason,
            message,
            conditioning: Vec::new(),
            rate_summary: None,
        },
    }
}

fn limits_used(limits: Limits) -> LimitsUsed {
    LimitsUsed {
        max_input_bytes: limits.max_input_bytes,
        max_samples: limits.max_samples,
        max_field_bytes: limits.max_field_bytes,
        max_columns: limits.max_columns,
        max_findings_per_rule: limits.max_findings_per_rule,
        timeout_ms: limits.timeout_ms,
    }
}

pub fn report_bytes(report: &Report) -> Result<Vec<u8>, serde_json::Error> {
    quatopsy_schema::canonical_json(report)
}
