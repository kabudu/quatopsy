//! Length-delimited SHA-256 analysis identity.

use sha2::{Digest, Sha256};

use crate::limits::Limits;
use quatopsy_schema::{
    NORM_ABS_TOLERANCE, NUMERIC_PROFILE_ID, PI_TIE_ABS_DOT, enabled_rules_canonical,
};

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

pub fn analysis_id(
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    engine_version: &str,
    limits: Limits,
) -> String {
    let mut buf = Vec::new();
    append_length_delimited(&mut buf, csv_bytes);
    append_length_delimited(&mut buf, manifest_bytes);
    append_length_delimited(&mut buf, engine_version.as_bytes());
    append_length_delimited(&mut buf, quatopsy_schema::RULE_SET_VERSION.as_bytes());
    append_length_delimited(&mut buf, NUMERIC_PROFILE_ID.as_bytes());
    append_length_delimited(&mut buf, &numeric_profile_bytes());
    append_length_delimited(&mut buf, enabled_rules_canonical().as_bytes());
    append_length_delimited(&mut buf, &limits.canonical_bytes());
    sha256_hex(&buf)
}

pub fn append_length_delimited(buf: &mut Vec<u8>, chunk: &[u8]) {
    buf.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
    buf.extend_from_slice(chunk);
}

fn numeric_profile_bytes() -> Vec<u8> {
    let mut out = Vec::with_capacity(24);
    out.extend_from_slice(&NORM_ABS_TOLERANCE.to_le_bytes());
    out.extend_from_slice(&PI_TIE_ABS_DOT.to_le_bytes());
    out.extend_from_slice(&quatopsy_schema::NEAR_ZERO_NORM.to_le_bytes());
    out
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;

    #[test]
    fn identity_changes_when_input_changes() {
        let limits = Limits::defaults();
        let a = analysis_id(b"a", b"m", "0.1.0", limits);
        let b = analysis_id(b"b", b"m", "0.1.0", limits);
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
        assert_eq!(analysis_id(b"a", b"m", "0.1.0", limits), a);
    }
}
