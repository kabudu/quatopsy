use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum, error::ErrorKind};
use quatopsy_core::cancel::Cancel;
use quatopsy_core::identity::{analysis_id, sha256_hex};
use quatopsy_core::ingest::ingest_bytes;
use quatopsy_core::limits::{
    Limits, SAFE_MAX_COLUMNS, SAFE_MAX_FIELD_BYTES, SAFE_MAX_FINDINGS_PER_RULE,
    SAFE_MAX_INPUT_BYTES, SAFE_MAX_SAMPLES, SAFE_TIMEOUT_MS,
};
use quatopsy_core::repair::{plan_by_id, render_repaired_csv};
use quatopsy_core::repro::{self, provenance, slice_csv};
use quatopsy_core::view::{build_view, empty_view};
use quatopsy_core::{AnalyzeRequest, analyze, report_bytes};
use quatopsy_schema::{
    AdoptionMode, RepairDisposition, Report, VIEW_MAX_POINTS, VIEW_SAFE_MAX_POINTS,
    report_schema_supported,
};

mod policy;

const USAGE_EXIT: u8 = 64;
const MAX_REPRO_SLICES: usize = 1024;

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
        /// Adoption mode. Affects process exit only; the report result is unchanged.
        #[arg(long, value_enum, default_value_t = PolicyArg::Required)]
        policy: PolicyArg,
        /// Rule IDs that fail the process in selective mode.
        #[arg(long = "fail-on")]
        fail_on: Vec<String>,
        /// Override document. Suppresses named findings from process failure only.
        #[arg(long = "override-file")]
        override_file: Option<PathBuf>,
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
    /// Write a local static viewer bundle for a canonical report.
    View {
        #[arg(long)]
        report: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long, default_value_t = VIEW_MAX_POINTS)]
        max_points: u64,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        #[arg(long, default_value_t = false)]
        clean: bool,
    },
    /// Convert an external trajectory into canonical CSV and manifest. Never emits a verdict.
    Adapt {
        #[arg(long)]
        format: String,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    /// Generate a candidate reference trajectory. Never assigns a report result.
    Plan {
        #[arg(long)]
        problem: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PolicyArg {
    Advisory,
    Selective,
    Required,
}

impl From<PolicyArg> for AdoptionMode {
    fn from(value: PolicyArg) -> Self {
        match value {
            PolicyArg::Advisory => Self::Advisory,
            PolicyArg::Selective => Self::Selective,
            PolicyArg::Required => Self::Required,
        }
    }
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
            policy,
            fail_on,
            override_file,
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
                policy.into(),
                fail_on,
                override_file,
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
        Commands::View {
            report,
            output,
            input,
            manifest,
            max_points,
            overwrite,
            clean,
        } => {
            if let Err(code) = run_view(
                report, output, input, manifest, max_points, overwrite, clean, &cancelled,
            ) {
                return code;
            }
            ExitCode::SUCCESS
        }
        Commands::Adapt {
            format,
            input,
            output_dir,
            overwrite,
        } => {
            if let Err(code) = run_adapt(format, input, output_dir, overwrite) {
                return code;
            }
            ExitCode::SUCCESS
        }
        Commands::Plan {
            problem,
            output_dir,
            overwrite,
        } => {
            if let Err(code) = run_plan(problem, output_dir, overwrite, &cancelled) {
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
    policy: AdoptionMode,
    fail_on: Vec<String>,
    override_file: Option<PathBuf>,
    cancelled: &AtomicBool,
) -> Result<(), ExitCode> {
    let mut temps = TempGuard::default();
    if policy == AdoptionMode::Selective && fail_on.is_empty() {
        eprintln!("error: selective policy requires at least one --fail-on rule");
        return Err(ExitCode::from(USAGE_EXIT));
    }
    policy::validate_fail_on(&fail_on).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(USAGE_EXIT)
    })?;
    refuse_if_exists(&report_path, overwrite)?;
    guard_path(&input, false)?;
    guard_path(&manifest, false)?;
    guard_output_path(&report_path)?;
    if clean {
        remove_sibling_tmp(&report_path);
    }
    let csv_bytes = read_bounded(&input, limits.max_input_bytes)?;
    let manifest_bytes = read_bounded(&manifest, limits.max_input_bytes)?;
    let overridden = if let Some(path) = override_file {
        let bytes = read_bounded(&path, limits.max_input_bytes)?;
        policy::load_overrides(&bytes, &sha256_hex(&csv_bytes), &utc_now_stamp()).map_err(
            |err| {
                eprintln!("error: {err}");
                ExitCode::from(2)
            },
        )?
    } else {
        Vec::new()
    };
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
    let mut outputs = vec![PendingOutput::new(report_path.clone(), encoded, overwrite)];
    if let Some(dir) = repairs_dir {
        prepare_repairs(
            &dir,
            &csv_bytes,
            &manifest_bytes,
            &report,
            limits,
            cancelled,
            overwrite,
            &mut outputs,
        )?;
    }
    if let Some(dir) = repro_dir
        && !report.findings.is_empty()
    {
        prepare_repro(
            &dir,
            &csv_bytes,
            &manifest_bytes,
            &report,
            include_paths.then(|| input.display().to_string()),
            overwrite,
            &mut outputs,
        )?;
    }
    commit_outputs(&mut outputs, &mut temps).map_err(|err| {
        eprintln!("error: could not commit analysis outputs: {err}");
        ExitCode::from(3)
    })?;
    print_summary(&report);
    temps.disarm();
    let code = policy::exit_code(&report, policy, &fail_on, &overridden);
    if code == 0 {
        Ok(())
    } else {
        Err(ExitCode::from(code as u8))
    }
}

fn utc_now_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_stamp(secs)
}

fn civil_stamp(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + i64::from(m <= 2);
    (year as i32, m as u32, d as u32)
}

fn run_adapt(
    format: String,
    input: PathBuf,
    output_dir: PathBuf,
    overwrite: bool,
) -> Result<(), ExitCode> {
    let parsed = quatopsy_adapt::AdapterFormat::parse(&format).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(USAGE_EXIT)
    })?;
    guard_path(&input, false)?;
    if output_dir.exists() && !output_dir.is_dir() {
        eprintln!("error: adapter output must be a directory");
        return Err(ExitCode::from(USAGE_EXIT));
    }
    fs::create_dir_all(&output_dir).map_err(|err| {
        eprintln!("error: could not create {}: {err}", output_dir.display());
        ExitCode::from(3)
    })?;
    let csv_path = output_dir.join("input.csv");
    let manifest_path = output_dir.join("manifest.json");
    let provenance_path = output_dir.join("provenance.json");
    for path in [&csv_path, &manifest_path, &provenance_path] {
        refuse_if_exists(path, overwrite)?;
        guard_output_path(path)?;
    }
    let bytes = read_bounded(&input, SAFE_MAX_INPUT_BYTES)?;
    let out = quatopsy_adapt::adapt(parsed, &bytes, env!("CARGO_PKG_VERSION")).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(2)
    })?;
    fs::write(&csv_path, out.csv.as_bytes()).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    fs::write(&manifest_path, out.manifest.as_bytes()).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    fs::write(&provenance_path, out.provenance.as_bytes()).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?;
    println!("adapter: {}", output_dir.display());
    Ok(())
}

