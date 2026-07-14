use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use thiserror::Error;
use work_harvest_core::{
    CheckpointInput, CheckpointWriteError, CheckpointWriteResult, DataRootWriter, IssueSeverity,
    WorkItemEditRevisions, WriteCommit, WriteError, WriteOperation, capture_checkpoint,
    inspect_data_root,
};

const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct WriteRequest {
    protocol_version: u32,
    root: PathBuf,
    operations: Option<Vec<ProtocolOperation>>,
    checkpoint_capture: Option<CheckpointCaptureRequest>,
}

#[derive(Debug, Deserialize)]
struct CheckpointCaptureRequest {
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: String,
}

#[derive(Debug, Deserialize)]
struct ProtocolOperation {
    path: PathBuf,
    contents: String,
    expectation: ProtocolExpectation,
    expected_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProtocolExpectation {
    Create,
    MatchSha256,
}

#[derive(Debug, Serialize)]
struct WriteResponse {
    protocol_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<WriteCommit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkpoint_capture: Option<CheckpointWriteResult>,
}

#[derive(Debug, Error)]
enum HelperError {
    #[error("Could not read write request: {0}")]
    Read(#[source] io::Error),
    #[error("Could not parse write request: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("Unsupported write-helper protocol version: {0}")]
    ProtocolVersion(u32),
    #[error("Replacement operation requires expected_sha256: {0}")]
    MissingRevision(String),
    #[error("Create operation must not include expected_sha256: {0}")]
    UnexpectedRevision(String),
    #[error("Write-helper request must contain exactly one operation kind")]
    InvalidRequest,
    #[error("Could not inspect data root before commit: {0}")]
    InspectBefore(String),
    #[error(transparent)]
    Write(#[from] WriteError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointWriteError),
    #[error("Could not serialize write response: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("Could not write response: {0}")]
    Respond(#[source] io::Error),
}

type IssueFingerprint = (String, String, String);

fn error_fingerprints(root: &std::path::Path) -> Result<BTreeSet<IssueFingerprint>, String> {
    let snapshot = inspect_data_root(root).map_err(|error| error.to_string())?;
    Ok(snapshot
        .issues
        .into_iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .map(|issue| (issue.path, issue.code, issue.message))
        .collect())
}

fn protocol_operation(operation: ProtocolOperation) -> Result<WriteOperation, HelperError> {
    match operation.expectation {
        ProtocolExpectation::Create => {
            if operation.expected_sha256.is_some() {
                return Err(HelperError::UnexpectedRevision(
                    operation.path.to_string_lossy().into_owned(),
                ));
            }
            Ok(WriteOperation::create(
                operation.path,
                operation.contents.into_bytes(),
            ))
        }
        ProtocolExpectation::MatchSha256 => {
            let expected = operation.expected_sha256.ok_or_else(|| {
                HelperError::MissingRevision(operation.path.to_string_lossy().into_owned())
            })?;
            Ok(WriteOperation::replace(
                operation.path,
                expected,
                operation.contents.into_bytes(),
            ))
        }
    }
}

fn execute(request: WriteRequest) -> Result<WriteResponse, HelperError> {
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(HelperError::ProtocolVersion(request.protocol_version));
    }

    match (request.operations, request.checkpoint_capture) {
        (Some(operations), None) => execute_operations(request.root, operations),
        (None, Some(checkpoint)) => {
            let result = capture_checkpoint(
                request.root,
                checkpoint.input,
                checkpoint.expected,
                &checkpoint.now,
            )?;
            Ok(WriteResponse {
                protocol_version: PROTOCOL_VERSION,
                commit: None,
                checkpoint_capture: Some(result),
            })
        }
        _ => Err(HelperError::InvalidRequest),
    }
}

fn execute_operations(
    root: PathBuf,
    operations: Vec<ProtocolOperation>,
) -> Result<WriteResponse, HelperError> {
    let operations = operations
        .into_iter()
        .map(protocol_operation)
        .collect::<Result<Vec<_>, _>>()?;
    let mut writer = DataRootWriter::acquire(&root)?;
    let baseline = error_fingerprints(writer.root()).map_err(HelperError::InspectBefore)?;
    let commit = writer.commit_validated(operations, move |root| {
        let after = error_fingerprints(root)?;
        let new_errors = after.difference(&baseline).cloned().collect::<Vec<_>>();
        if new_errors.is_empty() {
            return Ok(());
        }
        Err(new_errors
            .into_iter()
            .map(|(path, code, message)| format!("{path} [{code}]: {message}"))
            .collect::<Vec<_>>()
            .join("; "))
    })?;

    Ok(WriteResponse {
        protocol_version: PROTOCOL_VERSION,
        commit: Some(commit),
        checkpoint_capture: None,
    })
}

fn run() -> Result<(), HelperError> {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(HelperError::Read)?;
    let request = serde_json::from_slice(&input).map_err(HelperError::Parse)?;
    let response = execute(request)?;
    let mut output = serde_json::to_vec(&response).map_err(HelperError::Serialize)?;
    output.push(b'\n');
    io::stdout()
        .write_all(&output)
        .map_err(HelperError::Respond)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_invalid_protocol_shapes_before_writing() {
        let directory = tempdir().unwrap();
        let error = execute(WriteRequest {
            protocol_version: PROTOCOL_VERSION,
            root: directory.path().to_path_buf(),
            operations: Some(vec![ProtocolOperation {
                path: PathBuf::from("note.md"),
                contents: "note\n".to_string(),
                expectation: ProtocolExpectation::MatchSha256,
                expected_sha256: None,
            }]),
            checkpoint_capture: None,
        })
        .unwrap_err();

        assert!(matches!(error, HelperError::MissingRevision(_)));
        assert!(!directory.path().join("note.md").exists());
    }

    #[test]
    fn allows_preexisting_errors_but_rolls_back_new_data_errors() {
        let directory = tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("work-items/BROKEN")).unwrap();
        std::fs::write(
            directory.path().join("work-items/BROKEN/work-item.json"),
            "not-json\n",
        )
        .unwrap();

        let result = execute(WriteRequest {
            protocol_version: PROTOCOL_VERSION,
            root: directory.path().to_path_buf(),
            operations: Some(vec![ProtocolOperation {
                path: PathBuf::from("reports/note.md"),
                contents: "safe report\n".to_string(),
                expectation: ProtocolExpectation::Create,
                expected_sha256: None,
            }]),
            checkpoint_capture: None,
        })
        .unwrap();
        assert_eq!(result.commit.unwrap().written_paths, ["reports/note.md"]);

        let error = execute(WriteRequest {
            protocol_version: PROTOCOL_VERSION,
            root: directory.path().to_path_buf(),
            operations: Some(vec![ProtocolOperation {
                path: PathBuf::from("work-items/NEW/work-item.json"),
                contents: "{}\n".to_string(),
                expectation: ProtocolExpectation::Create,
                expected_sha256: None,
            }]),
            checkpoint_capture: None,
        })
        .unwrap_err();
        assert!(matches!(
            error,
            HelperError::Write(WriteError::ValidationFailed(_))
        ));
        assert!(
            !directory
                .path()
                .join("work-items/NEW/work-item.json")
                .exists()
        );
        assert!(directory.path().join("reports/note.md").is_file());
    }
}
