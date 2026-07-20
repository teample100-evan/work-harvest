use crate::schema::{self, DocumentKind};
use crate::work_items::{
    ContextFileInput, ContextVerificationInput, WorkContextDocument, WorkItemChangeOperation,
    WorkItemDocument, WorkItemEditRevisions, WorkItemFileChange, WorkItemWriteError, json_text,
    normalize_files, paths as work_item_paths, read_snapshot_from_root, render_context, revisions,
    validate_documents, verify_revision,
};
use crate::write::hash_bytes;
use crate::{DataRootWriter, IssueSeverity, WriteCommit, WriteError, WriteOperation};
use chrono::{DateTime, Datelike, Timelike};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

static CHECKPOINT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum CheckpointWriteError {
    #[error("Invalid checkpoint input: {0}")]
    InvalidInput(String),
    #[error("Could not inspect checkpoint relationships: {0}")]
    Inspect(String),
    #[error("Could not read checkpoint asset {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not parse checkpoint asset {path}: {source}")]
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
    #[error("Checkpoint assets are inconsistent: {0}")]
    Inconsistent(String),
    #[error(transparent)]
    WorkItem(#[from] WorkItemWriteError),
    #[error(transparent)]
    Write(#[from] WriteError),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointSourceInput {
    pub agent: Option<String>,
    pub surface: Option<String>,
    pub session_ref: Option<String>,
    pub task_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointSourceDocument {
    pub agent: String,
    pub surface: String,
    pub session_ref: Option<String>,
    pub task_title: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CheckpointWorkPeriodInput {
    pub start: Option<Value>,
    pub end: Option<Value>,
    pub precision: Option<String>,
    pub basis: Option<Vec<String>>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckpointWorkPeriodDocument {
    pub start: Value,
    pub end: Value,
    pub precision: String,
    pub basis: Vec<String>,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointDecisionDocument {
    pub summary: String,
    pub rationale: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointVerificationDocument {
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    pub status: String,
    pub command: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointOutcomeDocument {
    pub description: String,
    pub impact: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointEvidenceInput {
    pub commits: Option<Vec<String>>,
    pub pull_requests: Option<Vec<String>>,
    pub issues: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub commands: Option<Vec<String>>,
    pub urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointEvidenceDocument {
    pub commits: Vec<String>,
    pub pull_requests: Vec<String>,
    pub issues: Vec<String>,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointGitDocument {
    pub repository: String,
    pub branch: Option<String>,
    pub head_before: Option<String>,
    pub head_after: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CheckpointContextGitUpdate {
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub repository: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub branch: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub commit: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub checked_at: Option<Option<String>>,
}

fn deserialize_nullable_string<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CheckpointContextUpdate {
    pub current_state: Option<String>,
    pub decisions: Option<Vec<String>>,
    pub files: Option<Vec<ContextFileInput>>,
    pub verification: Option<ContextVerificationInput>,
    pub verification_completed: Option<Vec<String>>,
    pub verification_pending: Option<Vec<String>>,
    pub next_steps: Option<Vec<String>>,
    pub risks: Option<Vec<String>>,
    pub git: Option<CheckpointContextGitUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckpointInput {
    pub id: Option<String>,
    pub work_item_id: String,
    pub kind: Option<String>,
    pub source: Option<CheckpointSourceInput>,
    pub captured_at: Option<String>,
    pub work_period: Option<CheckpointWorkPeriodInput>,
    pub title: String,
    pub summary: String,
    pub status_after: Option<String>,
    pub activities: Option<Vec<String>>,
    pub decisions: Option<Vec<CheckpointDecisionDocument>>,
    pub verifications: Option<Vec<CheckpointVerificationDocument>>,
    pub outcomes: Option<Vec<CheckpointOutcomeDocument>>,
    pub blockers: Option<Vec<String>>,
    pub next_steps: Option<Vec<String>>,
    pub evidence: Option<CheckpointEvidenceInput>,
    pub git: Option<CheckpointGitDocument>,
    pub related_checkpoint_ids: Option<Vec<String>>,
    pub correction_of: Option<String>,
    pub confidentiality: Option<String>,
    pub context_update: Option<CheckpointContextUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckpointDocument {
    pub schema_version: String,
    pub id: String,
    pub work_item_id: String,
    pub project_id: String,
    pub kind: String,
    pub source: CheckpointSourceDocument,
    pub captured_at: String,
    pub work_period: CheckpointWorkPeriodDocument,
    pub title: String,
    pub summary: String,
    pub status_after: String,
    pub activities: Vec<String>,
    pub decisions: Vec<CheckpointDecisionDocument>,
    pub verifications: Vec<CheckpointVerificationDocument>,
    pub outcomes: Vec<CheckpointOutcomeDocument>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub evidence: CheckpointEvidenceDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<CheckpointGitDocument>,
    pub related_checkpoint_ids: Vec<String>,
    pub correction_of: Option<String>,
    pub confidentiality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointPaths {
    pub checkpoint: String,
    pub checkpoint_markdown: String,
    pub work_item: String,
    pub context_data: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CheckpointWritePreview {
    pub checkpoint: CheckpointDocument,
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub paths: CheckpointPaths,
    pub files: Vec<WorkItemFileChange>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CheckpointWriteResult {
    pub checkpoint: CheckpointDocument,
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub paths: CheckpointPaths,
    pub revisions: WorkItemEditRevisions,
    pub commit: WriteCommit,
}

fn checkpoint_calendar(
    captured_at: &str,
    timezone: &str,
) -> Result<DateTime<Tz>, CheckpointWriteError> {
    let captured = DateTime::parse_from_rfc3339(captured_at).map_err(|error| {
        CheckpointWriteError::InvalidInput(format!("captured_at is not RFC 3339: {error}"))
    })?;
    let timezone = timezone.parse::<Tz>().map_err(|error| {
        CheckpointWriteError::InvalidInput(format!("timezone is not supported: {error}"))
    })?;
    Ok(captured.with_timezone(&timezone))
}

fn generated_checkpoint_id(calendar: &DateTime<Tz>) -> String {
    let sequence = CHECKPOINT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seed = format!("{}:{nanos}:{sequence}", std::process::id());
    let suffix = &hash_bytes(seed.as_bytes())[..6];
    format!(
        "CP-{:04}{:02}{:02}-{:02}{:02}{:02}-{suffix}",
        calendar.year(),
        calendar.month(),
        calendar.day(),
        calendar.hour(),
        calendar.minute(),
        calendar.second()
    )
}

pub fn normalize_checkpoint(
    input: CheckpointInput,
    work_item: &WorkItemDocument,
    now: &str,
) -> Result<(CheckpointDocument, Option<CheckpointContextUpdate>), CheckpointWriteError> {
    if input.work_item_id != work_item.id {
        return Err(CheckpointWriteError::InvalidInput(format!(
            "work_item_id must be {}",
            work_item.id
        )));
    }
    if input.title.is_empty() || input.summary.is_empty() {
        return Err(CheckpointWriteError::InvalidInput(
            "title and summary are required".to_string(),
        ));
    }

    let captured_at = input.captured_at.unwrap_or_else(|| now.to_string());
    let period = input.work_period.unwrap_or_default();
    let timezone = period.timezone.unwrap_or_else(|| "Asia/Seoul".to_string());
    let calendar = checkpoint_calendar(&captured_at, &timezone)?;
    let work_date = format!(
        "{:04}-{:02}-{:02}",
        calendar.year(),
        calendar.month(),
        calendar.day()
    );
    let kind = input.kind.unwrap_or_else(|| "progress".to_string());
    let source = input.source.unwrap_or_default();
    let evidence = input.evidence.unwrap_or_default();
    let checkpoint = CheckpointDocument {
        schema_version: "1.0".to_string(),
        id: input
            .id
            .unwrap_or_else(|| generated_checkpoint_id(&calendar)),
        work_item_id: work_item.id.clone(),
        project_id: work_item.project_id.clone(),
        kind: kind.clone(),
        source: CheckpointSourceDocument {
            agent: source.agent.unwrap_or_else(|| "manual".to_string()),
            surface: source.surface.unwrap_or_else(|| "unknown".to_string()),
            session_ref: source.session_ref,
            task_title: source.task_title,
        },
        captured_at,
        work_period: CheckpointWorkPeriodDocument {
            start: period
                .start
                .unwrap_or_else(|| Value::String(work_date.clone())),
            end: period.end.unwrap_or_else(|| Value::String(work_date)),
            precision: period.precision.unwrap_or_else(|| "day".to_string()),
            basis: period
                .basis
                .unwrap_or_else(|| vec!["checkpoint".to_string()]),
            timezone,
        },
        title: input.title,
        summary: input.summary,
        status_after: input.status_after.unwrap_or_else(|| {
            if kind == "final" {
                "completed".to_string()
            } else {
                "in_progress".to_string()
            }
        }),
        activities: input.activities.unwrap_or_default(),
        decisions: input.decisions.unwrap_or_default(),
        verifications: input.verifications.unwrap_or_default(),
        outcomes: input.outcomes.unwrap_or_default(),
        blockers: input.blockers.unwrap_or_default(),
        next_steps: input.next_steps.unwrap_or_default(),
        evidence: CheckpointEvidenceDocument {
            commits: evidence.commits.unwrap_or_default(),
            pull_requests: evidence.pull_requests.unwrap_or_default(),
            issues: evidence.issues.unwrap_or_default(),
            files: evidence.files.unwrap_or_default(),
            commands: evidence.commands.unwrap_or_default(),
            urls: evidence.urls.unwrap_or_default(),
        },
        git: input.git,
        related_checkpoint_ids: input.related_checkpoint_ids.unwrap_or_default(),
        correction_of: input.correction_of,
        confidentiality: input
            .confidentiality
            .unwrap_or_else(|| "normal".to_string()),
    };

    validate_checkpoint(&checkpoint)?;
    Ok((checkpoint, input.context_update))
}

pub(crate) fn validate_checkpoint(
    checkpoint: &CheckpointDocument,
) -> Result<(), CheckpointWriteError> {
    let value =
        serde_json::to_value(checkpoint).map_err(|source| CheckpointWriteError::Serialize {
            document: "checkpoint.json",
            source,
        })?;
    let violations = schema::validate(DocumentKind::Checkpoint, &value).map_err(|details| {
        CheckpointWriteError::Validation {
            document: "Checkpoint",
            details,
        }
    })?;
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
    Err(CheckpointWriteError::Validation {
        document: "Checkpoint",
        details,
    })
}

fn updated_work_item(
    mut work_item: WorkItemDocument,
    checkpoint: &CheckpointDocument,
) -> WorkItemDocument {
    work_item.status = checkpoint.status_after.clone();
    work_item.updated_at = checkpoint.captured_at.clone();
    work_item.completed_at = if checkpoint.status_after == "completed" {
        Some(checkpoint.captured_at.clone())
    } else {
        None
    };
    work_item
}

fn merged_context(
    mut context: WorkContextDocument,
    update: Option<CheckpointContextUpdate>,
    checkpoint: &CheckpointDocument,
) -> WorkContextDocument {
    context.updated_at = checkpoint.captured_at.clone();
    context.last_checkpoint_id = Some(checkpoint.id.clone());
    if let Some(git) = checkpoint.git.as_ref() {
        if git.head_after.is_some() {
            context.last_verified_git_ref = git.head_after.clone();
        }
        context.git.repository = Some(git.repository.clone());
        context.git.branch = git.branch.clone();
        context.git.commit = git.head_after.clone();
        context.git.checked_at = Some(checkpoint.captured_at.clone());
    }

    let Some(update) = update else {
        return context;
    };
    if let Some(value) = update.current_state {
        context.current_state = value;
    }
    if let Some(value) = update.decisions {
        context.decisions = value;
    }
    if let Some(value) = update.files {
        context.files = normalize_files(Some(value));
    }
    if let Some(value) = update.verification_completed {
        context.verification.completed = value;
    }
    if let Some(value) = update.verification_pending {
        context.verification.pending = value;
    }
    if let Some(verification) = update.verification {
        if let Some(value) = verification.completed {
            context.verification.completed = value;
        }
        if let Some(value) = verification.pending {
            context.verification.pending = value;
        }
    }
    if let Some(value) = update.next_steps {
        context.next_steps = value;
    }
    if let Some(value) = update.risks {
        context.risks = value;
    }
    if let Some(git) = update.git {
        if let Some(value) = git.repository {
            context.git.repository = value;
        }
        if let Some(value) = git.branch {
            context.git.branch = value;
        }
        if let Some(value) = git.commit {
            context.git.commit = value;
        }
        if let Some(value) = git.checked_at {
            context.git.checked_at = value;
        }
    }
    context
}

fn yaml_scalar(value: &str, force_quotes: bool) -> String {
    let lower = value.to_ascii_lowercase();
    let looks_typed =
        matches!(lower.as_str(), "null" | "true" | "false" | "~") || value.parse::<f64>().is_ok();
    let unsafe_plain = value.is_empty()
        || value.trim() != value
        || value.contains('\n')
        || value.contains(": ")
        || value.contains(" #")
        || value.starts_with([
            '-', '?', ':', '!', '&', '*', '#', '{', '}', '[', ']', ',', '|', '>', '@', '`',
        ]);
    if force_quotes || looks_typed || unsafe_plain {
        serde_json::to_string(value).expect("serializing a string cannot fail")
    } else {
        value.to_string()
    }
}

fn yaml_optional(value: Option<&str>) -> String {
    value
        .map(|value| yaml_scalar(value, false))
        .unwrap_or_else(|| "null".to_string())
}

fn yaml_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::String(value) => yaml_scalar(value, false),
        other => other.to_string(),
    }
}

fn yaml_list(label: &str, values: &[String]) -> String {
    if values.is_empty() {
        return format!("{label}: []");
    }
    format!(
        "{label}:\n{}",
        values
            .iter()
            .map(|value| format!("  - {}", yaml_scalar(value, false)))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn markdown_list(values: &[String], fallback: &str) -> String {
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

pub fn render_checkpoint(checkpoint: &CheckpointDocument) -> String {
    let basis = if checkpoint.work_period.basis.is_empty() {
        "  basis: []".to_string()
    } else {
        format!(
            "  basis:\n{}",
            checkpoint
                .work_period
                .basis
                .iter()
                .map(|value| format!("    - {}", yaml_scalar(value, false)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let decisions = if checkpoint.decisions.is_empty() {
        "- 없음".to_string()
    } else {
        checkpoint
            .decisions
            .iter()
            .map(|value| {
                format!(
                    "- {}\n  - 이유: {}\n  - 상태: {}",
                    value.summary, value.rationale, value.status
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let verifications = if checkpoint.verifications.is_empty() {
        "- 없음".to_string()
    } else {
        checkpoint
            .verifications
            .iter()
            .map(|value| {
                format!(
                    "- {}\n  - 유형: {}\n  - 상태: {}\n  - 명령: {}\n  - 근거: {}",
                    value.description,
                    value.kind,
                    value.status,
                    value
                        .command
                        .as_deref()
                        .map(|command| format!("`{command}`"))
                        .unwrap_or_else(|| "없음".to_string()),
                    if value.evidence_refs.is_empty() {
                        "없음".to_string()
                    } else {
                        value.evidence_refs.join(", ")
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let outcomes = if checkpoint.outcomes.is_empty() {
        "- 없음".to_string()
    } else {
        checkpoint
            .outcomes
            .iter()
            .map(|value| {
                format!(
                    "- {}\n  - 영향: {}\n  - 근거: {}",
                    value.description,
                    value.impact.as_deref().unwrap_or("확인되지 않음"),
                    if value.evidence_refs.is_empty() {
                        "없음".to_string()
                    } else {
                        value.evidence_refs.join(", ")
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let evidence_line = |label: &str, values: &[String]| {
        format!(
            "- {label}: {}",
            if values.is_empty() {
                "없음".to_string()
            } else {
                values.join(", ")
            }
        )
    };

    let frontmatter = vec![
        "---".to_string(),
        "schema_version: \"1.0\"".to_string(),
        format!("id: {}", yaml_scalar(&checkpoint.id, false)),
        format!(
            "work_item_id: {}",
            yaml_scalar(&checkpoint.work_item_id, false)
        ),
        format!("project_id: {}", yaml_scalar(&checkpoint.project_id, false)),
        format!("kind: {}", yaml_scalar(&checkpoint.kind, false)),
        "source:".to_string(),
        format!("  agent: {}", yaml_scalar(&checkpoint.source.agent, false)),
        format!(
            "  surface: {}",
            yaml_scalar(&checkpoint.source.surface, false)
        ),
        format!(
            "  session_ref: {}",
            yaml_optional(checkpoint.source.session_ref.as_deref())
        ),
        format!(
            "  task_title: {}",
            yaml_optional(checkpoint.source.task_title.as_deref())
        ),
        format!(
            "captured_at: {}",
            yaml_scalar(&checkpoint.captured_at, false)
        ),
        "work_period:".to_string(),
        format!("  start: {}", yaml_value(&checkpoint.work_period.start)),
        format!("  end: {}", yaml_value(&checkpoint.work_period.end)),
        format!(
            "  precision: {}",
            yaml_scalar(&checkpoint.work_period.precision, false)
        ),
        basis,
        format!(
            "  timezone: {}",
            yaml_scalar(&checkpoint.work_period.timezone, false)
        ),
        format!(
            "status_after: {}",
            yaml_scalar(&checkpoint.status_after, false)
        ),
        yaml_list("related_checkpoint_ids", &checkpoint.related_checkpoint_ids),
        format!(
            "correction_of: {}",
            yaml_optional(checkpoint.correction_of.as_deref())
        ),
        format!(
            "confidentiality: {}",
            yaml_scalar(&checkpoint.confidentiality, false)
        ),
        "---".to_string(),
    ]
    .join("\n");
    let evidence = [
        evidence_line("커밋", &checkpoint.evidence.commits),
        evidence_line("PR", &checkpoint.evidence.pull_requests),
        evidence_line("이슈", &checkpoint.evidence.issues),
        evidence_line("파일", &checkpoint.evidence.files),
        evidence_line("명령", &checkpoint.evidence.commands),
        evidence_line("URL", &checkpoint.evidence.urls),
    ]
    .join("\n");

    format!(
        "{frontmatter}\n\n# {}\n\n## 요약\n\n{}\n\n## 진행한 작업\n\n{}\n\n## 결정 및 이유\n\n{}\n\n## 검증\n\n{}\n\n## 결과와 영향\n\n{}\n\n## 차단 요소\n\n{}\n\n## 다음 작업\n\n{}\n\n## 근거\n\n{evidence}\n",
        checkpoint.title,
        checkpoint.summary,
        markdown_list(&checkpoint.activities, "없음"),
        decisions,
        verifications,
        outcomes,
        markdown_list(&checkpoint.blockers, "없음"),
        markdown_list(&checkpoint.next_steps, "없음"),
    )
}

fn checkpoint_paths(
    checkpoint: &CheckpointDocument,
) -> Result<CheckpointPaths, CheckpointWriteError> {
    let calendar = checkpoint_calendar(&checkpoint.captured_at, &checkpoint.work_period.timezone)?;
    let directory = format!(
        "records/{:04}/{:02}/{:02}",
        calendar.year(),
        calendar.month(),
        calendar.day()
    );
    let work_paths = work_item_paths(&checkpoint.work_item_id);
    Ok(CheckpointPaths {
        checkpoint: format!("{directory}/{}.json", checkpoint.id),
        checkpoint_markdown: format!("{directory}/{}.md", checkpoint.id),
        work_item: work_paths.work_item,
        context_data: work_paths.context_data,
        context: work_paths.context,
    })
}

struct PreparedCapture {
    checkpoint: CheckpointDocument,
    work_item: WorkItemDocument,
    context: WorkContextDocument,
    paths: CheckpointPaths,
    files: Vec<WorkItemFileChange>,
    expected: WorkItemEditRevisions,
}

fn prepare_capture(
    writer: &DataRootWriter,
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: &str,
) -> Result<PreparedCapture, CheckpointWriteError> {
    let work_paths = work_item_paths(&input.work_item_id);
    verify_revision(writer, &work_paths.work_item, &expected.work_item)?;
    verify_revision(writer, &work_paths.context_data, &expected.context_data)?;
    verify_revision(writer, &work_paths.context, &expected.context)?;
    let snapshot = read_snapshot_from_root(writer.root(), &input.work_item_id)?;
    let (checkpoint, context_update) = normalize_checkpoint(input, &snapshot.work_item, now)?;
    let paths = checkpoint_paths(&checkpoint)?;
    for path in [&paths.checkpoint, &paths.checkpoint_markdown] {
        if writer.revision(path)?.is_some() {
            return Err(WriteError::CreateConflict(path.clone()).into());
        }
    }

    let known = crate::inspect_data_root(writer.root())
        .map_err(|error| CheckpointWriteError::Inspect(error.to_string()))?
        .checkpoint_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    for related in &checkpoint.related_checkpoint_ids {
        if !known.contains(related) {
            return Err(CheckpointWriteError::InvalidInput(format!(
                "related checkpoint was not found: {related}"
            )));
        }
    }
    if let Some(correction) = checkpoint.correction_of.as_ref() {
        if !known.contains(correction) {
            return Err(CheckpointWriteError::InvalidInput(format!(
                "correction checkpoint was not found: {correction}"
            )));
        }
    }

    let work_item = updated_work_item(snapshot.work_item, &checkpoint);
    let context = merged_context(snapshot.context, context_update, &checkpoint);
    validate_documents(&work_item, &context)?;
    let files = vec![
        WorkItemFileChange {
            path: paths.checkpoint.clone(),
            operation: WorkItemChangeOperation::Create,
            before: None,
            after: checkpoint_json(&checkpoint)?,
        },
        WorkItemFileChange {
            path: paths.checkpoint_markdown.clone(),
            operation: WorkItemChangeOperation::Create,
            before: None,
            after: render_checkpoint(&checkpoint),
        },
        WorkItemFileChange {
            path: paths.work_item.clone(),
            operation: WorkItemChangeOperation::Replace,
            before: Some(snapshot.work_item_json),
            after: json_text("work-item.json", &work_item)?,
        },
        WorkItemFileChange {
            path: paths.context_data.clone(),
            operation: WorkItemChangeOperation::Replace,
            before: Some(snapshot.context_json),
            after: json_text("context.json", &context)?,
        },
        WorkItemFileChange {
            path: paths.context.clone(),
            operation: WorkItemChangeOperation::Replace,
            before: Some(snapshot.markdown),
            after: render_context(&work_item, &context),
        },
    ];
    Ok(PreparedCapture {
        checkpoint,
        work_item,
        context,
        paths,
        files,
        expected,
    })
}

fn checkpoint_json(checkpoint: &CheckpointDocument) -> Result<String, CheckpointWriteError> {
    let mut bytes = serde_json::to_vec_pretty(checkpoint).map_err(|source| {
        CheckpointWriteError::Serialize {
            document: "checkpoint.json",
            source,
        }
    })?;
    bytes.push(b'\n');
    String::from_utf8(bytes).map_err(|_| {
        CheckpointWriteError::Inconsistent("serialized checkpoint is not UTF-8".to_string())
    })
}

pub fn preview_capture_checkpoint(
    root: impl AsRef<Path>,
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: &str,
) -> Result<CheckpointWritePreview, CheckpointWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_capture(&writer, input, expected, now)?;
    Ok(CheckpointWritePreview {
        checkpoint: prepared.checkpoint,
        work_item: prepared.work_item,
        context: prepared.context,
        paths: prepared.paths,
        files: prepared.files,
    })
}

fn validate_written_capture(root: &Path, paths: &CheckpointPaths) -> Result<(), String> {
    read_snapshot_from_root(root, &paths.work_item_id()).map_err(|error| error.to_string())?;
    let checkpoint_bytes =
        fs::read(root.join(&paths.checkpoint)).map_err(|error| error.to_string())?;
    let checkpoint: CheckpointDocument =
        serde_json::from_slice(&checkpoint_bytes).map_err(|error| error.to_string())?;
    validate_checkpoint(&checkpoint).map_err(|error| error.to_string())?;
    let markdown = fs::read_to_string(root.join(&paths.checkpoint_markdown))
        .map_err(|error| error.to_string())?;
    if markdown != render_checkpoint(&checkpoint) {
        return Err("checkpoint Markdown does not match its structured source".to_string());
    }
    let relevant = [
        &paths.checkpoint,
        &paths.checkpoint_markdown,
        &paths.work_item,
        &paths.context_data,
        &paths.context,
    ];
    let issues = crate::inspect_data_root(root)
        .map_err(|error| error.to_string())?
        .issues
        .into_iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .filter(|issue| relevant.contains(&&issue.path))
        .map(|issue| format!("{} [{}]: {}", issue.path, issue.code, issue.message))
        .collect::<Vec<_>>();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("; "))
    }
}

impl CheckpointPaths {
    fn work_item_id(&self) -> String {
        self.work_item
            .strip_prefix("work-items/")
            .and_then(|value| value.strip_suffix("/work-item.json"))
            .unwrap_or_default()
            .to_string()
    }
}

pub fn capture_checkpoint(
    root: impl AsRef<Path>,
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: &str,
) -> Result<CheckpointWriteResult, CheckpointWriteError> {
    let mut writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_capture(&writer, input, expected, now)?;
    let operations = prepared
        .files
        .iter()
        .map(|file| match file.operation {
            WorkItemChangeOperation::Create => {
                WriteOperation::create(PathBuf::from(&file.path), file.after.as_bytes().to_vec())
            }
            WorkItemChangeOperation::Replace => {
                let revision = if file.path == prepared.paths.work_item {
                    &prepared.expected.work_item
                } else if file.path == prepared.paths.context_data {
                    &prepared.expected.context_data
                } else {
                    &prepared.expected.context
                };
                WriteOperation::replace(
                    PathBuf::from(&file.path),
                    revision.sha256.clone(),
                    file.after.as_bytes().to_vec(),
                )
            }
        })
        .collect::<Vec<_>>();
    let validation_paths = prepared.paths.clone();
    let commit = writer.commit_validated(operations, move |root| {
        validate_written_capture(root, &validation_paths)
    })?;
    let item_paths = work_item_paths(&prepared.checkpoint.work_item_id);
    let next_revisions = revisions(&writer, &item_paths)?;
    Ok(CheckpointWriteResult {
        checkpoint: prepared.checkpoint,
        work_item: prepared.work_item,
        context: prepared.context,
        paths: prepared.paths,
        revisions: next_revisions,
        commit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WorkContextInput, WorkItemCreateInput, create_work_item, read_work_item_for_edit};
    use tempfile::tempdir;

    const CREATED_AT: &str = "2026-07-13T09:00:00+09:00";
    const CAPTURED_AT: &str = "2026-07-13T18:10:00+09:00";

    fn work_item_input() -> WorkItemCreateInput {
        WorkItemCreateInput {
            id: "AUTH-142".to_string(),
            project_id: "jajak-front".to_string(),
            title: "인증 개선".to_string(),
            status: Some("in_progress".to_string()),
            objective: "인증 갱신을 검증한다.".to_string(),
            desired_outcomes: None,
            classification: None,
            scope: Some("company".to_string()),
            reporting: None,
            repositories: None,
            links: None,
            context_path: None,
            created_at: Some(CREATED_AT.to_string()),
            updated_at: Some(CREATED_AT.to_string()),
            completed_at: None,
            context: Some(WorkContextInput {
                current_state: Some("기본 경로를 구현 중이다.".to_string()),
                next_steps: Some(vec!["테스트 작성".to_string()]),
                ..WorkContextInput::default()
            }),
        }
    }

    fn checkpoint_input(kind: &str) -> CheckpointInput {
        CheckpointInput {
            id: Some(if kind == "final" {
                "CP-20260713-999".to_string()
            } else {
                "CP-20260713-001".to_string()
            }),
            work_item_id: "AUTH-142".to_string(),
            kind: Some(kind.to_string()),
            source: Some(CheckpointSourceInput {
                agent: Some("codex".to_string()),
                surface: Some("desktop".to_string()),
                session_ref: Some("session-test".to_string()),
                task_title: Some("인증 테스트 코드 작성".to_string()),
            }),
            captured_at: Some(CAPTURED_AT.to_string()),
            work_period: None,
            title: "인증 테스트 진행".to_string(),
            summary: "인증 갱신 성공 경로를 검증했다.".to_string(),
            status_after: None,
            activities: Some(vec!["refresh token 테스트를 추가했다.".to_string()]),
            decisions: None,
            verifications: Some(vec![CheckpointVerificationDocument {
                kind: "test".to_string(),
                description: "인증 테스트".to_string(),
                status: "passed".to_string(),
                command: Some("pnpm test auth".to_string()),
                evidence_refs: vec!["tests/auth.test.ts".to_string()],
            }]),
            outcomes: if kind == "final" {
                Some(vec![CheckpointOutcomeDocument {
                    description: "테스트 스위트를 완료했다.".to_string(),
                    impact: None,
                    evidence_refs: Vec::new(),
                }])
            } else {
                None
            },
            blockers: None,
            next_steps: Some(vec!["동시 요청 테스트".to_string()]),
            evidence: Some(CheckpointEvidenceInput {
                files: Some(vec!["tests/auth.test.ts".to_string()]),
                commands: Some(vec!["pnpm test auth".to_string()]),
                ..CheckpointEvidenceInput::default()
            }),
            git: None,
            related_checkpoint_ids: None,
            correction_of: None,
            confidentiality: None,
            context_update: Some(CheckpointContextUpdate {
                current_state: Some("기본 성공 경로를 검증했다.".to_string()),
                verification: Some(ContextVerificationInput {
                    completed: Some(vec!["인증 테스트 통과".to_string()]),
                    pending: Some(vec!["동시 요청 테스트".to_string()]),
                }),
                next_steps: Some(vec!["동시 요청 테스트".to_string()]),
                ..CheckpointContextUpdate::default()
            }),
        }
    }

    #[test]
    fn rust_renderer_matches_the_canonical_checkpoint_markdown() {
        let checkpoint: CheckpointDocument = serde_json::from_str(include_str!(
            "../../../examples/records/2026/07/13/CP-20260713-001.json"
        ))
        .unwrap();
        assert_eq!(
            render_checkpoint(&checkpoint),
            include_str!("../../../examples/records/2026/07/13/CP-20260713-001.md")
        );
    }

    #[test]
    fn preview_matches_the_five_committed_files() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), work_item_input(), CREATED_AT).unwrap();
        let preview = preview_capture_checkpoint(
            directory.path(),
            checkpoint_input("progress"),
            created.revisions.clone(),
            CAPTURED_AT,
        )
        .unwrap();
        assert_eq!(preview.files.len(), 5);
        assert_eq!(
            preview
                .files
                .iter()
                .filter(|file| file.before.is_none())
                .count(),
            2
        );
        assert!(!directory.path().join(&preview.paths.checkpoint).exists());

        let result = capture_checkpoint(
            directory.path(),
            checkpoint_input("progress"),
            created.revisions,
            CAPTURED_AT,
        )
        .unwrap();
        for file in &preview.files {
            assert_eq!(
                fs::read_to_string(directory.path().join(&file.path)).unwrap(),
                file.after
            );
        }
        assert_eq!(result.checkpoint, preview.checkpoint);
        assert_eq!(result.work_item, preview.work_item);
        assert_eq!(result.context, preview.context);
    }

    #[test]
    fn stale_context_revision_rejects_all_five_files() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), work_item_input(), CREATED_AT).unwrap();
        fs::write(
            directory.path().join(&created.paths.context),
            "external change\n",
        )
        .unwrap();

        let error = capture_checkpoint(
            directory.path(),
            checkpoint_input("progress"),
            created.revisions,
            CAPTURED_AT,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CheckpointWriteError::WorkItem(WorkItemWriteError::Write(
                WriteError::RevisionConflict { .. }
            ))
        ));
        assert!(!directory.path().join("records").exists());
    }

    #[test]
    fn final_checkpoint_completes_the_work_item() {
        let directory = tempdir().unwrap();
        let created = create_work_item(directory.path(), work_item_input(), CREATED_AT).unwrap();
        let result = capture_checkpoint(
            directory.path(),
            checkpoint_input("final"),
            created.revisions,
            CAPTURED_AT,
        )
        .unwrap();
        assert_eq!(result.work_item.status, "completed");
        assert_eq!(result.work_item.completed_at.as_deref(), Some(CAPTURED_AT));
        assert_eq!(
            result.context.last_checkpoint_id.as_deref(),
            Some("CP-20260713-999")
        );
        assert!(read_work_item_for_edit(directory.path(), "AUTH-142").is_ok());
    }

    #[test]
    fn context_update_preserves_node_null_and_verification_precedence() {
        let decoded: CheckpointContextUpdate = serde_json::from_value(serde_json::json!({
            "git": { "branch": null, "commit": "def5678" }
        }))
        .unwrap();
        let decoded_git = decoded.git.unwrap();
        assert_eq!(decoded_git.repository, None);
        assert_eq!(decoded_git.branch, Some(None));
        assert_eq!(decoded_git.commit, Some(Some("def5678".to_string())));

        let directory = tempdir().unwrap();
        let mut work_item = work_item_input();
        work_item.context.as_mut().unwrap().verification = Some(ContextVerificationInput {
            completed: Some(vec!["이전 검증".to_string()]),
            pending: Some(vec!["이전 대기".to_string()]),
        });
        work_item.context.as_mut().unwrap().git = Some(crate::work_items::ContextGitInput {
            repository: Some("work-harvest".to_string()),
            branch: Some("main".to_string()),
            commit: Some("abc1234".to_string()),
            checked_at: Some(CREATED_AT.to_string()),
        });
        let created = create_work_item(directory.path(), work_item, CREATED_AT).unwrap();
        let mut input = checkpoint_input("progress");
        input.context_update = Some(CheckpointContextUpdate {
            verification_completed: Some(vec!["legacy 완료".to_string()]),
            verification_pending: Some(vec!["legacy 대기".to_string()]),
            verification: Some(ContextVerificationInput {
                completed: Some(vec!["중첩 완료".to_string()]),
                pending: Some(vec!["중첩 대기".to_string()]),
            }),
            git: Some(CheckpointContextGitUpdate {
                branch: Some(None),
                commit: Some(None),
                ..CheckpointContextGitUpdate::default()
            }),
            ..CheckpointContextUpdate::default()
        });

        let result =
            capture_checkpoint(directory.path(), input, created.revisions, CAPTURED_AT).unwrap();
        assert_eq!(result.context.verification.completed, ["중첩 완료"]);
        assert_eq!(result.context.verification.pending, ["중첩 대기"]);
        assert_eq!(
            result.context.git.repository.as_deref(),
            Some("work-harvest")
        );
        assert_eq!(result.context.git.branch, None);
        assert_eq!(result.context.git.commit, None);
        assert_eq!(result.context.git.checked_at.as_deref(), Some(CREATED_AT));
    }
}
