use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, error::ErrorKind};
use quatopsy_core::cancel::Cancel;
use quatopsy_core::identity::{analysis_id, sha256_hex};
use quatopsy_core::ingest::ingest_bytes;
use quatopsy_core::limits::{
    Limits, SAFE_MAX_COLUMNS, SAFE_MAX_FIELD_BYTES, SAFE_MAX_FINDINGS_PER_RULE,
    SAFE_MAX_INPUT_BYTES, SAFE_MAX_SAMPLES, SAFE_TIMEOUT_MS,
};
use quatopsy_core::repair::{plan_by_id, render_repaired_csv};
use quatopsy_core::repro::{self, provenance, slice_csv};
use quatopsy_core::{AnalyzeRequest, analyze, report_bytes};
use quatopsy_schema::{RepairDisposition, Report};

const USAGE_EXIT: u8 = 64;

#[derive(Parser, Debug)]
#[command(
    name = "quatopsy",
    version,
    about = "Local quaternion trajectory linter"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyse a CSV trajectory against an explicit convention manifest.
    Analyze {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        report: PathBuf,
        #[arg(long, default_value_t = SAFE_MAX_INPUT_BYTES)]
        max_input_bytes: u64,
        #[arg(long, default_value_t = SAFE_MAX_SAMPLES)]
        max_samples: u64,
        #[arg(long, default_value_t = SAFE_MAX_FIELD_BYTES)]
        max_field_bytes: u64,
        #[arg(long, default_value_t = SAFE_MAX_COLUMNS)]
        max_columns: u64,
        #[arg(long, default_value_t = SAFE_MAX_FINDINGS_PER_RULE)]
        max_findings_per_rule: u64,
        #[arg(long, default_value_t = SAFE_TIMEOUT_MS)]
        timeout_ms: u64,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        /// Delete leftover sibling `.tmp` files and run a fresh analysis.
        #[arg(long, default_value_t = false)]
        clean: bool,
        /// Write proposed repair CSVs into this directory.
        #[arg(long)]
        repairs_dir: Option<PathBuf>,
        /// Write a minimal reproducible slice into this directory.
        #[arg(long)]
        repro_dir: Option<PathBuf>,
        /// Include local paths in repro provenance. Off by default.
        #[arg(long, default_value_t = false)]
        include_paths: bool,
    },
    /// Materialise a proposed repair into a new CSV. Never overwrites the source.
    Repair {
        #[arg(long)]
        report: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        repair_id: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        #[arg(long, default_value_t = false)]
        clean: bool,
    },
}

