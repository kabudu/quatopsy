//! Canonical protocol types for Quatopsy reports and manifests.
//!
//! Field order on these structs is part of the `quatopsy.report/1` contract.

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError, ser::Error as SerError,
};

pub const REPORT_SCHEMA: &str = "quatopsy.report/1";
pub const VIEW_SCHEMA: &str = "quatopsy.view/1";
pub const MANIFEST_SCHEMA: &str = "quatopsy.manifest/1";
pub const EVIDENCE_SCHEMA: &str = "quatopsy.evidence/1";
pub const VIEW_MAX_POINTS: u64 = 4096;
pub const VIEW_SAFE_MAX_POINTS: u64 = 16_384;
pub const RULE_SET_VERSION: &str = "quatopsy.rules/1";
pub const NUMERIC_PROFILE_ID: &str = "quatopsy.numeric/1";
pub const ENGINE_NAME: &str = "quatopsy";

pub const RULE_NORM: &str = "QAT-NORM-001";
pub const RULE_TIME: &str = "QAT-TIME-001";
pub const RULE_LIFT: &str = "QAT-LIFT-001";
pub const RULE_SIGN: &str = "QAT-SIGN-001";
pub const RULE_RATE: &str = "QAT-RATE-001";
pub const RULE_PI: &str = "QAT-PI-001";
pub const RULE_REPAIR: &str = "QAT-REPAIR-001";
pub const RULE_UNWIND: &str = "QAT-UNWIND-001";
pub const RULE_CONV: &str = "QAT-CONV-001";
pub const RULE_OMEGA: &str = "QAT-OMEGA-001";
pub const RULE_VERSION: &str = "1";
pub const ALG_SIGN_LIFT: &str = "sign-lift";
pub const ALG_NORMALISE: &str = "normalise";
pub const ALG_VERSION: &str = "1";
pub const REPAIR_MATRIX_ABS_TOLERANCE: f64 = 1.0e-12;

/// Absolute tolerance on `|‖q‖ − 1|` for unit-norm acceptance.
pub const NORM_ABS_TOLERANCE: f64 = 1.0e-6;
/// Samples with `0 < ‖q‖ < NEAR_ZERO_NORM` are numerically unsafe and refused.
pub const NEAR_ZERO_NORM: f64 = 1.0e-12;
/// `|p · q| <= PI_TIE_ABS_DOT` is a non-unique lift tie after unit normalisation.
pub const PI_TIE_ABS_DOT: f64 = 1.0e-12;
/// Adjacent commanded covering longer than the quotient-shortest path by this amount is a finding.
pub const UNWIND_ABS_TOLERANCE: f64 = 1.0e-9;
/// Maximum absolute element error between a declared rotation matrix and `R(q)`.
pub const CONV_MATRIX_ABS_TOLERANCE: f64 = 1.0e-5;
/// Maximum absolute body-rate error in rad/s between supplied omega and the quaternion kinematics.
pub const OMEGA_ABS_TOLERANCE: f64 = 1.0e-3;
pub const SPACECRAFT_PROFILE_ID: &str = "quatopsy.spacecraft-csv/1";

pub fn enabled_rules() -> [&'static str; 10] {
    [
        RULE_NORM,
        RULE_TIME,
        RULE_LIFT,
        RULE_SIGN,
        RULE_RATE,
        RULE_PI,
        RULE_REPAIR,
        RULE_UNWIND,
        RULE_CONV,
        RULE_OMEGA,
    ]
}

