//! Uncompressed MCAP reader for declared JSON attitude poses.

use crate::{AdaptError, AdapterFormat, AdapterOutput, provenance_json};
use serde::Deserialize;
use std::collections::BTreeMap;

const MAGIC: &[u8] = b"\x89MCAP0\r\n";
const OP_HEADER: u8 = 0x01;
const OP_FOOTER: u8 = 0x02;
const OP_SCHEMA: u8 = 0x03;
const OP_CHANNEL: u8 = 0x04;
const OP_MESSAGE: u8 = 0x05;
const OP_CHUNK: u8 = 0x06;
const OP_DATA_END: u8 = 0x0F;
const MAX_RECORDS: usize = 10_000;
const MAX_MESSAGES: usize = 100_000;

#[derive(Debug, Deserialize)]
struct Pose {
    #[serde(default)]
    t: Option<f64>,
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

struct Channel {
    schema_id: u16,
    encoding: String,
    metadata: BTreeMap<String, String>,
}

struct Schema {
    name: String,
}

pub fn adapt_mcap(source: &[u8], version: &str) -> Result<AdapterOutput, AdaptError> {
    if source.len() < MAGIC.len() * 2 || !source.starts_with(MAGIC) {
        return Err(AdaptError::Refused("mcap magic is missing".to_string()));
    }
    if !source.ends_with(MAGIC) {
        return Err(AdaptError::Refused(
            "mcap trailing magic is missing".to_string(),
        ));
    }
    let mut pos = MAGIC.len();
    let mut schemas = BTreeMap::new();
    let mut channels = BTreeMap::new();
    let mut samples = Vec::new();
    let mut records = 0_usize;
    while pos + 9 <= source.len() - MAGIC.len() {
        records += 1;
        if records > MAX_RECORDS {
            return Err(AdaptError::Refused("mcap exceeds record limit".to_string()));
        }
        let opcode = source[pos];
        let len = read_u64(source, pos + 1)?;
        pos += 9;
        let end = pos
            .checked_add(len as usize)
            .ok_or_else(|| AdaptError::Refused("mcap record length overflow".to_string()))?;
        if end > source.len() - MAGIC.len() {
            return Err(AdaptError::Refused("mcap record overruns file".to_string()));
        }
        let body = &source[pos..end];
        pos = end;
        match opcode {
            OP_HEADER => {}
            OP_SCHEMA => {
                let (id, schema) = parse_schema(body)?;
                schemas.insert(id, schema);
            }
            OP_CHANNEL => {
                let (id, channel) = parse_channel(body)?;
                channels.insert(id, channel);
            }
            OP_MESSAGE => parse_message(body, &channels, &schemas, &mut samples)?,
            OP_CHUNK => parse_chunk(body, &mut schemas, &mut channels, &mut samples)?,
            OP_DATA_END | OP_FOOTER => {}
            0x07..=0x0E => {}
            other => {
                return Err(AdaptError::Refused(format!(
                    "mcap contains unsupported opcode {other}"
                )));
            }
        }
        if samples.len() > MAX_MESSAGES {
            return Err(AdaptError::Refused(
                "mcap exceeds message limit".to_string(),
            ));
        }
    }
    if samples.is_empty() {
        return Err(AdaptError::Refused(
            "mcap contains no JSON attitude poses".to_string(),
        ));
    }
    samples.sort_by(|a, b| a.0.total_cmp(&b.0));
    let t0 = samples[0].0;
    let mut csv = String::from("t,qx,qy,qz,qw\n");
    for (t, x, y, z, w) in &samples {
        csv.push_str(&format!("{},{},{},{},{}\n", t - t0, x, y, z, w));
    }
    let (frame_from, frame_to) = declared_frames(&channels)?;
    let manifest = serde_json::json!({
        "schema": "quatopsy.manifest/1",
        "component_order": "xyzw",
        "rotation_sense": "active",
        "frame_from": frame_from,
        "frame_to": frame_to,
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qx", "qy", "qz", "qw"]}
    })
    .to_string();
    Ok(AdapterOutput {
        csv,
        manifest,
        provenance: provenance_json(
            AdapterFormat::McapJson,
            source,
            version,
            samples.len() as u64,
        )?,
    })
}

fn declared_frames(channels: &BTreeMap<u16, Channel>) -> Result<(String, String), AdaptError> {
    let attitude: Vec<_> = channels
        .values()
        .filter(|ch| ch.encoding == "json")
        .collect();
    if attitude.len() != 1 {
        return Err(AdaptError::Refused(
            "mcap must contain exactly one json attitude channel".to_string(),
        ));
    }
    let meta = &attitude[0].metadata;
    let from = meta.get("frame_from").cloned().ok_or_else(|| {
        AdaptError::Refused("mcap channel metadata must declare frame_from".to_string())
    })?;
    let to = meta.get("frame_to").cloned().ok_or_else(|| {
        AdaptError::Refused("mcap channel metadata must declare frame_to".to_string())
    })?;
    if from.is_empty() || to.is_empty() || from == to {
        return Err(AdaptError::Refused(
            "mcap frame_from and frame_to must be distinct and non-empty".to_string(),
        ));
    }
    Ok((from, to))
}

fn parse_chunk(
    body: &[u8],
    schemas: &mut BTreeMap<u16, Schema>,
    channels: &mut BTreeMap<u16, Channel>,
    samples: &mut Vec<(f64, f64, f64, f64, f64)>,
) -> Result<(), AdaptError> {
    if body.len() < 36 {
        return Err(AdaptError::Refused("mcap chunk is truncated".to_string()));
    }
    let mut pos = 28;
    let compression = read_string(body, &mut pos)?;
    if !compression.is_empty() {
        return Err(AdaptError::Refused(
            "mcap compressed chunks are refused".to_string(),
        ));
    }
    let rec_len = read_u64_at(body, pos)?;
    pos += 8;
    let rec_end = pos
        .checked_add(rec_len as usize)
        .ok_or_else(|| AdaptError::Refused("mcap chunk records overflow".to_string()))?;
    if rec_end > body.len() {
        return Err(AdaptError::Refused(
            "mcap chunk records overrun".to_string(),
        ));
    }
    let mut inner = &body[pos..rec_end];
    let mut n = 0_usize;
    while inner.len() >= 9 {
        n += 1;
        if n > MAX_RECORDS {
            return Err(AdaptError::Refused(
                "mcap chunk exceeds record limit".to_string(),
            ));
        }
        let opcode = inner[0];
        let len = read_u64(inner, 1)?;
        let start = 9_usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| AdaptError::Refused("mcap nested record overflow".to_string()))?;
        if end > inner.len() {
            return Err(AdaptError::Refused(
                "mcap nested record overruns chunk".to_string(),
            ));
        }
        let rec = &inner[start..end];
        match opcode {
            OP_SCHEMA => {
                let (id, schema) = parse_schema(rec)?;
                schemas.insert(id, schema);
            }
            OP_CHANNEL => {
                let (id, channel) = parse_channel(rec)?;
                channels.insert(id, channel);
            }
            OP_MESSAGE => parse_message(rec, channels, schemas, samples)?,
            OP_CHUNK => {
                return Err(AdaptError::Refused(
                    "mcap nested chunks are refused".to_string(),
                ));
            }
            _ => {}
        }
        inner = &inner[end..];
    }
    Ok(())
}