fn main() -> ExitCode {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let _ = ctrlc::set_handler(move || {
        flag.store(true, Ordering::SeqCst);
    });

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            return match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => ExitCode::SUCCESS,
                _ => ExitCode::from(USAGE_EXIT),
            };
        }
    };

    match cli.command {
        Commands::Analyze {
            input,
            manifest,
            report,
            max_input_bytes,
            max_samples,
            max_field_bytes,
            max_columns,
            max_findings_per_rule,
            timeout_ms,
            overwrite,
            clean,
            repairs_dir,
            repro_dir,
            include_paths,
        } => {
            if let Err(code) = run_analyze(
                input,
                manifest,
                report,
                Limits {
                    max_input_bytes,
                    max_samples,
                    max_field_bytes,
                    max_columns,
                    max_findings_per_rule,
                    timeout_ms,
                },
                overwrite,
                clean,
                repairs_dir,
                repro_dir,
                include_paths,
                &cancelled,
            ) {
                return code;
            }
            ExitCode::SUCCESS
        }
        Commands::Repair {
            report,
            input,
            manifest,
            repair_id,
            output,
            overwrite,
            clean,
        } => {
            if let Err(code) = run_repair(
                report, input, manifest, repair_id, output, overwrite, clean, &cancelled,
            ) {
                return code;
            }
            ExitCode::SUCCESS
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_analyze(
    input: PathBuf,
    manifest: PathBuf,
    report_path: PathBuf,
    limits: Limits,
    overwrite: bool,
    clean: bool,
    repairs_dir: Option<PathBuf>,
    repro_dir: Option<PathBuf>,
    include_paths: bool,
    cancelled: &AtomicBool,
) -> Result<(), ExitCode> {
    let mut temps = TempGuard::default();
    refuse_if_exists(&report_path, overwrite)?;
    guard_path(&input, false)?;
    guard_path(&manifest, false)?;
    guard_output_path(&report_path)?;
    if clean {
        remove_sibling_tmp(&report_path);
    }
    let csv_bytes = read_bounded(&input, limits.max_input_bytes)?;
    let manifest_bytes = read_bounded(&manifest, limits.max_input_bytes)?;
    if cancelled.load(Ordering::Relaxed) {
        return cancelled_exit(&mut temps);
    }
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv_bytes,
        manifest_bytes: &manifest_bytes,
        engine_version: env!("CARGO_PKG_VERSION"),
        limits,
        cancelled: Some(cancelled),
    });
    let encoded = report_bytes(&report).map_err(|err| {
        eprintln!("error: canonical JSON serialization failed: {err}");
        ExitCode::from(3)
    })?;
    write_atomic(&report_path, &encoded, &mut temps).map_err(|err| {
        eprintln!(
            "error: could not write report {}: {err}",
            report_path.display()
        );
        ExitCode::from(3)
    })?;
    if let Some(dir) = repairs_dir {
        emit_repairs(
            &dir,
            &csv_bytes,
            &manifest_bytes,
            &report,
            limits,
            cancelled,
            overwrite,
            &mut temps,
        )?;
    }
    if let Some(dir) = repro_dir
        && !report.findings.is_empty()
    {
        emit_repro(
            &dir,
            &csv_bytes,
            &manifest_bytes,
            &report,
            include_paths.then(|| input.display().to_string()),
            overwrite,
            &mut temps,
        )?;
    }
    print_summary(&report);
    temps.disarm();
    let code = report.result.exit_code();
    if code == 0 {
        Ok(())
    } else {
        Err(ExitCode::from(code as u8))
    }
}

