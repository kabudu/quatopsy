use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use quatopsy_core::identity::{analysis_id, sha256_hex};
use quatopsy_core::limits::{Limits, SAFE_MAX_COLUMNS, SAFE_MAX_FIELD_BYTES, SAFE_TIMEOUT_MS};
use quatopsy_schema::{AdoptionMode, EVIDENCE_SCHEMA, Report, VIEW_MAX_POINTS};
use serde::{Deserialize, Serialize};

use super::{
    USAGE_EXIT, guard_path, read_bounded, run_adapt, run_analyze, run_control, run_plan, run_view,
};

const MAX_CASE_INPUT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CASE_PROBLEM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CONTEXT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CASE_SAMPLES: u64 = 1_000_000;
const MAX_CASE_FINDINGS_PER_RULE: u64 = 1_024;
const MAX_CASE_ID_BYTES: usize = 64;
const MAX_EVIDENCE_ARTIFACTS: usize = 10_000;
const MAX_EVIDENCE_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EVIDENCE_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_EVIDENCE_DEPTH: usize = 16;
const EVIDENCE_BOUNDARIES: [&str; 4] = [
    "advisory diagnostics are not flight approval",
    "context files are preserved but not interpreted",
    "plan and control outputs are separately named candidates",
    "no physical command or actuator interface is opened",
];

