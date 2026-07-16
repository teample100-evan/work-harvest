mod always_on;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError},
};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use work_harvest_core::{
    CheckpointInput, CheckpointWriteError, CheckpointWritePreview, CheckpointWriteResult,
    DataRootIndex, DataRootSnapshot, DataRootUpdate, PerformanceNoteInput,
    PerformanceNoteSourceRevision, PerformanceNoteWriteError, PerformanceNoteWritePreview,
    PerformanceNoteWriteResult, WorkItemCreateInput, WorkItemDetail, WorkItemEditRevisions,
    WorkItemEditSnapshot, WorkItemUpdatePatch, WorkItemWriteError, WorkItemWritePreview,
    WorkItemWriteResult, WriteError, capture_checkpoint as capture_checkpoint_record,
    checkpoint_markdown_path, context_markdown_path,
    create_performance_note as create_performance_note_record, create_work_item as create_item,
    get_work_item_detail as get_detail, performance_note_markdown_path,
    preview_capture_checkpoint as preview_checkpoint_record,
    preview_create_work_item as preview_create_item,
    preview_performance_note as preview_performance_note_record,
    preview_update_work_item as preview_update_item, read_work_item_for_edit,
    update_work_item as update_item, work_item_directory,
};

const WATCH_QUIET_PERIOD: Duration = Duration::from_millis(350);
const WATCH_MAX_LATENCY: Duration = Duration::from_secs(1);

#[derive(Default)]
struct DesktopState {
    root: Mutex<Option<PathBuf>>,
    index: Mutex<Option<DataRootIndex>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    watcher_generation: AtomicU64,
}

#[derive(Clone, Serialize)]
struct DataRootChange {
    #[serde(flatten)]
    update: DataRootUpdate,
    paths: Vec<String>,
    event_count: usize,
}

#[derive(Clone, Serialize)]
struct BuildInfo {
    version: &'static str,
    commit: &'static str,
    dirty: bool,
    built_at_unix: u64,
    profile: &'static str,
}

