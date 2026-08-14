use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, error::ErrorKind};
use quatopsy_core::limits::{
    Limits, SAFE_MAX_COLUMNS, SAFE_MAX_FIELD_BYTES, SAFE_MAX_FINDINGS_PER_RULE,
    SAFE_MAX_INPUT_BYTES, SAFE_MAX_SAMPLES, SAFE_TIMEOUT_MS,
};
use quatopsy_core::{AnalyzeRequest, analyze, report_bytes};

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
    },
}

fn main() -> ExitCode {
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
            ) {
                return code;
            }
            ExitCode::SUCCESS
        }
    }
}

fn run_analyze(
    input: PathBuf,
    manifest: PathBuf,
    report_path: PathBuf,
    limits: Limits,
    overwrite: bool,
) -> Result<(), ExitCode> {
    if report_path.exists() && !overwrite {
        eprintln!(
            "error: report {} exists; pass --overwrite to replace it",
            report_path.display()
        );
        return Err(ExitCode::from(USAGE_EXIT));
    }
    let csv_bytes = read_bounded(&input, limits.max_input_bytes)?;
    let manifest_bytes = read_bounded(&manifest, limits.max_input_bytes)?;
    let report = analyze(AnalyzeRequest {
        csv_bytes: &csv_bytes,
        manifest_bytes: &manifest_bytes,
        engine_version: env!("CARGO_PKG_VERSION"),
        limits,
    });
    let encoded = report_bytes(&report).map_err(|err| {
        eprintln!("error: canonical JSON serialization failed: {err}");
        ExitCode::from(3)
    })?;
    write_atomic(&report_path, &encoded).map_err(|err| {
        eprintln!(
            "error: could not write report {}: {err}",
            report_path.display()
        );
        ExitCode::from(3)
    })?;
    print_summary(&report);
    let code = report.result.exit_code();
    if code == 0 {
        Ok(())
    } else {
        Err(ExitCode::from(code as u8))
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

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "report.json".into());
    tmp_name.push(".tmp");
    let tmp_path = parent.join(tmp_name);
    if tmp_path.exists() {
        fs::remove_file(&tmp_path)?;
    }
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
    sync_parent(parent)?;
    Ok(())
}

fn sync_parent(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

fn print_summary(report: &quatopsy_schema::Report) {
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
    if !report.diagnostics.message.is_empty() {
        println!("message: {}", report.diagnostics.message);
    }
}
