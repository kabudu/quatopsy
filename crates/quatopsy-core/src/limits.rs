//! Resource limits for ingest and analysis. Compiled safe maxima cannot be
//! exceeded by CLI flags.

use quatopsy_schema::LimitsUsed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: u64,
    pub max_samples: u64,
    pub max_field_bytes: u64,
    pub max_columns: u64,
    pub max_findings_per_rule: u64,
    pub timeout_ms: u64,
}

pub const SAFE_MAX_INPUT_BYTES: u64 = 1 << 30;
pub const SAFE_MAX_SAMPLES: u64 = 10_000_000;
pub const SAFE_MAX_FIELD_BYTES: u64 = 4096;
pub const SAFE_MAX_COLUMNS: u64 = 32;
pub const SAFE_MAX_FINDINGS_PER_RULE: u64 = 10_000;
pub const SAFE_TIMEOUT_MS: u64 = 300_000;

impl Limits {
    pub fn defaults() -> Self {
        Self {
            max_input_bytes: SAFE_MAX_INPUT_BYTES,
            max_samples: SAFE_MAX_SAMPLES,
            max_field_bytes: SAFE_MAX_FIELD_BYTES,
            max_columns: SAFE_MAX_COLUMNS,
            max_findings_per_rule: SAFE_MAX_FINDINGS_PER_RULE,
            timeout_ms: SAFE_TIMEOUT_MS,
        }
    }

    pub fn from_report(used: &LimitsUsed) -> Self {
        Self {
            max_input_bytes: used.max_input_bytes,
            max_samples: used.max_samples,
            max_field_bytes: used.max_field_bytes,
            max_columns: used.max_columns,
            max_findings_per_rule: used.max_findings_per_rule,
            timeout_ms: used.timeout_ms,
        }
        .clamp_to_safe()
    }

    pub fn clamp_to_safe(mut self) -> Self {
        self.max_input_bytes = self.max_input_bytes.min(SAFE_MAX_INPUT_BYTES);
        self.max_samples = self.max_samples.min(SAFE_MAX_SAMPLES);
        self.max_field_bytes = self.max_field_bytes.min(SAFE_MAX_FIELD_BYTES);
        self.max_columns = self.max_columns.min(SAFE_MAX_COLUMNS);
        self.max_findings_per_rule = self.max_findings_per_rule.min(SAFE_MAX_FINDINGS_PER_RULE);
        self.timeout_ms = self.timeout_ms.min(SAFE_TIMEOUT_MS);
        self
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(48);
        for value in [
            self.max_input_bytes,
            self.max_samples,
            self.max_field_bytes,
            self.max_columns,
            self.max_findings_per_rule,
            self.timeout_ms,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::defaults()
    }
}