#[tauri::command]
fn get_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        commit: option_env!("WORK_HARVEST_BUILD_COMMIT").unwrap_or("unknown"),
        dirty: option_env!("WORK_HARVEST_BUILD_DIRTY") == Some("true"),
        built_at_unix: option_env!("WORK_HARVEST_BUILT_AT_UNIX")
            .and_then(|value| value.parse().ok())
            .unwrap_or_default(),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DesktopWriteErrorKind {
    RootRequired,
    NotFound,
    Validation,
    CreateConflict,
    RevisionConflict,
    LockBusy,
    WriteFailed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DesktopWriteError {
    kind: DesktopWriteErrorKind,
    message: String,
}

impl DesktopWriteError {
    fn root_required(message: String) -> Self {
        Self {
            kind: DesktopWriteErrorKind::RootRequired,
            message,
        }
    }
}

fn write_error_kind(error: &WriteError) -> DesktopWriteErrorKind {
    match error {
        WriteError::CreateConflict(_) => DesktopWriteErrorKind::CreateConflict,
        WriteError::RevisionConflict { .. } => DesktopWriteErrorKind::RevisionConflict,
        WriteError::LockBusy(_) => DesktopWriteErrorKind::LockBusy,
        _ => DesktopWriteErrorKind::WriteFailed,
    }
}

fn work_item_error_kind(error: &WorkItemWriteError) -> DesktopWriteErrorKind {
    match error {
        WorkItemWriteError::WorkItemNotFound(_) => DesktopWriteErrorKind::NotFound,
        WorkItemWriteError::InvalidInput(_)
        | WorkItemWriteError::Validation { .. }
        | WorkItemWriteError::Inconsistent(_) => DesktopWriteErrorKind::Validation,
        WorkItemWriteError::Write(error) => write_error_kind(error),
        WorkItemWriteError::Read { .. }
        | WorkItemWriteError::Parse { .. }
        | WorkItemWriteError::Serialize { .. } => DesktopWriteErrorKind::WriteFailed,
    }
}

fn desktop_write_error(error: WorkItemWriteError) -> DesktopWriteError {
    let kind = work_item_error_kind(&error);
    DesktopWriteError {
        kind,
        message: error.to_string(),
    }
}

fn checkpoint_error_kind(error: &CheckpointWriteError) -> DesktopWriteErrorKind {
    match error {
        CheckpointWriteError::InvalidInput(_)
        | CheckpointWriteError::Validation { .. }
        | CheckpointWriteError::Inconsistent(_) => DesktopWriteErrorKind::Validation,
        CheckpointWriteError::WorkItem(error) => work_item_error_kind(error),
        CheckpointWriteError::Write(error) => write_error_kind(error),
        CheckpointWriteError::Inspect(_)
        | CheckpointWriteError::Read { .. }
        | CheckpointWriteError::Parse { .. }
        | CheckpointWriteError::Serialize { .. } => DesktopWriteErrorKind::WriteFailed,
    }
}

fn desktop_checkpoint_error(error: CheckpointWriteError) -> DesktopWriteError {
    let kind = checkpoint_error_kind(&error);
    DesktopWriteError {
        kind,
        message: error.to_string(),
    }
}

fn desktop_performance_note_error(error: PerformanceNoteWriteError) -> DesktopWriteError {
    let kind = match &error {
        PerformanceNoteWriteError::InvalidInput(_) | PerformanceNoteWriteError::Inconsistent(_) => {
            DesktopWriteErrorKind::Validation
        }
        PerformanceNoteWriteError::WorkItem(error) => work_item_error_kind(error),
        PerformanceNoteWriteError::Checkpoint(error) => checkpoint_error_kind(error),
        PerformanceNoteWriteError::Write(error) => write_error_kind(error),
        PerformanceNoteWriteError::Read { .. }
        | PerformanceNoteWriteError::Parse { .. }
        | PerformanceNoteWriteError::Scan(_) => DesktopWriteErrorKind::WriteFailed,
    };
    DesktopWriteError {
        kind,
        message: error.to_string(),
    }
}

fn selected_write_root(state: &State<'_, DesktopState>) -> Result<PathBuf, DesktopWriteError> {
    selected_root(state).map_err(DesktopWriteError::root_required)
}

#[derive(Debug, Default)]
struct PathBatch {
    paths: BTreeSet<PathBuf>,
    event_count: usize,
}

impl PathBatch {
    fn push(&mut self, paths: Vec<PathBuf>) {
        self.event_count += 1;
        self.paths.extend(paths);
    }
}

fn lock_error(label: &str) -> String {
    format!("Could not lock desktop {label} state")
}

fn selected_root(state: &State<'_, DesktopState>) -> Result<PathBuf, String> {
    state
        .root
        .lock()
        .map_err(|_| lock_error("root"))?
        .clone()
        .ok_or_else(|| "Choose a Work Harvest data root first".to_string())
}

fn apply_watch_batch(app: &AppHandle, generation: u64, batch: PathBatch) {
    let state = app.state::<DesktopState>();
    if state.watcher_generation.load(Ordering::Acquire) != generation {
        return;
    }
    let paths = batch.paths.into_iter().collect::<Vec<_>>();
    let update = {
        let mut index = match state.index.lock() {
            Ok(index) => index,
            Err(_) => {
                let _ = app.emit("data-root-watch-error", lock_error("index"));
                return;
            }
        };
        if state.watcher_generation.load(Ordering::Acquire) != generation {
            return;
        }
        let Some(index) = index.as_mut() else {
            return;
        };
        match index.refresh_paths(&paths) {
            Ok(update) => update,
            Err(error) => {
                let _ = app.emit("data-root-watch-error", error.to_string());
                return;
            }
        }
    };
    if !update.applied {
        return;
    }
    if let Err(error) = always_on::update_menu(app, &update.snapshot) {
        let _ = app.emit("always-on-error", error);
    }
    let payload = DataRootChange {
        update,
        paths: paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        event_count: batch.event_count,
    };
    let _ = app.emit("data-root-updated", payload);
}

fn collect_path_batch(
    receiver: &Receiver<Vec<PathBuf>>,
    first_paths: Vec<PathBuf>,
) -> (PathBatch, bool) {
    let started_at = Instant::now();
    let mut batch = PathBatch::default();
    batch.push(first_paths);
    let disconnected = loop {
        let remaining = WATCH_MAX_LATENCY.saturating_sub(started_at.elapsed());
        if remaining.is_zero() {
            break false;
        }
        match receiver.recv_timeout(WATCH_QUIET_PERIOD.min(remaining)) {
            Ok(paths) => batch.push(paths),
            Err(RecvTimeoutError::Timeout) => break false,
            Err(RecvTimeoutError::Disconnected) => break true,
        }
    };
    (batch, disconnected)
}

fn run_watch_loop(app: AppHandle, generation: u64, receiver: Receiver<Vec<PathBuf>>) {
    while let Ok(paths) = receiver.recv() {
        let (batch, disconnected) = collect_path_batch(&receiver, paths);
        apply_watch_batch(&app, generation, batch);
        if disconnected {
            break;
        }
    }
}

fn start_watcher(
    app: &AppHandle,
    root: &Path,
    generation: u64,
) -> Result<RecommendedWatcher, String> {
    let error_app = app.clone();
    let (sender, receiver) = mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                let _ = sender.send(event.paths);
            }
            Err(error) => {
                let _ = error_app.emit("data-root-watch-error", error.to_string());
            }
        })
        .map_err(|error| format!("Could not create data root watcher: {error}"))?;

    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|error| format!("Could not watch data root: {error}"))?;
    let worker_app = app.clone();
    thread::Builder::new()
        .name("work-harvest-indexer".to_string())
        .spawn(move || run_watch_loop(worker_app, generation, receiver))
        .map_err(|error| format!("Could not start data root indexer: {error}"))?;
    Ok(watcher)
}

