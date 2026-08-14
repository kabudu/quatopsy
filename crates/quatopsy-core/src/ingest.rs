//! Bounded UTF-8 CSV and explicit-manifest ingest.

use std::collections::HashMap;

use csv::ReaderBuilder;
use quatopsy_schema::{ComponentOrder, Declarations, MANIFEST_SCHEMA, ManifestDocument, TimeUnit};
use serde_json::error::Category;
use thiserror::Error;

use crate::cancel::Cancel;
use crate::limits::Limits;
use crate::math::Quaternion;

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub source_row: u64,
    pub timestamp_ns: i64,
    pub raw: Quaternion,
    pub commanded: Option<Quaternion>,
    pub timestamp_finite: bool,
    pub timestamp_overflow: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTrajectory {
    pub manifest: ManifestDocument,
    pub declarations: Declarations,
    pub samples: Vec<Sample>,
    pub bom_stripped: bool,
}

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("{message}")]
    Refused {
        reason_code: &'static str,
        message: String,
    },
    #[error("{message}")]
    Failed {
        reason_code: &'static str,
        message: String,
    },
}

impl IngestError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::Refused { reason_code, .. } | Self::Failed { reason_code, .. } => reason_code,
        }
    }

    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused { .. })
    }

    fn refused(reason_code: &'static str, message: impl Into<String>) -> Self {
        Self::Refused {
            reason_code,
            message: message.into(),
        }
    }

    fn failed(reason_code: &'static str, message: impl Into<String>) -> Self {
        Self::Failed {
            reason_code,
            message: message.into(),
        }
    }

    pub(crate) fn failed_timeout() -> Self {
        Self::failed("timeout", "analysis exceeded wall-clock limit")
    }

    pub(crate) fn failed_cancelled() -> Self {
        Self::failed("cancelled", "analysis was cancelled")
    }
}

pub fn ingest_bytes(
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    limits: Limits,
    cancel: Cancel<'_>,
) -> Result<ParsedTrajectory, IngestError> {
    cancel.check()?;
    if csv_bytes.len() as u64 > limits.max_input_bytes {
        return Err(IngestError::failed(
            "input-byte-limit",
            format!(
                "CSV byte count {} exceeds limit {}",
                csv_bytes.len(),
                limits.max_input_bytes
            ),
        ));
    }

    let (csv_body, bom_stripped) = strip_utf8_bom(csv_bytes);
    if !csv_body.is_empty() && std::str::from_utf8(csv_body).is_err() {
        return Err(IngestError::failed(
            "invalid-utf8",
            "CSV is not valid UTF-8",
        ));
    }

    let manifest = parse_manifest(manifest_bytes)?;
    let declarations = declarations_from_manifest(&manifest)?;
    let samples = parse_csv(csv_body, &manifest, limits, cancel)?;
    if samples.is_empty() {
        return Err(IngestError::refused(
            "empty-input",
            "CSV contains no data samples",
        ));
    }

    Ok(ParsedTrajectory {
        manifest,
        declarations,
        samples,
        bom_stripped,
    })
}

fn strip_utf8_bom(bytes: &[u8]) -> (&[u8], bool) {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        (&bytes[3..], true)
    } else {
        (bytes, false)
    }
}

fn parse_manifest(bytes: &[u8]) -> Result<ManifestDocument, IngestError> {
    if std::str::from_utf8(bytes).is_err() {
        return Err(IngestError::failed(
            "invalid-utf8",
            "manifest is not valid UTF-8",
        ));
    }
    let parsed: ManifestDocument = serde_json::from_slice(bytes).map_err(|err| {
        if err.classify() == Category::Data {
            IngestError::refused(
                "manifest-refused",
                format!("manifest is missing, unknown, or unsupported: {err}"),
            )
        } else {
            IngestError::failed(
                "manifest-parse",
                format!("manifest JSON could not be parsed: {err}"),
            )
        }
    })?;
    if parsed.schema != MANIFEST_SCHEMA {
        return Err(IngestError::refused(
            "unsupported-manifest-schema",
            format!("unsupported manifest schema {}", parsed.schema),
        ));
    }
    Ok(parsed)
}

