use crate::checkpoints::{CheckpointDocument, CheckpointWriteError, validate_checkpoint};
use crate::work_items::{
    WorkContextDocument, WorkItemDocument, WorkItemPaths, WorkItemWriteError,
    paths as work_item_paths, validate_documents,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Work item was not found: {0}")]
    WorkItemNotFound(String),
    #[error("Could not scan data records: {0}")]
    Scan(String),
    #[error("Could not read data asset {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not parse data asset {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("Data assets are inconsistent: {0}")]
    Inconsistent(String),
    #[error(transparent)]
    WorkItem(#[from] WorkItemWriteError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointWriteError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCheckpointPaths {
    pub json: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredCheckpointRecord {
    pub checkpoint: CheckpointDocument,
    pub paths: StoredCheckpointPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredWorkItemRecord {
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub paths: WorkItemPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItemQueryResult {
    pub work_item: WorkItemDocument,
    pub context: WorkContextDocument,
    pub last_checkpoint: Option<StoredCheckpointRecord>,
    pub paths: WorkItemPaths,
}

fn read_json<T: for<'de> Deserialize<'de>>(
    root: &Path,
    relative_path: &str,
) -> Result<T, QueryError> {
    let bytes = fs::read(root.join(relative_path)).map_err(|source| QueryError::Read {
        path: relative_path.to_string(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| QueryError::Parse {
        path: relative_path.to_string(),
        source,
    })
}

fn portable_path(root: &Path, path: &Path) -> Result<String, QueryError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        QueryError::Inconsistent(format!("path escapes the data root: {}", path.display()))
    })?;
    relative
        .to_str()
        .map(|value| value.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| {
            QueryError::Inconsistent(format!("path is not UTF-8: {}", relative.display()))
        })
}

pub fn read_work_item_record(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<StoredWorkItemRecord, QueryError> {
    if !crate::is_identifier(work_item_id) {
        return Err(QueryError::WorkItemNotFound(work_item_id.to_string()));
    }
    let root = root.as_ref();
    let paths = work_item_paths(work_item_id);
    if !root.join(&paths.work_item).is_file() {
        return Err(QueryError::WorkItemNotFound(work_item_id.to_string()));
    }
    let work_item: WorkItemDocument = read_json(root, &paths.work_item)?;
    if work_item.id != work_item_id {
        return Err(QueryError::WorkItemNotFound(work_item_id.to_string()));
    }
    let context: WorkContextDocument = read_json(root, &paths.context_data)?;
    validate_documents(&work_item, &context)?;
    let context_path = root.join(&work_item.context_path);
    if !context_path.is_file() {
        return Err(QueryError::Inconsistent(format!(
            "context document is missing: {}",
            work_item.context_path
        )));
    }
    Ok(StoredWorkItemRecord {
        work_item,
        context,
        paths,
    })
}

pub fn list_work_item_records(
    root: impl AsRef<Path>,
) -> Result<Vec<StoredWorkItemRecord>, QueryError> {
    let root = root.as_ref();
    let directory = root.join("work-items");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in WalkDir::new(&directory) {
        let entry = entry.map_err(|error| QueryError::Scan(error.to_string()))?;
        if !entry.file_type().is_file() || entry.file_name() != OsStr::new("work-item.json") {
            continue;
        }
        let relative = portable_path(root, entry.path())?;
        let work_item: WorkItemDocument = read_json(root, &relative)?;
        records.push(read_work_item_record(root, &work_item.id)?);
    }
    records.sort_by(|left, right| {
        right
            .work_item
            .updated_at
            .cmp(&left.work_item.updated_at)
            .then_with(|| left.work_item.id.cmp(&right.work_item.id))
    });
    Ok(records)
}

fn load_checkpoint_records(root: &Path) -> Result<Vec<StoredCheckpointRecord>, QueryError> {
    let records = root.join("records");
    if !records.exists() {
        return Ok(Vec::new());
    }
    let mut checkpoints = Vec::new();
    for entry in WalkDir::new(&records) {
        let entry = entry.map_err(|error| QueryError::Scan(error.to_string()))?;
        if !entry.file_type().is_file() || entry.path().extension() != Some(OsStr::new("json")) {
            continue;
        }
        let json_path = portable_path(root, entry.path())?;
        let checkpoint: CheckpointDocument = read_json(root, &json_path)?;
        validate_checkpoint(&checkpoint)?;
        checkpoints.push(StoredCheckpointRecord {
            checkpoint,
            paths: StoredCheckpointPaths {
                json: json_path,
                markdown: portable_path(root, &entry.path().with_extension("md"))?,
            },
        });
    }
    checkpoints.sort_by(|left, right| {
        left.checkpoint
            .captured_at
            .cmp(&right.checkpoint.captured_at)
            .then_with(|| left.checkpoint.id.cmp(&right.checkpoint.id))
    });
    Ok(checkpoints)
}

pub fn list_checkpoints_for_work_item(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<Vec<StoredCheckpointRecord>, QueryError> {
    let root = root.as_ref();
    read_work_item_record(root, work_item_id)?;
    Ok(load_checkpoint_records(root)?
        .into_iter()
        .filter(|entry| entry.checkpoint.work_item_id == work_item_id)
        .collect())
}

pub fn find_last_checkpoint(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<Option<StoredCheckpointRecord>, QueryError> {
    Ok(list_checkpoints_for_work_item(root, work_item_id)?.pop())
}

pub fn show_work_item(
    root: impl AsRef<Path>,
    work_item_id: &str,
) -> Result<WorkItemQueryResult, QueryError> {
    let root = root.as_ref();
    let record = read_work_item_record(root, work_item_id)?;
    let last_checkpoint = find_last_checkpoint(root, work_item_id)?;
    Ok(WorkItemQueryResult {
        work_item: record.work_item,
        context: record.context,
        last_checkpoint,
        paths: record.paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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

    #[test]
    fn returns_node_compatible_work_item_and_checkpoint_shapes() {
        let directory = tempdir().unwrap();
        seed_examples(directory.path());
        let listed = list_work_item_records(directory.path()).unwrap();
        assert_eq!(listed.len(), 1);
        let shown = show_work_item(directory.path(), "AUTH-142").unwrap();
        assert_eq!(shown.work_item.id, "AUTH-142");
        assert_eq!(
            shown.last_checkpoint.unwrap().paths.markdown,
            "records/2026/07/13/CP-20260713-001.md"
        );
    }
}
