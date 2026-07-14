mod always_on;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use work_harvest_core::{
    DataRootSnapshot, WorkItemDetail, checkpoint_markdown_path, context_markdown_path,
    get_work_item_detail as get_detail, inspect_data_root as inspect_root, work_item_directory,
};

#[derive(Default)]
struct DesktopState {
    root: Mutex<Option<PathBuf>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

#[derive(Clone, Serialize)]
struct DataRootChange {
    paths: Vec<String>,
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

fn start_watcher(app: &AppHandle, root: &Path) -> Result<RecommendedWatcher, String> {
    let app = app.clone();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                let payload = DataRootChange {
                    paths: event
                        .paths
                        .into_iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect(),
                };
                let _ = app.emit("data-root-changed", payload);
            }
            Err(error) => {
                let _ = app.emit("data-root-watch-error", error.to_string());
            }
        })
        .map_err(|error| format!("Could not create data root watcher: {error}"))?;

    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|error| format!("Could not watch data root: {error}"))?;
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

    let snapshot = inspect_root(&canonical_root).map_err(|error| error.to_string())?;
    let watcher = start_watcher(&app, &canonical_root)?;

    *state.root.lock().map_err(|_| lock_error("root"))? = Some(canonical_root);
    *state.watcher.lock().map_err(|_| lock_error("watcher"))? = Some(watcher);

    if let Err(error) = always_on::update_menu(&app, &snapshot) {
        let _ = app.emit("always-on-error", error);
    }
    Ok(snapshot)
}

#[tauri::command]
fn inspect_data_root(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<DataRootSnapshot, String> {
    let root = selected_root(&state)?;
    let snapshot = inspect_root(root).map_err(|error| error.to_string())?;
    if let Err(error) = always_on::update_menu(&app, &snapshot) {
        let _ = app.emit("always-on-error", error);
    }
    Ok(snapshot)
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
            set_data_root,
            inspect_data_root,
            get_work_item_detail,
            reveal_work_item,
            open_context_markdown,
            open_checkpoint_markdown
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