#[tauri::command]
fn set_data_root(
    app: AppHandle,
    state: State<'_, DesktopState>,
    root: String,
) -> Result<DataRootSnapshot, String> {
    let canonical_root = PathBuf::from(root)
        .canonicalize()
        .map_err(|error| format!("Could not open data root: {error}"))?;
    if !canonical_root.is_dir() {
        return Err("The selected data root is not a directory".to_string());
    }

    let index = DataRootIndex::build(&canonical_root).map_err(|error| error.to_string())?;
    let snapshot = index.snapshot().clone();
    let generation = state.watcher_generation.load(Ordering::Relaxed) + 1;
    let watcher = start_watcher(&app, &canonical_root, generation)?;

    let mut selected_root = state.root.lock().map_err(|_| lock_error("root"))?;
    let mut selected_index = state.index.lock().map_err(|_| lock_error("index"))?;
    let mut selected_watcher = state.watcher.lock().map_err(|_| lock_error("watcher"))?;
    *selected_root = Some(canonical_root);
    *selected_index = Some(index);
    *selected_watcher = Some(watcher);
    state
        .watcher_generation
        .store(generation, Ordering::Release);

    if let Err(error) = always_on::update_menu(&app, &snapshot) {
        let _ = app.emit("always-on-error", error);
    }
    Ok(snapshot)
}

