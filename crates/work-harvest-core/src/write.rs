use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const CONTROL_DIRECTORY: &str = ".work-harvest";
const TRANSACTIONS_DIRECTORY: &str = "transactions";
const QUARANTINE_DIRECTORY: &str = "quarantine";
const LOCK_FILE: &str = "write.lock";
const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_VERSION: u32 = 1;

static TRANSACTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Failures that stop a write before it is committed or safely recovered.
#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Data root does not exist: {0}")]
    MissingRoot(String),
    #[error("Data root is not a directory: {0}")]
    InvalidRoot(String),
    #[error("Another Work Harvest writer is using data root: {0}")]
    LockBusy(String),
    #[error("Write transaction must contain at least one operation")]
    EmptyTransaction,
    #[error("Unsafe data root write path: {0}")]
    UnsafePath(String),
    #[error("Write transaction contains the same path more than once: {0}")]
    DuplicateTarget(String),
    #[error("Refusing to overwrite existing file: {0}")]
    CreateConflict(String),
    #[error("File changed since it was read: {path} (expected {expected}, actual {actual:?})")]
    RevisionConflict {
        path: String,
        expected: String,
        actual: Option<String>,
    },
    #[error("Could not {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid write transaction manifest {path}: {message}")]
    InvalidManifest { path: String, message: String },
    #[error("Post-write validation failed: {0}")]
    ValidationFailed(String),
    #[error("Transaction {transaction_id} could not be recovered safely at {path}")]
    RecoveryConflict {
        transaction_id: String,
        path: String,
    },
    #[error("Quarantined write transactions require attention: {0}")]
    QuarantinedTransactions(String),
}

/// Stable content token captured when an editable file is read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileRevision {
    pub sha256: String,
    pub bytes: u64,
}

/// Whether a target must be absent or match the previously read content hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteExpectation {
    Create,
    MatchSha256(String),
}

/// One prevalidated file to create or replace inside a data root transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOperation {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
    pub expectation: WriteExpectation,
}

impl WriteOperation {
    /// Builds a create-only operation that refuses to replace an existing path.
    pub fn create(relative_path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) -> Self {
        Self {
            relative_path: relative_path.into(),
            contents: contents.into(),
            expectation: WriteExpectation::Create,
        }
    }

    /// Builds a replacement guarded by the SHA-256 captured when the file was read.
    pub fn replace(
        relative_path: impl Into<PathBuf>,
        expected_sha256: impl Into<String>,
        contents: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            contents: contents.into(),
            expectation: WriteExpectation::MatchSha256(expected_sha256.into()),
        }
    }
}

/// Successfully committed transaction metadata.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WriteCommit {
    pub transaction_id: String,
    pub written_paths: Vec<String>,
}

