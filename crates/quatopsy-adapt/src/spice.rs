//! Little-endian IEEE DAF C-kernel type 3 reader. Discrete samples only.

use crate::{AdaptError, AdapterFormat, AdapterOutput, provenance_json};

const RECORD: usize = 1024;
const WORDS_PER_RECORD: i32 = 128;
const MAX_SEGMENTS: usize = 32;
const MAX_SAMPLES: usize = 100_000;

pub fn adapt_spice_ck(source: &[u8], version: &str) -> Result<AdapterOutput, AdaptError> {
    if source.len() < RECORD {
        return Err(AdaptError::Refused(
            "spice ck is shorter than one DAF record".to_string(),
        ));
    }
    if source.len() % RECORD != 0 {
        return Err(AdaptError::Refused(
            "spice ck is not a multiple of 1024 bytes".to_string(),
        ));
    }
    let locidw = std::str::from_utf8(&source[0..8])
        .map_err(|_| AdaptError::Refused("spice ck id word is not UTF-8".to_string()))?
        .trim();
    if locidw != "DAF/CK" {
        return Err(AdaptError::Refused(format!(
            "spice adapter requires DAF/CK, found {locidw}"
        )));
    }
    let nd = i32::from_le_bytes(source[8..12].try_into().unwrap());
    let ni = i32::from_le_bytes(source[12..16].try_into().unwrap());
    if nd != 2 || ni != 6 {
        return Err(AdaptError::Refused(
            "spice ck summaries must use ND=2 and NI=6".to_string(),
        ));
    }
    let locfmt = std::str::from_utf8(&source[88..96])
        .unwrap_or("")
        .trim_end();
    if locfmt != "LTL-IEEE" {
        return Err(AdaptError::Refused(
            "spice adapter reads LTL-IEEE kernels only".to_string(),
        ));
    }
    let fward = i32::from_le_bytes(source[76..80].try_into().unwrap());
    if fward < 2 {
        return Err(AdaptError::Refused(
            "spice ck summary pointer is invalid".to_string(),
        ));
    }
    let ss = nd as usize + (ni as usize).div_ceil(2);
    let mut samples = Vec::new();
    let mut frame_from = String::from("CK");
    let mut frame_to = String::from("NAIF");
    let mut recno = fward;
    let mut seen = 0_usize;
    while recno != 0 {
        seen += 1;
        if seen > MAX_SEGMENTS {
            return Err(AdaptError::Refused(
                "spice ck exceeds segment-record limit".to_string(),
            ));
        }
        let rec = record(source, recno)?;
        let next = rec[0] as i32;
        let nsum = rec[2] as usize;
        for i in 0..nsum {
            let base = 3 + i * ss;
            let summary = &rec[base..base + ss];
            let inst_frame = unpack_i32_pair(summary[2]);
            let type_av = unpack_i32_pair(summary[3]);
            let begin_end = unpack_i32_pair(summary[4]);
            let inst = inst_frame.0;
            let frame = inst_frame.1;
            let dtype = type_av.0;
            let avflag = type_av.1;
            let begin = begin_end.0;
            let end = begin_end.1;
            if dtype != 3 {
                return Err(AdaptError::Refused(format!(
                    "spice adapter supports CK type 3 only, found {dtype}"
                )));
            }
            frame_from = format!("CK-{inst}");
            frame_to = format!("NAIF-{frame}");
            let words = load_words(source, begin, end)?;
            extract_type3(&words, avflag == 1, &mut samples)?;
        }
        recno = next;
    }
    if samples.is_empty() {
        return Err(AdaptError::Refused(
            "spice ck contains no type 3 pointing".to_string(),
        ));
    }
    samples.sort_by(|a, b| a.0.total_cmp(&b.0));
    let t0 = samples[0].0;
    let mut csv = String::from("t,qw,qx,qy,qz\n");
    for (t, q0, q1, q2, q3) in &samples {
        csv.push_str(&format!("{:.9},{q0},{q1},{q2},{q3}\n", t - t0));
    }
    let manifest = serde_json::json!({
        "schema": "quatopsy.manifest/1",
        "component_order": "wxyz",
        "rotation_sense": "active",
        "frame_from": frame_from,
        "frame_to": frame_to,
        "time_unit": "s",
        "columns": {"time": "t", "quaternion": ["qw", "qx", "qy", "qz"]}
    })
    .to_string();
    Ok(AdapterOutput {
        csv,
        manifest,
        provenance: provenance_json(
            AdapterFormat::SpiceCk,
            source,
            version,
            samples.len() as u64,
        )?,
    })
}

fn extract_type3(
    words: &[f64],
    has_av: bool,
    samples: &mut Vec<(f64, f64, f64, f64, f64)>,
) -> Result<(), AdaptError> {
    if words.len() < 3 {
        return Err(AdaptError::Refused(
            "spice type 3 segment is truncated".to_string(),
        ));
    }
    let nprec = words[words.len() - 1] as usize;
    let _numint = words[words.len() - 2] as usize;
    if nprec == 0 || nprec > MAX_SAMPLES {
        return Err(AdaptError::Refused(
            "spice type 3 pointing count is invalid".to_string(),
        ));
    }
    let rec_len = if has_av { 7 } else { 4 };
    let dir_len = nprec.saturating_sub(1) / 100;
    if words.len() < rec_len * nprec + nprec + dir_len + 2 {
        return Err(AdaptError::Refused(
            "spice type 3 layout is inconsistent".to_string(),
        ));
    }
    let times = &words[rec_len * nprec..rec_len * nprec + nprec];
    for (i, time) in times.iter().enumerate() {
        let off = i * rec_len;
        samples.push((
            *time,
            words[off],
            words[off + 1],
            words[off + 2],
            words[off + 3],
        ));
    }
    Ok(())
}

