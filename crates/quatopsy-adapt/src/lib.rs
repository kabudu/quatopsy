//! Converters into canonical CSV plus manifest. No report or result types.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdaptError {
    #[error("{0}")]
    Refused(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterFormat {
    IdsJason1,
    RosJson,
    TubinStr,
}

impl AdapterFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IdsJason1 => "ids-jason1",
            Self::RosJson => "ros-json",
            Self::TubinStr => "tubin-str",
        }
    }

    pub fn parse(name: &str) -> Result<Self, AdaptError> {
        match name {
            "ids-jason1" => Ok(Self::IdsJason1),
            "ros-json" => Ok(Self::RosJson),
            "tubin-str" => Ok(Self::TubinStr),
            other => Err(AdaptError::Refused(format!(
                "unsupported adapter format {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdapterOutput {
    pub csv: String,
    pub manifest: String,
    pub provenance: String,
}

#[derive(Debug, Serialize)]
struct Provenance {
    schema: &'static str,
    format: &'static str,
    adapter: &'static str,
    adapter_version: String,
    source_sha256: String,
    sample_count: u64,
    notes: &'static str,
}

#[derive(Debug, Deserialize)]
struct RosDocument {
    schema: String,
    frame_from: String,
    frame_to: String,
    samples: Vec<RosSample>,
}

#[derive(Debug, Deserialize)]
struct RosSample {
    t: f64,
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

pub fn adapt(
    format: AdapterFormat,
    source: &[u8],
    version: &str,
) -> Result<AdapterOutput, AdaptError> {
    match format {
        AdapterFormat::IdsJason1 => adapt_ids_jason1(source, version),
        AdapterFormat::RosJson => adapt_ros_json(source, version),
        AdapterFormat::TubinStr => adapt_tubin_str(source, version),
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn provenance_json(
    format: AdapterFormat,
    source: &[u8],
    version: &str,
    sample_count: u64,
) -> Result<String, AdaptError> {
    let doc = Provenance {
        schema: "quatopsy.adapter-provenance/1",
        format: format.as_str(),
        adapter: "quatopsy-adapt",
        adapter_version: version.to_string(),
        source_sha256: digest_hex(source),
        sample_count,
        notes: "Adapter emits canonical CSV and manifest only. It does not assign Quatopsy verdicts.",
    };
    serde_json::to_string(&doc).map_err(|err| AdaptError::Refused(err.to_string()))
}

fn adapt_ids_jason1(source: &[u8], version: &str) -> Result<AdapterOutput, AdaptError> {
    let text = std::str::from_utf8(source)
        .map_err(|_| AdaptError::Refused("ids-jason1 source is not UTF-8".to_string()))?;
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(AdaptError::Refused(format!(
                "ids-jason1 line {} needs UTC date, time, and four quaternion components",
                idx + 1
            )));
        }
        let stamp = format!("{}T{}Z", parts[0].replace('/', "-"), parts[1]);
        let t = parse_ids_time(&stamp)?;
        let q0: f64 = parts[2]
            .parse()
            .map_err(|_| AdaptError::Refused(format!("invalid Q0 on line {}", idx + 1)))?;
        let q1: f64 = parts[3]
            .parse()
            .map_err(|_| AdaptError::Refused(format!("invalid Q1 on line {}", idx + 1)))?;
        let q2: f64 = parts[4]
            .parse()
            .map_err(|_| AdaptError::Refused(format!("invalid Q2 on line {}", idx + 1)))?;
        let q3: f64 = parts[5]
            .parse()
            .map_err(|_| AdaptError::Refused(format!("invalid Q3 on line {}", idx + 1)))?;
        rows.push((t, q0, q1, q2, q3));
    }
    if rows.is_empty() {
        return Err(AdaptError::Refused(
            "ids-jason1 source contains no data records".to_string(),
        ));
    }
    let t0 = rows[0].0;
    let mut csv = String::from("t,qw,qx,qy,qz\n");
    for (t, q0, q1, q2, q3) in &rows {
        csv.push_str(&format!("{:.9},{},{},{},{}\n", t - t0, q0, q1, q2, q3));
    }
    let manifest = serde_json::json!({
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "BODY",
        "frame_to": "J2000",
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qw", "qx", "qy", "qz"]}
    })
    .to_string();
    Ok(AdapterOutput {
        csv,
        manifest,
        provenance: provenance_json(AdapterFormat::IdsJason1, source, version, rows.len() as u64)?,
    })
}

fn parse_ids_time(stamp: &str) -> Result<f64, AdaptError> {
    // YYYY-MM-DDTHH:MM:SS.sssZ as seconds from Unix epoch using a civil-time conversion.
    let (date, rest) = stamp.split_once('T').ok_or_else(|| {
        AdaptError::Refused("ids-jason1 timestamp missing T separator".to_string())
    })?;
    let time = rest.trim_end_matches('Z');
    let mut d = date.split('-');
    let year: i32 = d
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 year".to_string()))?;
    let month: i32 = d
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 month".to_string()))?;
    let day: i32 = d
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 day".to_string()))?;
    let mut t = time.split(':');
    let hour: i32 = t
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 hour".to_string()))?;
    let minute: i32 = t
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 minute".to_string()))?;
    let second: f64 = t
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| AdaptError::Refused("ids-jason1 second".to_string()))?;
    Ok(julian_seconds(year, month, day, hour, minute, second))
}

fn julian_seconds(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: f64) -> f64 {
    let a = (14 - month) / 12;
    let y = year + 4800 - a;
    let m = month + 12 * a - 3;
    let jdn = day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    let days = jdn as f64 - 2_440_587.5;
    days * 86400.0 + f64::from(hour) * 3600.0 + f64::from(minute) * 60.0 + second
}

fn adapt_ros_json(source: &[u8], version: &str) -> Result<AdapterOutput, AdaptError> {
    let doc: RosDocument = serde_json::from_slice(source)
        .map_err(|err| AdaptError::Refused(format!("ros-json parse failed: {err}")))?;
    if doc.schema != "quatopsy.adapt.ros-json/1" {
        return Err(AdaptError::Refused(format!(
            "unsupported ros-json schema {}",
            doc.schema
        )));
    }
    if doc.samples.is_empty() {
        return Err(AdaptError::Refused(
            "ros-json contains no samples".to_string(),
        ));
    }
    if serde_json::from_slice::<serde_json::Value>(source)
        .ok()
        .and_then(|v| v.get("result").cloned())
        .is_some()
    {
        return Err(AdaptError::Refused(
            "adapter input must not contain a result field".to_string(),
        ));
    }
    let mut csv = String::from("t,qx,qy,qz,qw\n");
    for sample in &doc.samples {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            sample.t, sample.x, sample.y, sample.z, sample.w
        ));
    }
    let manifest = serde_json::json!({
        "schema": "quatopsy.manifest/1",
        "component_order": "xyzw",
        "rotation_sense": "active",
        "frame_from": doc.frame_from,
        "frame_to": doc.frame_to,
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qx", "qy", "qz", "qw"]}
    })
    .to_string();
    Ok(AdapterOutput {
        csv,
        manifest,
        provenance: provenance_json(
            AdapterFormat::RosJson,
            source,
            version,
            doc.samples.len() as u64,
        )?,
    })
}

fn adapt_tubin_str(source: &[u8], version: &str) -> Result<AdapterOutput, AdaptError> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(source);
    let headers = reader
        .headers()
        .map_err(|err| AdaptError::Refused(format!("tubin-str header: {err}")))?
        .clone();
    let time_i = header_index(&headers, "Timestamp [UTC]")?;
    let qs_i = header_index(&headers, "VOTER_Q_S")?;
    let qx_i = header_index(&headers, "VOTER_Q_X")?;
    let qy_i = header_index(&headers, "VOTER_Q_Y")?;
    let qz_i = header_index(&headers, "VOTER_Q_Z")?;
    let wx_i = header_index(&headers, "VOTER_X_RATE")?;
    let wy_i = header_index(&headers, "VOTER_Y_RATE")?;
    let wz_i = header_index(&headers, "VOTER_Z_RATE")?;
    let mut csv = String::from("t,qw,qx,qy,qz,wx,wy,wz\n");
    let mut count = 0_u64;
    let mut t0 = None;
    for (idx, record) in reader.records().enumerate() {
        let record = record
            .map_err(|err| AdaptError::Refused(format!("tubin-str row {}: {err}", idx + 2)))?;
        let qs = field(&record, qs_i);
        let qx = field(&record, qx_i);
        let qy = field(&record, qy_i);
        let qz = field(&record, qz_i);
        if qs.is_empty() || qx.is_empty() || qy.is_empty() || qz.is_empty() {
            continue;
        }
        let stamp = field(&record, time_i).replace(' ', "T");
        let stamp = format!(
            "{}Z",
            stamp.trim_end_matches("+00:00").trim_end_matches('Z')
        );
        let t = parse_ids_time(&stamp)?;
        let origin = *t0.get_or_insert(t);
        let t_rel = t - origin;
        let deg = std::f64::consts::PI / 180.0;
        let wx = parse_f64(field(&record, wx_i), "VOTER_X_RATE")? * deg;
        let wy = parse_f64(field(&record, wy_i), "VOTER_Y_RATE")? * deg;
        let wz = parse_f64(field(&record, wz_i), "VOTER_Z_RATE")? * deg;
        csv.push_str(&format!("{t_rel:.9},{qs},{qx},{qy},{qz},{wx},{wy},{wz}\n"));
        count += 1;
    }
    if count == 0 {
        return Err(AdaptError::Refused(
            "tubin-str source contains no voter quaternion rows".to_string(),
        ));
    }
    let manifest = serde_json::json!({
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": "SAT",
        "frame_to": "TOD",
        "time_unit": "s",
        "columns": {
            "time": "t",
            "quaternion": ["qw", "qx", "qy", "qz"],
            "angular_velocity": ["wx", "wy", "wz"]
        }
    })
    .to_string();
    Ok(AdapterOutput {
        csv,
        manifest,
        provenance: provenance_json(AdapterFormat::TubinStr, source, version, count)?,
    })
}

fn header_index(headers: &csv::StringRecord, needle: &str) -> Result<usize, AdaptError> {
    headers
        .iter()
        .position(|name| name.contains(needle))
        .ok_or_else(|| AdaptError::Refused(format!("tubin-str missing column {needle}")))
}

fn field(record: &csv::StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or("").trim()
}

fn parse_f64(value: &str, name: &str) -> Result<f64, AdaptError> {
    value
        .parse()
        .map_err(|_| AdaptError::Refused(format!("invalid {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_has_no_result_field() {
        let src = br#"{"schema":"quatopsy.adapt.ros-json/1","frame_from":"base","frame_to":"map","samples":[{"t":0,"x":0,"y":0,"z":0,"w":1}]}"#;
        let out = adapt(AdapterFormat::RosJson, src, "0.1.0").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.provenance).unwrap();
        assert!(v.get("result").is_none());
        assert_eq!(v["schema"], "quatopsy.adapter-provenance/1");
    }

    #[test]
    fn tubin_str_skips_empty_voter_rows() {
        let src = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/public/tubin_str/source.csv"
        ))
        .unwrap();
        let out = adapt(AdapterFormat::TubinStr, &src, "0.1.0").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.provenance).unwrap();
        assert!(v.get("result").is_none());
        assert_eq!(v["format"], "tubin-str");
        assert_eq!(v["sample_count"], 16);
        assert!(out.csv.lines().count() > 16);
        assert!(out.manifest.contains("angular_velocity"));
    }
}
