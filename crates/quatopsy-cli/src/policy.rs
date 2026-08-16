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
    let now =
        parse_utc_stamp(now).ok_or_else(|| "internal clock is not canonical UTC".to_string())?;
    let mut rules = Vec::new();
    for item in doc.overrides {
        if item.authority.trim().is_empty() || item.reason.trim().is_empty() {
            return Err("override authority and reason must be non-empty".to_string());
        }
        if !enabled_rules().contains(&item.rule.as_str()) {
            return Err(format!("override names unknown rule {}", item.rule));
        }
        let created = parse_utc_stamp(&item.created).ok_or_else(|| {
            format!(
                "override for {} has an invalid created timestamp",
                item.rule
            )
        })?;
        let expires = parse_utc_stamp(&item.expires)
            .ok_or_else(|| format!("override for {} has an invalid expiry timestamp", item.rule))?;
        if created > expires {
            return Err(format!(
                "override for {} expires before it was created",
                item.rule
            ));
        }
        if created > now {
            return Err(format!(
                "override for {} was created in the future",
                item.rule
            ));
        }
        if expires <= now {
            return Err(format!("override for {} has expired", item.rule));
        }
        if let Some(digest) = item.input_sha256
            && digest != csv_sha256
        {
            return Err("override input digest does not match this CSV".to_string());
        }
        if rules.contains(&item.rule) {
            return Err(format!("override duplicates rule {}", item.rule));
        }
        rules.push(item.rule);
    }
    Ok(rules)
}

pub fn validate_fail_on(fail_on: &[String]) -> Result<(), String> {
    let known = enabled_rules();
    let mut seen = std::collections::HashSet::new();
    for rule in fail_on {
        if !known.contains(&rule.as_str()) {
            return Err(format!("--fail-on names unknown rule {rule}"));
        }
        if !seen.insert(rule) {
            return Err(format!("--fail-on duplicates rule {rule}"));
        }
    }
    Ok(())
}

fn parse_utc_stamp(value: &str) -> Option<(u32, u32, u32, u32, u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let number = |start: usize, end: usize| value.get(start..end)?.parse::<u32>().ok();
    let stamp = (
        number(0, 4)?,
        number(5, 7)?,
        number(8, 10)?,
        number(11, 13)?,
        number(14, 16)?,
        number(17, 19)?,
    );
    let leap = stamp.0 % 4 == 0 && (stamp.0 % 100 != 0 || stamp.0 % 400 == 0);
    let days = match stamp.1 {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (stamp.2 >= 1 && stamp.2 <= days && stamp.3 < 24 && stamp.4 < 60 && stamp.5 < 60)
        .then_some(stamp)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_utc_parser_checks_calendar_boundaries() {
        assert!(parse_utc_stamp("2028-02-29T23:59:59Z").is_some());
        assert!(parse_utc_stamp("2027-02-29T00:00:00Z").is_none());
        assert!(parse_utc_stamp("2028-01-01T24:00:00Z").is_none());
        assert!(parse_utc_stamp("2028-01-01T00:00:00+00:00").is_none());
    }

    #[test]
    fn fail_on_registry_is_closed_and_duplicate_free() {
        assert!(validate_fail_on(&["QAT-SIGN-001".to_string()]).is_ok());
        assert!(validate_fail_on(&["QAT-SGIN-001".to_string()]).is_err());
        assert!(
            validate_fail_on(&["QAT-SIGN-001".to_string(), "QAT-SIGN-001".to_string()]).is_err()
        );
    }
}