pub fn enabled_rules_canonical() -> String {
    let mut rules: Vec<&'static str> = enabled_rules().into();
    rules.sort_unstable();
    rules.join("\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentOrder {
    Wxyz,
    Xyzw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationSense {
    Active,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Ns,
    Us,
    Ms,
    S,
}

impl TimeUnit {
    pub fn to_nanoseconds_scale(self) -> f64 {
        match self {
            Self::Ns => 1.0,
            Self::Us => 1_000.0,
            Self::Ms => 1_000_000.0,
            Self::S => 1_000_000_000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestColumns {
    pub time: String,
    pub quaternion: [String; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angular_velocity: Option<[String; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commanded_quaternion: Option<[String; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_matrix: Option<[String; 9]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestDocument {
    pub schema: String,
    pub component_order: ComponentOrder,
    pub rotation_sense: RotationSense,
    pub frame_from: String,
    pub frame_to: String,
    pub time_unit: TimeUnit,
    pub columns: ManifestColumns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultState {
    Pass,
    Findings,
    Refused,
    Error,
}

impl ResultState {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Pass => 0,
            Self::Findings => 1,
            Self::Refused => 2,
            Self::Error => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Findings => "findings",
            Self::Refused => "refused",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleState {
    Pass,
    Finding,
    Refused,
    Error,
}

impl RuleState {
    fn rank(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Finding => 1,
            Self::Refused => 2,
            Self::Error => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Finding => "finding",
            Self::Refused => "refused",
            Self::Error => "error",
        }
    }
}

pub fn aggregate_result(states: impl IntoIterator<Item = RuleState>) -> ResultState {
    let mut worst = RuleState::Pass;
    for state in states {
        if state.rank() > worst.rank() {
            worst = state;
        }
    }
    match worst {
        RuleState::Pass => ResultState::Pass,
        RuleState::Finding => ResultState::Findings,
        RuleState::Refused => ResultState::Refused,
        RuleState::Error => ResultState::Error,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingClass {
    InvalidData,
    RepresentationDiscontinuity,
    PhysicalDiscontinuity,
    ConventionMismatch,
    DynamicThreshold,
    Informational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Exact,
    NumericTolerance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if value.is_finite() {
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        } else {
            Err("non-finite numbers are not permitted in canonical JSON")
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if !self.0.is_finite() {
            return Err(S::Error::custom(
                "non-finite numbers are not permitted in canonical JSON",
            ));
        }
        serializer.serialize_f64(if self.0 == 0.0 { 0.0 } else { self.0 })
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub name: String,
    pub unit: String,
    pub number: FiniteF64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub rule: String,
    pub rule_version: String,
    pub class: FindingClass,
    pub severity: Severity,
    pub confidence: Confidence,
    pub source_row_start: u64,
    pub source_row_end: u64,
    pub timestamp_ns_start: i64,
    pub timestamp_ns_end: i64,
    pub evidence: Vec<Evidence>,
    pub summary: String,
    pub reason_code: String,
    pub repair_disposition: RepairDisposition,
    pub repair_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule: String,
    pub version: String,
    pub state: RuleState,
    pub finding_count: u64,
    pub truncated: bool,
    pub reason_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepairDisposition {
    None,
    Proposed,
    Inapplicable,
    Unsafe,
}

impl RepairDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Proposed => "proposed",
            Self::Inapplicable => "inapplicable",
            Self::Unsafe => "unsafe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repair {
    pub id: String,
    pub algorithm: String,
    pub algorithm_version: String,
    pub source_analysis_id: String,
    pub disposition: RepairDisposition,
    pub physical_orientation_equivalent: bool,
    pub affected_rows: Vec<u64>,
    pub preconditions: Vec<String>,
    pub numeric_tolerance: FiniteF64,
    pub max_norm_delta: Option<FiniteF64>,
    pub output_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
    pub rule_set_version: String,
    pub numeric_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputInfo {
    pub csv_sha256: String,
    pub manifest_sha256: String,
    pub sample_count: u64,
    pub byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declarations {
    pub component_order: ComponentOrder,
    pub rotation_sense: RotationSense,
    pub frame_from: String,
    pub frame_to: String,
    pub time_unit: TimeUnit,
    pub time_column: String,
    pub quaternion_columns: [String; 4],
    pub angular_velocity_columns: Option<[String; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commanded_quaternion_columns: Option<[String; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_matrix_columns: Option<[String; 9]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitsUsed {
    pub max_input_bytes: u64,
    pub max_samples: u64,
    pub max_field_bytes: u64,
    pub max_columns: u64,
    pub max_findings_per_rule: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateSummary {
    pub interval_count: u64,
    pub min_angle_rad: FiniteF64,
    pub max_angle_rad: FiniteF64,
    pub min_rate_rad_s: FiniteF64,
    pub max_rate_rad_s: FiniteF64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    pub complete: bool,
    pub bom_stripped: bool,
    pub reason_code: String,
    pub message: String,
    pub conditioning: Vec<String>,
    pub rate_summary: Option<RateSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    pub analysis_id: String,
    pub tool: ToolInfo,
    pub input: InputInfo,
    pub declarations: Declarations,
    pub limits: LimitsUsed,
    pub result: ResultState,
    pub rule_results: Vec<RuleResult>,
    pub findings: Vec<Finding>,
    pub repairs: Vec<Repair>,
    pub diagnostics: Diagnostics,
}

pub fn report_schema_supported(schema: &str) -> bool {
    schema == REPORT_SCHEMA
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdoptionMode {
    Advisory,
    Selective,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideRecord {
    pub rule: String,
    pub authority: String,
    pub reason: String,
    pub created: String,
    pub expires: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideDocument {
    pub schema: String,
    pub overrides: Vec<OverrideRecord>,
}

pub const OVERRIDE_SCHEMA: &str = "quatopsy.override/1";

pub fn canonical_json(report: &Report) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_dominates_refused_findings_and_pass() {
        assert_eq!(
            aggregate_result([RuleState::Pass, RuleState::Finding, RuleState::Refused]),
            ResultState::Refused
        );
        assert_eq!(
            aggregate_result([RuleState::Finding, RuleState::Error, RuleState::Pass]),
            ResultState::Error
        );
        assert_eq!(
            aggregate_result([RuleState::Pass, RuleState::Finding]),
            ResultState::Findings
        );
        assert_eq!(aggregate_result([RuleState::Pass]), ResultState::Pass);
    }

    #[test]
    fn enabled_rules_are_closed_and_sorted_for_identity() {
        let canonical = enabled_rules_canonical();
        assert_eq!(
            canonical,
            "QAT-CONV-001\nQAT-LIFT-001\nQAT-NORM-001\nQAT-OMEGA-001\nQAT-PI-001\nQAT-RATE-001\nQAT-REPAIR-001\nQAT-SIGN-001\nQAT-TIME-001\nQAT-UNWIND-001"
        );
        assert!(report_schema_supported(REPORT_SCHEMA));
        assert!(!report_schema_supported("quatopsy.report/99"));
    }

    #[test]
    fn evidence_schema_file_matches_protocol_constant() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/quatopsy.evidence.v1.json")).unwrap();
        assert_eq!(schema["properties"]["schema"]["const"], EVIDENCE_SCHEMA);
        assert_eq!(schema["additionalProperties"], false);
    }
}