fn declarations_from_manifest(manifest: &ManifestDocument) -> Result<Declarations, IngestError> {
    if manifest.frame_from.is_empty() || manifest.frame_to.is_empty() {
        return Err(IngestError::refused(
            "missing-frame",
            "frame_from and frame_to must be non-empty",
        ));
    }
    if manifest.frame_from == manifest.frame_to {
        return Err(IngestError::refused(
            "ambiguous-frame",
            "frame_from and frame_to must name a relationship between distinct frames",
        ));
    }
    if manifest.columns.time.is_empty() {
        return Err(IngestError::refused(
            "missing-time-column",
            "time column name must be non-empty",
        ));
    }
    let mut names = vec![manifest.columns.time.as_str()];
    for name in &manifest.columns.quaternion {
        if name.is_empty() {
            return Err(IngestError::refused(
                "missing-quaternion-column",
                "quaternion column names must be non-empty",
            ));
        }
        names.push(name.as_str());
    }
    if let Some(omega) = &manifest.columns.angular_velocity {
        for name in omega {
            if name.is_empty() {
                return Err(IngestError::refused(
                    "missing-omega-column",
                    "angular velocity column names must be non-empty",
                ));
            }
            names.push(name.as_str());
        }
    }
    if let Some(commanded) = &manifest.columns.commanded_quaternion {
        for name in commanded {
            if name.is_empty() {
                return Err(IngestError::refused(
                    "missing-commanded-column",
                    "commanded quaternion column names must be non-empty",
                ));
            }
            names.push(name.as_str());
        }
    }
    let unique = names.len()
        == names
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len();
    if !unique {
        return Err(IngestError::refused(
            "duplicate-column-declaration",
            "manifest column names must be unique",
        ));
    }
    Ok(Declarations {
        component_order: manifest.component_order,
        rotation_sense: manifest.rotation_sense,
        frame_from: manifest.frame_from.clone(),
        frame_to: manifest.frame_to.clone(),
        time_unit: manifest.time_unit,
        time_column: manifest.columns.time.clone(),
        quaternion_columns: manifest.columns.quaternion.clone(),
        angular_velocity_columns: manifest.columns.angular_velocity.clone(),
        commanded_quaternion_columns: manifest.columns.commanded_quaternion.clone(),
    })
}

fn parse_csv(
    csv_body: &[u8],
    manifest: &ManifestDocument,
    limits: Limits,
    cancel: Cancel<'_>,
) -> Result<Vec<Sample>, IngestError> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .trim(csv::Trim::None)
        .from_reader(csv_body);

    let headers = reader
        .headers()
        .map_err(|err| {
            IngestError::failed("csv-header", format!("CSV header could not be read: {err}"))
        })?
        .clone();
    if headers.len() as u64 > limits.max_columns {
        return Err(IngestError::failed(
            "column-limit",
            format!(
                "CSV has {} columns, limit is {}",
                headers.len(),
                limits.max_columns
            ),
        ));
    }
    for header in headers.iter() {
        if header.len() as u64 > limits.max_field_bytes {
            return Err(IngestError::failed(
                "field-limit",
                "CSV header exceeds the field-size limit",
            ));
        }
    }

    let index = column_index(&headers)?;
    let time_idx = required_column(&index, &manifest.columns.time)?;
    let quat_idx = [
        required_column(&index, &manifest.columns.quaternion[0])?,
        required_column(&index, &manifest.columns.quaternion[1])?,
        required_column(&index, &manifest.columns.quaternion[2])?,
        required_column(&index, &manifest.columns.quaternion[3])?,
    ];
    let commanded_idx = if let Some(cols) = &manifest.columns.commanded_quaternion {
        Some([
            required_column(&index, &cols[0])?,
            required_column(&index, &cols[1])?,
            required_column(&index, &cols[2])?,
            required_column(&index, &cols[3])?,
        ])
    } else {
        None
    };

    let mut samples = Vec::new();
    for record in reader.records() {
        cancel.check()?;
        let record = record.map_err(|err| {
            IngestError::failed("csv-record", format!("CSV record could not be read: {err}"))
        })?;
        if samples.len() as u64 >= limits.max_samples {
            return Err(IngestError::failed(
                "sample-limit",
                format!("CSV exceeds the sample limit {}", limits.max_samples),
            ));
        }
        for field in record.iter() {
            if field.len() as u64 > limits.max_field_bytes {
                return Err(IngestError::failed(
                    "field-limit",
                    format!(
                        "CSV field at line {} exceeds the field-size limit",
                        record.position().map(|p| p.line()).unwrap_or(0)
                    ),
                ));
            }
        }
        let source_row = record.position().map(|p| p.line()).unwrap_or(0);
        let (timestamp_ns, timestamp_finite, timestamp_overflow) =
            parse_timestamp(record.get(time_idx).unwrap_or(""), manifest.time_unit)?;
        let components = [
            parse_component(record.get(quat_idx[0]).unwrap_or(""))?,
            parse_component(record.get(quat_idx[1]).unwrap_or(""))?,
            parse_component(record.get(quat_idx[2]).unwrap_or(""))?,
            parse_component(record.get(quat_idx[3]).unwrap_or(""))?,
        ];
        let raw = assemble_quaternion(manifest.component_order, components);
        let commanded = if let Some(idx) = commanded_idx {
            let commanded_components = [
                parse_component(record.get(idx[0]).unwrap_or(""))?,
                parse_component(record.get(idx[1]).unwrap_or(""))?,
                parse_component(record.get(idx[2]).unwrap_or(""))?,
                parse_component(record.get(idx[3]).unwrap_or(""))?,
            ];
            Some(assemble_quaternion(
                manifest.component_order,
                commanded_components,
            ))
        } else {
            None
        };
        samples.push(Sample {
            source_row,
            timestamp_ns,
            raw,
            commanded,
            timestamp_finite,
            timestamp_overflow,
        });
    }
    Ok(samples)
}