#[tauri::command]
fn inspect_data_root(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<DataRootUpdate, String> {
    selected_root(&state)?;
    let update = state
        .index
        .lock()
        .map_err(|_| lock_error("index"))?
        .as_mut()
        .ok_or_else(|| "Choose a Work Harvest data root first".to_string())?
        .refresh_all()
        .map_err(|error| error.to_string())?;
    if let Err(error) = always_on::update_menu(&app, &update.snapshot) {
        let _ = app.emit("always-on-error", error);
    }
    Ok(update)
}

#[tauri::command]
fn get_work_item_detail(
    state: State<'_, DesktopState>,
    work_item_id: String,
) -> Result<WorkItemDetail, String> {
    let root = selected_root(&state)?;
    get_detail(root, &work_item_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_work_item_edit_snapshot(
    state: State<'_, DesktopState>,
    work_item_id: String,
) -> Result<WorkItemEditSnapshot, DesktopWriteError> {
    read_work_item_for_edit(selected_write_root(&state)?, &work_item_id)
        .map_err(desktop_write_error)
}

#[tauri::command]
fn preview_create_work_item(
    state: State<'_, DesktopState>,
    input: WorkItemCreateInput,
    now: String,
) -> Result<WorkItemWritePreview, DesktopWriteError> {
    preview_create_item(selected_write_root(&state)?, input, &now).map_err(desktop_write_error)
}

#[tauri::command]
fn preview_update_work_item(
    state: State<'_, DesktopState>,
    work_item_id: String,
    expected: WorkItemEditRevisions,
    patch: WorkItemUpdatePatch,
    now: String,
) -> Result<WorkItemWritePreview, DesktopWriteError> {
    preview_update_item(
        selected_write_root(&state)?,
        &work_item_id,
        expected,
        patch,
        &now,
    )
    .map_err(desktop_write_error)
}

#[tauri::command]
fn create_work_item(
    state: State<'_, DesktopState>,
    input: WorkItemCreateInput,
    now: String,
) -> Result<WorkItemWriteResult, DesktopWriteError> {
    create_item(selected_write_root(&state)?, input, &now).map_err(desktop_write_error)
}

#[tauri::command]
fn update_work_item(
    state: State<'_, DesktopState>,
    work_item_id: String,
    expected: WorkItemEditRevisions,
    patch: WorkItemUpdatePatch,
    now: String,
) -> Result<WorkItemWriteResult, DesktopWriteError> {
    update_item(
        selected_write_root(&state)?,
        &work_item_id,
        expected,
        patch,
        &now,
    )
    .map_err(desktop_write_error)
}

#[tauri::command]
fn preview_capture_checkpoint(
    state: State<'_, DesktopState>,
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: String,
) -> Result<CheckpointWritePreview, DesktopWriteError> {
    preview_checkpoint_record(selected_write_root(&state)?, input, expected, &now)
        .map_err(desktop_checkpoint_error)
}

#[tauri::command]
fn capture_checkpoint(
    state: State<'_, DesktopState>,
    input: CheckpointInput,
    expected: WorkItemEditRevisions,
    now: String,
) -> Result<CheckpointWriteResult, DesktopWriteError> {
    capture_checkpoint_record(selected_write_root(&state)?, input, expected, &now)
        .map_err(desktop_checkpoint_error)
}

#[tauri::command]
fn preview_performance_note(
    state: State<'_, DesktopState>,
    input: PerformanceNoteInput,
    generated_at: String,
) -> Result<PerformanceNoteWritePreview, DesktopWriteError> {
    preview_performance_note_record(selected_write_root(&state)?, input, &generated_at)
        .map_err(desktop_performance_note_error)
}

#[tauri::command]
fn create_performance_note(
    state: State<'_, DesktopState>,
    input: PerformanceNoteInput,
    expected: Vec<PerformanceNoteSourceRevision>,
    generated_at: String,
) -> Result<PerformanceNoteWriteResult, DesktopWriteError> {
    create_performance_note_record(selected_write_root(&state)?, input, expected, &generated_at)
        .map_err(desktop_performance_note_error)
}

#[tauri::command]
fn reveal_work_item(
    app: AppHandle,
    state: State<'_, DesktopState>,
    work_item_id: String,
) -> Result<(), String> {
    let path = work_item_directory(selected_root(&state)?, &work_item_id)
        .map_err(|error| error.to_string())?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| format!("Could not reveal work item in Finder: {error}"))
}

#[tauri::command]
fn open_context_markdown(
    app: AppHandle,
    state: State<'_, DesktopState>,
    work_item_id: String,
) -> Result<(), String> {
    let path = context_markdown_path(selected_root(&state)?, &work_item_id)
        .map_err(|error| error.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|error| format!("Could not open context Markdown: {error}"))
}

#[tauri::command]
fn open_checkpoint_markdown(
    app: AppHandle,
    state: State<'_, DesktopState>,
    checkpoint_id: String,
) -> Result<(), String> {
    let path = checkpoint_markdown_path(selected_root(&state)?, &checkpoint_id)
        .map_err(|error| error.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|error| format!("Could not open checkpoint Markdown: {error}"))
}

#[tauri::command]
fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    let is_web_url = url.starts_with("https://") || url.starts_with("http://");
    if !is_web_url {
        return Err("Only HTTP(S) URLs can be opened externally".to_string());
    }

    app.opener()
        .open_url(url, None::<String>)
        .map_err(|error| format!("Could not open URL in browser: {error}"))
}

#[tauri::command]
fn open_performance_note_markdown(
    app: AppHandle,
    state: State<'_, DesktopState>,
    report: String,
) -> Result<(), String> {
    let path = performance_note_markdown_path(selected_root(&state)?, &report)
        .map_err(|error| error.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|error| format!("Could not open performance note Markdown: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(always_on::window_state_flags())
                .build(),
        )
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![
            get_build_info,
            set_data_root,
            inspect_data_root,
            get_work_item_detail,
            get_work_item_edit_snapshot,
            preview_create_work_item,
            preview_update_work_item,
            create_work_item,
            update_work_item,
            preview_capture_checkpoint,
            capture_checkpoint,
            preview_performance_note,
            create_performance_note,
            reveal_work_item,
            open_context_markdown,
            open_checkpoint_markdown,
            open_external_url,
            open_performance_note_markdown
        ])
        .setup(|app| {
            always_on::install(app)?;
            let window = app
                .get_webview_window("main")
                .ok_or("main window was not created")?;
            window.set_title("Work Harvest")?;
            Ok(())
        })
        .on_window_event(always_on::handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running Work Harvest desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use std::sync::mpsc::TryRecvError;
    use tempfile::Builder;

    #[test]
    fn write_conflicts_are_structured_for_the_editor() {
        let error = desktop_write_error(WorkItemWriteError::Write(WriteError::RevisionConflict {
            path: "work-items/AUTH-142/context.md".to_string(),
            expected: "old".to_string(),
            actual: Some("new".to_string()),
        }));

        assert_eq!(error.kind, DesktopWriteErrorKind::RevisionConflict);
        assert!(error.message.contains("context.md"));

        let checkpoint_error = desktop_checkpoint_error(CheckpointWriteError::WorkItem(
            WorkItemWriteError::Write(WriteError::RevisionConflict {
                path: "work-items/AUTH-142/context.json".to_string(),
                expected: "old".to_string(),
                actual: Some("new".to_string()),
            }),
        ));
        assert_eq!(
            checkpoint_error.kind,
            DesktopWriteErrorKind::RevisionConflict
        );

        let performance_error = desktop_performance_note_error(PerformanceNoteWriteError::Write(
            WriteError::RevisionConflict {
                path: "records/2026/07/14/CP-001.json".to_string(),
                expected: "old".to_string(),
                actual: Some("new".to_string()),
            },
        ));
        assert_eq!(
            performance_error.kind,
            DesktopWriteErrorKind::RevisionConflict
        );
    }

    #[test]
    fn event_flood_collapses_to_unique_paths() {
        let mut batch = PathBatch::default();
        for event in 0..10_000 {
            batch.push(vec![PathBuf::from(format!(
                "/tmp/work-harvest/event-{}.json",
                event % 32
            ))]);
        }

        assert_eq!(batch.event_count, 10_000);
        assert_eq!(batch.paths.len(), 32);
    }

    #[test]
    #[ignore = "set WORK_HARVEST_SOAK_SECONDS=86400 for the daily watcher soak"]
    fn watcher_soak_converges_to_the_full_scan() {
        let seconds = std::env::var("WORK_HARVEST_SOAK_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60);
        let directory = Builder::new()
            .prefix("work-harvest-soak-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let work_item_directory = directory.path().join("work-items/AUTH-142");
        let record_directory = directory.path().join("records/2026/07/13");
        create_dir_all(&work_item_directory).unwrap();
        create_dir_all(&record_directory).unwrap();
        write(
            work_item_directory.join("work-item.json"),
            include_str!("../../../../examples/work-items/AUTH-142/work-item.json"),
        )
        .unwrap();
        let context_path = work_item_directory.join("context.json");
        write(
            &context_path,
            include_str!("../../../../examples/work-items/AUTH-142/context.json"),
        )
        .unwrap();
        write(work_item_directory.join("context.md"), "# Context\n").unwrap();
        write(
            record_directory.join("CP-20260713-001.json"),
            include_str!("../../../../examples/records/2026/07/13/CP-20260713-001.json"),
        )
        .unwrap();
        write(
            record_directory.join("CP-20260713-001.md"),
            "# Checkpoint\n",
        )
        .unwrap();

        let mut index = DataRootIndex::build(directory.path()).unwrap();
        let (event_sender, event_receiver) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                if let Ok(event) = result {
                    let _ = event_sender.send(event.paths);
                }
            })
            .unwrap();
        watcher
            .watch(directory.path(), RecursiveMode::Recursive)
            .unwrap();
        // FSEvents installs its stream asynchronously; let the watch become active
        // before the writer starts so the harness measures steady-state behavior.
        thread::sleep(Duration::from_secs(1));

        let writer_path = context_path.clone();
        let (done_sender, done_receiver) = mpsc::channel();
        let writer = thread::spawn(move || {
            let started_at = Instant::now();
            let mut iteration = 0_u64;
            while started_at.elapsed() < Duration::from_secs(seconds) {
                let mut context: serde_json::Value = serde_json::from_str(include_str!(
                    "../../../../examples/work-items/AUTH-142/context.json"
                ))
                .unwrap();
                context["current_state"] =
                    serde_json::Value::String(format!("watcher soak iteration {iteration}"));
                write(&writer_path, serde_json::to_vec(&context).unwrap()).unwrap();
                iteration += 1;
                thread::sleep(Duration::from_millis(25));
            }
            let _ = done_sender.send(iteration);
        });

        let mut batches = 0_usize;
        let mut raw_events = 0_usize;
        let iterations = loop {
            match event_receiver.recv_timeout(Duration::from_secs(2)) {
                Ok(paths) => {
                    let (batch, _) = collect_path_batch(&event_receiver, paths);
                    raw_events += batch.event_count;
                    batches += 1;
                    let paths = batch.paths.into_iter().collect::<Vec<_>>();
                    index.refresh_paths(&paths).unwrap();
                }
                Err(_) => match done_receiver.try_recv() {
                    Ok(iterations) => break iterations,
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => panic!("soak writer disconnected"),
                },
            }
            if let Ok(iterations) = done_receiver.try_recv() {
                thread::sleep(WATCH_QUIET_PERIOD);
                while let Ok(paths) = event_receiver.try_recv() {
                    let (batch, _) = collect_path_batch(&event_receiver, paths);
                    raw_events += batch.event_count;
                    batches += 1;
                    let paths = batch.paths.into_iter().collect::<Vec<_>>();
                    index.refresh_paths(&paths).unwrap();
                }
                break iterations;
            }
        };
        writer.join().unwrap();

        assert!(iterations > 0);
        assert!(raw_events >= batches);
        assert!(batches > 0);
        assert_eq!(
            index.snapshot(),
            &work_harvest_core::inspect_data_root(directory.path()).unwrap()
        );
    }
}
