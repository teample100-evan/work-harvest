mod checkpoints;
mod queries;
mod reports;
mod schema;
mod work_items;
mod write;

pub use checkpoints::{
    CheckpointContextGitUpdate, CheckpointContextUpdate, CheckpointDecisionDocument,
    CheckpointDocument, CheckpointEvidenceDocument, CheckpointEvidenceInput, CheckpointGitDocument,
    CheckpointInput, CheckpointOutcomeDocument, CheckpointPaths, CheckpointSourceDocument,
    CheckpointSourceInput, CheckpointVerificationDocument, CheckpointWorkPeriodDocument,
    CheckpointWorkPeriodInput, CheckpointWriteError, CheckpointWritePreview, CheckpointWriteResult,
    capture_checkpoint, normalize_checkpoint, preview_capture_checkpoint, render_checkpoint,
};
pub use queries::{
    QueryError, StoredCheckpointPaths, StoredCheckpointRecord, StoredWorkItemRecord,
    WorkItemQueryResult, find_last_checkpoint, list_checkpoints_for_work_item,
    list_work_item_records, read_work_item_record, show_work_item,
};
pub use reports::{
    PerformanceNoteCheckpoint, PerformanceNoteInput, PerformanceNotePaths,
    PerformanceNoteSourceRevision, PerformanceNoteWriteError, PerformanceNoteWritePreview,
    PerformanceNoteWriteResult, create_performance_note, create_performance_note_from_current,
    performance_note_markdown_path, preview_performance_note, render_performance_note,
};
pub use work_items::{
    ContextFileInput, ContextGitInput, ContextVerificationInput, StoredContextFile,
    StoredContextGit, StoredContextVerification, StoredWorkItemClassification, WorkContextDocument,
    WorkContextInput, WorkContextPatch, WorkItemChangeOperation, WorkItemClassificationInput,
    WorkItemCreateInput, WorkItemDocument, WorkItemEditRevisions, WorkItemEditSnapshot,
    WorkItemFileChange, WorkItemPaths, WorkItemUpdatePatch, WorkItemWriteError,
    WorkItemWritePreview, WorkItemWriteResult, create_work_item, normalize_work_item,
    preview_create_work_item, preview_update_work_item, read_work_item_for_edit, render_context,
    update_work_item,
};
pub use write::{
    DataRootWriter, FileRevision, WriteCommit, WriteError, WriteExpectation, WriteOperation,
    read_file_revision,
};

use chrono::NaiveDate;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