fn run_plan(
    problem: PathBuf,
    output_dir: PathBuf,
    overwrite: bool,
    cancelled: &AtomicBool,
) -> Result<(), ExitCode> {
    guard_path(&problem, false)?;
    if output_dir.exists() && !output_dir.is_dir() {
        eprintln!("error: planner output must be a directory");
        return Err(ExitCode::from(USAGE_EXIT));
    }
    let csv_path = output_dir.join("input.csv");
    let manifest_path = output_dir.join("manifest.json");
    let plan_path = output_dir.join("plan.json");
    for path in [&csv_path, &manifest_path, &plan_path] {
        refuse_if_exists(path, overwrite)?;
        guard_output_path(path)?;
    }
    let bytes = read_bounded(&problem, SAFE_MAX_INPUT_BYTES)?;
    if cancelled.load(Ordering::SeqCst) {
        eprintln!("error: planner was cancelled");
        return Err(ExitCode::from(3));
    }
    let out = match quatopsy_plan::plan(&bytes, env!("CARGO_PKG_VERSION")) {
        Ok(out) => out,
        Err(quatopsy_plan::PlanError::Infeasible(reason)) => {
            eprintln!("error: infeasible: {reason}");
            return Err(ExitCode::from(2));
        }
        Err(err) => {
            eprintln!("error: {err}");
            return Err(ExitCode::from(2));
        }
    };
    if cancelled.load(Ordering::SeqCst) {
        eprintln!("error: planner was cancelled");
        return Err(ExitCode::from(3));
    }
    fs::create_dir_all(&output_dir).map_err(|err| {
        eprintln!("error: could not create {}: {err}", output_dir.display());
        ExitCode::from(3)
    })?;
    let mut temps = TempGuard::default();
    let mut outputs = [
        PendingOutput::new(csv_path, out.csv.into_bytes(), overwrite),
        PendingOutput::new(manifest_path, out.manifest.into_bytes(), overwrite),
        PendingOutput::new(plan_path, out.plan.into_bytes(), overwrite),
    ];
    commit_outputs(&mut outputs, &mut temps).map_err(|err| {
        eprintln!("error: could not commit planner output: {err}");
        ExitCode::from(3)
    })?;
    temps.disarm();
    println!("planner: {}", output_dir.display());
    Ok(())
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
    if cancelled.load(Ordering::Relaxed) {
        return cancelled_exit(&mut temps);
    }
    write_atomic(&output, &rendered, overwrite, &mut temps).map_err(|err| {
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

const VIEWER_HTML: &str = include_str!("../../../viewer/index.html");
const VIEWER_JS: &str = include_str!("../../../viewer/viewer.js");
const VIEWER_CSS: &str = include_str!("../../../viewer/viewer.css");

#[allow(clippy::too_many_arguments)]
fn run_view(
    report_path: PathBuf,
    output: PathBuf,
    input: Option<PathBuf>,
    manifest: Option<PathBuf>,
    max_points: u64,
    overwrite: bool,
    clean: bool,
    cancelled: &AtomicBool,
) -> Result<(), ExitCode> {
    let mut temps = TempGuard::default();
    let index = output.join("index.html");
    let js_path = output.join("viewer.js");
    let css_path = output.join("viewer.css");
    refuse_if_exists(&index, overwrite)?;
    refuse_if_exists(&js_path, overwrite)?;
    refuse_if_exists(&css_path, overwrite)?;
    guard_path(&report_path, false)?;
    guard_output_path(&index)?;
    guard_output_path(&js_path)?;
    guard_output_path(&css_path)?;
    if output.exists() && output.is_file() {
        eprintln!("error: view output must be a directory");
        return Err(ExitCode::from(USAGE_EXIT));
    }
    if clean {
        remove_sibling_tmp(&index);
    }
    let max_points = max_points.clamp(8, VIEW_SAFE_MAX_POINTS);
    let report_bytes = read_bounded(&report_path, SAFE_MAX_INPUT_BYTES)?;
    let value: serde_json::Value = serde_json::from_slice(&report_bytes).map_err(|err| {
        eprintln!("error: report could not be parsed: {err}");
        ExitCode::from(USAGE_EXIT)
    })?;
    let schema = value
        .get("schema")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let report_json = embed_json(&String::from_utf8_lossy(&report_bytes));
    if !report_schema_supported(schema) {
        let view_json = embed_json(
            &serde_json::to_string(&empty_view("unsupported-schema")).map_err(|err| {
                eprintln!("error: {err}");
                ExitCode::from(3)
            })?,
        );
        write_bundle(
            &output,
            &index,
            &js_path,
            &css_path,
            &report_json,
            &view_json,
            overwrite,
            &mut temps,
        )?;
        println!("viewer: {}", index.display());
        println!("protocol: refused-unsupported-schema");
        temps.disarm();
        return Err(ExitCode::from(2));
    }
    let report: Report = serde_json::from_value(value).map_err(|err| {
        eprintln!("error: report could not be parsed: {err}");
        ExitCode::from(USAGE_EXIT)
    })?;
    let mut view = empty_view(&report.analysis_id);
    match (input, manifest) {
        (None, None) => {}
        (Some(input), Some(manifest)) => {
            guard_path(&input, false)?;
            guard_path(&manifest, false)?;
            let csv_bytes = read_bounded(&input, SAFE_MAX_INPUT_BYTES)?;
            let manifest_bytes = read_bounded(&manifest, SAFE_MAX_INPUT_BYTES)?;
            if sha256_hex(&csv_bytes) != report.input.csv_sha256
                || sha256_hex(&manifest_bytes) != report.input.manifest_sha256
            {
                eprintln!("error: input digest does not match the report analysis identity");
                return Err(ExitCode::from(2));
            }
            let limits = Limits::from_report(&report.limits);
            if analysis_id(&csv_bytes, &manifest_bytes, &report.tool.version, limits)
                != report.analysis_id
            {
                eprintln!("error: report analysis identity does not match these inputs and limits");
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
            let proposed = report.repairs.iter().find_map(|item| {
                if item.disposition == RepairDisposition::Proposed {
                    plan_by_id(&parsed.samples, &report.analysis_id, &item.id)
                        .map(|plan| plan.quaternions)
                } else {
                    None
                }
            });
            view = build_view(
                &parsed.samples,
                &report.findings,
                &report.analysis_id,
                proposed.as_deref(),
                max_points,
            );
        }
        _ => {
            eprintln!("error: --input and --manifest must be supplied together");
            return Err(ExitCode::from(USAGE_EXIT));
        }
    }
    let view_json = embed_json(&serde_json::to_string(&view).map_err(|err| {
        eprintln!("error: {err}");
        ExitCode::from(3)
    })?);
    write_bundle(
        &output,
        &index,
        &js_path,
        &css_path,
        &report_json,
        &view_json,
        overwrite,
        &mut temps,
    )?;
    println!("viewer: {}", index.display());
    println!("result: {}", report.result.as_str());
    temps.disarm();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_bundle(
    output: &Path,
    index: &Path,
    js_path: &Path,
    css_path: &Path,
    report_json: &str,
    view_json: &str,
    overwrite: bool,
    temps: &mut TempGuard,
) -> Result<(), ExitCode> {
    fs::create_dir_all(output).map_err(|err| {
        eprintln!("error: cannot create {}: {err}", output.display());
        ExitCode::from(3)
    })?;
    let html = VIEWER_HTML
        .replace("%%QUATOPSY_REPORT%%", report_json)
        .replace("%%QUATOPSY_VIEW%%", view_json);
    let mut outputs = [
        PendingOutput::new(index.to_path_buf(), html.into_bytes(), overwrite),
        PendingOutput::new(
            js_path.to_path_buf(),
            VIEWER_JS.as_bytes().to_vec(),
            overwrite,
        ),
        PendingOutput::new(
            css_path.to_path_buf(),
            VIEWER_CSS.as_bytes().to_vec(),
            overwrite,
        ),
    ];
    commit_outputs(&mut outputs, temps).map_err(|err| {
        eprintln!("error: could not commit viewer bundle: {err}");
        ExitCode::from(3)
    })?;
    Ok(())
}

fn embed_json(raw: &str) -> String {
    raw.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

#[allow(clippy::too_many_arguments)]
fn prepare_repairs(
    dir: &Path,
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    report: &Report,
    limits: Limits,
    cancelled: &AtomicBool,
    overwrite: bool,
    outputs: &mut Vec<PendingOutput>,
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
        guard_output_path(&path)?;
        outputs.push(PendingOutput::new(path, rendered, overwrite));
    }
    Ok(())
}

fn prepare_repro(
    dir: &Path,
    csv_bytes: &[u8],
    manifest_bytes: &[u8],
    report: &Report,
    input_path: Option<String>,
    overwrite: bool,
    outputs: &mut Vec<PendingOutput>,
) -> Result<(), ExitCode> {
    if report.findings.len() > MAX_REPRO_SLICES {
        eprintln!(
            "error: repro export has {} findings, limit is {MAX_REPRO_SLICES}",
            report.findings.len()
        );
        return Err(ExitCode::from(3));
    }
    fs::create_dir_all(dir).map_err(|err| {
        eprintln!("error: cannot create {}: {err}", dir.display());
        ExitCode::from(3)
    })?;
    for (index, finding) in report.findings.iter().enumerate() {
        let target = if report.findings.len() == 1 {
            dir.to_path_buf()
        } else {
            dir.join(format!("finding-{:04}", index + 1))
        };
        let (start, end) = repro::finding_repro_bounds(finding);
        let slice = slice_csv(csv_bytes, start, end).map_err(|err| {
            eprintln!("error: {err}");
            ExitCode::from(3)
        })?;
        let slice_path = target.join("slice.csv");
        let manifest_path = target.join("manifest.json");
        let provenance_path = target.join("provenance.json");
        for path in [&slice_path, &manifest_path, &provenance_path] {
            refuse_if_exists(path, overwrite)?;
            guard_output_path(path)?;
        }
        let meta = provenance(
            report.analysis_id.clone(),
            report.input.csv_sha256.clone(),
            report.input.manifest_sha256.clone(),
            start,
            end,
            std::slice::from_ref(finding),
            input_path.clone(),
        );
        let encoded = serde_json::to_vec_pretty(&meta).map_err(|err| {
            eprintln!("error: {err}");
            ExitCode::from(3)
        })?;
        outputs.push(PendingOutput::new(slice_path, slice, overwrite));
        outputs.push(PendingOutput::new(
            manifest_path,
            manifest_bytes.to_vec(),
            overwrite,
        ));
        outputs.push(PendingOutput::new(provenance_path, encoded, overwrite));
    }
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
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        eprintln!(
            "error: refusing parent-directory output path {}",
            path.display()
        );
        return Err(ExitCode::from(3));
    }
    if let Some(parent) = path.parent().filter(|item| !item.as_os_str().is_empty()) {
        match fs::symlink_metadata(parent) {
            Ok(meta) if meta.file_type().is_symlink() => {
                eprintln!("error: refusing symlink output parent {}", parent.display());
                return Err(ExitCode::from(3));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                eprintln!(
                    "error: cannot stat output parent {}: {err}",
                    parent.display()
                );
                return Err(ExitCode::from(USAGE_EXIT));
            }
        }
    }
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

struct PendingOutput {
    path: PathBuf,
    bytes: Vec<u8>,
    overwrite: bool,
}

impl PendingOutput {
    fn new(path: PathBuf, bytes: Vec<u8>, overwrite: bool) -> Self {
        Self {
            path,
            bytes,
            overwrite,
        }
    }
}

fn commit_outputs(outputs: &mut [PendingOutput], temps: &mut TempGuard) -> io::Result<()> {
    static NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let transaction = NONCE.fetch_add(1, Ordering::Relaxed);
    let mut staged = Vec::with_capacity(outputs.len());
    for (index, output) in outputs.iter().enumerate() {
        refuse_for_commit(output)?;
        let parent = output
            .path
            .parent()
            .filter(|item| !item.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let name = output
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let temporary = parent.join(format!(
            ".{name}.quatopsy-{}-{transaction}-{index}.tmp",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&output.bytes)?;
        file.sync_all()?;
        drop(file);
        temps.paths.push(temporary.clone());
        staged.push(temporary);
    }

    let mut committed: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for (index, output) in outputs.iter().enumerate() {
        let parent = output
            .path
            .parent()
            .filter(|item| !item.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let backup = if output.overwrite && output.path.exists() {
            let name = output
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let backup = parent.join(format!(
                ".{name}.quatopsy-{}-{transaction}-{index}.backup",
                std::process::id()
            ));
            if let Err(err) = fs::rename(&output.path, &backup) {
                rollback_outputs(&committed);
                return Err(err);
            }
            temps.paths.push(backup.clone());
            Some(backup)
        } else {
            None
        };
        let result = if output.overwrite {
            fs::rename(&staged[index], &output.path)
        } else {
            fs::hard_link(&staged[index], &output.path)
                .and_then(|()| fs::remove_file(&staged[index]))
        };
        if let Err(err) = result {
            if let Some(path) = &backup {
                let _ = fs::rename(path, &output.path);
            }
            rollback_outputs(&committed);
            return Err(err);
        }
        temps.paths.retain(|item| item != &staged[index]);
        committed.push((output.path.clone(), backup));
    }
    for (path, _) in &committed {
        if let Some(parent) = path.parent()
            && let Err(err) = File::open(parent).and_then(|directory| directory.sync_all())
        {
            rollback_outputs(&committed);
            return Err(err);
        }
    }
    for (_, backup) in &committed {
        if let Some(backup) = backup {
            let _ = fs::remove_file(backup);
            temps.paths.retain(|item| item != backup);
        }
    }
    Ok(())
}

fn refuse_for_commit(output: &PendingOutput) -> io::Result<()> {
    match fs::symlink_metadata(&output.path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.file_type().is_file() => Err(
            io::Error::new(io::ErrorKind::InvalidInput, "unsafe output target"),
        ),
        Ok(_) if !output.overwrite => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "output target already exists",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn rollback_outputs(committed: &[(PathBuf, Option<PathBuf>)]) {
    for (path, backup) in committed.iter().rev() {
        let _ = fs::remove_file(path);
        if let Some(backup) = backup {
            let _ = fs::rename(backup, path);
        }
    }
}

fn write_atomic(
    path: &Path,
    bytes: &[u8],
    overwrite: bool,
    temps: &mut TempGuard,
) -> io::Result<()> {
    let mut output = [PendingOutput::new(
        path.to_path_buf(),
        bytes.to_vec(),
        overwrite,
    )];
    commit_outputs(&mut output, temps)
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

#[cfg(test)]
mod transaction_tests {
    use super::*;

    fn temporary_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "quatopsy-output-transaction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn failed_batch_commit_restores_overwritten_target() {
        let dir = temporary_dir();
        let target = dir.join("result.json");
        fs::write(&target, b"original").unwrap();
        let mut outputs = [
            PendingOutput::new(target.clone(), b"replacement".to_vec(), true),
            PendingOutput::new(target.clone(), b"collision".to_vec(), false),
        ];
        let mut temps = TempGuard::default();
        assert!(commit_outputs(&mut outputs, &mut temps).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_clobber_commit_preserves_concurrently_created_target() {
        let dir = temporary_dir();
        let target = dir.join("result.json");
        let mut outputs = [PendingOutput::new(target.clone(), b"new".to_vec(), false)];
        fs::write(&target, b"racer").unwrap();
        let mut temps = TempGuard::default();
        assert!(commit_outputs(&mut outputs, &mut temps).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"racer");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repro_limit_refuses_before_creating_output_directory() {
        let csv = b"t,qw,qx,qy,qz\n0,1,0,0,0\n1,-1,0,0,0\n";
        let manifest = br#"{"schema":"quatopsy.manifest/1","component_order":"wxyz","rotation_sense":"active","frame_from":"BODY","frame_to":"J2000","time_unit":"s","columns":{"time":"t","quaternion":["qw","qx","qy","qz"]}}"#;
        let mut report = analyze(AnalyzeRequest {
            csv_bytes: csv,
            manifest_bytes: manifest,
            engine_version: "0.1.0",
            limits: Limits::defaults(),
            cancelled: None,
        });
        let finding = report.findings[0].clone();
        report.findings = vec![finding; MAX_REPRO_SLICES + 1];
        let root = temporary_dir();
        let output = root.join("repro");
        let mut pending = Vec::new();
        assert!(
            prepare_repro(&output, csv, manifest, &report, None, false, &mut pending,).is_err()
        );
        assert!(!output.exists());
        assert!(pending.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
