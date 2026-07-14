mod schema;

use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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
    pub current_state: Option<String>,
    pub last_checkpoint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataRootSnapshot {
    pub root: String,
    pub counts: DataRootCounts,
    pub issues: Vec<DataIssue>,
    pub work_items: Vec<WorkItemSummary>,
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
    contexts: &HashMap<PathBuf, Value>,
    issues: &mut Vec<DataIssue>,
) -> Option<WorkItemSummary> {
    let value = read_json(root, path, issues)?;
    validate_document(root, path, DocumentKind::WorkItem, &value, issues);
    let id = required_string(root, path, &value, "id", issues)?;
    let project_id = required_string(root, path, &value, "project_id", issues)?;
    let title = required_string(root, path, &value, "title", issues)?;
    let status = required_string(root, path, &value, "status", issues)?;
    let updated_at = required_string(root, path, &value, "updated_at", issues)?;

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
        current_state: context
            .and_then(|value| value.get("current_state"))
            .and_then(Value::as_str)
            .map(str::to_string),
        last_checkpoint_id: context
            .and_then(|value| value.get("last_checkpoint_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
    })
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

pub fn inspect_data_root(root: impl AsRef<Path>) -> Result<DataRootSnapshot, CoreError> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(CoreError::MissingRoot(root.to_string_lossy().into_owned()));
    }
    if !root.is_dir() {
        return Err(CoreError::InvalidRoot(root.to_string_lossy().into_owned()));
    }

    let work_data_files = json_files(&root.join("work-items"))?;
    let work_item_json_files = work_data_files
        .iter()
        .filter(|path| path.file_name().and_then(|value| value.to_str()) == Some("work-item.json"))
        .cloned()
        .collect::<Vec<_>>();
    let context_files = work_data_files
        .into_iter()
        .filter(|path| path.file_name().and_then(|value| value.to_str()) == Some("context.json"))
        .collect::<Vec<_>>();
    let checkpoint_files = json_files(&root.join("records"))?;

    let mut issues = Vec::new();
    let mut work_items = Vec::new();
    let mut work_item_ids = HashSet::new();
    let mut work_item_projects = HashMap::new();
    let mut contexts = HashMap::new();

    for path in &context_files {
        if let Some(value) = read_json(root, path, &mut issues) {
            validate_document(root, path, DocumentKind::WorkContext, &value, &mut issues);
            contexts.insert(path.clone(), value);
        }
    }

    for path in &work_item_json_files {
        if let Some(summary) = inspect_work_item(root, path, &contexts, &mut issues) {
            if !work_item_ids.insert(summary.id.clone()) {
                issues.push(issue(
                    root,
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
    for path in &checkpoint_files {
        let Some(value) = read_json(root, path, &mut issues) else {
            continue;
        };
        validate_document(root, path, DocumentKind::Checkpoint, &value, &mut issues);
        let checkpoint_id = required_string(root, path, &value, "id", &mut issues);
        let work_item_id = required_string(root, path, &value, "work_item_id", &mut issues);
        let project_id = required_string(root, path, &value, "project_id", &mut issues);
        if let Some(checkpoint_id) = checkpoint_id {
            if !checkpoint_ids.insert(checkpoint_id.clone()) {
                issues.push(issue(
                    root,
                    path,
                    "duplicate_checkpoint_id",
                    format!("중복 체크포인트 ID입니다: {checkpoint_id}"),
                ));
            }
        }
        if let Some(work_item_id) = work_item_id {
            match work_item_projects.get(&work_item_id) {
                None => issues.push(issue(
                    root,
                    path,
                    "unknown_work_item",
                    format!("알 수 없는 업무 항목을 참조합니다: {work_item_id}"),
                )),
                Some(expected_project) if project_id.as_ref() != Some(expected_project) => {
                    issues.push(issue(
                        root,
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
                root,
                &markdown_path,
                "missing_checkpoint_markdown",
                "파생 체크포인트 Markdown이 없습니다.",
            ));
        }
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

    Ok(DataRootSnapshot {
        root: root.to_string_lossy().into_owned(),
        counts: DataRootCounts {
            work_items: work_item_json_files.len(),
            contexts: context_files.len(),
            checkpoints: checkpoint_files.len(),
        },
        issues,
        work_items,
    })
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
        assert_eq!(snapshot.work_items[0].id, "AUTH-142");
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
    fn inspects_the_repository_examples() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");

        let snapshot = inspect_data_root(examples).unwrap();

        assert_eq!(snapshot.counts.work_items, 1);
        assert_eq!(snapshot.counts.contexts, 1);
        assert_eq!(snapshot.counts.checkpoints, 1);
        assert!(snapshot.issues.is_empty(), "{:#?}", snapshot.issues);
    }
}
