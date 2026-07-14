use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

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
    issues: &mut Vec<DataIssue>,
) -> Option<WorkItemSummary> {
    let value = read_json(root, path, issues)?;
    let id = required_string(root, path, &value, "id", issues)?;
    let project_id = required_string(root, path, &value, "project_id", issues)?;
    let title = required_string(root, path, &value, "title", issues)?;
    let status = required_string(root, path, &value, "status", issues)?;
    let updated_at = required_string(root, path, &value, "updated_at", issues)?;

    let directory = path.parent().unwrap_or(root);
    let context_data_path = directory.join("context.json");
    let context_markdown_path = directory.join("context.md");
    let context = if context_data_path.exists() {
        read_json(root, &context_data_path, issues)
    } else {
        issues.push(issue(
            root,
            &context_data_path,
            "missing_context_data",
            "구조화된 context.json이 없습니다.",
        ));
        None
    };
    if !context_markdown_path.exists() {
        issues.push(issue(
            root,
            &context_markdown_path,
            "missing_context_markdown",
            "파생 context.md가 없습니다.",
        ));
    }

    Some(WorkItemSummary {
        id,
        project_id,
        title,
        status,
        updated_at,
        current_state: context
            .as_ref()
            .and_then(|value| value.get("current_state"))
            .and_then(Value::as_str)
            .map(str::to_string),
        last_checkpoint_id: context
            .as_ref()
            .and_then(|value| value.get("last_checkpoint_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
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

    for path in &work_item_json_files {
        if let Some(summary) = inspect_work_item(root, path, &mut issues) {
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
        let record_dir = root.join("records/2026/07/14");
        create_dir_all(&work_item_dir).unwrap();
        create_dir_all(&record_dir).unwrap();
        write(
            work_item_dir.join("work-item.json"),
            r#"{
              "id":"AUTH-142",
              "project_id":"jajak-front",
              "title":"인증 개선",
              "status":"in_progress",
              "updated_at":"2026-07-14T10:00:00+09:00"
            }"#,
        )
        .unwrap();
        write(
            work_item_dir.join("context.json"),
            r#"{
              "current_state":"기본 경로를 구현했다.",
              "last_checkpoint_id":"CP-20260714-001"
            }"#,
        )
        .unwrap();
        write(work_item_dir.join("context.md"), "# Context\n").unwrap();
        write(
            record_dir.join("CP-20260714-001.json"),
            r#"{
              "id":"CP-20260714-001",
              "work_item_id":"AUTH-142",
              "project_id":"jajak-front"
            }"#,
        )
        .unwrap();
        write(record_dir.join("CP-20260714-001.md"), "# Checkpoint\n").unwrap();
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
            Some("기본 경로를 구현했다.")
        );
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
