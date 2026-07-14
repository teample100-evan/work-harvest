use crate::schema::{self, DocumentKind};
use crate::write::hash_bytes;
use crate::{DataRootWriter, FileRevision, WriteCommit, WriteError, WriteOperation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const DEFAULT_CURRENT_STATE: &str = "업무 항목을 생성했으며 구체적인 작업을 시작하기 전이다.";

#[derive(Debug, Error)]
pub enum WorkItemWriteError {
    #[error("Invalid work item input: {0}")]
    InvalidInput(String),
    #[error("Work item was not found: {0}")]
    WorkItemNotFound(String),
    #[error("Could not read work item asset {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not parse work item asset {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("Could not serialize {document}: {source}")]
    Serialize {
        document: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("{document} validation failed: {details}")]
    Validation {
        document: &'static str,
        details: String,
    },
    #[error("Work item assets are inconsistent: {0}")]
    Inconsistent(String),
    #[error(transparent)]
    Write(#[from] WriteError),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkItemClassificationInput {
    pub initiative_id: Option<String>,
    pub work_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredWorkItemClassification {
    pub initiative_id: Option<String>,
    pub work_types: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContextFileInput {
    Path(String),
    Detail {
        path: String,
        description: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredContextFile {
    pub path: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextVerificationInput {
    pub completed: Option<Vec<String>>,
    pub pending: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredContextVerification {
    pub completed: Vec<String>,
    pub pending: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGitInput {
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredContextGit {
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkContextInput {
    pub last_checkpoint_id: Option<String>,
    pub last_verified_git_ref: Option<String>,
    pub current_state: Option<String>,
    pub decisions: Option<Vec<String>>,
    pub files: Option<Vec<ContextFileInput>>,
    pub verification: Option<ContextVerificationInput>,
    pub verification_completed: Option<Vec<String>>,
    pub verification_pending: Option<Vec<String>>,
    pub next_steps: Option<Vec<String>>,
    pub risks: Option<Vec<String>>,
    pub git: Option<ContextGitInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItemCreateInput {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: Option<String>,
    pub objective: String,
    pub desired_outcomes: Option<Vec<String>>,
    pub classification: Option<WorkItemClassificationInput>,
    pub repositories: Option<Vec<Value>>,
    pub links: Option<Vec<Value>>,
    pub context_path: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub context: Option<WorkContextInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItemDocument {
    pub schema_version: String,
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub objective: String,
    pub desired_outcomes: Vec<String>,
    pub classification: StoredWorkItemClassification,
    pub repositories: Vec<Value>,
    pub links: Vec<Value>,
    pub context_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkContextDocument {
    pub schema_version: String,
    pub work_item_id: String,
    pub project_id: String,
    pub updated_at: String,
    pub last_checkpoint_id: Option<String>,
    pub last_verified_git_ref: Option<String>,
    pub current_state: String,
    pub decisions: Vec<String>,
    pub files: Vec<StoredContextFile>,
    pub verification: StoredContextVerification,
    pub next_steps: Vec<String>,
    pub risks: Vec<String>,
    pub git: StoredContextGit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkContextPatch {
    pub current_state: Option<String>,
    pub decisions: Option<Vec<String>>,
    pub files: Option<Vec<ContextFileInput>>,
    pub verification: Option<ContextVerificationInput>,
    pub next_steps: Option<Vec<String>>,
    pub risks: Option<Vec<String>>,
    pub git: Option<ContextGitInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkItemUpdatePatch {
    pub title: Option<String>,
    pub status: Option<String>,
    pub objective: Option<String>,
    pub desired_outcomes: Option<Vec<String>>,
    pub classification: Option<WorkItemClassificationInput>,
    pub repositories: Option<Vec<Value>>,
    pub links: Option<Vec<Value>>,
    pub completed_at: Option<String>,
    pub context: Option<WorkContextPatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemPaths {
    pub work_item: String,
    pub context_data: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemEditRevisions {
    pub work_item: FileRevision,
    pub context_data: FileRevision,
    pub context: FileRevision,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkItemEditSnapshot {
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub markdown: String,
    pub paths: WorkItemPaths,
    pub revisions: WorkItemEditRevisions,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorkItemWriteResult {
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub paths: WorkItemPaths,
    pub revisions: WorkItemEditRevisions,
    pub commit: WriteCommit,
}

fn context_path(work_item_id: &str) -> String {
    format!("work-items/{work_item_id}/context.md")
}

fn paths(work_item_id: &str) -> WorkItemPaths {
    let directory = format!("work-items/{work_item_id}");
    WorkItemPaths {
        work_item: format!("{directory}/work-item.json"),
        context_data: format!("{directory}/context.json"),
        context: format!("{directory}/context.md"),
    }
}

fn normalize_classification(input: WorkItemClassificationInput) -> StoredWorkItemClassification {
    StoredWorkItemClassification {
        initiative_id: input.initiative_id,
        work_types: input.work_types.unwrap_or_default(),
        tags: input.tags.unwrap_or_default(),
    }
}

fn normalize_files(input: Option<Vec<ContextFileInput>>) -> Vec<StoredContextFile> {
    input
        .unwrap_or_default()
        .into_iter()
        .map(|file| match file {
            ContextFileInput::Path(path) => StoredContextFile {
                path,
                description: None,
            },
            ContextFileInput::Detail { path, description } => {
                StoredContextFile { path, description }
            }
        })
        .collect()
}

fn normalize_git(input: ContextGitInput) -> StoredContextGit {
    StoredContextGit {
        repository: input.repository,
        branch: input.branch,
        commit: input.commit,
        checked_at: input.checked_at,
    }
}

pub fn normalize_work_item(
    input: WorkItemCreateInput,
    now: &str,
) -> Result<(WorkItemDocument, WorkContextDocument), WorkItemWriteError> {
    let canonical_context_path = context_path(&input.id);
    if input
        .context_path
        .as_ref()
        .is_some_and(|value| value != &canonical_context_path)
    {
        return Err(WorkItemWriteError::InvalidInput(format!(
            "context_path must be {canonical_context_path}"
        )));
    }

    let status = input.status.unwrap_or_else(|| "planned".to_string());
    let work_item = WorkItemDocument {
        schema_version: "1.0".to_string(),
        id: input.id,
        project_id: input.project_id,
        title: input.title,
        status: status.clone(),
        objective: input.objective,
        desired_outcomes: input.desired_outcomes.unwrap_or_default(),
        classification: normalize_classification(input.classification.unwrap_or_default()),
        repositories: input.repositories.unwrap_or_default(),
        links: input.links.unwrap_or_default(),
        context_path: canonical_context_path,
        created_at: input.created_at.unwrap_or_else(|| now.to_string()),
        updated_at: input.updated_at.unwrap_or_else(|| now.to_string()),
        completed_at: if status == "completed" {
            Some(input.completed_at.unwrap_or_else(|| now.to_string()))
        } else {
            None
        },
    };
    let context_input = input.context.unwrap_or_default();
    let verification = context_input.verification.unwrap_or_default();
    let context = WorkContextDocument {
        schema_version: "1.0".to_string(),
        work_item_id: work_item.id.clone(),
        project_id: work_item.project_id.clone(),
        updated_at: work_item.updated_at.clone(),
        last_checkpoint_id: context_input.last_checkpoint_id,
        last_verified_git_ref: context_input.last_verified_git_ref,
        current_state: context_input
            .current_state
            .unwrap_or_else(|| DEFAULT_CURRENT_STATE.to_string()),
        decisions: context_input.decisions.unwrap_or_default(),
        files: normalize_files(context_input.files),
        verification: StoredContextVerification {
            completed: verification
                .completed
                .or(context_input.verification_completed)
                .unwrap_or_default(),
            pending: verification
                .pending
                .or(context_input.verification_pending)
                .unwrap_or_default(),
        },
        next_steps: context_input.next_steps.unwrap_or_default(),
        risks: context_input.risks.unwrap_or_default(),
        git: normalize_git(context_input.git.unwrap_or_default()),
    };

    validate_documents(&work_item, &context)?;
    Ok((work_item, context))
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

fn optional_frontmatter_string(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(json_string)
        .unwrap_or_else(|| "null".to_string())
}

fn bullets(values: &[String], fallback: &str) -> String {
    if values.is_empty() {
        format!("- {fallback}")
    } else {
        values
            .iter()
            .map(|value| format!("- {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn file_bullets(values: &[StoredContextFile]) -> String {
    if values.is_empty() {
        return "- 아직 지정하지 않음".to_string();
    }
    values
        .iter()
        .map(|value| {
            match value
                .description
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                Some(description) => format!("- `{}`: {description}", value.path),
                None => format!("- `{}`", value.path),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn git_value(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(|value| format!("`{value}`"))
        .unwrap_or_else(|| "지정하지 않음".to_string())
}

pub fn render_context(work_item: &WorkItemDocument, context: &WorkContextDocument) -> String {
    format!(
        "---\n\
schema_version: \"1.0\"\n\
work_item_id: {}\n\
project_id: {}\n\
title: {}\n\
status: {}\n\
updated_at: {}\n\
last_checkpoint_id: {}\n\
last_verified_git_ref: {}\n\
---\n\n\
# {} {}\n\n\
## 목표\n\n\
{}\n\n\
## 현재 상태\n\n\
{}\n\n\
## 주요 결정과 이유\n\n\
{}\n\n\
## 주요 파일과 문서\n\n\
{}\n\n\
## 검증 상태\n\n\
### 완료\n\n\
{}\n\n\
### 미완료\n\n\
{}\n\n\
## 남은 작업\n\n\
{}\n\n\
## 리스크와 확인할 사항\n\n\
{}\n\n\
## 마지막으로 확인한 Git 기준점\n\n\
- 저장소: {}\n\
- 브랜치: {}\n\
- 커밋: {}\n\
- 확인 시각: {}\n",
        json_string(&work_item.id),
        json_string(&work_item.project_id),
        json_string(&work_item.title),
        json_string(&work_item.status),
        json_string(&context.updated_at),
        optional_frontmatter_string(context.last_checkpoint_id.as_deref()),
        optional_frontmatter_string(context.last_verified_git_ref.as_deref()),
        work_item.id,
        work_item.title,
        work_item.objective,
        context.current_state,
        bullets(&context.decisions, "아직 확정된 결정 없음"),
        file_bullets(&context.files),
        bullets(&context.verification.completed, "완료된 검증 없음"),
        bullets(&context.verification.pending, "예정된 검증 없음"),
        bullets(&context.next_steps, "다음 작업을 구체화해야 함"),
        bullets(&context.risks, "현재 확인된 리스크 없음"),
        git_value(context.git.repository.as_deref()),
        git_value(context.git.branch.as_deref()),
        git_value(context.git.commit.as_deref()),
        context.git.checked_at.as_deref().unwrap_or("지정하지 않음"),
    )
}

fn validate_schema(
    document: &'static str,
    kind: DocumentKind,
    value: &Value,
) -> Result<(), WorkItemWriteError> {
    let violations = schema::validate(kind, value)
        .map_err(|details| WorkItemWriteError::Validation { document, details })?;
    if violations.is_empty() {
        return Ok(());
    }
    let details = violations
        .into_iter()
        .map(|violation| {
            let path = if violation.instance_path.is_empty() {
                "$".to_string()
            } else {
                format!("${}", violation.instance_path)
            };
            format!("{path}: {}", violation.message)
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(WorkItemWriteError::Validation { document, details })
}

fn validate_documents(
    work_item: &WorkItemDocument,
    context: &WorkContextDocument,
) -> Result<(), WorkItemWriteError> {
    let work_item_value =
        serde_json::to_value(work_item).map_err(|source| WorkItemWriteError::Serialize {
            document: "work-item.json",
            source,
        })?;
    let context_value =
        serde_json::to_value(context).map_err(|source| WorkItemWriteError::Serialize {
            document: "context.json",
            source,
        })?;
    validate_schema("work-item.json", DocumentKind::WorkItem, &work_item_value)?;
    validate_schema("context.json", DocumentKind::WorkContext, &context_value)?;
    if context.work_item_id != work_item.id {
        return Err(WorkItemWriteError::Inconsistent(
            "context work_item_id does not match work item id".to_string(),
        ));
    }
    if context.project_id != work_item.project_id {
        return Err(WorkItemWriteError::Inconsistent(
            "context project_id does not match work item project_id".to_string(),
        ));
    }
    if context.updated_at != work_item.updated_at {
        return Err(WorkItemWriteError::Inconsistent(
            "context updated_at does not match work item updated_at".to_string(),
        ));
    }
    if work_item.context_path != context_path(&work_item.id) {
        return Err(WorkItemWriteError::Inconsistent(
            "work item context_path is not canonical".to_string(),
        ));
    }
    Ok(())
}

fn json_bytes<T: Serialize>(
    document: &'static str,
    value: &T,
) -> Result<Vec<u8>, WorkItemWriteError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|source| WorkItemWriteError::Serialize { document, source })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn file_revision(bytes: &[u8]) -> FileRevision {
    FileRevision {
        sha256: hash_bytes(bytes),
        bytes: bytes.len() as u64,
    }
}

fn read_bytes(
    root: &Path,
    relative_path: &str,
) -> Result<(Vec<u8>, FileRevision), WorkItemWriteError> {
    let path = root.join(relative_path);
    let bytes = fs::read(&path).map_err(|source| WorkItemWriteError::Read {
        path: relative_path.to_string(),
        source,
    })?;
    let revision = file_revision(&bytes);
    Ok((bytes, revision))
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    relative_path: &str,
    bytes: &[u8],
) -> Result<T, WorkItemWriteError> {
    serde_json::from_slice(bytes).map_err(|source| WorkItemWriteError::Parse {
        path: relative_path.to_string(),
        source,
    })
}

fn read_snapshot_from_root(
    root: &Path,
    work_item_id: &str,
) -> Result<WorkItemEditSnapshot, WorkItemWriteError> {
    if !crate::is_identifier(work_item_id) {
        return Err(WorkItemWriteError::WorkItemNotFound(
            work_item_id.to_string(),
        ));
    }
    let paths = paths(work_item_id);
    let (work_item_bytes, work_item_revision) = read_bytes(root, &paths.work_item)?;
    let (context_bytes, context_data_revision) = read_bytes(root, &paths.context_data)?;
    let (markdown_bytes, context_revision) = read_bytes(root, &paths.context)?;
    let work_item: WorkItemDocument = parse_json(&paths.work_item, &work_item_bytes)?;
    let context: WorkContextDocument = parse_json(&paths.context_data, &context_bytes)?;
    let markdown =
        String::from_utf8(markdown_bytes).map_err(|source| WorkItemWriteError::Read {
            path: paths.context.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        })?;
    if work_item.id != work_item_id {
        return Err(WorkItemWriteError::WorkItemNotFound(
            work_item_id.to_string(),
        ));
    }
    validate_documents(&work_item, &context)?;
    if markdown != render_context(&work_item, &context) {
        return Err(WorkItemWriteError::Inconsistent(
            "context.md does not match its structured source".to_string(),
        ));
    }
    Ok(WorkItemEditSnapshot {
        work_item,
        context,
        markdown,
        paths,
        revisions: WorkItemEditRevisions {
            work_item: work_item_revision,
            context_data: context_data_revision,
            context: context_revision,
        },
    })
}

fn validate_written_triplet(root: &Path, work_item_id: &str) -> Result<(), String> {
    read_snapshot_from_root(root, work_item_id)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn revisions(
    writer: &DataRootWriter,
    paths: &WorkItemPaths,
) -> Result<WorkItemEditRevisions, WorkItemWriteError> {
    let revision = |path: &str| -> Result<FileRevision, WorkItemWriteError> {
        writer.revision(path)?.ok_or_else(|| {
            WorkItemWriteError::Inconsistent(format!("committed file is missing: {path}"))
        })
    };
    Ok(WorkItemEditRevisions {
        work_item: revision(&paths.work_item)?,
        context_data: revision(&paths.context_data)?,
        context: revision(&paths.context)?,
    })
}

pub fn create_work_item(
    root: impl AsRef<Path>,
    input: WorkItemCreateInput,
    now: &str,
) -> Result<WorkItemWriteResult, WorkItemWriteError> {
    let (work_item, context) = normalize_work_item(input, now)?;
    let paths = paths(&work_item.id);
    let context_markdown = render_context(&work_item, &context);
    let operations = vec![
        WriteOperation::create(
            PathBuf::from(&paths.context_data),
            json_bytes("context.json", &context)?,
        ),
        WriteOperation::create(PathBuf::from(&paths.context), context_markdown.into_bytes()),
        WriteOperation::create(
            PathBuf::from(&paths.work_item),
            json_bytes("work-item.json", &work_item)?,
        ),
    ];
    let mut writer = DataRootWriter::acquire(root)?;
    let work_item_id = work_item.id.clone();
    let commit = writer.commit_validated(operations, move |root| {
        validate_written_triplet(root, &work_item_id)
    })?;
    let revisions = revisions(&writer, &paths)?;
    Ok(WorkItemWriteResult {
        work_item,
        context,
        paths,
        revisions,
        commit,
    })
}

pub fn read_work_item_for_edit(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<WorkItemEditSnapshot, WorkItemWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    read_snapshot_from_root(writer.root(), work_item_id)
}

fn verify_revision(
    writer: &DataRootWriter,
    path: &str,
    expected: &FileRevision,
) -> Result<(), WorkItemWriteError> {
    let actual = writer.revision(path)?;
    if actual.as_ref().map(|value| value.sha256.as_str()) != Some(expected.sha256.as_str()) {
        return Err(WriteError::RevisionConflict {
            path: path.to_string(),
            expected: expected.sha256.clone(),
            actual: actual.map(|value| value.sha256),
        }
        .into());
    }
    Ok(())
}

fn apply_context_patch(context: &mut WorkContextDocument, patch: WorkContextPatch) {
    if let Some(value) = patch.current_state {
        context.current_state = value;
    }
    if let Some(value) = patch.decisions {
        context.decisions = value;
    }
    if let Some(value) = patch.files {
        context.files = normalize_files(Some(value));
    }
    if let Some(value) = patch.verification {
        if let Some(completed) = value.completed {
            context.verification.completed = completed;
        }
        if let Some(pending) = value.pending {
            context.verification.pending = pending;
        }
    }
    if let Some(value) = patch.next_steps {
        context.next_steps = value;
    }
    if let Some(value) = patch.risks {
        context.risks = value;
    }
    if let Some(value) = patch.git {
        context.git = normalize_git(value);
    }
}

fn apply_update(
    mut work_item: WorkItemDocument,
    mut context: WorkContextDocument,
    patch: WorkItemUpdatePatch,
    now: &str,
) -> Result<(WorkItemDocument, WorkContextDocument), WorkItemWriteError> {
    if let Some(value) = patch.title {
        work_item.title = value;
    }
    if let Some(value) = patch.status {
        work_item.status = value;
    }
    if let Some(value) = patch.objective {
        work_item.objective = value;
    }
    if let Some(value) = patch.desired_outcomes {
        work_item.desired_outcomes = value;
    }
    if let Some(value) = patch.classification {
        work_item.classification = normalize_classification(value);
    }
    if let Some(value) = patch.repositories {
        work_item.repositories = value;
    }
    if let Some(value) = patch.links {
        work_item.links = value;
    }
    work_item.completed_at = if work_item.status == "completed" {
        Some(
            patch
                .completed_at
                .or(work_item.completed_at)
                .unwrap_or_else(|| now.to_string()),
        )
    } else {
        None
    };
    if let Some(value) = patch.context {
        apply_context_patch(&mut context, value);
    }
    work_item.updated_at = now.to_string();
    context.updated_at = now.to_string();
    validate_documents(&work_item, &context)?;
    Ok((work_item, context))
}

pub fn update_work_item(
    root: impl AsRef<Path>,
    work_item_id: &str,
    expected: WorkItemEditRevisions,
    patch: WorkItemUpdatePatch,
    now: &str,
) -> Result<WorkItemWriteResult, WorkItemWriteError> {
    if !crate::is_identifier(work_item_id) {
        return Err(WorkItemWriteError::WorkItemNotFound(
            work_item_id.to_string(),
        ));
    }
    let mut writer = DataRootWriter::acquire(root)?;
    let paths = paths(work_item_id);
    verify_revision(&writer, &paths.work_item, &expected.work_item)?;
    verify_revision(&writer, &paths.context_data, &expected.context_data)?;
    verify_revision(&writer, &paths.context, &expected.context)?;
    let snapshot = read_snapshot_from_root(writer.root(), work_item_id)?;
    let (work_item, context) = apply_update(snapshot.work_item, snapshot.context, patch, now)?;
    let operations = vec![
        WriteOperation::replace(
            PathBuf::from(&paths.context_data),
            expected.context_data.sha256,
            json_bytes("context.json", &context)?,
        ),
        WriteOperation::replace(
            PathBuf::from(&paths.context),
            expected.context.sha256,
            render_context(&work_item, &context).into_bytes(),
        ),
        WriteOperation::replace(
            PathBuf::from(&paths.work_item),
            expected.work_item.sha256,
            json_bytes("work-item.json", &work_item)?,
        ),
    ];
    let work_item_id = work_item.id.clone();
    let commit = writer.commit_validated(operations, move |root| {
        validate_written_triplet(root, &work_item_id)
    })?;
    let revisions = revisions(&writer, &paths)?;
    Ok(WorkItemWriteResult {
        work_item,
        context,
        paths,
        revisions,
        commit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::tempdir;

    const CREATED_AT: &str = "2026-07-14T09:30:00.000Z";
    const UPDATED_AT: &str = "2026-07-14T10:45:00.000Z";

    fn input(id: &str) -> WorkItemCreateInput {
        WorkItemCreateInput {
            id: id.to_string(),
            project_id: "jajak-front".to_string(),
            title: "인증 \"재시도\" 개선".to_string(),
            status: Some("in_progress".to_string()),
            objective: "토큰 만료 시 요청을 안전하게 재시도한다.".to_string(),
            desired_outcomes: Some(vec!["인증 갱신 동작을 테스트로 검증한다.".to_string()]),
            classification: Some(WorkItemClassificationInput {
                initiative_id: Some("authentication".to_string()),
                work_types: Some(vec!["implementation".to_string(), "testing".to_string()]),
                tags: Some(vec!["auth".to_string()]),
            }),
            repositories: Some(vec![json!({
                "name": "jajak-front",
                "path": "/workspace/jajak/front",
                "remote_url": null,
                "default_branch": "main"
            })]),
            links: Some(vec![json!({
                "type": "issue",
                "label": "AUTH-142",
                "external_id": "142",
                "url": "https://example.com/issues/142"
            })]),
            context_path: None,
            created_at: Some(CREATED_AT.to_string()),
            updated_at: Some(CREATED_AT.to_string()),
            completed_at: None,
            context: Some(WorkContextInput {
                current_state: Some("인증 테스트 작업을 시작하기 전이다.".to_string()),
                decisions: Some(vec!["refresh token은 한 번만 갱신한다.".to_string()]),
                files: Some(vec![
                    ContextFileInput::Path("src/auth.ts".to_string()),
                    ContextFileInput::Detail {
                        path: "test/auth.test.ts".to_string(),
                        description: Some("동시 요청 검증".to_string()),
                    },
                ]),
                verification: Some(ContextVerificationInput {
                    completed: Some(vec!["기존 테스트 통과".to_string()]),
                    pending: Some(vec!["동시 요청 테스트".to_string()]),
                }),
                next_steps: Some(vec!["기본 성공 경로 테스트 작성".to_string()]),
                risks: Some(vec!["중복 refresh 요청".to_string()]),
                git: Some(ContextGitInput {
                    repository: Some("jajak-front".to_string()),
                    branch: Some("feature/auth".to_string()),
                    commit: Some("abc1234".to_string()),
                    checked_at: Some(CREATED_AT.to_string()),
                }),
                ..WorkContextInput::default()
            }),
        }
    }

    fn node_golden(input: &WorkItemCreateInput) -> Value {
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let script = r#"
import { normalizeContextState, normalizeWorkItem, renderContext } from "./src/work-items.js";
const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const envelope = JSON.parse(Buffer.concat(chunks).toString("utf8"));
const workItem = normalizeWorkItem(envelope.input, envelope.now);
const context = normalizeContextState(envelope.input.context, workItem);
process.stdout.write(JSON.stringify({
  work_item_json: `${JSON.stringify(workItem, null, 2)}\n`,
  context_json: `${JSON.stringify(context, null, 2)}\n`,
  context_markdown: renderContext(workItem, context),
}));
"#;
        let mut child = Command::new("node")
            .args(["--input-type=module", "--eval", script])
            .current_dir(repository_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required for the cross-runtime compatibility test");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                serde_json::to_string(&json!({ "input": input, "now": CREATED_AT }))
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "Node golden failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn assert_node_compatible(input: WorkItemCreateInput) {
        let golden = node_golden(&input);
        let (work_item, context) = normalize_work_item(input, CREATED_AT).unwrap();
        assert_eq!(
            String::from_utf8(json_bytes("work-item.json", &work_item).unwrap()).unwrap(),
            golden["work_item_json"].as_str().unwrap()
        );
        assert_eq!(
            String::from_utf8(json_bytes("context.json", &context).unwrap()).unwrap(),
            golden["context_json"].as_str().unwrap()
        );
        assert_eq!(
            render_context(&work_item, &context),
            golden["context_markdown"].as_str().unwrap()
        );
    }

    #[test]
    fn rust_normalization_and_rendering_match_node_bytes() {
        assert_node_compatible(input("AUTH-142"));

        let mut defaults_and_completion = input("DOC-7");
        defaults_and_completion.status = Some("completed".to_string());
        defaults_and_completion.desired_outcomes = None;
        defaults_and_completion.classification = None;
        defaults_and_completion.repositories = None;
        defaults_and_completion.links = None;
        defaults_and_completion.completed_at = None;
        defaults_and_completion.context = Some(WorkContextInput {
            verification_completed: Some(vec!["문서 링크 확인".to_string()]),
            verification_pending: Some(vec!["동료 검토".to_string()]),
            files: Some(vec![ContextFileInput::Path("docs/release.md".to_string())]),
            ..WorkContextInput::default()
        });
        assert_node_compatible(defaults_and_completion);
    }

    #[test]
    fn creates_and_reads_one_consistent_edit_snapshot() {
        let directory = tempdir().unwrap();

        let created = create_work_item(directory.path(), input("AUTH-142"), CREATED_AT).unwrap();
        let snapshot = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();

        assert_eq!(created.work_item, snapshot.work_item);
        assert_eq!(created.context, snapshot.context);
        assert_eq!(created.revisions, snapshot.revisions);
        assert_eq!(created.commit.written_paths.len(), 3);
        assert_eq!(
            snapshot.markdown,
            render_context(&snapshot.work_item, &snapshot.context)
        );
        assert!(
            crate::inspect_data_root(directory.path())
                .unwrap()
                .issues
                .is_empty()
        );
    }

    #[test]
    fn duplicate_create_preserves_the_complete_existing_triplet() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), input("AUTH-142"), CREATED_AT).unwrap();
        let before = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();

        let error = create_work_item(directory.path(), input("AUTH-142"), UPDATED_AT).unwrap_err();

        assert!(matches!(
            error,
            WorkItemWriteError::Write(WriteError::CreateConflict(_))
        ));
        let after = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();
        assert_eq!(before, after);
        assert_eq!(created.revisions, after.revisions);
    }

    #[test]
    fn updates_all_derived_assets_with_one_revision_set() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), input("AUTH-142"), CREATED_AT).unwrap();
        let immutable_project = created.work_item.project_id.clone();
        let immutable_created_at = created.work_item.created_at.clone();

        let updated = update_work_item(
            directory.path(),
            "AUTH-142",
            created.revisions.clone(),
            WorkItemUpdatePatch {
                title: Some("인증 재시도 완료".to_string()),
                status: Some("completed".to_string()),
                context: Some(WorkContextPatch {
                    current_state: Some("동시 요청 검증까지 완료했다.".to_string()),
                    verification: Some(ContextVerificationInput {
                        completed: Some(vec!["동시 요청 테스트 통과".to_string()]),
                        pending: Some(Vec::new()),
                    }),
                    ..WorkContextPatch::default()
                }),
                ..WorkItemUpdatePatch::default()
            },
            UPDATED_AT,
        )
        .unwrap();

        assert_eq!(updated.work_item.project_id, immutable_project);
        assert_eq!(updated.work_item.created_at, immutable_created_at);
        assert_eq!(updated.work_item.updated_at, UPDATED_AT);
        assert_eq!(updated.work_item.completed_at.as_deref(), Some(UPDATED_AT));
        assert_eq!(updated.context.updated_at, UPDATED_AT);
        assert_ne!(updated.revisions.work_item, created.revisions.work_item);
        assert_ne!(
            updated.revisions.context_data,
            created.revisions.context_data
        );
        assert_ne!(updated.revisions.context, created.revisions.context);
        let snapshot = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();
        assert_eq!(snapshot.work_item, updated.work_item);
        assert!(snapshot.markdown.contains("# AUTH-142 인증 재시도 완료"));
        assert!(
            crate::inspect_data_root(directory.path())
                .unwrap()
                .issues
                .is_empty()
        );
    }

    #[test]
    fn stale_context_revision_rejects_the_whole_update() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), input("AUTH-142"), CREATED_AT).unwrap();
        let work_item_before = fs::read(directory.path().join(&created.paths.work_item)).unwrap();
        let context_data_before =
            fs::read(directory.path().join(&created.paths.context_data)).unwrap();
        fs::write(
            directory.path().join(&created.paths.context),
            "external editor change\n",
        )
        .unwrap();

        let error = update_work_item(
            directory.path(),
            "AUTH-142",
            created.revisions,
            WorkItemUpdatePatch {
                title: Some("덮어쓰면 안 되는 제목".to_string()),
                ..WorkItemUpdatePatch::default()
            },
            UPDATED_AT,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            WorkItemWriteError::Write(WriteError::RevisionConflict { .. })
        ));
        assert_eq!(
            fs::read(directory.path().join("work-items/AUTH-142/work-item.json")).unwrap(),
            work_item_before
        );
        assert_eq!(
            fs::read(directory.path().join("work-items/AUTH-142/context.json")).unwrap(),
            context_data_before
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("work-items/AUTH-142/context.md")).unwrap(),
            "external editor change\n"
        );
    }

    #[test]
    fn invalid_patch_is_rejected_before_any_file_is_replaced() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), input("AUTH-142"), CREATED_AT).unwrap();
        let before = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();

        let error = update_work_item(
            directory.path(),
            "AUTH-142",
            created.revisions,
            WorkItemUpdatePatch {
                status: Some("unknown".to_string()),
                ..WorkItemUpdatePatch::default()
            },
            UPDATED_AT,
        )
        .unwrap_err();

        assert!(matches!(error, WorkItemWriteError::Validation { .. }));
        let after = read_work_item_for_edit(directory.path(), "AUTH-142").unwrap();
        assert_eq!(before, after);
    }
}