fn parse_schema(body: &[u8]) -> Result<(u16, Schema), AdaptError> {
    let mut pos = 0;
    let id = read_u16_at(body, pos)?;
    pos += 2;
    let name = read_string(body, &mut pos)?;
    let _encoding = read_string(body, &mut pos)?;
    Ok((id, Schema { name }))
}

fn parse_channel(body: &[u8]) -> Result<(u16, Channel), AdaptError> {
    let mut pos = 0;
    let id = read_u16_at(body, pos)?;
    pos += 2;
    let schema_id = read_u16_at(body, pos)?;
    pos += 2;
    let _topic = read_string(body, &mut pos)?;
    let encoding = read_string(body, &mut pos)?;
    let metadata = read_map(body, &mut pos)?;
    Ok((
        id,
        Channel {
            schema_id,
            encoding,
            metadata,
        },
    ))
}

fn parse_message(
    body: &[u8],
    channels: &BTreeMap<u16, Channel>,
    schemas: &BTreeMap<u16, Schema>,
    samples: &mut Vec<(f64, f64, f64, f64, f64)>,
) -> Result<(), AdaptError> {
    if body.len() < 22 {
        return Err(AdaptError::Refused("mcap message is truncated".to_string()));
    }
    let channel_id = read_u16_at(body, 0)?;
    let log_time = read_u64_at(body, 6)?;
    let data = &body[22..];
    let channel = channels.get(&channel_id).ok_or_else(|| {
        AdaptError::Refused(format!(
            "mcap message references unknown channel {channel_id}"
        ))
    })?;
    if channel.encoding != "json" {
        return Ok(());
    }
    if let Some(schema) = schemas.get(&channel.schema_id)
        && schema.name != "quatopsy.adapt.ros-json/1"
        && schema.name != "quatopsy.adapt.mcap-json/1"
    {
        return Err(AdaptError::Refused(format!(
            "unsupported mcap schema {}",
            schema.name
        )));
    }
    if serde_json::from_slice::<serde_json::Value>(data)
        .ok()
        .and_then(|v| v.get("result").cloned())
        .is_some()
    {
        return Err(AdaptError::Refused(
            "adapter input must not contain a result field".to_string(),
        ));
    }
    let pose: Pose = serde_json::from_slice(data)
        .map_err(|err| AdaptError::Refused(format!("mcap json pose parse failed: {err}")))?;
    let t = pose.t.unwrap_or(log_time as f64 / 1_000_000_000.0);
    samples.push((t, pose.x, pose.y, pose.z, pose.w));
    Ok(())
}