/// Exclusive, recoverable writer for one canonical data root.
#[derive(Debug)]
pub struct DataRootWriter {
    root: PathBuf,
    lock_file: File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ManifestState {
    Prepared,
    Applying,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ManifestExpectation {
    Create,
    MatchSha256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestOperation {
    target: String,
    staged: String,
    backup: String,
    expectation: ManifestExpectation,
    original_sha256: Option<String>,
    staged_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionManifest {
    version: u32,
    id: String,
    state: ManifestState,
    operations: Vec<ManifestOperation>,
}

impl DataRootWriter {
    /// Acquires the data-root lock and recovers any interrupted transaction first.
    pub fn acquire(root: impl AsRef<Path>) -> Result<Self, WriteError> {
        let root = canonical_data_root(root.as_ref())?;
        let control = root.join(CONTROL_DIRECTORY);
        let transactions = control.join(TRANSACTIONS_DIRECTORY);
        let quarantine = control.join(QUARANTINE_DIRECTORY);
        create_directory(&control)?;
        create_directory(&transactions)?;
        create_directory(&quarantine)?;

        let lock_path = control.join(LOCK_FILE);
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|source| io_error("open write lock", &lock_path, source))?;
        if let Err(source) = lock_file.try_lock_exclusive() {
            if source.kind() == fs2::lock_contended_error().kind() {
                return Err(WriteError::LockBusy(root.to_string_lossy().into_owned()));
            }
            return Err(io_error("acquire write lock", &lock_path, source));
        }

        let mut writer = Self { root, lock_file };
        writer.ensure_quarantine_is_empty()?;
        writer.recover_pending_transactions()?;
        Ok(writer)
    }

    /// Returns the canonical data root protected by this writer.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads a revision token while the writer lock is held.
    pub fn revision(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<Option<FileRevision>, WriteError> {
        read_file_revision(&self.root, relative_path)
    }

    /// Commits prevalidated operations without an additional root validation callback.
    pub fn commit(&mut self, operations: Vec<WriteOperation>) -> Result<WriteCommit, WriteError> {
        self.commit_inner(operations, None, |_| Ok(()))
    }

    /// Installs prevalidated operations, validates the resulting root, then commits or rolls back.
    pub fn commit_validated<F>(
        &mut self,
        operations: Vec<WriteOperation>,
        validate: F,
    ) -> Result<WriteCommit, WriteError>
    where
        F: FnOnce(&Path) -> Result<(), String>,
    {
        self.commit_inner(operations, None, validate)
    }

    fn commit_inner<F>(
        &mut self,
        operations: Vec<WriteOperation>,
        fail_after_operations: Option<usize>,
        validate: F,
    ) -> Result<WriteCommit, WriteError>
    where
        F: FnOnce(&Path) -> Result<(), String>,
    {
        if operations.is_empty() {
            return Err(WriteError::EmptyTransaction);
        }
        self.ensure_quarantine_is_empty()?;
        self.recover_pending_transactions()?;

        let transaction_id = transaction_id();
        let transaction_directory = self.transactions_directory().join(transaction_id.as_str());
        let staged_directory = transaction_directory.join("staged");
        let backup_directory = transaction_directory.join("backup");
        create_directory(&staged_directory)?;
        create_directory(&backup_directory)?;

        let result = self.prepare_manifest(
            &transaction_id,
            &transaction_directory,
            &staged_directory,
            operations,
        );
        let mut manifest = match result {
            Ok(manifest) => manifest,
            Err(error) => {
                let _ = fs::remove_dir_all(&transaction_directory);
                return Err(error);
            }
        };
        if let Err(error) = write_manifest(&transaction_directory, &manifest) {
            let _ = fs::remove_dir_all(&transaction_directory);
            return Err(error);
        }
        manifest.state = ManifestState::Applying;
        if let Err(error) = write_manifest(&transaction_directory, &manifest) {
            let _ = fs::remove_dir_all(&transaction_directory);
            return Err(error);
        }

        for (index, operation) in manifest.operations.iter().enumerate() {
            if fail_after_operations == Some(index) {
                let error = io_error(
                    "apply transaction",
                    &transaction_directory,
                    std::io::Error::other(format!("injected failure after {index} operation(s)")),
                );
                return self.fail_and_rollback(&transaction_directory, &manifest, error);
            }
            if let Err(error) = self.apply_operation(&transaction_directory, operation) {
                return self.fail_and_rollback(&transaction_directory, &manifest, error);
            }
        }

        if let Err(message) = validate(&self.root) {
            return self.fail_and_rollback(
                &transaction_directory,
                &manifest,
                WriteError::ValidationFailed(message),
            );
        }

        manifest.state = ManifestState::Committed;
        if let Err(error) = write_manifest(&transaction_directory, &manifest) {
            return self.fail_and_rollback(&transaction_directory, &manifest, error);
        }
        let written_paths = manifest
            .operations
            .iter()
            .map(|operation| operation.target.clone())
            .collect();
        let _ = fs::remove_dir_all(&transaction_directory);
        Ok(WriteCommit {
            transaction_id,
            written_paths,
        })
    }

    fn prepare_manifest(
        &self,
        transaction_id: &str,
        transaction_directory: &Path,
        staged_directory: &Path,
        operations: Vec<WriteOperation>,
    ) -> Result<TransactionManifest, WriteError> {
        let mut targets = HashSet::new();
        let mut manifest_operations = Vec::with_capacity(operations.len());
        for (index, operation) in operations.into_iter().enumerate() {
            let target = secure_target(&self.root, &operation.relative_path)?;
            let target_name = portable_relative_path(&operation.relative_path)?;
            if !targets.insert(target_name.clone()) {
                return Err(WriteError::DuplicateTarget(target_name));
            }

            let (expectation, original_sha256) = match operation.expectation {
                WriteExpectation::Create => {
                    if target.exists() {
                        return Err(WriteError::CreateConflict(target_name));
                    }
                    (ManifestExpectation::Create, None)
                }
                WriteExpectation::MatchSha256(expected) => {
                    let actual = if target.exists() {
                        Some(file_revision(&target)?.sha256)
                    } else {
                        None
                    };
                    if actual.as_deref() != Some(expected.as_str()) {
                        return Err(WriteError::RevisionConflict {
                            path: target_name,
                            expected,
                            actual,
                        });
                    }
                    (ManifestExpectation::MatchSha256, Some(expected))
                }
            };

            let staged_name = format!("staged/{index:04}");
            let backup_name = format!("backup/{index:04}");
            let staged_path = staged_directory.join(format!("{index:04}"));
            write_new_file(&staged_path, &operation.contents)?;
            manifest_operations.push(ManifestOperation {
                target: target_name,
                staged: staged_name,
                backup: backup_name,
                expectation,
                original_sha256,
                staged_sha256: hash_bytes(&operation.contents),
            });
        }
        sync_directory(staged_directory)?;
        sync_directory(transaction_directory)?;

        Ok(TransactionManifest {
            version: MANIFEST_VERSION,
            id: transaction_id.to_string(),
            state: ManifestState::Prepared,
            operations: manifest_operations,
        })
    }

    fn apply_operation(
        &self,
        transaction_directory: &Path,
        operation: &ManifestOperation,
    ) -> Result<(), WriteError> {
        let target = secure_target(&self.root, Path::new(&operation.target))?;
        let staged = transaction_asset(transaction_directory, &operation.staged)?;
        let backup = transaction_asset(transaction_directory, &operation.backup)?;
        let parent = target
            .parent()
            .ok_or_else(|| WriteError::UnsafePath(operation.target.clone()))?;
        create_directory(parent)?;
        let target = secure_target(&self.root, Path::new(&operation.target))?;

        match operation.expectation {
            ManifestExpectation::Create => {
                if target.exists() {
                    return Err(WriteError::CreateConflict(operation.target.clone()));
                }
            }
            ManifestExpectation::MatchSha256 => {
                let Some(expected) = operation.original_sha256.as_ref() else {
                    return Err(WriteError::InvalidManifest {
                        path: transaction_directory
                            .join(MANIFEST_FILE)
                            .to_string_lossy()
                            .into_owned(),
                        message: format!("missing original hash for {}", operation.target),
                    });
                };
                if !target.exists() {
                    return Err(WriteError::RevisionConflict {
                        path: operation.target.clone(),
                        expected: expected.clone(),
                        actual: None,
                    });
                }
                fs::rename(&target, &backup)
                    .map_err(|source| io_error("back up original file", &target, source))?;
                sync_directory(parent)?;
                if let Some(backup_parent) = backup.parent() {
                    sync_directory(backup_parent)?;
                }
                let actual = file_revision(&backup)?.sha256;
                if &actual != expected {
                    fs::rename(&backup, &target)
                        .map_err(|source| io_error("restore changed file", &target, source))?;
                    return Err(WriteError::RevisionConflict {
                        path: operation.target.clone(),
                        expected: expected.clone(),
                        actual: Some(actual),
                    });
                }
            }
        }

        if let Err(source) = fs::hard_link(&staged, &target) {
            if backup.exists() && !target.exists() {
                fs::rename(&backup, &target)
                    .map_err(|restore| io_error("restore original file", &target, restore))?;
            }
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(WriteError::CreateConflict(operation.target.clone()));
            }
            return Err(io_error("install staged file", &target, source));
        }
        fs::remove_file(&staged)
            .map_err(|source| io_error("remove installed staging file", &staged, source))?;
        sync_directory(parent)?;
        if let Some(staged_parent) = staged.parent() {
            sync_directory(staged_parent)?;
        }
        Ok(())
    }

    fn fail_and_rollback(
        &self,
        transaction_directory: &Path,
        manifest: &TransactionManifest,
        original_error: WriteError,
    ) -> Result<WriteCommit, WriteError> {
        match self.rollback_transaction(transaction_directory, manifest) {
            Ok(()) => {
                let _ = fs::remove_dir_all(transaction_directory);
                Err(original_error)
            }
            Err(recovery_error) => {
                let _ = self.quarantine_transaction(transaction_directory, &manifest.id);
                Err(recovery_error)
            }
        }
    }

    fn recover_pending_transactions(&mut self) -> Result<(), WriteError> {
        let transactions_directory = self.transactions_directory();
        let entries = fs::read_dir(&transactions_directory).map_err(|source| {
            io_error("list pending transactions", &transactions_directory, source)
        })?;
        let mut transaction_directories = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| {
                io_error("read pending transaction", &transactions_directory, source)
            })?;
            let file_type = entry
                .file_type()
                .map_err(|source| io_error("inspect pending transaction", &entry.path(), source))?;
            if file_type.is_dir() {
                transaction_directories.push(entry.path());
            }
        }
        transaction_directories.sort();

        for directory in transaction_directories {
            let manifest_path = directory.join(MANIFEST_FILE);
            if !manifest_path.exists() {
                fs::remove_dir_all(&directory).map_err(|source| {
                    io_error("remove unprepared transaction", &directory, source)
                })?;
                continue;
            }
            let directory_id = directory.file_name().and_then(|name| name.to_str());
            let manifest = match read_manifest(&manifest_path) {
                Ok(manifest) => manifest,
                Err(error) => {
                    let id = directory_id.unwrap_or("unknown");
                    let _ = self.quarantine_transaction(&directory, id);
                    return Err(error);
                }
            };
            if manifest.version != MANIFEST_VERSION || directory_id != Some(manifest.id.as_str()) {
                let id = directory_id.unwrap_or("unknown").to_string();
                let _ = self.quarantine_transaction(&directory, &id);
                return Err(WriteError::InvalidManifest {
                    path: manifest_path.to_string_lossy().into_owned(),
                    message: "unsupported version or transaction id mismatch".to_string(),
                });
            }
            match manifest.state {
                ManifestState::Prepared | ManifestState::Committed => {
                    fs::remove_dir_all(&directory).map_err(|source| {
                        io_error("clean completed transaction", &directory, source)
                    })?;
                }
                ManifestState::Applying => {
                    if let Err(error) = self.rollback_transaction(&directory, &manifest) {
                        let _ = self.quarantine_transaction(&directory, &manifest.id);
                        return Err(error);
                    }
                    fs::remove_dir_all(&directory).map_err(|source| {
                        io_error("clean recovered transaction", &directory, source)
                    })?;
                }
            }
        }
        Ok(())
    }

    fn rollback_transaction(
        &self,
        transaction_directory: &Path,
        manifest: &TransactionManifest,
    ) -> Result<(), WriteError> {
        for operation in manifest.operations.iter().rev() {
            let target = secure_target(&self.root, Path::new(&operation.target))?;
            let backup = transaction_asset(transaction_directory, &operation.backup)?;
            match operation.expectation {
                ManifestExpectation::Create => {
                    if target.exists() {
                        self.ensure_staged_target(manifest, operation, &target)?;
                        fs::remove_file(&target).map_err(|source| {
                            io_error("roll back created file", &target, source)
                        })?;
                        if let Some(parent) = target.parent() {
                            sync_directory(parent)?;
                        }
                    }
                }
                ManifestExpectation::MatchSha256 => {
                    if !backup.exists() {
                        continue;
                    }
                    if target.exists() {
                        self.ensure_staged_target(manifest, operation, &target)?;
                        fs::remove_file(&target).map_err(|source| {
                            io_error("remove partially replaced file", &target, source)
                        })?;
                    }
                    if let Some(parent) = target.parent() {
                        create_directory(parent)?;
                    }
                    fs::rename(&backup, &target).map_err(|source| {
                        io_error("restore transaction backup", &target, source)
                    })?;
                    if let Some(parent) = target.parent() {
                        sync_directory(parent)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn ensure_staged_target(
        &self,
        manifest: &TransactionManifest,
        operation: &ManifestOperation,
        target: &Path,
    ) -> Result<(), WriteError> {
        let actual = file_revision(target)?.sha256;
        if actual == operation.staged_sha256 {
            return Ok(());
        }
        Err(WriteError::RecoveryConflict {
            transaction_id: manifest.id.clone(),
            path: operation.target.clone(),
        })
    }

    fn ensure_quarantine_is_empty(&self) -> Result<(), WriteError> {
        let quarantine = self.quarantine_directory();
        let has_entries = fs::read_dir(&quarantine)
            .map_err(|source| io_error("list quarantined transactions", &quarantine, source))?
            .next()
            .is_some();
        if has_entries {
            return Err(WriteError::QuarantinedTransactions(
                quarantine.to_string_lossy().into_owned(),
            ));
        }
        Ok(())
    }

    fn quarantine_transaction(
        &self,
        transaction_directory: &Path,
        transaction_id: &str,
    ) -> Result<(), WriteError> {
        let destination = self.quarantine_directory().join(transaction_id);
        fs::rename(transaction_directory, &destination)
            .map_err(|source| io_error("quarantine transaction", &destination, source))
    }

    fn transactions_directory(&self) -> PathBuf {
        self.root
            .join(CONTROL_DIRECTORY)
            .join(TRANSACTIONS_DIRECTORY)
    }

    fn quarantine_directory(&self) -> PathBuf {
        self.root.join(CONTROL_DIRECTORY).join(QUARANTINE_DIRECTORY)
    }
}

impl Drop for DataRootWriter {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock_file);
    }
}

fn canonical_data_root(root: &Path) -> Result<PathBuf, WriteError> {
    if !root.exists() {
        return Err(WriteError::MissingRoot(root.to_string_lossy().into_owned()));
    }
    if !root.is_dir() {
        return Err(WriteError::InvalidRoot(root.to_string_lossy().into_owned()));
    }
    root.canonicalize()
        .map_err(|source| io_error("canonicalize data root", root, source))
}

/// Reads a file revision without holding a write lock, suitable for edit-session conflict tokens.
pub fn read_file_revision(
    root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<Option<FileRevision>, WriteError> {
    let root = canonical_data_root(root.as_ref())?;
    let target = secure_target(&root, relative_path.as_ref())?;
    if !target.exists() {
        return Ok(None);
    }
    Ok(Some(file_revision(&target)?))
}

fn secure_target(root: &Path, relative_path: &Path) -> Result<PathBuf, WriteError> {
    validate_relative_path(relative_path)?;
    let target = root.join(relative_path);
    let mut cursor = root.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(component) = component else {
            return Err(WriteError::UnsafePath(
                relative_path.to_string_lossy().into_owned(),
            ));
        };
        cursor.push(component);
        if cursor.exists()
            && fs::symlink_metadata(&cursor)
                .map_err(|source| io_error("inspect write path", &cursor, source))?
                .file_type()
                .is_symlink()
        {
            return Err(WriteError::UnsafePath(
                relative_path.to_string_lossy().into_owned(),
            ));
        }
    }
    let mut existing = target.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| WriteError::UnsafePath(relative_path.to_string_lossy().into_owned()))?;
    }
    let canonical_existing = existing
        .canonicalize()
        .map_err(|source| io_error("canonicalize write path", existing, source))?;
    if !canonical_existing.starts_with(root) {
        return Err(WriteError::UnsafePath(
            relative_path.to_string_lossy().into_owned(),
        ));
    }
    if target.exists() {
        let canonical_target = target
            .canonicalize()
            .map_err(|source| io_error("canonicalize target file", &target, source))?;
        if !canonical_target.starts_with(root) {
            return Err(WriteError::UnsafePath(
                relative_path.to_string_lossy().into_owned(),
            ));
        }
    }
    Ok(target)
}

fn validate_relative_path(path: &Path) -> Result<(), WriteError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(WriteError::UnsafePath(path.to_string_lossy().into_owned()));
    }
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return Err(WriteError::UnsafePath(path.to_string_lossy().into_owned()));
    };
    if first == CONTROL_DIRECTORY {
        return Err(WriteError::UnsafePath(path.to_string_lossy().into_owned()));
    }
    if components.any(|component| !matches!(component, Component::Normal(_))) {
        return Err(WriteError::UnsafePath(path.to_string_lossy().into_owned()));
    }
    Ok(())
}

