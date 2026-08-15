//! Adoption modes affect process exit only. They never rewrite report results.

use quatopsy_schema::{
    AdoptionMode, OVERRIDE_SCHEMA, OverrideDocument, Report, ResultState, enabled_rules,
};

pub fn load_overrides(bytes: &[u8], csv_sha256: &str, now: &str) -> Result<Vec<String>, String> {
    let doc: OverrideDocument = serde_json::from_slice(bytes)
        .map_err(|err| format!("override document could not be parsed: {err}"))?;
    if doc.schema != OVERRIDE_SCHEMA {
        return Err(format!("unsupported override schema {}", doc.schema));
    }
    let mut rules = Vec::new();
    for item in doc.overrides {
        if item.authority.trim().is_empty() || item.reason.trim().is_empty() {
            return Err("override authority and reason must be non-empty".to_string());
        }
        if !enabled_rules().contains(&item.rule.as_str()) {
            return Err(format!("override names unknown rule {}", item.rule));
        }
        if item.expires.as_str() < now {
            return Err(format!("override for {} has expired", item.rule));
        }
        if let Some(digest) = item.input_sha256
            && digest != csv_sha256
        {
            return Err("override input digest does not match this CSV".to_string());
        }
        rules.push(item.rule);
    }
    Ok(rules)
}

pub fn exit_code(
    report: &Report,
    mode: AdoptionMode,
    fail_on: &[String],
    overridden: &[String],
) -> i32 {
    match report.result {
        ResultState::Pass => 0,
        ResultState::Refused => 2,
        ResultState::Error => 3,
        ResultState::Findings => match mode {
            AdoptionMode::Advisory => 0,
            AdoptionMode::Required => 1,
            AdoptionMode::Selective => {
                let blocking = report.findings.iter().any(|finding| {
                    fail_on.iter().any(|rule| rule == &finding.rule)
                        && !overridden.iter().any(|rule| rule == &finding.rule)
                });
                i32::from(blocking)
            }
        },
    }
}