fn record(source: &[u8], recno: i32) -> Result<[f64; 128], AdaptError> {
    if recno < 1 {
        return Err(AdaptError::Refused(
            "spice ck record number is invalid".to_string(),
        ));
    }
    let start = (recno as usize - 1) * RECORD;
    let end = start + RECORD;
    if end > source.len() {
        return Err(AdaptError::Refused(
            "spice ck record is outside the file".to_string(),
        ));
    }
    let mut out = [0.0; 128];
    for (i, slot) in out.iter_mut().enumerate() {
        let o = start + i * 8;
        *slot = f64::from_le_bytes(source[o..o + 8].try_into().unwrap());
    }
    Ok(out)
}

fn load_words(source: &[u8], begin: i32, end: i32) -> Result<Vec<f64>, AdaptError> {
    if begin < 1 || end < begin {
        return Err(AdaptError::Refused(
            "spice ck array addresses are invalid".to_string(),
        ));
    }
    let count = (end - begin + 1) as usize;
    if count > MAX_SAMPLES * 8 {
        return Err(AdaptError::Refused(
            "spice ck array exceeds size limit".to_string(),
        ));
    }
    let mut words = Vec::with_capacity(count);
    for addr in begin..=end {
        let off = (addr as usize - 1) * 8;
        if off + 8 > source.len() {
            return Err(AdaptError::Refused(
                "spice ck array overruns file".to_string(),
            ));
        }
        words.push(f64::from_le_bytes(source[off..off + 8].try_into().unwrap()));
    }
    Ok(words)
}

fn unpack_i32_pair(word: f64) -> (i32, i32) {
    let b = word.to_le_bytes();
    (
        i32::from_le_bytes(b[0..4].try_into().unwrap()),
        i32::from_le_bytes(b[4..8].try_into().unwrap()),
    )
}

fn pack_i32_pair(a: i32, b: i32) -> f64 {
    let mut bts = [0u8; 8];
    bts[0..4].copy_from_slice(&a.to_le_bytes());
    bts[4..8].copy_from_slice(&b.to_le_bytes());
    f64::from_le_bytes(bts)
}

/// Discrete type 3 CK with one interpolation interval and no angular velocity.
pub fn encode_ck_type3(inst: i32, frame: i32, samples: &[(f64, f64, f64, f64, f64)]) -> Vec<u8> {
    let nprec = samples.len() as i32;
    let mut array = Vec::new();
    for sample in samples {
        array.extend_from_slice(&[sample.1, sample.2, sample.3, sample.4]);
    }
    for sample in samples {
        array.push(sample.0);
    }
    array.push(samples[0].0);
    array.push(1.0);
    array.push(f64::from(nprec));
    let begin = 3 * WORDS_PER_RECORD + 1;
    let end = begin + array.len() as i32 - 1;
    let free = end + 1;
    let mut file = vec![0u8; RECORD * 4];
    file[0..8].copy_from_slice(b"DAF/CK  ");
    file[8..12].copy_from_slice(&2i32.to_le_bytes());
    file[12..16].copy_from_slice(&6i32.to_le_bytes());
    let mut ifname = [b' '; 60];
    let label = b"quatopsy type 3 fixture";
    ifname[..label.len()].copy_from_slice(label);
    file[16..76].copy_from_slice(&ifname);
    file[76..80].copy_from_slice(&2i32.to_le_bytes());
    file[80..84].copy_from_slice(&2i32.to_le_bytes());
    file[84..88].copy_from_slice(&free.to_le_bytes());
    file[88..96].copy_from_slice(b"LTL-IEEE");
    let mut summary = [0.0f64; 128];
    summary[2] = 1.0;
    summary[3] = samples[0].0;
    summary[4] = samples.last().unwrap().0;
    summary[5] = pack_i32_pair(inst, frame);
    summary[6] = pack_i32_pair(3, 0);
    summary[7] = pack_i32_pair(begin, end);
    write_record(&mut file, 2, &summary);
    let mut names = [0u8; RECORD];
    let label = b"QUATOPSY_TYPE3";
    names[..label.len()].copy_from_slice(label);
    file[RECORD * 2..RECORD * 3].copy_from_slice(&names);
    for (i, word) in array.iter().enumerate() {
        let off = (begin as usize - 1 + i) * 8;
        if off + 8 > file.len() {
            file.resize((off / RECORD + 2) * RECORD, 0);
        }
        file[off..off + 8].copy_from_slice(&word.to_le_bytes());
    }
    if file.len() % RECORD != 0 {
        file.resize((file.len() / RECORD + 1) * RECORD, 0);
    }
    file
}

fn write_record(file: &mut [u8], recno: i32, words: &[f64; 128]) {
    let start = (recno as usize - 1) * RECORD;
    for (i, word) in words.iter().enumerate() {
        let o = start + i * 8;
        file[o..o + 8].copy_from_slice(&word.to_le_bytes());
    }
}