use schema::DocumentKind;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Data root does not exist: {0}")]
    MissingRoot(String),
    #[error("Data root is not a directory: {0}")]
    InvalidRoot(String),
    #[error("Could not scan data root {path}: {source}")]
    Scan {
        path: String,
        source: walkdir::Error,
    },
    #[error("Could not read data file {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("Could not parse JSON data file {path}: {source}")]
    Parse {
        path: String,
        source: serde_json::Error,
    },
    #[error("Work item was not found: {0}")]
    WorkItemNotFound(String),
    #[error("Data asset was not found: {0}")]
    DataAssetNotFound(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataRootCounts {
    pub work_items: usize,
    pub contexts: usize,
    pub checkpoints: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkItemSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub updated_at: String,
    pub activity_dates: Vec<String>,
    pub current_state: Option<String>,
    pub last_checkpoint_id: Option<String>,
    pub last_checkpoint_confidentiality: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataRootSnapshot {
    pub root: String,
    pub counts: DataRootCounts,
    pub issues: Vec<DataIssue>,
    pub work_items: Vec<WorkItemSummary>,
    pub checkpoint_ids: Vec<String>,
    pub restricted_checkpoint_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataRootUpdate {
    pub snapshot: DataRootSnapshot,
    pub changed_work_item_ids: Vec<String>,
    pub full_rescan: bool,
    pub reloaded_files: usize,
    pub revision: u64,
    pub applied: bool,
}

#[derive(Debug, Clone)]
struct IndexedDocument {
    value: Option<Value>,
    issues: Vec<DataIssue>,
}

#[derive(Debug, Clone)]
pub struct DataRootIndex {
    root: PathBuf,
    documents: HashMap<PathBuf, IndexedDocument>,
    snapshot: DataRootSnapshot,
    revision: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkItemClassification {
    pub initiative_id: Option<String>,
    pub work_types: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContextFile {
    pub path: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerificationState {
    pub completed: Vec<String>,
    pub pending: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkContextDetail {
    pub updated_at: String,
    pub last_checkpoint_id: Option<String>,
    pub current_state: String,
    pub decisions: Vec<String>,
    pub files: Vec<ContextFile>,
    pub verification: VerificationState,
    pub next_steps: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckpointVerification {
    pub kind: String,
    pub description: String,
    pub status: String,
    pub command: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckpointDecision {
    pub summary: String,
    pub rationale: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckpointEvidence {
    pub commits: Vec<String>,
    pub pull_requests: Vec<String>,
    pub issues: Vec<String>,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckpointGit {
    pub repository: String,
    pub branch: Option<String>,
    pub head_before: Option<String>,
    pub head_after: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckpointSummary {
    pub id: String,
    pub kind: String,
    pub captured_at: String,
    pub title: String,
    pub summary: String,
    pub status_after: String,
    pub confidentiality: String,
    pub markdown_path: String,
    pub activities: Vec<String>,
    pub decisions: Vec<CheckpointDecision>,
    pub outcomes: Vec<String>,
    pub verifications: Vec<CheckpointVerification>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub evidence: CheckpointEvidence,
    pub git: Option<CheckpointGit>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkItemDetail {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub objective: String,
    pub desired_outcomes: Vec<String>,
    pub classification: WorkItemClassification,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub context: Option<WorkContextDetail>,
    pub checkpoints: Vec<CheckpointSummary>,
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn issue(root: &Path, path: &Path, code: &str, message: impl Into<String>) -> DataIssue {
    DataIssue {
        severity: IssueSeverity::Error,
        code: code.to_string(),
        path: relative_path(root, path),
        message: message.into(),
    }
}

fn validate_document(
    root: &Path,
    path: &Path,
    kind: DocumentKind,
    value: &Value,
    issues: &mut Vec<DataIssue>,
) {
    match schema::validate(kind, value) {
        Ok(violations) => {
            issues.extend(violations.into_iter().map(|violation| {
                let location = if violation.instance_path.is_empty() {
                    "$".to_string()
                } else {
                    format!("${}", violation.instance_path)
                };
                issue(
                    root,
                    path,
                    "schema_validation",
                    format!("{location}: {}", violation.message),
                )
            }));
        }
        Err(error) => issues.push(issue(root, path, "schema_setup_failed", error)),
    }
}

fn read_json(root: &Path, path: &Path, issues: &mut Vec<DataIssue>) -> Option<Value> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            issues.push(issue(
                root,
                path,
                "read_failed",
                format!("JSON 파일을 읽을 수 없습니다: {error}"),
            ));
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(issue(
                root,
                path,
                "invalid_json",
                format!("JSON 형식이 올바르지 않습니다: {error}"),
            ));
            None
        }
    }
}

fn required_string(
    root: &Path,
    path: &Path,
    value: &Value,
    field: &str,
    issues: &mut Vec<DataIssue>,
) -> Option<String> {
    match value.get(field).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => Some(value.to_string()),
        _ => {
            issues.push(issue(
                root,
                path,
                "missing_required_field",
                format!("필수 문자열 필드 `{field}`가 없습니다."),
            ));
            None
        }
    }
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>, CoreError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(directory) {
        let entry = entry.map_err(|source| CoreError::Scan {
            path: directory.to_string_lossy().into_owned(),
            source,
        })?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        {
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

fn inspect_work_item(
    root: &Path,
    path: &Path,
    value: &Value,
    contexts: &HashMap<PathBuf, Value>,
    issues: &mut Vec<DataIssue>,
) -> Option<WorkItemSummary> {
    validate_document(root, path, DocumentKind::WorkItem, value, issues);
    let id = required_string(root, path, value, "id", issues)?;
    let project_id = required_string(root, path, value, "project_id", issues)?;
    let title = required_string(root, path, value, "title", issues)?;
    let status = required_string(root, path, value, "status", issues)?;
    let updated_at = required_string(root, path, value, "updated_at", issues)?;

    let directory = path.parent().unwrap_or(root);
    let context_data_path = directory.join("context.json");
    let context_markdown_path = directory.join("context.md");
    let context = contexts.get(&context_data_path);
    if !context_data_path.exists() {
        issues.push(issue(
            root,
            &context_data_path,
            "missing_context_data",
            "구조화된 context.json이 없습니다.",
        ));
    }
    if !context_markdown_path.exists() {
        issues.push(issue(
            root,
            &context_markdown_path,
            "missing_context_markdown",
            "파생 context.md가 없습니다.",
        ));
    }
    if let Some(context) = context {
        if context.get("work_item_id").and_then(Value::as_str) != Some(id.as_str()) {
            issues.push(issue(
                root,
                &context_data_path,
                "context_work_item_mismatch",
                format!("context가 업무 항목 {id}를 참조하지 않습니다."),
            ));
        }
        if context.get("project_id").and_then(Value::as_str) != Some(project_id.as_str()) {
            issues.push(issue(
                root,
                &context_data_path,
                "context_project_mismatch",
                format!("context의 프로젝트가 업무 항목 {id}와 일치하지 않습니다."),
            ));
        }
    }

    Some(WorkItemSummary {
        id,
        project_id,
        title,
        status,
        updated_at,
        activity_dates: Vec::new(),
        current_state: context
            .and_then(|value| value.get("current_state"))
            .and_then(Value::as_str)
            .map(str::to_string),
        last_checkpoint_id: context
            .and_then(|value| value.get("last_checkpoint_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        last_checkpoint_confidentiality: None,
    })
}

fn checkpoint_activity_dates(value: &Value) -> Vec<String> {
    fn parse_date(value: Option<&Value>) -> Option<NaiveDate> {
        let value = value?.as_str()?;
        let prefix = value.get(..10)?;
        NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok()
    }

    let work_period = value.get("work_period").unwrap_or(&Value::Null);
    let start = parse_date(work_period.get("start"));
    let end = parse_date(work_period.get("end"));
    match (start, end) {
        (Some(start), Some(end)) if start <= end => {
            let span = end.signed_duration_since(start).num_days();
            if span > 366 {
                return vec![start.to_string(), end.to_string()];
            }
            let mut dates = Vec::with_capacity(span as usize + 1);
            let mut date = start;
            loop {
                dates.push(date.to_string());
                if date == end {
                    break;
                }
                let Some(next) = date.succ_opt() else {
                    break;
                };
                date = next;
            }
            dates
        }
        (Some(date), _) | (_, Some(date)) => vec![date.to_string()],
        _ => Vec::new(),
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

fn string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn context_detail(value: &Value) -> WorkContextDetail {
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|file| {
            Some(ContextFile {
                path: file.get("path")?.as_str()?.to_string(),
                description: optional_string_field(file, "description"),
            })
        })
        .collect();
    let verification = value.get("verification").unwrap_or(&Value::Null);

    WorkContextDetail {
        updated_at: string_field(value, "updated_at"),
        last_checkpoint_id: optional_string_field(value, "last_checkpoint_id"),
        current_state: string_field(value, "current_state"),
        decisions: string_array(value, "decisions"),
        files,
        verification: VerificationState {
            completed: string_array(verification, "completed"),
            pending: string_array(verification, "pending"),
        },
        next_steps: string_array(value, "next_steps"),
        risks: string_array(value, "risks"),
    }
}

fn checkpoint_summary(root: &Path, path: &Path, value: &Value) -> CheckpointSummary {
    let outcomes = value
        .get("outcomes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|outcome| outcome.get("description")?.as_str().map(str::to_string))
        .collect();
    let decisions = value
        .get("decisions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|decision| {
            Some(CheckpointDecision {
                summary: decision.get("summary")?.as_str()?.to_string(),
                rationale: decision.get("rationale")?.as_str()?.to_string(),
                status: decision.get("status")?.as_str()?.to_string(),
            })
        })
        .collect();
    let verifications = value
        .get("verifications")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|verification| {
            Some(CheckpointVerification {
                kind: verification.get("type")?.as_str()?.to_string(),
                description: verification.get("description")?.as_str()?.to_string(),
                status: verification.get("status")?.as_str()?.to_string(),
                command: optional_string_field(verification, "command"),
                evidence_refs: string_array(verification, "evidence_refs"),
            })
        })
        .collect();
    let evidence = value.get("evidence").unwrap_or(&Value::Null);
    let git = value.get("git").and_then(|git| {
        Some(CheckpointGit {
            repository: git.get("repository")?.as_str()?.to_string(),
            branch: optional_string_field(git, "branch"),
            head_before: optional_string_field(git, "head_before"),
            head_after: optional_string_field(git, "head_after"),
            dirty: git.get("dirty").and_then(Value::as_bool),
        })
    });

    CheckpointSummary {
        id: string_field(value, "id"),
        kind: string_field(value, "kind"),
        captured_at: string_field(value, "captured_at"),
        title: string_field(value, "title"),
        summary: string_field(value, "summary"),
        status_after: string_field(value, "status_after"),
        confidentiality: string_field(value, "confidentiality"),
        markdown_path: relative_path(root, &path.with_extension("md")),
        activities: string_array(value, "activities"),
        decisions,
        outcomes,
        verifications,
        blockers: string_array(value, "blockers"),
        next_steps: string_array(value, "next_steps"),
        evidence: CheckpointEvidence {
            commits: string_array(evidence, "commits"),
            pull_requests: string_array(evidence, "pull_requests"),
            issues: string_array(evidence, "issues"),
            files: string_array(evidence, "files"),
            commands: string_array(evidence, "commands"),
            urls: string_array(evidence, "urls"),
        },
        git,
    }
}

fn read_data_json(path: &Path) -> Result<Value, CoreError> {
    let text = fs::read_to_string(path).map_err(|source| CoreError::Read {
        path: path.to_string_lossy().into_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| CoreError::Parse {
        path: path.to_string_lossy().into_owned(),
        source,
    })
}

fn is_identifier(value: &str) -> bool {
    (2..=64).contains(&value.len())
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn validate_data_root(root: &Path) -> Result<(), CoreError> {
    if !root.exists() {
        return Err(CoreError::MissingRoot(root.to_string_lossy().into_owned()));
    }
    if !root.is_dir() {
        return Err(CoreError::InvalidRoot(root.to_string_lossy().into_owned()));
    }
    Ok(())
}

fn canonical_asset(root: &Path, path: &Path) -> Result<PathBuf, CoreError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| CoreError::MissingRoot(root.to_string_lossy().into_owned()))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_| CoreError::DataAssetNotFound(path.to_string_lossy().into_owned()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(CoreError::DataAssetNotFound(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(canonical_path)
}

fn work_item_data_path(root: &Path, work_item_id: &str) -> Result<PathBuf, CoreError> {
    validate_data_root(root)?;
    if !is_identifier(work_item_id) {
        return Err(CoreError::WorkItemNotFound(work_item_id.to_string()));
    }
    let path = root
        .join("work-items")
        .join(work_item_id)
        .join("work-item.json");
    let path = canonical_asset(root, &path)
        .map_err(|_| CoreError::WorkItemNotFound(work_item_id.to_string()))?;
    let value = read_data_json(&path)?;
    if value.get("id").and_then(Value::as_str) != Some(work_item_id) {
        return Err(CoreError::WorkItemNotFound(work_item_id.to_string()));
    }
    Ok(path)
}

pub fn work_item_directory(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<PathBuf, CoreError> {
    let path = work_item_data_path(root.as_ref(), work_item_id)?;
    path.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| CoreError::WorkItemNotFound(work_item_id.to_string()))
}

pub fn context_markdown_path(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<PathBuf, CoreError> {
    let root = root.as_ref();
    let path = work_item_data_path(root, work_item_id)?.with_file_name("context.md");
    canonical_asset(root, &path)
}

pub fn checkpoint_markdown_path(
    root: impl AsRef<Path>,
    checkpoint_id: &str,
) -> Result<PathBuf, CoreError> {
    let root = root.as_ref();
    validate_data_root(root)?;
    if !is_identifier(checkpoint_id) {
        return Err(CoreError::DataAssetNotFound(checkpoint_id.to_string()));
    }

    for path in json_files(&root.join("records"))? {
        let Ok(value) = read_data_json(&path) else {
            continue;
        };
        if value.get("id").and_then(Value::as_str) == Some(checkpoint_id) {
            return canonical_asset(root, &path.with_extension("md"));
        }
    }
    Err(CoreError::DataAssetNotFound(checkpoint_id.to_string()))
}

pub fn get_work_item_detail(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<WorkItemDetail, CoreError> {
    let root = root.as_ref();
    let work_item_path = work_item_data_path(root, work_item_id)?;
    let work_item = read_data_json(&work_item_path)?;

    let context_path = work_item_path.with_file_name("context.json");
    let context = if context_path.is_file() {
        Some(context_detail(&read_data_json(&context_path)?))
    } else {
        None
    };

    let mut checkpoints = Vec::new();
    for path in json_files(&root.join("records"))? {
        let Ok(value) = read_data_json(&path) else {
            continue;
        };
        if value.get("work_item_id").and_then(Value::as_str) == Some(work_item_id) {
            checkpoints.push(checkpoint_summary(root, &path, &value));
        }
    }
    checkpoints.sort_by(|left, right| {
        right
            .captured_at
            .cmp(&left.captured_at)
            .then_with(|| right.id.cmp(&left.id))
    });

    let classification = work_item.get("classification").unwrap_or(&Value::Null);
    Ok(WorkItemDetail {
        id: string_field(&work_item, "id"),
        project_id: string_field(&work_item, "project_id"),
        title: string_field(&work_item, "title"),
        status: string_field(&work_item, "status"),
        objective: string_field(&work_item, "objective"),
        desired_outcomes: string_array(&work_item, "desired_outcomes"),
        classification: WorkItemClassification {
            initiative_id: optional_string_field(classification, "initiative_id"),
            work_types: string_array(classification, "work_types"),
            tags: string_array(classification, "tags"),
        },
        created_at: string_field(&work_item, "created_at"),
        updated_at: string_field(&work_item, "updated_at"),
        completed_at: optional_string_field(&work_item, "completed_at"),
        context,
        checkpoints,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexedDocumentKind {
    WorkItem,
    Context,
    Checkpoint,
}

fn indexed_document_kind(root: &Path, path: &Path) -> Option<IndexedDocumentKind> {
    let relative = path.strip_prefix(root).ok()?;
    let file_name = path.file_name()?.to_str()?;
    if relative.starts_with("work-items") {
        return match file_name {
            "work-item.json" => Some(IndexedDocumentKind::WorkItem),
            "context.json" => Some(IndexedDocumentKind::Context),
            _ => None,
        };
    }
    if relative.starts_with("records")
        && path.extension().and_then(|extension| extension.to_str()) == Some("json")
    {
        return Some(IndexedDocumentKind::Checkpoint);
    }
    None
}

fn indexed_document_paths(root: &Path) -> Result<Vec<PathBuf>, CoreError> {
    let mut paths = json_files(&root.join("work-items"))?
        .into_iter()
        .filter(|path| indexed_document_kind(root, path).is_some())
        .collect::<Vec<_>>();
    paths.extend(json_files(&root.join("records"))?);
    paths.sort();
    Ok(paths)
}

fn load_indexed_document(root: &Path, path: &Path) -> IndexedDocument {
    let mut issues = Vec::new();
    let value = read_json(root, path, &mut issues);
    IndexedDocument { value, issues }
}

impl DataRootIndex {
    pub fn build(root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = root.as_ref();
        validate_data_root(root)?;
        let root = root.to_path_buf();
        let mut documents = HashMap::new();
        for path in indexed_document_paths(&root)? {
            documents.insert(path.clone(), load_indexed_document(&root, &path));
        }
        let mut index = Self {
            snapshot: DataRootSnapshot {
                root: root.to_string_lossy().into_owned(),
                counts: DataRootCounts {
                    work_items: 0,
                    contexts: 0,
                    checkpoints: 0,
                },
                issues: Vec::new(),
                work_items: Vec::new(),
                checkpoint_ids: Vec::new(),
                restricted_checkpoint_ids: Vec::new(),
            },
            root,
            documents,
            revision: 1,
        };
        index.snapshot = index.rebuild_snapshot();
        Ok(index)
    }

    pub fn snapshot(&self) -> &DataRootSnapshot {
        &self.snapshot
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn refresh_all(&mut self) -> Result<DataRootUpdate, CoreError> {
        validate_data_root(&self.root)?;
        let mut changed_work_item_ids = self.all_work_item_ids();
        let paths = indexed_document_paths(&self.root)?;
        self.documents = paths
            .iter()
            .map(|path| (path.clone(), load_indexed_document(&self.root, path)))
            .collect();
        changed_work_item_ids.extend(self.all_work_item_ids());
        self.snapshot = self.rebuild_snapshot();
        self.revision += 1;
        Ok(DataRootUpdate {
            snapshot: self.snapshot.clone(),
            changed_work_item_ids: changed_work_item_ids.into_iter().collect(),
            full_rescan: true,
            reloaded_files: paths.len(),
            revision: self.revision,
            applied: true,
        })
    }

    pub fn refresh_paths(&mut self, paths: &[PathBuf]) -> Result<DataRootUpdate, CoreError> {
        let normalized_paths = paths
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    self.root.join(path)
                }
            })
            .filter(|path| path.starts_with(&self.root))
            .collect::<BTreeSet<_>>();
        let mut changed_work_item_ids = BTreeSet::new();
        let mut reload_paths = BTreeSet::new();
        let mut relevant = false;
        let mut full_rescan = false;

        for path in &normalized_paths {
            if self.requires_full_rescan(path) {
                relevant = true;
                full_rescan = true;
                break;
            }
            if indexed_document_kind(&self.root, path).is_some() {
                relevant = true;
                if let Some(work_item_id) = self.work_item_id_for_path(path) {
                    changed_work_item_ids.insert(work_item_id);
                }
                reload_paths.insert(path.clone());
            } else if self.is_related_markdown(path) {
                relevant = true;
                if let Some(work_item_id) = self.work_item_id_for_path(path) {
                    changed_work_item_ids.insert(work_item_id);
                }
            }
        }

        if !relevant {
            return Ok(DataRootUpdate {
                snapshot: self.snapshot.clone(),
                changed_work_item_ids: Vec::new(),
                full_rescan: false,
                reloaded_files: 0,
                revision: self.revision,
                applied: false,
            });
        }
        if full_rescan {
            return self.refresh_all();
        }

        for path in &reload_paths {
            self.documents.remove(path);
            if path.is_file() {
                self.documents
                    .insert(path.clone(), load_indexed_document(&self.root, path));
            }
        }
        for path in &normalized_paths {
            if let Some(work_item_id) = self.work_item_id_for_path(path) {
                changed_work_item_ids.insert(work_item_id);
            }
        }
        self.snapshot = self.rebuild_snapshot();
        self.revision += 1;
        Ok(DataRootUpdate {
            snapshot: self.snapshot.clone(),
            changed_work_item_ids: changed_work_item_ids.into_iter().collect(),
            full_rescan: false,
            reloaded_files: reload_paths.len(),
            revision: self.revision,
            applied: true,
        })
    }

    fn requires_full_rescan(&self, path: &Path) -> bool {
        if path == self.root {
            return true;
        }
        if indexed_document_kind(&self.root, path).is_some() || self.is_related_markdown(path) {
            return false;
        }
        let Ok(relative) = path.strip_prefix(&self.root) else {
            return false;
        };
        if !(relative.starts_with("work-items") || relative.starts_with("records")) {
            return false;
        }
        path.is_dir()
            || self
                .documents
                .keys()
                .any(|document| document.starts_with(path))
    }

    fn is_related_markdown(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.root) else {
            return false;
        };
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            return false;
        }
        (relative.starts_with("work-items")
            && path.file_name().and_then(|name| name.to_str()) == Some("context.md"))
            || relative.starts_with("records")
    }

    fn work_item_id_for_path(&self, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(&self.root).ok()?;
        if relative.starts_with("records") {
            let json_path = path.with_extension("json");
            return self
                .documents
                .get(&json_path)
                .and_then(|document| document.value.as_ref())
                .and_then(|value| value.get("work_item_id"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if relative.starts_with("work-items") {
            let directory = path.parent()?;
            return self
                .documents
                .get(&directory.join("work-item.json"))
                .and_then(|document| document.value.as_ref())
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    directory
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                });
        }
        None
    }

    fn all_work_item_ids(&self) -> BTreeSet<String> {
        let mut ids = self
            .snapshot
            .work_items
            .iter()
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>();
        for (path, document) in &self.documents {
            let Some(value) = &document.value else {
                continue;
            };
            let field = match indexed_document_kind(&self.root, path) {
                Some(IndexedDocumentKind::WorkItem) => "id",
                Some(IndexedDocumentKind::Context | IndexedDocumentKind::Checkpoint) => {
                    "work_item_id"
                }
                None => continue,
            };
            if let Some(id) = value.get(field).and_then(Value::as_str) {
                ids.insert(id.to_string());
            }
        }
        ids
    }

    fn rebuild_snapshot(&self) -> DataRootSnapshot {
        let mut issues = self
            .documents
            .values()
            .flat_map(|document| document.issues.clone())
            .collect::<Vec<_>>();
        let mut work_item_files = Vec::new();
        let mut context_files = Vec::new();
        let mut checkpoint_files = Vec::new();
        for (path, document) in &self.documents {
            match indexed_document_kind(&self.root, path) {
                Some(IndexedDocumentKind::WorkItem) => work_item_files.push((path, document)),
                Some(IndexedDocumentKind::Context) => context_files.push((path, document)),
                Some(IndexedDocumentKind::Checkpoint) => checkpoint_files.push((path, document)),
                None => {}
            }
        }
        work_item_files.sort_by_key(|(path, _)| *path);
        context_files.sort_by_key(|(path, _)| *path);
        checkpoint_files.sort_by_key(|(path, _)| *path);

        let mut contexts = HashMap::new();
        for (path, document) in &context_files {
            if let Some(value) = &document.value {
                validate_document(
                    &self.root,
                    path,
                    DocumentKind::WorkContext,
                    value,
                    &mut issues,
                );
                contexts.insert((*path).clone(), value.clone());
            }
        }

        let mut work_items = Vec::new();
        let mut work_item_ids = HashSet::new();
        let mut work_item_projects = HashMap::new();
        for (path, document) in &work_item_files {
            let Some(value) = &document.value else {
                continue;
            };
            if let Some(summary) =
                inspect_work_item(&self.root, path, value, &contexts, &mut issues)
            {
                if !work_item_ids.insert(summary.id.clone()) {
                    issues.push(issue(
                        &self.root,
                        path,
                        "duplicate_work_item_id",
                        format!("중복 업무 항목 ID입니다: {}", summary.id),
                    ));
                }
                work_item_projects.insert(summary.id.clone(), summary.project_id.clone());
                work_items.push(summary);
            }
        }

        let mut checkpoint_ids = HashSet::new();
        let mut restricted_checkpoint_ids = BTreeSet::new();
        let mut checkpoint_confidentiality = HashMap::new();
        let mut activity_dates_by_work_item: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (path, document) in &checkpoint_files {
            let Some(value) = &document.value else {
                continue;
            };
            validate_document(
                &self.root,
                path,
                DocumentKind::Checkpoint,
                value,
                &mut issues,
            );
            let checkpoint_id = required_string(&self.root, path, value, "id", &mut issues);
            let work_item_id =
                required_string(&self.root, path, value, "work_item_id", &mut issues);
            let project_id = required_string(&self.root, path, value, "project_id", &mut issues);
            let confidentiality = value
                .get("confidentiality")
                .and_then(Value::as_str)
                .unwrap_or("normal")
                .to_string();
            if let Some(checkpoint_id) = checkpoint_id {
                if !checkpoint_ids.insert(checkpoint_id.clone()) {
                    issues.push(issue(
                        &self.root,
                        path,
                        "duplicate_checkpoint_id",
                        format!("중복 체크포인트 ID입니다: {checkpoint_id}"),
                    ));
                }
                if confidentiality == "restricted" {
                    restricted_checkpoint_ids.insert(checkpoint_id.clone());
                }
                checkpoint_confidentiality.insert(checkpoint_id, confidentiality.clone());
            }
            if let Some(work_item_id) = work_item_id {
                activity_dates_by_work_item
                    .entry(work_item_id.clone())
                    .or_default()
                    .extend(checkpoint_activity_dates(value));
                match work_item_projects.get(&work_item_id) {
                    None => issues.push(issue(
                        &self.root,
                        path,
                        "unknown_work_item",
                        format!("알 수 없는 업무 항목을 참조합니다: {work_item_id}"),
                    )),
                    Some(expected_project) if project_id.as_ref() != Some(expected_project) => {
                        issues.push(issue(
                            &self.root,
                            path,
                            "project_mismatch",
                            format!("업무 항목 {work_item_id}의 프로젝트와 일치하지 않습니다."),
                        ));
                    }
                    Some(_) => {}
                }
            }
            let markdown_path = path.with_extension("md");
            if !markdown_path.exists() {
                issues.push(issue(
                    &self.root,
                    &markdown_path,
                    "missing_checkpoint_markdown",
                    "파생 체크포인트 Markdown이 없습니다.",
                ));
            }
        }

        for work_item in &mut work_items {
            work_item.activity_dates = activity_dates_by_work_item
                .remove(&work_item.id)
                .map(|dates| dates.into_iter().collect())
                .unwrap_or_default();
            work_item.last_checkpoint_confidentiality = work_item
                .last_checkpoint_id
                .as_ref()
                .and_then(|id| checkpoint_confidentiality.get(id))
                .cloned();
        }

        work_items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.code.cmp(&right.code))
        });
        let mut checkpoint_ids = checkpoint_ids.into_iter().collect::<Vec<_>>();
        checkpoint_ids.sort();

        DataRootSnapshot {
            root: self.root.to_string_lossy().into_owned(),
            counts: DataRootCounts {
                work_items: work_item_files.len(),
                contexts: context_files.len(),
                checkpoints: checkpoint_files.len(),
            },
            issues,
            work_items,
            checkpoint_ids,
            restricted_checkpoint_ids: restricted_checkpoint_ids.into_iter().collect(),
        }
    }
}

pub fn inspect_data_root(root: impl AsRef<Path>) -> Result<DataRootSnapshot, CoreError> {
    Ok(DataRootIndex::build(root)?.snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use tempfile::tempdir;

    fn write_fixture(root: &Path) {
        let work_item_dir = root.join("work-items/AUTH-142");
        let record_dir = root.join("records/2026/07/13");
        create_dir_all(&work_item_dir).unwrap();
        create_dir_all(&record_dir).unwrap();
        write(
            work_item_dir.join("work-item.json"),
            include_str!("../../../examples/work-items/AUTH-142/work-item.json"),
        )
        .unwrap();
        write(
            work_item_dir.join("context.json"),
            include_str!("../../../examples/work-items/AUTH-142/context.json"),
        )
        .unwrap();
        write(work_item_dir.join("context.md"), "# Context\n").unwrap();
        write(
            record_dir.join("CP-20260713-001.json"),
            include_str!("../../../examples/records/2026/07/13/CP-20260713-001.json"),
        )
        .unwrap();
        write(record_dir.join("CP-20260713-001.md"), "# Checkpoint\n").unwrap();
    }

    #[test]
    fn inspects_a_valid_minimal_data_root() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());

        let snapshot = inspect_data_root(directory.path()).unwrap();

        assert_eq!(
            snapshot.counts,
            DataRootCounts {
                work_items: 1,
                contexts: 1,
                checkpoints: 1,
            }
        );
        assert!(snapshot.issues.is_empty());
        assert_eq!(snapshot.checkpoint_ids, ["CP-20260713-001"]);
        assert_eq!(snapshot.restricted_checkpoint_ids, Vec::<String>::new());
        assert_eq!(snapshot.work_items[0].id, "AUTH-142");
        assert_eq!(snapshot.work_items[0].activity_dates, ["2026-07-13"]);
        assert_eq!(
            snapshot.work_items[0]
                .last_checkpoint_confidentiality
                .as_deref(),
            Some("normal")
        );
        assert_eq!(
            snapshot.work_items[0].current_state.as_deref(),
            Some(
                "refresh token 서비스와 interceptor 연동을 완료했다. 동시 요청 테스트를 작성하는 중이다."
            )
        );
    }

    #[test]
    fn reports_json_schema_violations() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());
        let work_item_path = directory.path().join("work-items/AUTH-142/work-item.json");
        let mut work_item: Value = read_data_json(&work_item_path).unwrap();
        work_item.as_object_mut().unwrap().remove("objective");
        write(
            &work_item_path,
            serde_json::to_string_pretty(&work_item).unwrap(),
        )
        .unwrap();

        let snapshot = inspect_data_root(directory.path()).unwrap();

        assert!(snapshot.issues.iter().any(|issue| {
            issue.code == "schema_validation"
                && issue.path == "work-items/AUTH-142/work-item.json"
                && issue.message.contains("objective")
        }));
    }

    #[test]
    fn returns_work_item_detail_with_checkpoint_timeline() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());

        let detail = get_work_item_detail(directory.path(), "AUTH-142").unwrap();

        assert_eq!(detail.title, "인증 시스템 개선");
        assert_eq!(detail.context.unwrap().next_steps.len(), 3);
        assert_eq!(detail.checkpoints.len(), 1);
        assert_eq!(detail.checkpoints[0].id, "CP-20260713-001");
        assert_eq!(detail.checkpoints[0].verifications[0].status, "passed");
        assert_eq!(detail.checkpoints[0].decisions.len(), 1);
        assert_eq!(detail.checkpoints[0].evidence.commits, ["abc1234"]);
        assert_eq!(
            detail.checkpoints[0].markdown_path,
            "records/2026/07/13/CP-20260713-001.md"
        );
    }

    #[test]
    fn rejects_unsafe_work_item_identifiers() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());

        let error = get_work_item_detail(directory.path(), "../AUTH-142").unwrap_err();

        assert!(matches!(error, CoreError::WorkItemNotFound(_)));
    }

    #[test]
    fn resolves_only_known_markdown_assets() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());

        let context = context_markdown_path(directory.path(), "AUTH-142").unwrap();
        let checkpoint = checkpoint_markdown_path(directory.path(), "CP-20260713-001").unwrap();

        assert!(context.ends_with("work-items/AUTH-142/context.md"));
        assert!(checkpoint.ends_with("records/2026/07/13/CP-20260713-001.md"));
        assert!(matches!(
            checkpoint_markdown_path(directory.path(), "../secret"),
            Err(CoreError::DataAssetNotFound(_))
        ));
    }

    #[test]
    fn reports_invalid_json_without_aborting_the_scan() {
        let directory = tempdir().unwrap();
        let work_item_dir = directory.path().join("work-items/BROKEN");
        create_dir_all(&work_item_dir).unwrap();
        write(work_item_dir.join("work-item.json"), "{broken").unwrap();

        let snapshot = inspect_data_root(directory.path()).unwrap();

        assert_eq!(snapshot.counts.work_items, 1);
        assert_eq!(snapshot.work_items.len(), 0);
        assert!(
            snapshot
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_json")
        );
    }

    #[test]
    fn incrementally_reloads_only_the_changed_document() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());
        let context_path = directory.path().join("work-items/AUTH-142/context.json");
        let mut index = DataRootIndex::build(directory.path()).unwrap();
        let initial_revision = index.revision();
        let mut context = read_data_json(&context_path).unwrap();
        context["current_state"] = Value::String("증분 인덱스 갱신 완료".to_string());
        write(
            &context_path,
            serde_json::to_string_pretty(&context).unwrap(),
        )
        .unwrap();

        let update = index
            .refresh_paths(std::slice::from_ref(&context_path))
            .unwrap();

        assert!(update.applied);
        assert!(!update.full_rescan);
        assert_eq!(update.reloaded_files, 1);
        assert_eq!(update.revision, initial_revision + 1);
        assert_eq!(update.changed_work_item_ids, ["AUTH-142"]);
        assert_eq!(
            update.snapshot.work_items[0].current_state.as_deref(),
            Some("증분 인덱스 갱신 완료")
        );
        assert_eq!(
            update.snapshot,
            inspect_data_root(directory.path()).unwrap()
        );

        let full_update = index.refresh_all().unwrap();
        assert_eq!(full_update.changed_work_item_ids, ["AUTH-142"]);
    }

    #[test]
    fn ignores_changes_outside_indexed_data() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());
        let mut index = DataRootIndex::build(directory.path()).unwrap();
        let revision = index.revision();
        let unrelated_path = directory.path().join("README.md");
        write(&unrelated_path, "not indexed").unwrap();

        let update = index
            .refresh_paths(std::slice::from_ref(&unrelated_path))
            .unwrap();

        assert!(!update.applied);
        assert_eq!(update.revision, revision);
        assert_eq!(update.reloaded_files, 0);
        assert!(update.changed_work_item_ids.is_empty());
    }

    #[test]
    fn markdown_changes_update_relationship_issues_without_json_reload() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());
        let mut index = DataRootIndex::build(directory.path()).unwrap();
        let context_markdown = directory.path().join("work-items/AUTH-142/context.md");
        fs::remove_file(&context_markdown).unwrap();

        let update = index
            .refresh_paths(std::slice::from_ref(&context_markdown))
            .unwrap();

        assert!(update.applied);
        assert_eq!(update.reloaded_files, 0);
        assert_eq!(update.changed_work_item_ids, ["AUTH-142"]);
        assert!(
            update
                .snapshot
                .issues
                .iter()
                .any(|issue| issue.code == "missing_context_markdown")
        );
    }

    #[test]
    fn repeated_incremental_updates_converge_with_a_full_scan() {
        let directory = tempdir().unwrap();
        write_fixture(directory.path());
        let context_path = directory.path().join("work-items/AUTH-142/context.json");
        let mut context = read_data_json(&context_path).unwrap();
        let mut index = DataRootIndex::build(directory.path()).unwrap();

        for iteration in 0..250 {
            context["current_state"] = Value::String(format!("soak iteration {iteration}"));
            write(&context_path, serde_json::to_string(&context).unwrap()).unwrap();
            let update = index
                .refresh_paths(std::slice::from_ref(&context_path))
                .unwrap();
            assert_eq!(update.reloaded_files, 1);
        }

        assert_eq!(
            index.snapshot(),
            &inspect_data_root(directory.path()).unwrap()
        );
        assert_eq!(
            index.snapshot().work_items[0].current_state.as_deref(),
            Some("soak iteration 249")
        );
    }

    #[test]
    fn inspects_the_repository_examples() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");

        let snapshot = inspect_data_root(examples).unwrap();

        assert_eq!(snapshot.counts.work_items, 1);
        assert_eq!(snapshot.counts.contexts, 1);
        assert_eq!(snapshot.counts.checkpoints, 1);
        assert!(snapshot.issues.is_empty(), "{:#?}", snapshot.issues);
    }
}