#[allow(clippy::too_many_arguments)]
fn run_repair(
    report_path: PathBuf,
    input: PathBuf,
    manifest: PathBuf,
    repair_id: String,
    output: PathBuf,
    overwrite: bool,
    clean: bool,
    cancelled: &AtomicBool,
) -> Result<(), ExitCode> {
    let mut temps = TempGuard::default();
    refuse_if_exists(&output, overwrite)?;
    guard_path(&report_path, false)?;
    guard_path(&input, false)?;
    guard_path(&manifest, false)?;
    guard_output_path(&output)?;
    if output == input {
        eprintln!("error: repair output must not be the source CSV");
        return Err(ExitCode::from(USAGE_EXIT));
    }
    if clean {
        remove_sibling_tmp(&output);
    }
    let report_bytes = read_bounded(&report_path, SAFE_MAX_INPUT_BYTES)?;
    let report: Report = serde_json::from_slice(&report_bytes).map_err(|err| {
        eprintln!("error: report could not be parsed: {err}");
        ExitCode::from(USAGE_EXIT)
    })?;
    if report.schema != quatopsy_schema::REPORT_SCHEMA {
        eprintln!("error: unsupported report schema {}", report.schema);
        return Err(ExitCode::from(2));
    }
    let csv_bytes = read_bounded(&input, SAFE_MAX_INPUT_BYTES)?;
    let manifest_bytes = read_bounded(&manifest, SAFE_MAX_INPUT_BYTES)?;
    if sha256_hex(&csv_bytes) != report.input.csv_sha256
        || sha256_hex(&manifest_bytes) != report.input.manifest_sha256
    {
        eprintln!("error: input digest does not match the report analysis identity");
        return Err(ExitCode::from(2));
    }
    let limits = Limits::from_report(&report.limits);
    if analysis_id(&csv_bytes, &manifest_bytes, &report.tool.version, limits) != report.analysis_id
    {
        eprintln!("error: report analysis identity does not match these inputs and limits");
        return Err(ExitCode::from(2));
    }
    if !report
        .repairs
        .iter()
        .any(|item| item.id == repair_id && item.disposition == RepairDisposition::Proposed)
    {
        eprintln!("error: repair {repair_id} is not a proposed repair in this report");
        return Err(ExitCode::from(2));
    }
    if cancelled.load(Ordering::Relaxed) {
        return cancelled_exit(&mut temps);
    }
    let parsed = ingest_bytes(
        &csv_bytes,
        &manifest_bytes,
        limits,
        Cancel {
            deadline: Instant::now() + Duration::from_millis(limits.timeout_ms.max(1)),
            flag: Some(cancelled),
        },
    )
    .map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    let Some(mut plan) = plan_by_id(&parsed.samples, &report.analysis_id, &repair_id) else {
        eprintln!("error: repair {repair_id} is not available for this analysis");
        return Err(ExitCode::from(2));
    };
    if plan.repair.source_analysis_id != report.analysis_id {
        eprintln!("error: repair is bound to a different analysis");
        return Err(ExitCode::from(2));
    }
    let rendered = render_repaired_csv(
        &csv_bytes,
        &parsed.declarations,
        &parsed.samples,
        &plan.quaternions,
    )
    .map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    plan.repair.output_digest = Some(sha256_hex(&rendered));
    write_atomic(&output, &rendered, &mut temps).map_err(|err| {
        eprintln!("error: could not write {}: {err}", output.display());
        ExitCode::from(3)
    })?;
    println!("repair: {}", plan.repair.id);
    println!(
        "output_digest: {}",
        plan.repair.output_digest.as_deref().unwrap_or("")
    );
    temps.disarm();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_repairs(
    dir: &Path,
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    report: &Report,
    limits: Limits,
    cancelled: &AtomicBool,
    overwrite: bool,
    temps: &mut TempGuard,
) -> Result<(), ExitCode> {
    fs::create_dir_all(dir).map_err(|err| {
        eprintln!("error: cannot create {}: {err}", dir.display());
        ExitCode::from(3)
    })?;
    let parsed = ingest_bytes(
        csv_bytes,
        manifest_bytes,
        limits,
        Cancel {
            deadline: Instant::now() + Duration::from_millis(limits.timeout_ms.max(1)),
            flag: Some(cancelled),
        },
    )
    .map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    for repair in &report.repairs {
        if repair.disposition != RepairDisposition::Proposed {
            continue;
        }
        let Some(plan) = plan_by_id(&parsed.samples, &report.analysis_id, &repair.id) else {
            continue;
        };
        let rendered = render_repaired_csv(
            csv_bytes,
            &parsed.declarations,
            &parsed.samples,
            &plan.quaternions,
        )
        .map_err(|err| {
            eprintln!("error: {err}");
            ExitCode::from(3)
        })?;
        let path = dir.join(format!("{}.csv", repair.id.replace(':', "_")));
        refuse_if_exists(&path, overwrite)?;
        write_atomic(&path, &rendered, temps).map_err(|err| {
            eprintln!("error: could not write {}: {err}", path.display());
            ExitCode::from(3)
        })?;
    }
    Ok(())
}

fn emit_repro(
    dir: &Path,
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    report: &Report,
    input_path: Option<String>,
    overwrite: bool,
    temps: &mut TempGuard,
) -> Result<(), ExitCode> {
    let Some((start, end)) = repro::repro_bounds(&report.findings, report.input.sample_count)
    else {
        return Ok(());
    };
    fs::create_dir_all(dir).map_err(|err| {
        eprintln!("error: cannot create {}: {err}", dir.display());
        ExitCode::from(3)
    })?;
    let slice = slice_csv(csv_bytes, start, end).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    let slice_path = dir.join("slice.csv");
    let manifest_path = dir.join("manifest.json");
    let provenance_path = dir.join("provenance.json");
    refuse_if_exists(&slice_path, overwrite)?;
    refuse_if_exists(&manifest_path, overwrite)?;
    refuse_if_exists(&provenance_path, overwrite)?;
    write_atomic(&slice_path, &slice, temps).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    write_atomic(&manifest_path, manifest_bytes, temps).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    let meta = provenance(
        report.analysis_id.clone(),
        report.input.csv_sha256.clone(),
        report.input.manifest_sha256.clone(),
        start,
        end,
        &report.findings,
        input_path,
    );
    let encoded = serde_json::to_vec_pretty(&meta).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    write_atomic(&provenance_path, &encoded, temps).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    Ok(())
}

fn refuse_if_exists(path: &Path, overwrite: bool) -> Result<(), ExitCode> {
    if path.exists() && !overwrite {
        eprintln!(
            "error: {} exists; pass --overwrite to replace it",
            path.display()
        );
        return Err(ExitCode::from(USAGE_EXIT));
    }
    Ok(())
}

fn guard_path(path: &Path, missing_ok: bool) -> Result<(), ExitCode> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                eprintln!("error: refusing symlink {}", path.display());
                return Err(ExitCode::from(3));
            }
            if !meta.file_type().is_file() {
                eprintln!("error: refusing special file {}", path.display());
                return Err(ExitCode::from(3));
            }
            Ok(())
        }
        Err(err) if missing_ok && err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => {
            eprintln!("error: cannot stat {}: {err}", path.display());
            Err(ExitCode::from(USAGE_EXIT))
        }
    }
}