fn portable_relative_path(path: &Path) -> Result<String, WriteError> {
    validate_relative_path(path)?;
    path.to_str()
        .map(|value| value.replace(std::path::MAIN_SEPARATOR, "/"))
        .ok_or_else(|| WriteError::UnsafePath(path.to_string_lossy().into_owned()))
}

fn transaction_asset(transaction_directory: &Path, relative: &str) -> Result<PathBuf, WriteError> {
    let relative = Path::new(relative);
    validate_relative_path(relative)?;
    let path = transaction_directory.join(relative);
    if !path.starts_with(transaction_directory) {
        return Err(WriteError::UnsafePath(
            relative.to_string_lossy().into_owned(),
        ));
    }
    Ok(path)
}

fn create_directory(path: &Path) -> Result<(), WriteError> {
    fs::create_dir_all(path).map_err(|source| io_error("create directory", path, source))
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), WriteError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| io_error("create staged file", path, source))?;
    file.write_all(contents)
        .map_err(|source| io_error("write staged file", path, source))?;
    file.sync_all()
        .map_err(|source| io_error("sync staged file", path, source))
}

fn write_manifest(
    transaction_directory: &Path,
    manifest: &TransactionManifest,
) -> Result<(), WriteError> {
    let manifest_path = transaction_directory.join(MANIFEST_FILE);
    let temporary_path = transaction_directory.join("manifest.next");
    let mut contents =
        serde_json::to_vec_pretty(manifest).map_err(|error| WriteError::InvalidManifest {
            path: manifest_path.to_string_lossy().into_owned(),
            message: error.to_string(),
        })?;
    contents.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary_path)
        .map_err(|source| io_error("create transaction manifest", &temporary_path, source))?;
    file.write_all(&contents)
        .map_err(|source| io_error("write transaction manifest", &temporary_path, source))?;
    file.sync_all()
        .map_err(|source| io_error("sync transaction manifest", &temporary_path, source))?;
    fs::rename(&temporary_path, &manifest_path)
        .map_err(|source| io_error("publish transaction manifest", &manifest_path, source))?;
    sync_directory(transaction_directory)
}