pub(crate) struct Request {
    pub(crate) case_id: String,
    pub(crate) input: PathBuf,
    pub(crate) manifest: Option<PathBuf>,
    pub(crate) format: Option<String>,
    pub(crate) output_dir: PathBuf,
    pub(crate) event_log: Option<PathBuf>,
    pub(crate) command_log: Option<PathBuf>,
    pub(crate) notes: Option<PathBuf>,
    pub(crate) plan_problem: Option<PathBuf>,
    pub(crate) control_problem: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema: String,
    case_id: String,
    bundle_id: String,
    tool: Tool,
    observed: AnalysisEvidence,
    candidates: Vec<CandidateEvidence>,
    context: ContextEvidence,
    artifacts: Vec<Artifact>,
    boundaries: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    name: String,
    version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisEvidence {
    analysis_id: String,
    result: String,
    csv_sha256: String,
    manifest_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateEvidence {
    kind: String,
    analysis_id: String,
    result: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextEvidence {
    interpreted: bool,
    event_log: bool,
    command_log: bool,
    notes: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    role: String,
    bytes: u64,
    sha256: String,
}

struct OutputGuard {
    path: PathBuf,
    armed: bool,
}

impl Drop for OutputGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub(crate) fn run(request: Request, cancelled: &AtomicBool) -> Result<(), ExitCode> {
    validate_case_id(&request.case_id)?;
    guard_source_files(&request)?;
    create_owned_output(&request.output_dir)?;
    let mut output_guard = OutputGuard {
        path: request.output_dir.clone(),
        armed: true,
    };

    let source_dir = request.output_dir.join("source");
    let context_dir = request.output_dir.join("context");
    fs::create_dir(&source_dir).map_err(write_error)?;
    fs::create_dir(&context_dir).map_err(write_error)?;

    let (source_input, source_manifest) = snapshot_observed(&request, &source_dir)?;
    copy_context(
        request.event_log.as_deref(),
        &context_dir.join("events.log"),
    )?;
    copy_context(
        request.command_log.as_deref(),
        &context_dir.join("commands.log"),
    )?;
    copy_context(request.notes.as_deref(), &context_dir.join("notes.txt"))?;
    cancel_check(cancelled)?;

    let observed_dir = request.output_dir.join("observed");
    let observed = analyze_and_view(
        &observed_dir,
        &source_input,
        &source_manifest,
        true,
        cancelled,
    )?;

    let mut candidates = Vec::new();
    if let Some(problem) = request.plan_problem.as_deref() {
        candidates.push(run_candidate(
            "plan",
            problem,
            &request.output_dir,
            cancelled,
        )?);
    }
    if let Some(problem) = request.control_problem.as_deref() {
        candidates.push(run_candidate(
            "control",
            problem,
            &request.output_dir,
            cancelled,
        )?);
    }
    cancel_check(cancelled)?;

    let artifacts = collect_artifacts(&request.output_dir)?;
    let tool_version = env!("CARGO_PKG_VERSION").to_owned();
    let bundle_id = bundle_id(&request.case_id, &tool_version, &artifacts);
    let evidence = Evidence {
        schema: EVIDENCE_SCHEMA.to_owned(),
        case_id: request.case_id,
        bundle_id,
        tool: Tool {
            name: "quatopsy".to_owned(),
            version: tool_version,
        },
        observed,
        candidates,
        context: ContextEvidence {
            interpreted: false,
            event_log: request.event_log.is_some(),
            command_log: request.command_log.is_some(),
            notes: request.notes.is_some(),
        },
        artifacts,
        boundaries: evidence_boundaries(),
    };
    let mut encoded = serde_json::to_vec_pretty(&evidence).map_err(|err| {
        eprintln!("error: could not encode evidence manifest: {err}");
        ExitCode::from(3)
    })?;
    encoded.push(b'\n');
    fs::write(request.output_dir.join("evidence.json"), encoded).map_err(write_error)?;
    output_guard.armed = false;
    println!("investigation: {}", request.output_dir.display());
    println!("bundle_id: {}", evidence.bundle_id);
    println!("observed_result: {}", evidence.observed.result);
    Ok(())
}

pub(crate) fn verify(bundle: PathBuf) -> Result<(), ExitCode> {
    let evidence_path = bundle.join("evidence.json");
    guard_path(&evidence_path, false)?;
    let encoded = read_bounded(&evidence_path, MAX_CASE_INPUT_BYTES)?;
    let evidence: Evidence = serde_json::from_slice(&encoded).map_err(|err| {
        eprintln!("error: evidence manifest could not be parsed: {err}");
        ExitCode::from(2)
    })?;
    validate_case_id(&evidence.case_id)?;
    if evidence.schema != EVIDENCE_SCHEMA || evidence.tool.name != "quatopsy" {
        eprintln!("error: unsupported evidence protocol or tool identity");
        return Err(ExitCode::from(2));
    }
    if evidence.boundaries != evidence_boundaries() || evidence.context.interpreted {
        eprintln!("error: evidence safety boundary mismatch");
        return Err(ExitCode::from(2));
    }
    let actual = collect_artifacts(&bundle)?;
    if actual != evidence.artifacts
        || bundle_id(&evidence.case_id, &evidence.tool.version, &actual) != evidence.bundle_id
    {
        eprintln!(
            "error: evidence artifact digest, role, size, order, or bundle identity mismatch"
        );
        return Err(ExitCode::from(2));
    }
    let canonical_source =
        bundle.join("source/input.csv").exists() && bundle.join("source/manifest.json").exists();
    let adapted_source = bundle.join("source/external-input.bin").exists()
        && bundle.join("source/adapted/input.csv").exists()
        && bundle.join("source/adapted/manifest.json").exists()
        && bundle.join("source/adapted/provenance.json").exists();
    if canonical_source == adapted_source {
        eprintln!("error: evidence must contain exactly one complete observed source form");
        return Err(ExitCode::from(2));
    }
    let (observed_input, observed_manifest) = if canonical_source {
        (
            bundle.join("source/input.csv"),
            bundle.join("source/manifest.json"),
        )
    } else {
        (
            bundle.join("source/adapted/input.csv"),
            bundle.join("source/adapted/manifest.json"),
        )
    };
    let observed = verify_analysis_binding(
        &bundle.join("observed/report.json"),
        &observed_input,
        &observed_manifest,
    )?;
    if observed.analysis_id != evidence.observed.analysis_id
        || observed.result != evidence.observed.result
        || observed.csv_sha256 != evidence.observed.csv_sha256
        || observed.manifest_sha256 != evidence.observed.manifest_sha256
    {
        eprintln!("error: observed report does not match evidence manifest");
        return Err(ExitCode::from(2));
    }
    let expected_context = ContextEvidence {
        interpreted: false,
        event_log: bundle.join("context/events.log").exists(),
        command_log: bundle.join("context/commands.log").exists(),
        notes: bundle.join("context/notes.txt").exists(),
    };
    if evidence.context.interpreted != expected_context.interpreted
        || evidence.context.event_log != expected_context.event_log
        || evidence.context.command_log != expected_context.command_log
        || evidence.context.notes != expected_context.notes
    {
        eprintln!("error: evidence context declaration does not match preserved files");
        return Err(ExitCode::from(2));
    }
    if evidence.candidates.len() > 2 {
        eprintln!("error: evidence contains too many candidate declarations");
        return Err(ExitCode::from(2));
    }
    let mut candidate_kinds = BTreeSet::new();
    for candidate in &evidence.candidates {
        if candidate.kind != "plan" && candidate.kind != "control" {
            eprintln!("error: unsupported candidate kind {}", candidate.kind);
            return Err(ExitCode::from(2));
        }
        if !candidate_kinds.insert(candidate.kind.as_str()) {
            eprintln!("error: duplicate {} candidate declaration", candidate.kind);
            return Err(ExitCode::from(2));
        }
        let candidate_root = bundle.join("candidates").join(&candidate.kind);
        let analysis = verify_analysis_binding(
            &candidate_root.join("analysis/report.json"),
            &candidate_root.join("generated/input.csv"),
            &candidate_root.join("generated/manifest.json"),
        )?;
        if analysis.analysis_id != candidate.analysis_id || analysis.result != candidate.result {
            eprintln!(
                "error: {} candidate report does not match evidence manifest",
                candidate.kind
            );
            return Err(ExitCode::from(2));
        }
    }
    for kind in ["plan", "control"] {
        if bundle.join("candidates").join(kind).exists() != candidate_kinds.contains(kind) {
            eprintln!("error: {kind} candidate directory and declaration do not match");
            return Err(ExitCode::from(2));
        }
    }
    println!("evidence: verified");
    println!("bundle_id: {}", evidence.bundle_id);
    println!("observed_result: {}", evidence.observed.result);
    Ok(())
}

fn validate_case_id(case_id: &str) -> Result<(), ExitCode> {
    if case_id.is_empty()
        || case_id.len() > MAX_CASE_ID_BYTES
        || !case_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        eprintln!(
            "error: --case-id must be 1 to {MAX_CASE_ID_BYTES} ASCII letters, digits, dot, underscore, or hyphen"
        );
        return Err(ExitCode::from(USAGE_EXIT));
    }
    Ok(())
}

fn guard_source_files(request: &Request) -> Result<(), ExitCode> {
    guard_path(&request.input, false)?;
    match (&request.manifest, &request.format) {
        (Some(manifest), None) => guard_path(manifest, false)?,
        (None, Some(_)) => {}
        _ => {
            eprintln!("error: supply exactly one of --manifest or --format");
            return Err(ExitCode::from(USAGE_EXIT));
        }
    }
    for path in [
        request.event_log.as_deref(),
        request.command_log.as_deref(),
        request.notes.as_deref(),
        request.plan_problem.as_deref(),
        request.control_problem.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        guard_path(path, false)?;
    }
    Ok(())
}

fn snapshot_observed(request: &Request, source_dir: &Path) -> Result<(PathBuf, PathBuf), ExitCode> {
    match (&request.manifest, &request.format) {
        (Some(manifest), None) => {
            let input = source_dir.join("input.csv");
            let manifest_copy = source_dir.join("manifest.json");
            fs::write(&input, read_bounded(&request.input, MAX_CASE_INPUT_BYTES)?)
                .map_err(write_error)?;
            fs::write(
                &manifest_copy,
                read_bounded(manifest, MAX_CASE_INPUT_BYTES)?,
            )
            .map_err(write_error)?;
            Ok((input, manifest_copy))
        }
        (None, Some(format)) => {
            let original = source_dir.join("external-input.bin");
            fs::write(
                &original,
                read_bounded(&request.input, MAX_CASE_INPUT_BYTES)?,
            )
            .map_err(write_error)?;
            let adapted = source_dir.join("adapted");
            run_adapt(format.clone(), original, adapted.clone(), false)?;
            Ok((adapted.join("input.csv"), adapted.join("manifest.json")))
        }
        _ => unreachable!("source mode is validated before output creation"),
    }
}

fn create_owned_output(output: &Path) -> Result<(), ExitCode> {
    if output
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        eprintln!(
            "error: refusing parent-directory investigation output {}",
            output.display()
        );
        return Err(ExitCode::from(3));
    }
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = fs::symlink_metadata(parent).map_err(|err| {
        eprintln!(
            "error: investigation output parent {} must already exist: {err}",
            parent.display()
        );
        ExitCode::from(USAGE_EXIT)
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        eprintln!(
            "error: refusing non-directory or symlink output parent {}",
            parent.display()
        );
        return Err(ExitCode::from(3));
    }
    fs::create_dir(output).map_err(|err| {
        eprintln!(
            "error: investigation output {} must not already exist: {err}",
            output.display()
        );
        ExitCode::from(USAGE_EXIT)
    })
}

fn copy_context(source: Option<&Path>, target: &Path) -> Result<(), ExitCode> {
    if let Some(source) = source {
        fs::write(target, read_bounded(source, MAX_CONTEXT_BYTES)?).map_err(write_error)?;
    }
    Ok(())
}

fn analyze_and_view(
    root: &Path,
    input: &Path,
    manifest: &Path,
    include_repro: bool,
    cancelled: &AtomicBool,
) -> Result<AnalysisEvidence, ExitCode> {
    fs::create_dir_all(root).map_err(write_error)?;
    let report_path = root.join("report.json");
    run_analyze(
        input.to_path_buf(),
        manifest.to_path_buf(),
        report_path.clone(),
        case_limits(),
        false,
        false,
        include_repro.then(|| root.join("repairs")),
        include_repro.then(|| root.join("repro")),
        false,
        AdoptionMode::Advisory,
        Vec::new(),
        None,
        cancelled,
    )?;
    run_view(
        report_path.clone(),
        root.join("viewer"),
        Some(input.to_path_buf()),
        Some(manifest.to_path_buf()),
        VIEW_MAX_POINTS,
        false,
        false,
        cancelled,
    )?;
    verify_analysis_binding(&report_path, input, manifest)
}

fn run_candidate(
    kind: &'static str,
    problem: &Path,
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<CandidateEvidence, ExitCode> {
    let root = output.join("candidates").join(kind);
    fs::create_dir_all(&root).map_err(write_error)?;
    let snapshot = root.join("problem.json");
    fs::write(&snapshot, read_bounded(problem, MAX_CASE_PROBLEM_BYTES)?).map_err(write_error)?;
    let generated = root.join("generated");
    match kind {
        "plan" => run_plan(snapshot, generated.clone(), false, cancelled)?,
        "control" => run_control(snapshot, generated.clone(), false, cancelled)?,
        _ => unreachable!("candidate kind is fixed by the CLI"),
    }
    let analysis = analyze_and_view(
        &root.join("analysis"),
        &generated.join("input.csv"),
        &generated.join("manifest.json"),
        false,
        cancelled,
    )?;
    Ok(CandidateEvidence {
        kind: kind.to_owned(),
        analysis_id: analysis.analysis_id,
        result: analysis.result,
    })
}

fn verify_analysis_binding(
    report_path: &Path,
    input_path: &Path,
    manifest_path: &Path,
) -> Result<AnalysisEvidence, ExitCode> {
    let bytes = read_bounded(report_path, MAX_CASE_INPUT_BYTES)?;
    let report: Report = serde_json::from_slice(&bytes).map_err(|err| {
        eprintln!("error: generated report could not be parsed: {err}");
        ExitCode::from(3)
    })?;
    let input = read_bounded(input_path, MAX_CASE_INPUT_BYTES)?;
    let manifest = read_bounded(manifest_path, MAX_CASE_INPUT_BYTES)?;
    let limits = Limits::from_report(&report.limits);
    if sha256_hex(&input) != report.input.csv_sha256
        || sha256_hex(&manifest) != report.input.manifest_sha256
        || analysis_id(&input, &manifest, &report.tool.version, limits) != report.analysis_id
    {
        eprintln!("error: report is not bound to its bundled input, manifest, version, and limits");
        return Err(ExitCode::from(2));
    }
    Ok(AnalysisEvidence {
        analysis_id: report.analysis_id,
        result: report.result.as_str().to_owned(),
        csv_sha256: report.input.csv_sha256,
        manifest_sha256: report.input.manifest_sha256,
    })
}

fn collect_artifacts(root: &Path) -> Result<Vec<Artifact>, ExitCode> {
    let mut paths = Vec::new();
    visit_files(root, root, &mut paths, 0)?;
    paths.sort();
    let mut artifacts = Vec::with_capacity(paths.len());
    let mut total_bytes = 0_u64;
    for relative in paths {
        let path = root.join(&relative);
        let size = fs::metadata(&path).map_err(write_error)?.len();
        total_bytes = total_bytes.checked_add(size).ok_or_else(|| {
            eprintln!("error: evidence artifact byte total overflow");
            ExitCode::from(3)
        })?;
        if size > MAX_EVIDENCE_ARTIFACT_BYTES || total_bytes > MAX_EVIDENCE_TOTAL_BYTES {
            eprintln!(
                "error: evidence artifact size limit exceeded at {}",
                path.display()
            );
            return Err(ExitCode::from(3));
        }
        let bytes = fs::read(&path).map_err(write_error)?;
        artifacts.push(Artifact {
            role: artifact_role(&relative).to_owned(),
            path: relative.to_string_lossy().replace('\\', "/"),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        });
    }
    Ok(artifacts)
}

fn visit_files(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), ExitCode> {
    if depth > MAX_EVIDENCE_DEPTH {
        eprintln!("error: evidence directory depth limit exceeded");
        return Err(ExitCode::from(3));
    }
    for entry in fs::read_dir(directory).map_err(write_error)? {
        let entry = entry.map_err(write_error)?;
        let metadata = entry.file_type().map_err(write_error)?;
        if metadata.is_symlink() {
            eprintln!(
                "error: evidence tree contains symlink {}",
                entry.path().display()
            );
            return Err(ExitCode::from(3));
        }
        if metadata.is_dir() {
            visit_files(root, &entry.path(), paths, depth + 1)?;
        } else if metadata.is_file() {
            let relative = entry.path().strip_prefix(root).unwrap().to_path_buf();
            if relative != Path::new("evidence.json") {
                paths.push(relative);
                if paths.len() > MAX_EVIDENCE_ARTIFACTS {
                    eprintln!("error: evidence artifact count limit exceeded");
                    return Err(ExitCode::from(3));
                }
            }
        } else {
            eprintln!(
                "error: evidence tree contains special file {}",
                entry.path().display()
            );
            return Err(ExitCode::from(3));
        }
    }
    Ok(())
}

fn artifact_role(path: &Path) -> &'static str {
    let value = path.to_string_lossy();
    if value.starts_with("source/") {
        "observed-source"
    } else if value.starts_with("context/") {
        "uninterpreted-context"
    } else if value.contains("/viewer/") {
        "static-viewer"
    } else if value.contains("/repro/") {
        "reproducer"
    } else if value.contains("/repairs/") {
        "repair-candidate"
    } else if value.starts_with("candidates/") {
        "generated-candidate"
    } else {
        "analysis"
    }
}

fn evidence_boundaries() -> Vec<String> {
    EVIDENCE_BOUNDARIES.into_iter().map(str::to_owned).collect()
}

fn bundle_id(case_id: &str, tool_version: &str, artifacts: &[Artifact]) -> String {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(case_id.as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(tool_version.as_bytes());
    canonical.push(b'\n');
    for artifact in artifacts {
        canonical.extend_from_slice(artifact.path.as_bytes());
        canonical.push(0);
        canonical.extend_from_slice(artifact.sha256.as_bytes());
        canonical.push(b'\n');
    }
    sha256_hex(&canonical)
}

fn case_limits() -> Limits {
    Limits {
        max_input_bytes: MAX_CASE_INPUT_BYTES,
        max_samples: MAX_CASE_SAMPLES,
        max_field_bytes: SAFE_MAX_FIELD_BYTES,
        max_columns: SAFE_MAX_COLUMNS,
        max_findings_per_rule: MAX_CASE_FINDINGS_PER_RULE,
        timeout_ms: SAFE_TIMEOUT_MS.min(120_000),
    }
}

fn cancel_check(cancelled: &AtomicBool) -> Result<(), ExitCode> {
    if cancelled.load(Ordering::SeqCst) {
        eprintln!("error: investigation was cancelled");
        Err(ExitCode::from(3))
    } else {
        Ok(())
    }
}

fn write_error(err: std::io::Error) -> ExitCode {
    eprintln!("error: could not write investigation evidence: {err}");
    ExitCode::from(3)
}