fn guard_output_path(path: &Path) -> Result<(), ExitCode> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                eprintln!("error: refusing symlink output {}", path.display());
                return Err(ExitCode::from(3));
            }
            if !meta.file_type().is_file() {
                eprintln!("error: refusing special output {}", path.display());
                return Err(ExitCode::from(3));
            }
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => {
            eprintln!("error: cannot stat {}: {err}", path.display());
            Err(ExitCode::from(USAGE_EXIT))
        }
    }
}

fn remove_sibling_tmp(path: &Path) {
    let parent = path
        .parent()
        .filter(|item| !item.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if let Some(name) = path.file_name() {
        let mut tmp = name.to_os_string();
        tmp.push(".tmp");
        let tmp_path = parent.join(tmp);
        let _ = fs::remove_file(tmp_path);
    }
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, ExitCode> {
    let meta = fs::metadata(path).map_err(|err| {
        eprintln!("error: cannot stat {}: {err}", path.display());
        ExitCode::from(USAGE_EXIT)
    })?;
    if meta.len() > max_bytes {
        eprintln!(
            "error: {} is {} bytes, limit is {max_bytes}",
            path.display(),
            meta.len()
        );
        return Err(ExitCode::from(3));
    }
    fs::read(path).map_err(|err| {
        eprintln!("error: cannot read {}: {err}", path.display());
        ExitCode::from(USAGE_EXIT)
    })
}

fn write_atomic(path: &Path, bytes: &[u8], temps: &mut TempGuard) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|item| !item.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut tmp_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "output.tmp".into());
    tmp_name.push(".tmp");
    let tmp_path = parent.join(tmp_name);
    if tmp_path.exists() {
        fs::remove_file(&tmp_path)?;
    }
    temps.paths.push(tmp_path.clone());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp_path, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp_path);
    })?;
    temps.paths.retain(|item| item != &tmp_path);
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn cancelled_exit(temps: &mut TempGuard) -> Result<(), ExitCode> {
    temps.cleanup();
    eprintln!("error: analysis was cancelled");
    Err(ExitCode::from(3))
}

#[derive(Default)]
struct TempGuard {
    paths: Vec<PathBuf>,
    disarmed: bool,
}

impl TempGuard {
    fn disarm(&mut self) {
        self.disarmed = true;
        self.paths.clear();
    }

    fn cleanup(&mut self) {
        for path in self.paths.drain(..) {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            self.cleanup();
        }
    }
}

fn print_summary(report: &Report) {
    println!("result: {}", report.result.as_str());
    println!("analysis_id: {}", report.analysis_id);
    println!("samples: {}", report.input.sample_count);
    for rule in &report.rule_results {
        println!(
            "rule {}: {} ({})",
            rule.rule,
            rule.state.as_str(),
            rule.finding_count
        );
    }
    for repair in &report.repairs {
        println!(
            "repair {}: {} ({})",
            repair.id,
            repair.disposition.as_str(),
            repair.affected_rows.len()
        );
    }
    if !report.diagnostics.message.is_empty() {
        println!("message: {}", report.diagnostics.message);
    }
}