fn read_manifest(path: &Path) -> Result<TransactionManifest, WriteError> {
    let contents =
        fs::read(path).map_err(|source| io_error("read transaction manifest", path, source))?;
    serde_json::from_slice(&contents).map_err(|error| WriteError::InvalidManifest {
        path: path.to_string_lossy().into_owned(),
        message: error.to_string(),
    })
}

fn file_revision(path: &Path) -> Result<FileRevision, WriteError> {
    let mut file =
        File::open(path).map_err(|source| io_error("open file for hashing", path, source))?;
    let mut bytes = 0_u64;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| io_error("hash file", path, source))?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok(FileRevision {
        sha256: hex_digest(hasher.finalize()),
        bytes,
    })
}

pub(crate) fn hash_bytes(contents: &[u8]) -> String {
    hex_digest(Sha256::digest(contents))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = digest.as_ref();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn sync_directory(path: &Path) -> Result<(), WriteError> {
    let directory =
        File::open(path).map_err(|source| io_error("open directory for sync", path, source))?;
    directory
        .sync_all()
        .map_err(|source| io_error("sync directory", path, source))
}

fn transaction_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = TRANSACTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{}-{timestamp}-{sequence}", std::process::id())
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> WriteError {
    WriteError::Io {
        operation,
        path: path.to_string_lossy().into_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, read, write};
    use tempfile::tempdir;

    #[test]
    fn only_one_writer_can_hold_the_data_root_lock() {
        let directory = tempdir().unwrap();
        let first = DataRootWriter::acquire(directory.path()).unwrap();

        let error = DataRootWriter::acquire(directory.path()).unwrap_err();

        assert!(matches!(error, WriteError::LockBusy(_)));
        drop(first);
        DataRootWriter::acquire(directory.path()).unwrap();
    }

    #[test]
    fn commits_create_and_revision_guarded_replace_together() {
        let directory = tempdir().unwrap();
        let existing_path = directory.path().join("work-items/SAFE-1/context.json");
        create_dir_all(existing_path.parent().unwrap()).unwrap();
        write(&existing_path, b"before").unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();
        let revision = writer
            .revision("work-items/SAFE-1/context.json")
            .unwrap()
            .unwrap();

        let commit = writer
            .commit(vec![
                WriteOperation::create("records/2026/07/14/CP-1.json", b"checkpoint"),
                WriteOperation::replace(
                    "work-items/SAFE-1/context.json",
                    revision.sha256,
                    b"after",
                ),
            ])
            .unwrap();

        assert_eq!(
            commit.written_paths,
            [
                "records/2026/07/14/CP-1.json",
                "work-items/SAFE-1/context.json"
            ]
        );
        assert_eq!(read(existing_path).unwrap(), b"after");
        assert_eq!(
            read(directory.path().join("records/2026/07/14/CP-1.json")).unwrap(),
            b"checkpoint"
        );
    }

    #[test]
    fn rejects_stale_revision_without_writing_any_target() {
        let directory = tempdir().unwrap();
        let existing_path = directory.path().join("work-items/SAFE-1/context.json");
        create_dir_all(existing_path.parent().unwrap()).unwrap();
        write(&existing_path, b"external change").unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();

        let error = writer
            .commit(vec![
                WriteOperation::create("records/new.json", b"new"),
                WriteOperation::replace(
                    "work-items/SAFE-1/context.json",
                    hash_bytes(b"stale"),
                    b"replacement",
                ),
            ])
            .unwrap_err();

        assert!(matches!(error, WriteError::RevisionConflict { .. }));
        assert_eq!(read(existing_path).unwrap(), b"external change");
        assert!(!directory.path().join("records/new.json").exists());
    }

    #[test]
    fn create_only_operation_never_overwrites_an_existing_file() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("records/existing.json");
        create_dir_all(target.parent().unwrap()).unwrap();
        write(&target, b"original").unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();

        let error = writer
            .commit(vec![WriteOperation::create(
                "records/existing.json",
                b"replacement",
            )])
            .unwrap_err();

        assert!(matches!(error, WriteError::CreateConflict(_)));
        assert_eq!(read(target).unwrap(), b"original");
    }

    #[test]
    fn rolls_back_all_targets_after_a_partial_apply_failure() {
        let directory = tempdir().unwrap();
        let existing_path = directory.path().join("work-items/SAFE-1/context.json");
        create_dir_all(existing_path.parent().unwrap()).unwrap();
        write(&existing_path, b"before").unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();
        let revision = writer
            .revision("work-items/SAFE-1/context.json")
            .unwrap()
            .unwrap();

        let error = writer
            .commit_inner(
                vec![
                    WriteOperation::replace(
                        "work-items/SAFE-1/context.json",
                        revision.sha256,
                        b"after",
                    ),
                    WriteOperation::create("records/new.json", b"new"),
                ],
                Some(1),
                |_| Ok(()),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            WriteError::Io {
                operation: "apply transaction",
                ..
            }
        ));
        assert_eq!(read(existing_path).unwrap(), b"before");
        assert!(!directory.path().join("records/new.json").exists());
    }

    #[test]
    fn post_write_validation_failure_restores_every_target() {
        let directory = tempdir().unwrap();
        let existing_path = directory.path().join("work-items/SAFE-1/context.json");
        create_dir_all(existing_path.parent().unwrap()).unwrap();
        write(&existing_path, b"before").unwrap();
        let revision = read_file_revision(
            directory.path(),
            Path::new("work-items/SAFE-1/context.json"),
        )
        .unwrap()
        .unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();

        let error = writer
            .commit_validated(
                vec![
                    WriteOperation::replace(
                        "work-items/SAFE-1/context.json",
                        revision.sha256,
                        b"after",
                    ),
                    WriteOperation::create("records/new.json", b"new"),
                ],
                |_| Err("schema relationship mismatch".to_string()),
            )
            .unwrap_err();

        assert!(matches!(error, WriteError::ValidationFailed(_)));
        assert_eq!(read(existing_path).unwrap(), b"before");
        assert!(!directory.path().join("records/new.json").exists());
    }

    #[test]
    fn exposes_stable_sha256_revision_tokens_without_holding_a_write_lock() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("context.json");
        write(&target, b"abc").unwrap();

        let revision = read_file_revision(directory.path(), "context.json")
            .unwrap()
            .unwrap();

        assert_eq!(revision.bytes, 3);
        assert_eq!(
            revision.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn recovers_an_interrupted_applying_transaction_on_next_lock() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("work-items/SAFE-1/context.json");
        create_dir_all(target.parent().unwrap()).unwrap();
        write(&target, b"before").unwrap();
        let writer = DataRootWriter::acquire(directory.path()).unwrap();
        let transaction_id = "interrupted";
        let transaction_directory = writer.transactions_directory().join(transaction_id);
        create_dir_all(transaction_directory.join("staged")).unwrap();
        create_dir_all(transaction_directory.join("backup")).unwrap();
        let staged = transaction_directory.join("staged/0000");
        let backup = transaction_directory.join("backup/0000");
        write(&staged, b"after").unwrap();
        fs::rename(&target, &backup).unwrap();
        fs::hard_link(&staged, &target).unwrap();
        fs::remove_file(&staged).unwrap();
        let manifest = TransactionManifest {
            version: MANIFEST_VERSION,
            id: transaction_id.to_string(),
            state: ManifestState::Applying,
            operations: vec![ManifestOperation {
                target: "work-items/SAFE-1/context.json".to_string(),
                staged: "staged/0000".to_string(),
                backup: "backup/0000".to_string(),
                expectation: ManifestExpectation::MatchSha256,
                original_sha256: Some(hash_bytes(b"before")),
                staged_sha256: hash_bytes(b"after"),
            }],
        };
        write_manifest(&transaction_directory, &manifest).unwrap();
        drop(writer);

        DataRootWriter::acquire(directory.path()).unwrap();

        assert_eq!(read(target).unwrap(), b"before");
        assert!(!transaction_directory.exists());
    }

    #[test]
    fn keeps_committed_targets_when_cleaning_after_a_restart() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("records/committed.json");
        create_dir_all(target.parent().unwrap()).unwrap();
        write(&target, b"committed").unwrap();
        let writer = DataRootWriter::acquire(directory.path()).unwrap();
        let transaction_id = "committed";
        let transaction_directory = writer.transactions_directory().join(transaction_id);
        create_dir_all(transaction_directory.join("staged")).unwrap();
        create_dir_all(transaction_directory.join("backup")).unwrap();
        let manifest = TransactionManifest {
            version: MANIFEST_VERSION,
            id: transaction_id.to_string(),
            state: ManifestState::Committed,
            operations: vec![ManifestOperation {
                target: "records/committed.json".to_string(),
                staged: "staged/0000".to_string(),
                backup: "backup/0000".to_string(),
                expectation: ManifestExpectation::Create,
                original_sha256: None,
                staged_sha256: hash_bytes(b"committed"),
            }],
        };
        write_manifest(&transaction_directory, &manifest).unwrap();
        drop(writer);

        DataRootWriter::acquire(directory.path()).unwrap();

        assert_eq!(read(&target).unwrap(), b"committed");
        assert!(!transaction_directory.exists());
    }

    #[test]
    fn quarantines_recovery_when_target_was_changed_again() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("records/new.json");
        create_dir_all(target.parent().unwrap()).unwrap();
        let writer = DataRootWriter::acquire(directory.path()).unwrap();
        let transaction_id = "conflicted";
        let transaction_directory = writer.transactions_directory().join(transaction_id);
        create_dir_all(transaction_directory.join("staged")).unwrap();
        create_dir_all(transaction_directory.join("backup")).unwrap();
        write(&target, b"external").unwrap();
        let manifest = TransactionManifest {
            version: MANIFEST_VERSION,
            id: transaction_id.to_string(),
            state: ManifestState::Applying,
            operations: vec![ManifestOperation {
                target: "records/new.json".to_string(),
                staged: "staged/0000".to_string(),
                backup: "backup/0000".to_string(),
                expectation: ManifestExpectation::Create,
                original_sha256: None,
                staged_sha256: hash_bytes(b"new"),
            }],
        };
        write_manifest(&transaction_directory, &manifest).unwrap();
        drop(writer);

        let error = DataRootWriter::acquire(directory.path()).unwrap_err();

        assert!(matches!(error, WriteError::RecoveryConflict { .. }));
        assert_eq!(read(&target).unwrap(), b"external");
        assert!(
            directory
                .path()
                .join(".work-harvest/quarantine/conflicted")
                .exists()
        );
        assert!(matches!(
            DataRootWriter::acquire(directory.path()).unwrap_err(),
            WriteError::QuarantinedTransactions(_)
        ));
    }

    #[test]
    fn quarantines_a_corrupt_transaction_manifest() {
        let directory = tempdir().unwrap();
        let writer = DataRootWriter::acquire(directory.path()).unwrap();
        let transaction_directory = writer.transactions_directory().join("corrupt");
        create_dir_all(&transaction_directory).unwrap();
        write(transaction_directory.join(MANIFEST_FILE), b"{not json").unwrap();
        drop(writer);

        let error = DataRootWriter::acquire(directory.path()).unwrap_err();

        assert!(matches!(error, WriteError::InvalidManifest { .. }));
        assert!(
            directory
                .path()
                .join(".work-harvest/quarantine/corrupt")
                .exists()
        );
        assert!(matches!(
            DataRootWriter::acquire(directory.path()).unwrap_err(),
            WriteError::QuarantinedTransactions(_)
        ));
    }

    #[test]
    fn rejects_escaping_reserved_and_duplicate_paths() {
        let directory = tempdir().unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();

        assert!(matches!(
            writer.commit(vec![WriteOperation::create("../escape", b"bad")]),
            Err(WriteError::UnsafePath(_))
        ));
        assert!(matches!(
            writer.commit(vec![WriteOperation::create(
                ".work-harvest/manifest.json",
                b"bad"
            )]),
            Err(WriteError::UnsafePath(_))
        ));
        assert!(matches!(
            writer.commit(vec![
                WriteOperation::create("records/same.json", b"one"),
                WriteOperation::create("records/same.json", b"two"),
            ]),
            Err(WriteError::DuplicateTarget(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_targets_reached_through_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().unwrap();
        let outside = tempdir().unwrap();
        symlink(outside.path(), directory.path().join("linked")).unwrap();
        let mut writer = DataRootWriter::acquire(directory.path()).unwrap();

        let error = writer
            .commit(vec![WriteOperation::create("linked/escape.json", b"bad")])
            .unwrap_err();

        assert!(matches!(error, WriteError::UnsafePath(_)));
        assert!(!outside.path().join("escape.json").exists());
    }
}