fn column_index(headers: &csv::StringRecord) -> Result<HashMap<String, usize>, IngestError> {
    let mut index = HashMap::new();
    for (idx, name) in headers.iter().enumerate() {
        if name.is_empty() {
            return Err(IngestError::refused(
                "empty-header",
                "CSV header names must be non-empty",
            ));
        }
        if index.insert(name.to_string(), idx).is_some() {
            return Err(IngestError::refused(
                "duplicate-header",
                format!("CSV header {name} is duplicated"),
            ));
        }
    }
    Ok(index)
}

fn required_column(index: &HashMap<String, usize>, name: &str) -> Result<usize, IngestError> {
    index.get(name).copied().ok_or_else(|| {
        IngestError::refused(
            "missing-declared-column",
            format!("CSV does not contain declared column {name}"),
        )
    })
}

fn parse_timestamp(raw: &str, unit: TimeUnit) -> Result<(i64, bool, bool), IngestError> {
    let parsed = parse_f64_field(raw)?;
    if !parsed.is_finite() {
        return Ok((0, false, false));
    }
    let scaled = parsed * unit.to_nanoseconds_scale();
    if !scaled.is_finite() {
        return Ok((0, false, true));
    }
    let rounded = scaled.round();
    if rounded > i64::MAX as f64 || rounded < i64::MIN as f64 {
        return Ok((0, true, true));
    }
    Ok((rounded as i64, true, false))
}

fn parse_component(raw: &str) -> Result<f64, IngestError> {
    parse_f64_field(raw)
}

fn parse_f64_field(raw: &str) -> Result<f64, IngestError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(IngestError::failed(
            "empty-numeric-field",
            "numeric CSV field is empty",
        ));
    }
    if trimmed.starts_with('=') || trimmed.starts_with('@') {
        return Err(IngestError::failed(
            "formula-text",
            "numeric CSV field looks like spreadsheet formula text",
        ));
    }
    trimmed.parse::<f64>().map_err(|_| {
        IngestError::failed(
            "invalid-number",
            format!("CSV field {trimmed:?} is not a number"),
        )
    })
}

fn assemble_quaternion(order: ComponentOrder, components: [f64; 4]) -> Quaternion {
    match order {
        ComponentOrder::Wxyz => {
            Quaternion::new(components[0], components[1], components[2], components[3])
        }
        ComponentOrder::Xyzw => {
            Quaternion::new(components[3], components[0], components[1], components[2])
        }
    }
}
