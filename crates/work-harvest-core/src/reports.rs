use crate::checkpoints::{
    CheckpointDocument, CheckpointEvidenceDocument, CheckpointWriteError, validate_checkpoint,
};
use crate::work_items::{
    WorkContextDocument, WorkItemChangeOperation, WorkItemDocument, WorkItemFileChange,
    WorkItemPaths, WorkItemWriteError, paths as work_item_paths, validate_documents,
};
use crate::write::hash_bytes;
use crate::{
    DataRootWriter, FileRevision, WriteCommit, WriteError, WriteOperation, read_file_revision,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum PerformanceNoteWriteError {
    #[error("Invalid performance note input: {0}")]
    InvalidInput(String),
    #[error("Could not read performance note source {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not parse performance note source {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("Could not scan checkpoint sources: {0}")]
    Scan(String),
    #[error("Performance note sources are inconsistent: {0}")]
    Inconsistent(String),
    #[error(transparent)]
    WorkItem(#[from] WorkItemWriteError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointWriteError),
    #[error(transparent)]
    Write(#[from] WriteError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerformanceNoteInput {
    pub work_item_id: String,
    pub output: Option<String>,
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerformanceNoteSourceRevision {
    pub path: String,
    pub revision: FileRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerformanceNotePaths {
    pub report: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceNoteCheckpoint {
    pub checkpoint: CheckpointDocument,
    pub markdown_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PerformanceNoteWritePreview {
    pub work_item: WorkItemDocument,
    pub checkpoint_count: usize,
    pub redacted_checkpoint_count: usize,
    pub excluded_checkpoint_count: usize,
    pub paths: PerformanceNotePaths,
    pub source_revisions: Vec<PerformanceNoteSourceRevision>,
    pub files: Vec<WorkItemFileChange>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PerformanceNoteWriteResult {
    pub work_item: WorkItemDocument,
    pub checkpoint_count: usize,
    pub redacted_checkpoint_count: usize,
    pub excluded_checkpoint_count: usize,
    pub paths: PerformanceNotePaths,
    pub source_revisions: Vec<PerformanceNoteSourceRevision>,
    pub commit: WriteCommit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeeklyReportInput {
    pub start_date: String,
    pub end_date: String,
    pub output: Option<String>,
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeeklyReportStats {
    pub work_item_count: usize,
    pub checkpoint_count: usize,
    pub redacted_checkpoint_count: usize,
    pub excluded_checkpoint_count: usize,
    pub unknown_period_checkpoint_count: usize,
    pub git_commit_count: usize,
    pub verification_count: usize,
    pub passed_verification_count: usize,
    pub failed_verification_count: usize,
    pub partial_verification_count: usize,
    pub not_run_verification_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WeeklyReportWritePreview {
    pub start_date: String,
    pub end_date: String,
    pub stats: WeeklyReportStats,
    pub paths: PerformanceNotePaths,
    pub source_revisions: Vec<PerformanceNoteSourceRevision>,
    pub files: Vec<WorkItemFileChange>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WeeklyReportWriteResult {
    pub start_date: String,
    pub end_date: String,
    pub stats: WeeklyReportStats,
    pub paths: PerformanceNotePaths,
    pub source_revisions: Vec<PerformanceNoteSourceRevision>,
    pub commit: WriteCommit,
}

#[derive(Debug)]
struct StoredCheckpoint {
    checkpoint: CheckpointDocument,
    json_path: String,
    markdown_path: String,
    revision: FileRevision,
}

#[derive(Debug)]
struct PreparedPerformanceNote {
    work_item: WorkItemDocument,
    checkpoint_count: usize,
    redacted_checkpoint_count: usize,
    excluded_checkpoint_count: usize,
    paths: PerformanceNotePaths,
    source_revisions: Vec<PerformanceNoteSourceRevision>,
    markdown: String,
}

#[derive(Debug)]
struct PreparedWeeklyReport {
    start_date: String,
    end_date: String,
    stats: WeeklyReportStats,
    paths: PerformanceNotePaths,
    source_revisions: Vec<PerformanceNoteSourceRevision>,
    markdown: String,
}

#[derive(Debug)]
struct PerformanceNoteSources {
    work_item: WorkItemDocument,
    context: WorkContextDocument,
    paths: WorkItemPaths,
    work_item_revision: FileRevision,
    context_revision: FileRevision,
}

fn read_source_json<T: for<'de> Deserialize<'de>>(
    root: &Path,
    path: &str,
) -> Result<(T, FileRevision), PerformanceNoteWriteError> {
    let absolute = root.join(path);
    let bytes = fs::read(&absolute).map_err(|source| PerformanceNoteWriteError::Read {
        path: path.to_string(),
        source,
    })?;
    let document =
        serde_json::from_slice(&bytes).map_err(|source| PerformanceNoteWriteError::Parse {
            path: path.to_string(),
            source,
        })?;
    let revision = FileRevision {
        sha256: hash_bytes(&bytes),
        bytes: bytes.len() as u64,
    };
    Ok((document, revision))
}

fn read_performance_note_sources(
    root: &Path,
    work_item_id: &str,
) -> Result<PerformanceNoteSources, PerformanceNoteWriteError> {
    if !crate::is_identifier(work_item_id) {
        return Err(WorkItemWriteError::WorkItemNotFound(work_item_id.to_string()).into());
    }
    let paths = work_item_paths(work_item_id);
    if read_file_revision(root, &paths.work_item)?.is_none() {
        return Err(WorkItemWriteError::WorkItemNotFound(work_item_id.to_string()).into());
    }
    if read_file_revision(root, &paths.context_data)?.is_none() {
        return Err(PerformanceNoteWriteError::Inconsistent(format!(
            "structured context is missing: {}",
            paths.context_data
        )));
    }
    let (work_item, work_item_revision): (WorkItemDocument, _) =
        read_source_json(root, &paths.work_item)?;
    let (context, context_revision): (WorkContextDocument, _) =
        read_source_json(root, &paths.context_data)?;
    if work_item.id != work_item_id {
        return Err(WorkItemWriteError::WorkItemNotFound(work_item_id.to_string()).into());
    }
    validate_documents(&work_item, &context)?;
    if read_file_revision(root, &work_item.context_path)?.is_none() {
        return Err(PerformanceNoteWriteError::Inconsistent(format!(
            "context document is missing: {}",
            work_item.context_path
        )));
    }
    Ok(PerformanceNoteSources {
        work_item,
        context,
        paths,
        work_item_revision,
        context_revision,
    })
}

fn portable_path(root: &Path, path: &Path) -> Result<String, PerformanceNoteWriteError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        PerformanceNoteWriteError::Inconsistent(format!(
            "source path escapes the data root: {}",
            path.display()
        ))
    })?;
    relative
        .to_str()
        .map(|value| value.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| {
            PerformanceNoteWriteError::InvalidInput(format!(
                "source path is not UTF-8: {}",
                relative.display()
            ))
        })
}

fn load_checkpoints(root: &Path) -> Result<Vec<StoredCheckpoint>, PerformanceNoteWriteError> {
    let records = root.join("records");
    if !records.exists() {
        return Ok(Vec::new());
    }
    let mut checkpoints = Vec::new();
    for entry in WalkDir::new(&records) {
        let entry = entry.map_err(|error| PerformanceNoteWriteError::Scan(error.to_string()))?;
        if !entry.file_type().is_file() || entry.path().extension() != Some(OsStr::new("json")) {
            continue;
        }
        let path = entry.path();
        let bytes = fs::read(path).map_err(|source| PerformanceNoteWriteError::Read {
            path: path.to_string_lossy().into_owned(),
            source,
        })?;
        let checkpoint: CheckpointDocument =
            serde_json::from_slice(&bytes).map_err(|source| PerformanceNoteWriteError::Parse {
                path: path.to_string_lossy().into_owned(),
                source,
            })?;
        validate_checkpoint(&checkpoint)?;
        checkpoints.push(StoredCheckpoint {
            checkpoint,
            json_path: portable_path(root, path)?,
            markdown_path: portable_path(root, &path.with_extension("md"))?,
            revision: FileRevision {
                sha256: hash_bytes(&bytes),
                bytes: bytes.len() as u64,
            },
        });
    }
    Ok(checkpoints)
}

fn checkpoint_for_report(entry: &StoredCheckpoint) -> Option<PerformanceNoteCheckpoint> {
    if entry.checkpoint.confidentiality == "restricted" {
        return None;
    }

    let mut checkpoint = entry.checkpoint.clone();
    if checkpoint.confidentiality == "sensitive" {
        checkpoint.activities =
            vec!["민감 기록의 세부 활동은 보고서에서 생략했습니다.".to_string()];
        checkpoint.decisions.clear();
        checkpoint.blockers.clear();
        checkpoint.next_steps.clear();
        checkpoint.evidence = CheckpointEvidenceDocument {
            commits: Vec::new(),
            pull_requests: Vec::new(),
            issues: Vec::new(),
            files: Vec::new(),
            commands: Vec::new(),
            urls: Vec::new(),
        };
        checkpoint.git = None;
        checkpoint.source.session_ref = None;
        checkpoint.source.task_title = None;
        for verification in &mut checkpoint.verifications {
            verification.command = None;
            verification.evidence_refs.clear();
        }
        for outcome in &mut checkpoint.outcomes {
            outcome.evidence_refs.clear();
        }
    }

    Some(PerformanceNoteCheckpoint {
        checkpoint,
        markdown_path: entry.markdown_path.clone(),
    })
}

fn js_value_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(js_value_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
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

fn checkpoint_evidence(checkpoint: &CheckpointDocument) -> String {
    let mut entries = Vec::new();
    for (kind, values) in [
        ("commits", &checkpoint.evidence.commits),
        ("pull_requests", &checkpoint.evidence.pull_requests),
        ("issues", &checkpoint.evidence.issues),
        ("files", &checkpoint.evidence.files),
        ("commands", &checkpoint.evidence.commands),
        ("urls", &checkpoint.evidence.urls),
    ] {
        entries.extend(values.iter().map(|value| format!("{kind}: {value}")));
    }
    bullets(&entries, "근거 미기록")
}

fn verification(checkpoints: &[PerformanceNoteCheckpoint], kind: &str) -> String {
    let matches = checkpoints
        .iter()
        .flat_map(|entry| {
            entry
                .checkpoint
                .verifications
                .iter()
                .filter(move |item| item.kind == kind)
                .map(move |item| {
                    format!(
                        "{} — {} ({}){}",
                        entry.checkpoint.id,
                        item.description,
                        item.status,
                        item.command
                            .as_deref()
                            .map(|command| format!(": `{command}`"))
                            .unwrap_or_default()
                    )
                })
        })
        .collect::<Vec<_>>();
    bullets(&matches, "미확인")
}

fn render_batches(checkpoints: &[PerformanceNoteCheckpoint]) -> String {
    if checkpoints.is_empty() {
        return "- 체크포인트가 없어 작업 내용을 확인할 수 없습니다.".to_string();
    }
    checkpoints
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let checkpoint = &entry.checkpoint;
            let verification = checkpoint
                .verifications
                .iter()
                .map(|item| {
                    format!(
                        "{}: {} ({}){}",
                        item.kind,
                        item.description,
                        item.status,
                        item.command
                            .as_deref()
                            .map(|command| format!(" — `{command}`"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>();
            let outcomes = checkpoint
                .outcomes
                .iter()
                .map(|item| item.description.clone())
                .collect::<Vec<_>>();
            format!(
                "## 배치 {}: {}\n\n- 대상: {} ~ {}\n- 변경 내용:\n{}\n- 검증 방법:\n{}\n- 결과:\n{}\n- 근거:\n{}",
                index + 1,
                checkpoint.title,
                js_value_string(&checkpoint.work_period.start),
                js_value_string(&checkpoint.work_period.end),
                bullets(&checkpoint.activities, "미기록"),
                bullets(&verification, "미확인"),
                bullets(&outcomes, &checkpoint.summary),
                checkpoint_evidence(checkpoint)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn render_performance_note(
    work_item: &WorkItemDocument,
    context: &WorkContextDocument,
    checkpoints: &[PerformanceNoteCheckpoint],
    generated_at: &str,
) -> String {
    let repository = context
        .git
        .repository
        .clone()
        .or_else(|| {
            work_item
                .repositories
                .first()
                .and_then(|repository| repository.get("url"))
                .filter(|value| !value.is_null())
                .map(js_value_string)
        })
        .unwrap_or_else(|| "미확인".to_string());
    let branch = context
        .git
        .branch
        .clone()
        .unwrap_or_else(|| "미확인".to_string());
    let latest = checkpoints.last().map(|entry| &entry.checkpoint);
    let decisions = checkpoints
        .iter()
        .flat_map(|entry| {
            entry
                .checkpoint
                .decisions
                .iter()
                .map(|item| format!("{} — {} ({})", item.summary, item.rationale, item.status))
        })
        .collect::<Vec<_>>();
    let outcomes = checkpoints
        .iter()
        .flat_map(|entry| {
            entry
                .checkpoint
                .outcomes
                .iter()
                .map(|item| item.description.clone())
        })
        .collect::<Vec<_>>();
    let risks = context
        .risks
        .iter()
        .cloned()
        .chain(
            checkpoints
                .iter()
                .flat_map(|entry| entry.checkpoint.blockers.iter().cloned()),
        )
        .collect::<Vec<_>>();
    let links = work_item
        .links
        .iter()
        .map(|link| {
            link.get("url")
                .filter(|value| !value.is_null())
                .map(js_value_string)
                .unwrap_or_else(|| js_value_string(link))
        })
        .collect::<Vec<_>>();
    let not_run = checkpoints
        .iter()
        .flat_map(|entry| {
            entry
                .checkpoint
                .verifications
                .iter()
                .filter(|item| item.status == "not_run")
                .map(|item| format!("{} ({})", item.description, item.kind))
        })
        .collect::<Vec<_>>();
    let followups = if context.next_steps.is_empty() {
        vec!["후속 확인 필요".to_string()]
    } else {
        context.next_steps.clone()
    }
    .iter()
    .map(|item| format!("- [ ] {item}"))
    .collect::<Vec<_>>()
    .join("\n");
    let checkpoint_ids = checkpoints
        .iter()
        .map(|entry| entry.checkpoint.id.clone())
        .collect::<Vec<_>>();
    let report_confidentiality = if checkpoints
        .iter()
        .any(|entry| entry.checkpoint.confidentiality == "sensitive")
    {
        "sensitive"
    } else {
        "normal"
    };
    let related_notes = checkpoints
        .iter()
        .map(|entry| entry.markdown_path.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let latest_summary = latest
        .map(|checkpoint| checkpoint.summary.as_str())
        .unwrap_or(&work_item.objective);
    let latest_outcome = outcomes.first().map(String::as_str).unwrap_or("미확인");
    let latest_prs = latest
        .map(|checkpoint| checkpoint.evidence.pull_requests.join(", "))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "미확인".to_string());
    let latest_urls = latest
        .map(|checkpoint| checkpoint.evidence.urls.join(", "))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "미확인".to_string());

    format!(
        "---
work_item_id: {}
project_id: {}
generated_from_checkpoints: {}
generated_at: {}
confidentiality: {report_confidentiality}
---

# {} 성과 노트

> 체크포인트 원본에서 생성한 초안입니다. 작업 규모에 맞지 않는 섹션은 삭제하고, 정량 수치·배포 결과처럼 근거가 없는 내용은 확인 후 보완합니다.

# 1. 작업 개요

- 작업명: {}
- 작업 유형: {}
- 저장소: {repository}
- 브랜치: {branch}
- PR: 미확인
- 배포 대상: 미확인
- 작성일: {}
- 작성자: {}
- 상태: {}
- 관련 문서 또는 체크리스트:
{}

# 2. 요약

- 한 줄 요약: {latest_summary}
- 핵심 결과:
{}
- 영향 범위: {}
- 최종 상태: {}

# 3. 작업 배경

- 기존 문제: {}
- 사용자 경험에서의 문제: 미확인
- 개발 경험에서의 문제: 미확인
- 지금 정리해야 하는 이유: {}

# 4. 작업 목표

{}
- 제외 범위: 미확인

# 5. 작업 기준

- 전환 또는 수정 기준: 미확인
- 유지 기준: 미확인
- 제외 기준: 미확인
- 의사결정 기준:
{}

# 6. 진행한 작업

{}

# 7. 작업 후 확인된 내용

## 정량 성과

- 대상 수: 미확인 → 미확인
- 코드 라인 수: 미확인 → 미확인
- 파일 수: 미확인 → 미확인
- 중복 또는 분기 감소: 미확인
- 오류 또는 경고 변화: 미확인

## 정성 성과

- DX 개선: 미확인
- UI/UX 개선: 미확인
- 유지보수성 개선: 미확인
- 정책 또는 문서화 개선:
{}

## 예외 처리

- 예외 대상: 미확인
- 유지 이유: 미확인
- 후속 판단 조건: 미확인

# 8. 기대되는 변화

- 사용자 경험: 미확인
- 개발 경험: 미확인
- 운영 또는 배포 관점: 미확인
- 장기 유지보수 관점: 미확인

# 9. 확인한 검증

- 자동 검증:
{}
- 수동 검증:
{}
- 브라우저 확인:
{}
- 배포 확인: 미확인
- 미실행 검증과 사유:
{}

# 10. 남은 주의사항

- 회귀 가능성:
{}
- 예외 케이스: 미확인
- 모니터링 포인트: 미확인
- 롤백 또는 복구 시 고려사항: 미확인

# 11. 후속 확인 사항

{followups}
- [ ] 문서 또는 정책 반영
- [ ] 운영 환경 확인

# 12. 정리

- 이번 작업에서 확정된 기준:
{}
- 다음 작업자가 따라야 할 방식: {}
- 반복해서 재사용할 수 있는 패턴: 미확인

# 13. 업무 요약

- 공유용 한 문장: {latest_summary}
- PR 또는 슬랙용 요약: {latest_outcome}
- 릴리즈 노트 후보: {latest_outcome}

# 참고 링크

- PR: {latest_prs}
- 체크리스트: 미확인
- 관련 노트: {}
- 배포 또는 CI 링크: {latest_urls}
",
        serde_json::to_string(&work_item.id).expect("serializing a string cannot fail"),
        serde_json::to_string(&work_item.project_id).expect("serializing a string cannot fail"),
        serde_json::to_string(&checkpoint_ids).expect("serializing checkpoint IDs cannot fail"),
        serde_json::to_string(generated_at).expect("serializing a string cannot fail"),
        work_item.title,
        work_item.title,
        if work_item.classification.work_types.is_empty() {
            "미확인".to_string()
        } else {
            work_item.classification.work_types.join(" / ")
        },
        latest
            .map(|checkpoint| checkpoint.captured_at.as_str())
            .unwrap_or("미확인"),
        latest
            .map(|checkpoint| checkpoint.source.agent.as_str())
            .unwrap_or("미확인"),
        work_item.status,
        bullets(&links, "미확인"),
        bullets(&outcomes, "미확인"),
        if work_item.desired_outcomes.is_empty() {
            "미확인".to_string()
        } else {
            work_item.desired_outcomes.join(" / ")
        },
        context.current_state,
        work_item.objective,
        work_item.objective,
        bullets(&work_item.desired_outcomes, &work_item.objective),
        bullets(&decisions, "미확인"),
        render_batches(checkpoints),
        bullets(&decisions, "미확인"),
        verification(checkpoints, "test"),
        verification(checkpoints, "manual"),
        verification(checkpoints, "review"),
        bullets(&not_run, "미확인"),
        bullets(&risks, "미확인"),
        bullets(&decisions, "미확인"),
        context.current_state,
        if related_notes.is_empty() {
            "미확인"
        } else {
            &related_notes
        },
    )
}

fn parse_report_date(value: &str, field: &str) -> Result<NaiveDate, PerformanceNoteWriteError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|error| {
        PerformanceNoteWriteError::InvalidInput(format!(
            "{field} must be a calendar date in YYYY-MM-DD format: {error}"
        ))
    })
}

fn temporal_boundary_date(value: &Value) -> Option<NaiveDate> {
    value
        .as_str()
        .and_then(|value| value.get(0..10))
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
}

fn captured_date(checkpoint: &CheckpointDocument) -> Option<NaiveDate> {
    checkpoint
        .captured_at
        .get(0..10)
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
}

fn checkpoint_overlaps_period(
    checkpoint: &CheckpointDocument,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> bool {
    match (
        temporal_boundary_date(&checkpoint.work_period.start),
        temporal_boundary_date(&checkpoint.work_period.end),
    ) {
        (Some(start), Some(end)) => start <= end_date && end >= start_date,
        (Some(date), None) | (None, Some(date)) => date >= start_date && date <= end_date,
        (None, None) => false,
    }
}

fn weekly_checkpoints(
    root: &Path,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<StoredCheckpoint>, PerformanceNoteWriteError> {
    let mut checkpoints = load_checkpoints(root)?
        .into_iter()
        .filter(|entry| checkpoint_overlaps_period(&entry.checkpoint, start_date, end_date))
        .collect::<Vec<_>>();
    checkpoints.sort_by(|left, right| {
        left.checkpoint
            .captured_at
            .cmp(&right.checkpoint.captured_at)
            .then_with(|| left.checkpoint.id.cmp(&right.checkpoint.id))
    });
    Ok(checkpoints)
}

fn weekly_work_item_sources(
    root: &Path,
    checkpoints: &[StoredCheckpoint],
) -> Result<Vec<PerformanceNoteSources>, PerformanceNoteWriteError> {
    let work_item_ids = checkpoints
        .iter()
        .map(|entry| entry.checkpoint.work_item_id.clone())
        .collect::<BTreeSet<_>>();
    work_item_ids
        .iter()
        .map(|work_item_id| read_performance_note_sources(root, work_item_id))
        .collect()
}

fn weekly_report_stats(
    checkpoints: &[PerformanceNoteCheckpoint],
    redacted_checkpoint_count: usize,
    excluded_checkpoint_count: usize,
    unknown_period_checkpoint_count: usize,
) -> WeeklyReportStats {
    let work_item_count = checkpoints
        .iter()
        .map(|entry| entry.checkpoint.work_item_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let git_commits = checkpoints
        .iter()
        .flat_map(|entry| {
            let mut commits = entry.checkpoint.evidence.commits.clone();
            if let Some(head_after) = entry
                .checkpoint
                .git
                .as_ref()
                .and_then(|git| git.head_after.clone())
            {
                commits.push(head_after);
            }
            commits
        })
        .filter(|commit| !commit.trim().is_empty())
        .collect::<BTreeSet<_>>();
    let verifications = checkpoints
        .iter()
        .flat_map(|entry| entry.checkpoint.verifications.iter())
        .collect::<Vec<_>>();

    WeeklyReportStats {
        work_item_count,
        checkpoint_count: checkpoints.len(),
        redacted_checkpoint_count,
        excluded_checkpoint_count,
        unknown_period_checkpoint_count,
        git_commit_count: git_commits.len(),
        verification_count: verifications.len(),
        passed_verification_count: verifications
            .iter()
            .filter(|verification| verification.status == "passed")
            .count(),
        failed_verification_count: verifications
            .iter()
            .filter(|verification| verification.status == "failed")
            .count(),
        partial_verification_count: verifications
            .iter()
            .filter(|verification| verification.status == "partial")
            .count(),
        not_run_verification_count: verifications
            .iter()
            .filter(|verification| verification.status == "not_run")
            .count(),
    }
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn weekly_work_item_sections(
    sources: &[PerformanceNoteSources],
    checkpoints: &[PerformanceNoteCheckpoint],
) -> String {
    let sources = sources
        .iter()
        .map(|source| (source.work_item.id.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    let mut grouped = BTreeMap::<&str, Vec<&PerformanceNoteCheckpoint>>::new();
    for checkpoint in checkpoints {
        grouped
            .entry(checkpoint.checkpoint.work_item_id.as_str())
            .or_default()
            .push(checkpoint);
    }
    if grouped.is_empty() {
        return "- 이 기간에 보고할 수 있는 체크포인트가 없습니다.".to_string();
    }

    grouped
        .into_iter()
        .filter_map(|(work_item_id, entries)| {
            let source = sources.get(work_item_id)?;
            let outcomes = entries
                .iter()
                .flat_map(|entry| entry.checkpoint.outcomes.iter())
                .map(|outcome| outcome.description.clone())
                .collect::<Vec<_>>();
            let summaries = entries
                .iter()
                .map(|entry| entry.checkpoint.summary.clone())
                .collect::<Vec<_>>();
            let latest_status = entries
                .last()
                .map(|entry| entry.checkpoint.status_after.as_str())
                .unwrap_or(source.work_item.status.as_str());
            Some(format!(
                "### {} · {}\n\n- 업무 ID: `{}`\n- 기간 종료 상태: {}\n- 핵심 결과:\n{}\n- 작업 요약:\n{}",
                source.work_item.project_id,
                source.work_item.title,
                source.work_item.id,
                latest_status,
                bullets(&outcomes, "구조화된 결과 미기록"),
                bullets(&summaries, "요약 미기록")
            ))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn weekly_git_summary(checkpoints: &[PerformanceNoteCheckpoint]) -> String {
    let mut entries = BTreeSet::new();
    for entry in checkpoints {
        let checkpoint = &entry.checkpoint;
        if let Some(git) = &checkpoint.git {
            entries.insert(format!(
                "{} · {} · {} → {} · 작업 트리 {}",
                git.repository,
                git.branch.as_deref().unwrap_or("브랜치 미확인"),
                git.head_before.as_deref().unwrap_or("시작 커밋 미확인"),
                git.head_after.as_deref().unwrap_or("종료 커밋 미확인"),
                match git.dirty {
                    Some(true) => "수정 포함",
                    Some(false) => "깨끗함",
                    None => "미확인",
                }
            ));
        }
        entries.extend(
            checkpoint
                .evidence
                .commits
                .iter()
                .map(|commit| format!("커밋 `{commit}` · {}", checkpoint.title)),
        );
        entries.extend(
            checkpoint
                .evidence
                .pull_requests
                .iter()
                .map(|pull_request| format!("PR {pull_request} · {}", checkpoint.title)),
        );
    }
    bullets(
        &entries.into_iter().collect::<Vec<_>>(),
        "기록된 Git 변경 없음",
    )
}

fn weekly_verification_table(checkpoints: &[PerformanceNoteCheckpoint]) -> String {
    let rows = checkpoints
        .iter()
        .flat_map(|entry| {
            entry
                .checkpoint
                .verifications
                .iter()
                .map(move |verification| {
                    format!(
                        "| {} | {} | {} | {} | {} |",
                        markdown_cell(&entry.checkpoint.work_item_id),
                        markdown_cell(&verification.kind),
                        markdown_cell(&verification.status),
                        markdown_cell(&verification.description),
                        verification
                            .command
                            .as_deref()
                            .map(markdown_cell)
                            .unwrap_or_else(|| "—".to_string())
                    )
                })
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return "- 기록된 검증 결과가 없습니다.".to_string();
    }
    format!(
        "| 업무 | 유형 | 상태 | 설명 | 명령 |\n| --- | --- | --- | --- | --- |\n{}",
        rows.join("\n")
    )
}

fn weekly_followups(
    sources: &[PerformanceNoteSources],
    checkpoints: &[PerformanceNoteCheckpoint],
) -> (Vec<String>, Vec<String>) {
    let included_work_items = checkpoints
        .iter()
        .map(|entry| entry.checkpoint.work_item_id.as_str())
        .collect::<BTreeSet<_>>();
    let risks = sources
        .iter()
        .filter(|source| included_work_items.contains(source.work_item.id.as_str()))
        .flat_map(|source| source.context.risks.iter().cloned())
        .chain(
            checkpoints
                .iter()
                .flat_map(|entry| entry.checkpoint.blockers.iter().cloned()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let next_steps = sources
        .iter()
        .filter(|source| included_work_items.contains(source.work_item.id.as_str()))
        .flat_map(|source| source.context.next_steps.iter().cloned())
        .chain(
            checkpoints
                .iter()
                .flat_map(|entry| entry.checkpoint.next_steps.iter().cloned()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    (risks, next_steps)
}

fn render_weekly_report(
    sources: &[PerformanceNoteSources],
    checkpoints: &[PerformanceNoteCheckpoint],
    stats: &WeeklyReportStats,
    start_date: &str,
    end_date: &str,
    generated_at: &str,
) -> String {
    let checkpoint_ids = checkpoints
        .iter()
        .map(|entry| entry.checkpoint.id.clone())
        .collect::<Vec<_>>();
    let confidentiality = if stats.redacted_checkpoint_count > 0 {
        "sensitive"
    } else {
        "normal"
    };
    let (risks, next_steps) = weekly_followups(sources, checkpoints);
    let source_notes = checkpoints
        .iter()
        .map(|entry| format!("{}: {}", entry.checkpoint.id, entry.markdown_path))
        .collect::<Vec<_>>();

    format!(
        "---\nreport_type: weekly\nstart_date: {}\nend_date: {}\ngenerated_from_checkpoints: {}\ngenerated_at: {}\nconfidentiality: {confidentiality}\n---\n\n# 주간 성과보고서 · {start_date} ~ {end_date}\n\n> 구조화된 체크포인트에서 자동 집계한 초안입니다. 기간, Git 변경, 검증 결과와 다음 작업을 확인한 뒤 공유하세요.\n\n## 한눈에 보기\n\n- 업무: {}개\n- 포함 체크포인트: {}개\n- 민감 체크포인트: {}개 세부 정보 생략\n- 제한 체크포인트: {}개 제외\n- 기간 미확인 체크포인트: {}개 제외\n- Git 커밋: {}개\n- 검증: {}개 · 통과 {} · 실패 {} · 부분 통과 {} · 미실행 {}\n\n## 업무별 성과\n\n{}\n\n## Git 변경\n\n{}\n\n## 테스트·검증 결과\n\n{}\n\n## 위험 및 차단 사항\n\n{}\n\n## 다음 주 우선순위\n\n{}\n\n## 근거 기록\n\n{}\n",
        serde_json::to_string(start_date).expect("serializing a date cannot fail"),
        serde_json::to_string(end_date).expect("serializing a date cannot fail"),
        serde_json::to_string(&checkpoint_ids).expect("serializing checkpoint IDs cannot fail"),
        serde_json::to_string(generated_at).expect("serializing a timestamp cannot fail"),
        stats.work_item_count,
        stats.checkpoint_count,
        stats.redacted_checkpoint_count,
        stats.excluded_checkpoint_count,
        stats.unknown_period_checkpoint_count,
        stats.git_commit_count,
        stats.verification_count,
        stats.passed_verification_count,
        stats.failed_verification_count,
        stats.partial_verification_count,
        stats.not_run_verification_count,
        weekly_work_item_sections(sources, checkpoints),
        weekly_git_summary(checkpoints),
        weekly_verification_table(checkpoints),
        bullets(&risks, "등록된 위험 또는 차단 사항 없음"),
        bullets(&next_steps, "등록된 다음 작업 없음"),
        bullets(&source_notes, "포함된 체크포인트 없음"),
    )
}

fn sorted_checkpoints(
    root: &Path,
    work_item_id: &str,
) -> Result<Vec<StoredCheckpoint>, PerformanceNoteWriteError> {
    let mut checkpoints = load_checkpoints(root)?
        .into_iter()
        .filter(|entry| entry.checkpoint.work_item_id == work_item_id)
        .collect::<Vec<_>>();
    checkpoints.sort_by(|left, right| {
        left.checkpoint
            .captured_at
            .cmp(&right.checkpoint.captured_at)
            .then_with(|| left.checkpoint.id.cmp(&right.checkpoint.id))
    });
    Ok(checkpoints)
}

fn normalize_output_path(value: &str) -> Result<PathBuf, PerformanceNoteWriteError> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.extension() != Some(OsStr::new("md"))
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(PerformanceNoteWriteError::InvalidInput(
            "report output must be a relative .md file inside the data root".to_string(),
        ));
    }
    Ok(path)
}

fn validate_report_confidentiality(
    markdown: &str,
    expected: &str,
) -> Result<(), PerformanceNoteWriteError> {
    let frontmatter = markdown
        .strip_prefix("---\n")
        .and_then(|value| value.split_once("\n---"))
        .map(|(frontmatter, _)| frontmatter);
    let actual = frontmatter.and_then(|frontmatter| {
        frontmatter.lines().find_map(|line| {
            line.strip_prefix("confidentiality:")
                .map(str::trim)
                .map(|value| value.trim_matches(['\"', '\'']))
        })
    });
    if actual != Some(expected) {
        return Err(PerformanceNoteWriteError::InvalidInput(format!(
            "report Markdown must preserve confidentiality: {expected}"
        )));
    }
    Ok(())
}

fn prepare_performance_note(
    writer: &DataRootWriter,
    input: PerformanceNoteInput,
    generated_at: &str,
) -> Result<PreparedPerformanceNote, PerformanceNoteWriteError> {
    let snapshot = read_performance_note_sources(writer.root(), &input.work_item_id)?;
    let checkpoints = sorted_checkpoints(writer.root(), &input.work_item_id)?;
    let report_checkpoints = checkpoints
        .iter()
        .filter_map(checkpoint_for_report)
        .collect::<Vec<_>>();
    let redacted_checkpoint_count = checkpoints
        .iter()
        .filter(|entry| entry.checkpoint.confidentiality == "sensitive")
        .count();
    let excluded_checkpoint_count = checkpoints
        .iter()
        .filter(|entry| entry.checkpoint.confidentiality == "restricted")
        .count();
    let date = report_checkpoints
        .last()
        .and_then(|entry| entry.checkpoint.work_period.end.as_str())
        .or_else(|| generated_at.get(0..10))
        .unwrap_or(generated_at)
        .replace('-', "");
    let output = input.output.unwrap_or_else(|| {
        format!(
            "reports/performance-notes/{}-{date}.md",
            snapshot.work_item.id
        )
    });
    let output_path = normalize_output_path(&output)?;
    let output = output_path
        .to_str()
        .expect("normalize_output_path accepts UTF-8 input")
        .replace(std::path::MAIN_SEPARATOR, "/");
    if writer.revision(&output)?.is_some() {
        return Err(WriteError::CreateConflict(output).into());
    }

    let mut source_revisions = vec![
        PerformanceNoteSourceRevision {
            path: snapshot.paths.work_item.clone(),
            revision: snapshot.work_item_revision.clone(),
        },
        PerformanceNoteSourceRevision {
            path: snapshot.paths.context_data.clone(),
            revision: snapshot.context_revision.clone(),
        },
    ];
    for checkpoint in &checkpoints {
        source_revisions.push(PerformanceNoteSourceRevision {
            path: checkpoint.json_path.clone(),
            revision: checkpoint.revision.clone(),
        });
    }
    source_revisions.sort_by(|left, right| left.path.cmp(&right.path));
    let markdown = input.markdown.unwrap_or_else(|| {
        render_performance_note(
            &snapshot.work_item,
            &snapshot.context,
            &report_checkpoints,
            generated_at,
        )
    });
    validate_report_confidentiality(
        &markdown,
        if redacted_checkpoint_count > 0 {
            "sensitive"
        } else {
            "normal"
        },
    )?;
    Ok(PreparedPerformanceNote {
        work_item: snapshot.work_item,
        checkpoint_count: report_checkpoints.len(),
        redacted_checkpoint_count,
        excluded_checkpoint_count,
        paths: PerformanceNotePaths { report: output },
        source_revisions,
        markdown,
    })
}

fn verify_source_revisions(
    expected: &[PerformanceNoteSourceRevision],
    actual: &[PerformanceNoteSourceRevision],
) -> Result<(), PerformanceNoteWriteError> {
    let expected_by_path = expected
        .iter()
        .map(|source| (source.path.as_str(), &source.revision))
        .collect::<BTreeMap<_, _>>();
    if expected_by_path.len() != expected.len() {
        return Err(PerformanceNoteWriteError::InvalidInput(
            "source revisions contain duplicate paths".to_string(),
        ));
    }
    let actual_by_path = actual
        .iter()
        .map(|source| (source.path.as_str(), &source.revision))
        .collect::<BTreeMap<_, _>>();
    for (path, expected_revision) in &expected_by_path {
        let Some(actual_revision) = actual_by_path.get(path) else {
            return Err(WriteError::RevisionConflict {
                path: (*path).to_string(),
                expected: expected_revision.sha256.clone(),
                actual: None,
            }
            .into());
        };
        if actual_revision.sha256 != expected_revision.sha256 {
            return Err(WriteError::RevisionConflict {
                path: (*path).to_string(),
                expected: expected_revision.sha256.clone(),
                actual: Some(actual_revision.sha256.clone()),
            }
            .into());
        }
    }
    if let Some((path, revision)) = actual_by_path
        .iter()
        .find(|(path, _)| !expected_by_path.contains_key(**path))
    {
        return Err(WriteError::RevisionConflict {
            path: (*path).to_string(),
            expected: "absent".to_string(),
            actual: Some(revision.sha256.clone()),
        }
        .into());
    }
    Ok(())
}

fn current_source_revisions(
    root: &Path,
    work_item_id: &str,
) -> Result<Vec<PerformanceNoteSourceRevision>, PerformanceNoteWriteError> {
    let snapshot = read_performance_note_sources(root, work_item_id)?;
    let checkpoints = sorted_checkpoints(root, work_item_id)?;
    let mut sources = vec![
        PerformanceNoteSourceRevision {
            path: snapshot.paths.work_item,
            revision: snapshot.work_item_revision,
        },
        PerformanceNoteSourceRevision {
            path: snapshot.paths.context_data,
            revision: snapshot.context_revision,
        },
    ];
    for checkpoint in checkpoints {
        sources.push(PerformanceNoteSourceRevision {
            path: checkpoint.json_path,
            revision: checkpoint.revision,
        });
    }
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sources)
}

pub fn preview_performance_note(
    root: impl AsRef<Path>,
    input: PerformanceNoteInput,
    generated_at: &str,
) -> Result<PerformanceNoteWritePreview, PerformanceNoteWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_performance_note(&writer, input, generated_at)?;
    Ok(PerformanceNoteWritePreview {
        work_item: prepared.work_item,
        checkpoint_count: prepared.checkpoint_count,
        redacted_checkpoint_count: prepared.redacted_checkpoint_count,
        excluded_checkpoint_count: prepared.excluded_checkpoint_count,
        paths: prepared.paths.clone(),
        source_revisions: prepared.source_revisions,
        files: vec![WorkItemFileChange {
            path: prepared.paths.report,
            operation: WorkItemChangeOperation::Create,
            before: None,
            after: prepared.markdown,
        }],
    })
}

fn commit_performance_note(
    mut writer: DataRootWriter,
    prepared: PreparedPerformanceNote,
    expected: Vec<PerformanceNoteSourceRevision>,
) -> Result<PerformanceNoteWriteResult, PerformanceNoteWriteError> {
    verify_source_revisions(&expected, &prepared.source_revisions)?;
    let report_path = prepared.paths.report.clone();
    let expected_markdown = prepared.markdown.clone();
    let work_item_id = prepared.work_item.id.clone();
    let validation_sources = prepared.source_revisions.clone();
    let commit = writer.commit_validated(
        vec![WriteOperation::create(
            PathBuf::from(&report_path),
            prepared.markdown.into_bytes(),
        )],
        move |root| {
            let actual_sources =
                current_source_revisions(root, &work_item_id).map_err(|error| error.to_string())?;
            verify_source_revisions(&validation_sources, &actual_sources)
                .map_err(|error| error.to_string())?;
            let actual =
                fs::read_to_string(root.join(&report_path)).map_err(|error| error.to_string())?;
            if actual != expected_markdown {
                return Err("performance note does not match the reviewed Markdown".to_string());
            }
            Ok(())
        },
    )?;
    Ok(PerformanceNoteWriteResult {
        work_item: prepared.work_item,
        checkpoint_count: prepared.checkpoint_count,
        redacted_checkpoint_count: prepared.redacted_checkpoint_count,
        excluded_checkpoint_count: prepared.excluded_checkpoint_count,
        paths: prepared.paths,
        source_revisions: prepared.source_revisions,
        commit,
    })
}

pub fn create_performance_note(
    root: impl AsRef<Path>,
    input: PerformanceNoteInput,
    expected: Vec<PerformanceNoteSourceRevision>,
    generated_at: &str,
) -> Result<PerformanceNoteWriteResult, PerformanceNoteWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_performance_note(&writer, input, generated_at)?;
    commit_performance_note(writer, prepared, expected)
}

pub fn create_performance_note_from_current(
    root: impl AsRef<Path>,
    input: PerformanceNoteInput,
    generated_at: &str,
) -> Result<PerformanceNoteWriteResult, PerformanceNoteWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_performance_note(&writer, input, generated_at)?;
    let expected = prepared.source_revisions.clone();
    commit_performance_note(writer, prepared, expected)
}

fn prepare_weekly_report(
    writer: &DataRootWriter,
    input: WeeklyReportInput,
    generated_at: &str,
) -> Result<PreparedWeeklyReport, PerformanceNoteWriteError> {
    let WeeklyReportInput {
        start_date,
        end_date,
        output,
        markdown,
    } = input;
    let start = parse_report_date(&start_date, "start_date")?;
    let end = parse_report_date(&end_date, "end_date")?;
    if start > end {
        return Err(PerformanceNoteWriteError::InvalidInput(
            "weekly report start_date must be on or before end_date".to_string(),
        ));
    }

    let all_checkpoints = load_checkpoints(writer.root())?;
    let unknown_period_checkpoint_count = all_checkpoints
        .iter()
        .filter(|entry| {
            temporal_boundary_date(&entry.checkpoint.work_period.start).is_none()
                && temporal_boundary_date(&entry.checkpoint.work_period.end).is_none()
                && captured_date(&entry.checkpoint).is_some_and(|date| date >= start && date <= end)
        })
        .count();
    let mut checkpoints = all_checkpoints
        .into_iter()
        .filter(|entry| checkpoint_overlaps_period(&entry.checkpoint, start, end))
        .collect::<Vec<_>>();
    checkpoints.sort_by(|left, right| {
        left.checkpoint
            .captured_at
            .cmp(&right.checkpoint.captured_at)
            .then_with(|| left.checkpoint.id.cmp(&right.checkpoint.id))
    });
    let sources = weekly_work_item_sources(writer.root(), &checkpoints)?;
    let report_checkpoints = checkpoints
        .iter()
        .filter_map(checkpoint_for_report)
        .collect::<Vec<_>>();
    let redacted_checkpoint_count = checkpoints
        .iter()
        .filter(|entry| entry.checkpoint.confidentiality == "sensitive")
        .count();
    let excluded_checkpoint_count = checkpoints
        .iter()
        .filter(|entry| entry.checkpoint.confidentiality == "restricted")
        .count();
    let stats = weekly_report_stats(
        &report_checkpoints,
        redacted_checkpoint_count,
        excluded_checkpoint_count,
        unknown_period_checkpoint_count,
    );
    let output = output.unwrap_or_else(|| {
        format!(
            "reports/weekly/{}_to_{}.md",
            start_date.replace('-', ""),
            end_date.replace('-', "")
        )
    });
    let output_path = normalize_output_path(&output)?;
    let output = output_path
        .to_str()
        .expect("normalize_output_path accepts UTF-8 input")
        .replace(std::path::MAIN_SEPARATOR, "/");
    if writer.revision(&output)?.is_some() {
        return Err(WriteError::CreateConflict(output).into());
    }

    let mut source_revisions = Vec::new();
    for source in &sources {
        source_revisions.push(PerformanceNoteSourceRevision {
            path: source.paths.work_item.clone(),
            revision: source.work_item_revision.clone(),
        });
        source_revisions.push(PerformanceNoteSourceRevision {
            path: source.paths.context_data.clone(),
            revision: source.context_revision.clone(),
        });
    }
    source_revisions.extend(
        checkpoints
            .iter()
            .map(|checkpoint| PerformanceNoteSourceRevision {
                path: checkpoint.json_path.clone(),
                revision: checkpoint.revision.clone(),
            }),
    );
    source_revisions.sort_by(|left, right| left.path.cmp(&right.path));

    let markdown = markdown.unwrap_or_else(|| {
        render_weekly_report(
            &sources,
            &report_checkpoints,
            &stats,
            &start_date,
            &end_date,
            generated_at,
        )
    });
    validate_report_confidentiality(
        &markdown,
        if redacted_checkpoint_count > 0 {
            "sensitive"
        } else {
            "normal"
        },
    )?;

    Ok(PreparedWeeklyReport {
        start_date,
        end_date,
        stats,
        paths: PerformanceNotePaths { report: output },
        source_revisions,
        markdown,
    })
}

fn current_weekly_source_revisions(
    root: &Path,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<PerformanceNoteSourceRevision>, PerformanceNoteWriteError> {
    let start = parse_report_date(start_date, "start_date")?;
    let end = parse_report_date(end_date, "end_date")?;
    let checkpoints = weekly_checkpoints(root, start, end)?;
    let sources = weekly_work_item_sources(root, &checkpoints)?;
    let mut revisions = Vec::new();
    for source in sources {
        revisions.push(PerformanceNoteSourceRevision {
            path: source.paths.work_item,
            revision: source.work_item_revision,
        });
        revisions.push(PerformanceNoteSourceRevision {
            path: source.paths.context_data,
            revision: source.context_revision,
        });
    }
    revisions.extend(
        checkpoints
            .into_iter()
            .map(|checkpoint| PerformanceNoteSourceRevision {
                path: checkpoint.json_path,
                revision: checkpoint.revision,
            }),
    );
    revisions.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(revisions)
}

pub fn preview_weekly_report(
    root: impl AsRef<Path>,
    input: WeeklyReportInput,
    generated_at: &str,
) -> Result<WeeklyReportWritePreview, PerformanceNoteWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_weekly_report(&writer, input, generated_at)?;
    Ok(WeeklyReportWritePreview {
        start_date: prepared.start_date,
        end_date: prepared.end_date,
        stats: prepared.stats,
        paths: prepared.paths.clone(),
        source_revisions: prepared.source_revisions,
        files: vec![WorkItemFileChange {
            path: prepared.paths.report,
            operation: WorkItemChangeOperation::Create,
            before: None,
            after: prepared.markdown,
        }],
    })
}

fn commit_weekly_report(
    mut writer: DataRootWriter,
    prepared: PreparedWeeklyReport,
    expected: Vec<PerformanceNoteSourceRevision>,
) -> Result<WeeklyReportWriteResult, PerformanceNoteWriteError> {
    verify_source_revisions(&expected, &prepared.source_revisions)?;
    let report_path = prepared.paths.report.clone();
    let expected_markdown = prepared.markdown.clone();
    let start_date = prepared.start_date.clone();
    let end_date = prepared.end_date.clone();
    let validation_sources = prepared.source_revisions.clone();
    let validation_start = start_date.clone();
    let validation_end = end_date.clone();
    let commit = writer.commit_validated(
        vec![WriteOperation::create(
            PathBuf::from(&report_path),
            prepared.markdown.into_bytes(),
        )],
        move |root| {
            let actual_sources =
                current_weekly_source_revisions(root, &validation_start, &validation_end)
                    .map_err(|error| error.to_string())?;
            verify_source_revisions(&validation_sources, &actual_sources)
                .map_err(|error| error.to_string())?;
            let actual =
                fs::read_to_string(root.join(&report_path)).map_err(|error| error.to_string())?;
            if actual != expected_markdown {
                return Err("weekly report does not match the reviewed Markdown".to_string());
            }
            Ok(())
        },
    )?;
    Ok(WeeklyReportWriteResult {
        start_date,
        end_date,
        stats: prepared.stats,
        paths: prepared.paths,
        source_revisions: prepared.source_revisions,
        commit,
    })
}

pub fn create_weekly_report(
    root: impl AsRef<Path>,
    input: WeeklyReportInput,
    expected: Vec<PerformanceNoteSourceRevision>,
    generated_at: &str,
) -> Result<WeeklyReportWriteResult, PerformanceNoteWriteError> {
    let writer = DataRootWriter::acquire(root)?;
    let prepared = prepare_weekly_report(&writer, input, generated_at)?;
    commit_weekly_report(writer, prepared, expected)
}

pub fn weekly_report_markdown_path(
    root: impl AsRef<Path>,
    report: &str,
) -> Result<PathBuf, PerformanceNoteWriteError> {
    performance_note_markdown_path(root, report)
}

pub fn performance_note_markdown_path(
    root: impl AsRef<Path>,
    report: &str,
) -> Result<PathBuf, PerformanceNoteWriteError> {
    let relative = normalize_output_path(report)?;
    let root = root
        .as_ref()
        .canonicalize()
        .map_err(|source| PerformanceNoteWriteError::Read {
            path: root.as_ref().to_string_lossy().into_owned(),
            source,
        })?;
    let path = root.join(relative);
    let canonical = path
        .canonicalize()
        .map_err(|source| PerformanceNoteWriteError::Read {
            path: path.to_string_lossy().into_owned(),
            source,
        })?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(PerformanceNoteWriteError::InvalidInput(
            "performance note path is outside the data root".to_string(),
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const GENERATED_AT: &str = "2026-07-14T00:00:00.000Z";

    fn seed_examples(root: &Path) {
        let files = [
            (
                "work-items/AUTH-142/work-item.json",
                include_str!("../../../examples/work-items/AUTH-142/work-item.json"),
            ),
            (
                "work-items/AUTH-142/context.json",
                include_str!("../../../examples/work-items/AUTH-142/context.json"),
            ),
            (
                "work-items/AUTH-142/context.md",
                include_str!("../../../examples/work-items/AUTH-142/context.md"),
            ),
            (
                "records/2026/07/13/CP-20260713-001.json",
                include_str!("../../../examples/records/2026/07/13/CP-20260713-001.json"),
            ),
            (
                "records/2026/07/13/CP-20260713-001.md",
                include_str!("../../../examples/records/2026/07/13/CP-20260713-001.md"),
            ),
        ];
        for (relative, contents) in files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    fn input() -> PerformanceNoteInput {
        PerformanceNoteInput {
            work_item_id: "AUTH-142".to_string(),
            output: None,
            markdown: None,
        }
    }

    fn weekly_input() -> WeeklyReportInput {
        WeeklyReportInput {
            start_date: "2026-07-13".to_string(),
            end_date: "2026-07-19".to_string(),
            output: None,
            markdown: None,
        }
    }

    #[test]
    fn rust_renderer_matches_the_node_performance_note_contract() {
        let work_item: WorkItemDocument = serde_json::from_str(include_str!(
            "../../../examples/work-items/AUTH-142/work-item.json"
        ))
        .unwrap();
        let context: WorkContextDocument = serde_json::from_str(include_str!(
            "../../../examples/work-items/AUTH-142/context.json"
        ))
        .unwrap();
        let checkpoint: CheckpointDocument = serde_json::from_str(include_str!(
            "../../../examples/records/2026/07/13/CP-20260713-001.json"
        ))
        .unwrap();
        let rendered = render_performance_note(
            &work_item,
            &context,
            &[PerformanceNoteCheckpoint {
                checkpoint,
                markdown_path: "records/2026/07/13/CP-20260713-001.md".to_string(),
            }],
            GENERATED_AT,
        );
        assert_eq!(
            rendered,
            include_str!("../../../examples/reports/performance-note-contract.md")
        );
    }

    #[test]
    fn preview_matches_the_create_only_report_commit() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let preview = preview_performance_note(directory.path(), input(), GENERATED_AT).unwrap();
        assert_eq!(preview.checkpoint_count, 1);
        assert_eq!(preview.redacted_checkpoint_count, 0);
        assert_eq!(preview.excluded_checkpoint_count, 0);
        assert_eq!(preview.files.len(), 1);
        assert_eq!(preview.source_revisions.len(), 3);
        assert_eq!(
            preview.paths.report,
            "reports/performance-notes/AUTH-142-20260713.md"
        );
        assert!(!directory.path().join(&preview.paths.report).exists());

        let result = create_performance_note(
            directory.path(),
            input(),
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap();
        assert_eq!(result.checkpoint_count, 1);
        assert_eq!(
            fs::read_to_string(directory.path().join(&result.paths.report)).unwrap(),
            preview.files[0].after
        );
    }

    #[test]
    fn weekly_report_collects_period_git_and_verification_evidence() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());

        let preview =
            preview_weekly_report(directory.path(), weekly_input(), GENERATED_AT).unwrap();
        let markdown = &preview.files[0].after;

        assert_eq!(preview.start_date, "2026-07-13");
        assert_eq!(preview.end_date, "2026-07-19");
        assert_eq!(preview.stats.work_item_count, 1);
        assert_eq!(preview.stats.checkpoint_count, 1);
        assert_eq!(preview.stats.git_commit_count, 1);
        assert_eq!(preview.stats.verification_count, 1);
        assert_eq!(preview.stats.passed_verification_count, 1);
        assert_eq!(preview.source_revisions.len(), 3);
        assert_eq!(
            preview.paths.report,
            "reports/weekly/20260713_to_20260719.md"
        );
        assert!(markdown.contains("# 주간 성과보고서 · 2026-07-13 ~ 2026-07-19"));
        assert!(markdown.contains("jajak-front · 인증 시스템 개선"));
        assert!(markdown.contains("커밋 `abc1234`"));
        assert!(markdown.contains("인증 갱신 기본 성공 경로 테스트"));
        assert!(!directory.path().join(&preview.paths.report).exists());

        let result = create_weekly_report(
            directory.path(),
            weekly_input(),
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap();
        assert_eq!(result.stats.checkpoint_count, 1);
        assert_eq!(
            fs::read_to_string(directory.path().join(&result.paths.report)).unwrap(),
            markdown.as_str()
        );
    }

    #[test]
    fn weekly_report_redacts_sensitive_excludes_restricted_and_rejects_downgrades() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let checkpoint_path = directory
            .path()
            .join("records/2026/07/13/CP-20260713-001.json");
        let mut sensitive: Value =
            serde_json::from_str(&fs::read_to_string(&checkpoint_path).unwrap()).unwrap();
        sensitive["confidentiality"] = Value::String("sensitive".to_string());
        fs::write(
            &checkpoint_path,
            serde_json::to_string_pretty(&sensitive).unwrap(),
        )
        .unwrap();

        let mut restricted = sensitive.clone();
        restricted["id"] = Value::String("CP-20260714-002".to_string());
        restricted["captured_at"] = Value::String("2026-07-14T18:10:00+09:00".to_string());
        restricted["work_period"]["start"] = Value::String("2026-07-14".to_string());
        restricted["work_period"]["end"] = Value::String("2026-07-14".to_string());
        restricted["title"] = Value::String("제한 제목 노출 금지".to_string());
        restricted["summary"] = Value::String("제한 요약 노출 금지".to_string());
        restricted["confidentiality"] = Value::String("restricted".to_string());
        let restricted_path = directory
            .path()
            .join("records/2026/07/14/CP-20260714-002.json");
        fs::create_dir_all(restricted_path.parent().unwrap()).unwrap();
        fs::write(
            &restricted_path,
            serde_json::to_string_pretty(&restricted).unwrap(),
        )
        .unwrap();

        let preview =
            preview_weekly_report(directory.path(), weekly_input(), GENERATED_AT).unwrap();
        let markdown = &preview.files[0].after;
        assert_eq!(preview.stats.checkpoint_count, 1);
        assert_eq!(preview.stats.redacted_checkpoint_count, 1);
        assert_eq!(preview.stats.excluded_checkpoint_count, 1);
        assert_eq!(preview.stats.git_commit_count, 0);
        assert!(markdown.contains("confidentiality: sensitive"));
        assert!(!markdown.contains("abc1234"));
        assert!(!markdown.contains("pnpm test auth"));
        assert!(!markdown.contains("제한 제목 노출 금지"));

        let error = create_weekly_report(
            directory.path(),
            WeeklyReportInput {
                start_date: preview.start_date.clone(),
                end_date: preview.end_date.clone(),
                output: Some(preview.paths.report.clone()),
                markdown: Some(
                    markdown.replace("confidentiality: sensitive", "confidentiality: normal"),
                ),
            },
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap_err();
        assert!(matches!(error, PerformanceNoteWriteError::InvalidInput(_)));
        assert!(!directory.path().join(&preview.paths.report).exists());
    }

    #[test]
    fn performance_note_redacts_sensitive_and_excludes_restricted_checkpoints() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let checkpoint_path = directory
            .path()
            .join("records/2026/07/13/CP-20260713-001.json");
        let mut sensitive: Value =
            serde_json::from_str(&fs::read_to_string(&checkpoint_path).unwrap()).unwrap();
        sensitive["confidentiality"] = Value::String("sensitive".to_string());
        fs::write(
            &checkpoint_path,
            serde_json::to_string_pretty(&sensitive).unwrap(),
        )
        .unwrap();

        let mut restricted = sensitive.clone();
        restricted["id"] = Value::String("CP-20260714-002".to_string());
        restricted["captured_at"] = Value::String("2026-07-14T18:10:00+09:00".to_string());
        restricted["work_period"]["start"] = Value::String("2026-07-14".to_string());
        restricted["work_period"]["end"] = Value::String("2026-07-14".to_string());
        restricted["title"] = Value::String("제한 제목 노출 금지".to_string());
        restricted["summary"] = Value::String("제한 요약 노출 금지".to_string());
        restricted["confidentiality"] = Value::String("restricted".to_string());
        let restricted_path = directory
            .path()
            .join("records/2026/07/14/CP-20260714-002.json");
        fs::create_dir_all(restricted_path.parent().unwrap()).unwrap();
        fs::write(
            &restricted_path,
            serde_json::to_string_pretty(&restricted).unwrap(),
        )
        .unwrap();

        let preview = preview_performance_note(directory.path(), input(), GENERATED_AT).unwrap();
        let markdown = &preview.files[0].after;

        assert_eq!(preview.checkpoint_count, 1);
        assert_eq!(preview.redacted_checkpoint_count, 1);
        assert_eq!(preview.excluded_checkpoint_count, 1);
        assert_eq!(preview.source_revisions.len(), 4);
        assert!(markdown.contains("refresh token 갱신과 요청 재시도의 기본 성공 경로"));
        assert!(markdown.contains("민감 기록의 세부 활동은 보고서에서 생략했습니다."));
        assert!(!markdown.contains("pnpm test auth"));
        assert!(!markdown.contains("abc1234"));
        assert!(!markdown.contains("제한 제목 노출 금지"));
        assert!(!markdown.contains("제한 요약 노출 금지"));

        let error = create_performance_note(
            directory.path(),
            PerformanceNoteInput {
                work_item_id: "AUTH-142".to_string(),
                output: Some(preview.paths.report.clone()),
                markdown: Some(
                    markdown.replace("confidentiality: sensitive", "confidentiality: normal"),
                ),
            },
            preview.source_revisions.clone(),
            GENERATED_AT,
        )
        .unwrap_err();
        assert!(matches!(error, PerformanceNoteWriteError::InvalidInput(_)));
        assert!(!directory.path().join(&preview.paths.report).exists());
    }

    #[test]
    fn reviewed_markdown_is_committed_without_regeneration() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let preview = preview_performance_note(directory.path(), input(), GENERATED_AT).unwrap();
        let reviewed =
            "---\nconfidentiality: normal\n---\n\n# 사용자가 검토한 성과 노트\n\n- 확정된 결과\n";
        let result = create_performance_note(
            directory.path(),
            PerformanceNoteInput {
                work_item_id: "AUTH-142".to_string(),
                output: Some(preview.paths.report.clone()),
                markdown: Some(reviewed.to_string()),
            },
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(directory.path().join(result.paths.report)).unwrap(),
            reviewed
        );
    }

    #[test]
    fn stale_report_source_revision_creates_no_report() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let preview = preview_performance_note(directory.path(), input(), GENERATED_AT).unwrap();
        let context_path = directory.path().join("work-items/AUTH-142/context.json");
        let context = fs::read_to_string(&context_path).unwrap();
        fs::write(&context_path, format!("{context}\n")).unwrap();

        let error = create_performance_note(
            directory.path(),
            input(),
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PerformanceNoteWriteError::Write(WriteError::RevisionConflict { .. })
        ));
        assert!(
            !directory
                .path()
                .join("reports/performance-notes/AUTH-142-20260713.md")
                .exists()
        );
    }

    #[test]
    fn checkpoint_added_after_preview_creates_no_report() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let input = PerformanceNoteInput {
            work_item_id: "AUTH-142".to_string(),
            output: Some("reports/performance-notes/reviewed.md".to_string()),
            markdown: None,
        };
        let preview =
            preview_performance_note(directory.path(), input.clone(), GENERATED_AT).unwrap();
        let checkpoint = include_str!("../../../examples/records/2026/07/13/CP-20260713-001.json")
            .replace("CP-20260713-001", "CP-20260714-002")
            .replace("2026-07-13T", "2026-07-14T");
        let added = directory
            .path()
            .join("records/2026/07/14/CP-20260714-002.json");
        fs::create_dir_all(added.parent().unwrap()).unwrap();
        fs::write(added, checkpoint).unwrap();

        let error = create_performance_note(
            directory.path(),
            input,
            preview.source_revisions,
            GENERATED_AT,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PerformanceNoteWriteError::Write(WriteError::RevisionConflict { .. })
        ));
        assert!(
            !directory
                .path()
                .join("reports/performance-notes/reviewed.md")
                .exists()
        );
    }

    #[test]
    fn duplicate_report_is_never_overwritten() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let first =
            create_performance_note_from_current(directory.path(), input(), GENERATED_AT).unwrap();
        let path = directory.path().join(&first.paths.report);
        let original = fs::read_to_string(&path).unwrap();
        let error = create_performance_note_from_current(directory.path(), input(), GENERATED_AT)
            .unwrap_err();
        assert!(matches!(
            error,
            PerformanceNoteWriteError::Write(WriteError::CreateConflict(_))
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }
}