fn read_string(buf: &[u8], pos: &mut usize) -> Result<String, AdaptError> {
    let len = read_u32_at(buf, *pos)? as usize;
    *pos += 4;
    let end = pos
        .checked_add(len)
        .ok_or_else(|| AdaptError::Refused("mcap string overflow".to_string()))?;
    if end > buf.len() {
        return Err(AdaptError::Refused(
            "mcap string overruns record".to_string(),
        ));
    }
    let text = std::str::from_utf8(&buf[*pos..end])
        .map_err(|_| AdaptError::Refused("mcap string is not UTF-8".to_string()))?;
    *pos = end;
    Ok(text.to_string())
}

fn read_map(buf: &[u8], pos: &mut usize) -> Result<BTreeMap<String, String>, AdaptError> {
    let count = read_u32_at(buf, *pos)? as usize;
    *pos += 4;
    let mut map = BTreeMap::new();
    for _ in 0..count {
        let key = read_string(buf, pos)?;
        let value = read_string(buf, pos)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn read_u16_at(buf: &[u8], pos: usize) -> Result<u16, AdaptError> {
    buf.get(pos..pos + 2)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| AdaptError::Refused("mcap truncated u16".to_string()))
}

fn read_u32_at(buf: &[u8], pos: usize) -> Result<u32, AdaptError> {
    buf.get(pos..pos + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| AdaptError::Refused("mcap truncated u32".to_string()))
}

fn read_u64(buf: &[u8], pos: usize) -> Result<u64, AdaptError> {
    read_u64_at(buf, pos)
}

fn read_u64_at(buf: &[u8], pos: usize) -> Result<u64, AdaptError> {
    buf.get(pos..pos + 8)
        .and_then(|s| s.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| AdaptError::Refused("mcap truncated u64".to_string()))
}

pub fn encode_mcap_json_poses(
    frame_from: &str,
    frame_to: &str,
    poses: &[(f64, f64, f64, f64, f64)],
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(MAGIC);
    write_record(&mut data, OP_HEADER, |buf| {
        put_str(buf, "quatopsy");
        put_str(buf, "quatopsy-adapt");
    });
    write_record(&mut data, OP_SCHEMA, |buf| {
        buf.extend_from_slice(&1u16.to_le_bytes());
        put_str(buf, "quatopsy.adapt.mcap-json/1");
        put_str(buf, "jsonschema");
        put_bytes(buf, b"{}");
    });
    write_record(&mut data, OP_CHANNEL, |buf| {
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        put_str(buf, "/attitude");
        put_str(buf, "json");
        buf.extend_from_slice(&2u32.to_le_bytes());
        put_str(buf, "frame_from");
        put_str(buf, frame_from);
        put_str(buf, "frame_to");
        put_str(buf, frame_to);
    });
    for (i, (t, x, y, z, w)) in poses.iter().enumerate() {
        let payload = format!(r#"{{"t":{t},"x":{x},"y":{y},"z":{z},"w":{w}}}"#);
        write_record(&mut data, OP_MESSAGE, |buf| {
            buf.extend_from_slice(&1u16.to_le_bytes());
            buf.extend_from_slice(&(i as u32).to_le_bytes());
            let ns = (*t * 1_000_000_000.0) as u64;
            buf.extend_from_slice(&ns.to_le_bytes());
            buf.extend_from_slice(&ns.to_le_bytes());
            buf.extend_from_slice(payload.as_bytes());
        });
    }
    write_record(&mut data, OP_DATA_END, |buf| {
        buf.extend_from_slice(&0u32.to_le_bytes());
    });
    write_record(&mut data, OP_FOOTER, |buf| {
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
    });
    data.extend_from_slice(MAGIC);
    data
}

fn write_record(out: &mut Vec<u8>, opcode: u8, fill: impl FnOnce(&mut Vec<u8>)) {
    let mut body = Vec::new();
    fill(&mut body);
    out.push(opcode);
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(&body);
}

fn put_str(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value.as_bytes());
}

fn put_bytes(buf: &mut Vec<u8>, value: &[u8]) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value);
}
